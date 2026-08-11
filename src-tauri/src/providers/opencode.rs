use crate::{
    reliability::{self, SourceKind, SyncSourceConfig, UsageProvenance},
    UsageEvent,
};
use chrono::{TimeZone, Utc};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LocalSyncOutcome {
    pub imported: usize,
    pub scanned_files: usize,
    pub source_kind: String,
    pub source_path: String,
    pub detail: String,
}

fn candidates(custom: &str) -> Vec<PathBuf> {
    let mut rows = Vec::new();
    if !custom.trim().is_empty() {
        rows.push(PathBuf::from(custom.trim()));
    }
    if let Ok(value) = std::env::var("XDG_DATA_HOME") {
        rows.push(PathBuf::from(value).join("opencode"));
    }
    if let Some(home) = dirs::home_dir() {
        rows.push(home.join(".local").join("share").join("opencode"));
    }
    if let Some(roaming) = dirs::data_dir() {
        rows.push(roaming.join("opencode"));
    }
    if let Some(local) = dirs::data_local_dir() {
        rows.push(local.join("opencode"));
    }
    rows
}

fn sqlite_candidate(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if matches!(
            extension.to_ascii_lowercase().as_str(),
            "db" | "sqlite" | "sqlite3"
        ) {
            return Some(path.to_path_buf());
        }
        return None;
    }
    for name in ["opencode.db", "opencode.sqlite", "opencode.sqlite3"] {
        let candidate = path.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn json_candidate(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if matches!(extension.to_ascii_lowercase().as_str(), "json" | "jsonl") {
            return Some(path.to_path_buf());
        }
        return None;
    }
    for candidate in [path.join("storage"), path.to_path_buf()] {
        if candidate.join("message").is_dir() || candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

fn table_columns(db: &Connection, table: &str) -> Result<Vec<String>, String> {
    let mut statement = db
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| error.to_string())?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(columns)
}

fn has_columns(actual: &[String], required: &[&str]) -> bool {
    required
        .iter()
        .all(|required| actual.iter().any(|column| column == required))
}

fn model_name(raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("id")
                .or_else(|| value.get("modelID"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "OpenCode".into())
}

fn sync_sqlite(
    target: &Connection,
    path: &Path,
    source: &SyncSourceConfig,
    force: bool,
) -> Result<LocalSyncOutcome, String> {
    let external = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("无法只读打开 OpenCode SQLite：{error}"))?;
    external
        .busy_timeout(std::time::Duration::from_millis(800))
        .map_err(|error| error.to_string())?;
    let columns = table_columns(&external, "session")?;
    let required = [
        "id",
        "time_updated",
        "model",
        "cost",
        "tokens_input",
        "tokens_output",
        "tokens_reasoning",
        "tokens_cache_read",
        "tokens_cache_write",
    ];
    if !has_columns(&columns, &required) {
        return Err(format!(
            "当前 OpenCode 数据库版本暂未适配：session 表缺少用量字段（已发现 {} 个字段）",
            columns.len()
        ));
    }
    let cursor = if force {
        0
    } else {
        target
            .query_row(
                "SELECT cursor_json FROM sync_cursors WHERE source_id=?1",
                [&source.id],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|value| value.get("sqlite_time_updated").and_then(Value::as_i64))
            .unwrap_or(0)
    };
    let mut statement = external
        .prepare(
            "SELECT id,time_updated,model,cost,tokens_input,tokens_output,tokens_reasoning,tokens_cache_read,tokens_cache_write
             FROM session WHERE time_updated>?1 AND (tokens_input>0 OR tokens_output>0 OR tokens_reasoning>0 OR tokens_cache_read>0 OR tokens_cache_write>0)",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([cursor], |row| {
            let id: String = row.get(0)?;
            let time_updated: i64 = row.get(1)?;
            let model: Option<String> = row.get(2)?;
            let cost: f64 = row.get::<_, Option<f64>>(3)?.unwrap_or(0.0);
            let direct_input = row.get::<_, i64>(4)?.max(0) as u64;
            let direct_output = row.get::<_, i64>(5)?.max(0) as u64;
            let reasoning = row.get::<_, i64>(6)?.max(0) as u64;
            let cache_read = row.get::<_, i64>(7)?.max(0) as u64;
            let cache_write = row.get::<_, i64>(8)?.max(0) as u64;
            let at = Utc
                .timestamp_millis_opt(time_updated)
                .single()
                .unwrap_or_else(Utc::now);
            Ok((
                id,
                UsageEvent {
                    id: String::new(),
                    provider: "OpenCode Go".into(),
                    model: model
                        .as_deref()
                        .map(model_name)
                        .unwrap_or_else(|| "OpenCode".into()),
                    at,
                    input: direct_input + cache_read + cache_write,
                    output: direct_output + reasoning,
                    cached: cache_read,
                    cost: cost * 7.2,
                    task: "其他".into(),
                },
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut imported = 0usize;
    let mut newest = cursor;
    for row in rows {
        let (session_id, mut event) = row.map_err(|error| error.to_string())?;
        newest = newest.max(event.at.timestamp_millis());
        event.id = format!("opencode-sqlite:session:{session_id}");
        let mut provenance = UsageProvenance::observed(&source.id, SourceKind::LocalSqlite);
        provenance.source_ref = Some(session_id);
        provenance.price_version = "opencode-session-reported".into();
        if reliability::upsert_usage(target, &event, &provenance)
            .map_err(|error| error.to_string())?
        {
            imported += 1;
        }
    }
    target
        .execute(
            "INSERT INTO sync_cursors(source_id,cursor_json,last_success_at,last_error,updated_at)
             VALUES(?1,?2,?3,NULL,?3)
             ON CONFLICT(source_id) DO UPDATE SET cursor_json=excluded.cursor_json,
             last_success_at=excluded.last_success_at,last_error=NULL,updated_at=excluded.updated_at",
            rusqlite::params![
                source.id,
                serde_json::json!({"sqlite_time_updated":newest}).to_string(),
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(LocalSyncOutcome {
        imported,
        scanned_files: 1,
        source_kind: SourceKind::LocalSqlite.as_str().into(),
        source_path: path.to_string_lossy().into_owned(),
        detail: format!(
            "已从 OpenCode 官方 session 汇总表同步 {imported} 条记录；未读取提示词和回复正文"
        ),
    })
}

pub fn sync(
    target: &Connection,
    source: &SyncSourceConfig,
    force: bool,
) -> Result<LocalSyncOutcome, String> {
    let mode = source.mode.as_str();
    let roots = candidates(&source.path);
    if mode != "json" {
        if let Some(path) = roots.iter().find_map(|root| sqlite_candidate(root)) {
            return sync_sqlite(target, &path, source, force);
        }
        if mode == "sqlite" {
            return Err("未发现 OpenCode SQLite 数据库；请选择 opencode.db 或改用自动模式".into());
        }
    }
    let json_path = roots
        .iter()
        .find_map(|root| json_candidate(root))
        .ok_or_else(|| "未发现 OpenCode SQLite 或 JSON/JSONL 本地记录".to_string())?;
    crate::sync_opencode_json_into(
        target,
        Some(json_path.to_string_lossy().into_owned()),
        30,
        &source.id,
        force,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reliability;

    #[test]
    fn reads_official_session_totals_without_message_body() {
        let temp = std::env::temp_dir().join(format!(
            "tm-opencode-{}.db",
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let external = Connection::open(&temp).unwrap();
        external.execute_batch("CREATE TABLE session(id TEXT PRIMARY KEY,time_updated INTEGER,model TEXT,cost REAL,tokens_input INTEGER,tokens_output INTEGER,tokens_reasoning INTEGER,tokens_cache_read INTEGER,tokens_cache_write INTEGER); INSERT INTO session VALUES('ses_1',1786400000000,'{\"id\":\"gpt-5\",\"providerID\":\"openai\"}',0.25,100,40,10,20,5);").unwrap();
        drop(external);
        let target = Connection::open_in_memory().unwrap();
        target.execute_batch("CREATE TABLE usage_events(id TEXT PRIMARY KEY,provider TEXT,model TEXT,at TEXT,input_tokens INTEGER,output_tokens INTEGER,cached_tokens INTEGER,cost REAL,task TEXT);").unwrap();
        reliability::migrate_database(&target).unwrap();
        let source = SyncSourceConfig {
            id: "opencode-local".into(),
            provider: "OpenCode Go".into(),
            kind: "local_sqlite".into(),
            mode: "sqlite".into(),
            path: temp.to_string_lossy().into_owned(),
            enabled: true,
            interval_seconds: 30,
        };
        let result = sync(&target, &source, true).unwrap();
        assert_eq!(result.imported, 1);
        let row: (i64, i64, i64, String) = target
            .query_row(
                "SELECT input_tokens,output_tokens,cached_tokens,model FROM usage_events",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(row, (125, 50, 20, "gpt-5".into()));
        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn rejects_unknown_schema_instead_of_returning_zero() {
        let temp = std::env::temp_dir().join(format!(
            "tm-opencode-unknown-{}.db",
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let external = Connection::open(&temp).unwrap();
        external
            .execute_batch("CREATE TABLE session(id TEXT PRIMARY KEY,title TEXT);")
            .unwrap();
        drop(external);
        let target = Connection::open_in_memory().unwrap();
        target.execute_batch("CREATE TABLE usage_events(id TEXT PRIMARY KEY,provider TEXT,model TEXT,at TEXT,input_tokens INTEGER,output_tokens INTEGER,cached_tokens INTEGER,cost REAL,task TEXT);").unwrap();
        reliability::migrate_database(&target).unwrap();
        let source = SyncSourceConfig {
            id: "opencode-local".into(),
            provider: "OpenCode Go".into(),
            kind: "local_sqlite".into(),
            mode: "sqlite".into(),
            path: temp.to_string_lossy().into_owned(),
            enabled: true,
            interval_seconds: 30,
        };
        let error = sync(&target, &source, true).unwrap_err();
        assert!(error.contains("暂未适配"));
        let _ = std::fs::remove_file(temp);
    }
}
