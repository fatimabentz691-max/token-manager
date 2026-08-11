use crate::{providers::opencode, AppDb, UsageEvent};
use chrono::{DateTime, Utc};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};

pub const DATABASE_VERSION: i64 = 11;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    OfficialApi,
    LocalProxy,
    LocalLog,
    LocalJson,
    LocalSqlite,
    BillImport,
    Estimate,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OfficialApi => "official_api",
            Self::LocalProxy => "local_proxy",
            Self::LocalLog => "local_log",
            Self::LocalJson => "local_json",
            Self::LocalSqlite => "local_sqlite",
            Self::BillImport => "bill_import",
            Self::Estimate => "estimate",
        }
    }

    fn priority(self) -> i64 {
        match self {
            Self::OfficialApi => 700,
            Self::LocalProxy => 600,
            Self::LocalSqlite => 500,
            Self::LocalJson => 400,
            Self::LocalLog => 300,
            Self::BillImport => 200,
            Self::Estimate => 100,
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "official_api" => Self::OfficialApi,
            "local_proxy" => Self::LocalProxy,
            "local_sqlite" => Self::LocalSqlite,
            "local_json" => Self::LocalJson,
            "bill_import" => Self::BillImport,
            "estimate" => Self::Estimate,
            _ => Self::LocalLog,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Accuracy {
    Official,
    Observed,
    Imported,
    Estimated,
}

impl Accuracy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Observed => "observed",
            Self::Imported => "imported",
            Self::Estimated => "estimated",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UsageProvenance {
    pub source_id: String,
    pub source_kind: SourceKind,
    pub accuracy: Accuracy,
    pub account_id: Option<String>,
    pub source_ref: Option<String>,
    pub price_version: String,
    pub currency: String,
    pub collected_at: DateTime<Utc>,
}

impl UsageProvenance {
    pub fn observed(source_id: impl Into<String>, source_kind: SourceKind) -> Self {
        Self {
            source_id: source_id.into(),
            source_kind,
            accuracy: Accuracy::Observed,
            account_id: None,
            source_ref: None,
            price_version: "bundled-v0.11.0".into(),
            currency: "CNY".into(),
            collected_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageEventV2 {
    #[serde(flatten)]
    pub usage: UsageEvent,
    pub source_id: String,
    pub source_kind: String,
    pub accuracy: String,
    pub account_id: Option<String>,
    pub collected_at: String,
    pub source_ref: Option<String>,
    pub price_version: String,
    pub currency: String,
    pub canonical_key: String,
    pub is_shadowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSourceConfig {
    pub id: String,
    pub provider: String,
    pub kind: String,
    pub mode: String,
    pub path: String,
    pub enabled: bool,
    pub interval_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHealth {
    pub source_id: String,
    pub provider: String,
    pub source_kind: String,
    pub status: String,
    pub last_success_at: Option<String>,
    pub next_refresh_at: Option<String>,
    pub last_error: Option<String>,
    pub latency_ms: u64,
    pub imported: usize,
    pub detail: String,
}

impl SyncHealth {
    fn idle(source: &SyncSourceConfig) -> Self {
        Self {
            source_id: source.id.clone(),
            provider: source.provider.clone(),
            source_kind: source.kind.clone(),
            status: if source.enabled { "idle" } else { "disabled" }.into(),
            last_success_at: None,
            next_refresh_at: None,
            last_error: None,
            latency_ms: 0,
            imported: 0,
            detail: if source.enabled {
                "等待同步".into()
            } else {
                "数据源已停用".into()
            },
        }
    }
}

pub struct SyncCoordinator {
    health: Mutex<HashMap<String, SyncHealth>>,
    watchers: Mutex<HashMap<String, RecommendedWatcher>>,
}

impl Default for SyncCoordinator {
    fn default() -> Self {
        Self {
            health: Mutex::new(HashMap::new()),
            watchers: Mutex::new(HashMap::new()),
        }
    }
}

fn column_exists(db: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut statement = db.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row?.eq_ignore_ascii_case(column) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_column(
    db: &Connection,
    table: &str,
    column: &str,
    sql_type: &str,
) -> rusqlite::Result<()> {
    if !column_exists(db, table, column)? {
        db.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {sql_type};"
        ))?;
    }
    Ok(())
}

pub fn migrate_database(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        ensure_column(
            db,
            "usage_events",
            "source_id",
            "TEXT NOT NULL DEFAULT 'legacy-local'",
        )?;
        ensure_column(
            db,
            "usage_events",
            "source_kind",
            "TEXT NOT NULL DEFAULT 'local_log'",
        )?;
        ensure_column(
            db,
            "usage_events",
            "accuracy",
            "TEXT NOT NULL DEFAULT 'observed'",
        )?;
        ensure_column(db, "usage_events", "account_id", "TEXT")?;
        ensure_column(
            db,
            "usage_events",
            "collected_at",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        ensure_column(db, "usage_events", "source_ref", "TEXT")?;
        ensure_column(
            db,
            "usage_events",
            "price_version",
            "TEXT NOT NULL DEFAULT 'legacy'",
        )?;
        ensure_column(
            db,
            "usage_events",
            "currency",
            "TEXT NOT NULL DEFAULT 'CNY'",
        )?;
        ensure_column(
            db,
            "usage_events",
            "canonical_key",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        ensure_column(
            db,
            "usage_events",
            "is_shadowed",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_sources(
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                kind TEXT NOT NULL,
                mode TEXT NOT NULL DEFAULT 'auto',
                path TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                interval_seconds INTEGER NOT NULL DEFAULT 30,
                updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sync_cursors(
                source_id TEXT PRIMARY KEY,
                cursor_json TEXT NOT NULL DEFAULT '{}',
                last_success_at TEXT,
                last_error TEXT,
                updated_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_usage_source_at ON usage_events(source_id,at);
             CREATE INDEX IF NOT EXISTS idx_usage_canonical ON usage_events(canonical_key,is_shadowed);
             CREATE INDEX IF NOT EXISTS idx_usage_account_model_at ON usage_events(account_id,model,at);",
        )?;
        db.execute(
            "UPDATE usage_events SET collected_at=at WHERE collected_at='' OR collected_at IS NULL",
            [],
        )?;
        db.execute(
            "UPDATE usage_events SET source_kind='local_proxy',source_id='legacy-proxy' WHERE id LIKE 'evt-%' AND source_id='legacy-local'",
            [],
        )?;
        db.execute(
            "UPDATE usage_events SET source_kind='local_json',source_id='opencode-local' WHERE id LIKE 'opencode-json:%'",
            [],
        )?;
        db.execute(
            "UPDATE usage_events SET canonical_key=id WHERE canonical_key='' OR canonical_key IS NULL",
            [],
        )?;
        let now = Utc::now().to_rfc3339();
        for (id, provider, kind, mode) in [
            ("codex-local", "Codex", "local_log", "auto"),
            ("claude-local", "Claude Code", "local_log", "auto"),
            ("opencode-local", "OpenCode Go", "local_sqlite", "auto"),
        ] {
            db.execute(
                "INSERT OR IGNORE INTO usage_sources(id,provider,kind,mode,path,enabled,interval_seconds,updated_at) VALUES(?1,?2,?3,?4,'',1,30,?5)",
                params![id, provider, kind, mode, now],
            )?;
        }
        db.pragma_update(None, "user_version", DATABASE_VERSION)?;
        Ok::<_, rusqlite::Error>(())
    })();
    match result {
        Ok(()) => db.execute_batch("COMMIT"),
        Err(error) => {
            let _ = db.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

fn canonical_key(event: &UsageEvent, provenance: &UsageProvenance) -> String {
    let bucket = event.at.timestamp() / 3;
    let stable = provenance.source_ref.as_deref().unwrap_or_default();
    let raw = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        event.provider,
        provenance.account_id.as_deref().unwrap_or_default(),
        event.model,
        event.input,
        event.output,
        event.cached,
        bucket,
        stable,
        event.task
    );
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

pub fn upsert_usage(
    db: &Connection,
    event: &UsageEvent,
    provenance: &UsageProvenance,
) -> rusqlite::Result<bool> {
    let canonical = canonical_key(event, provenance);
    let existing = db.query_row(
        "SELECT id,source_kind FROM usage_events WHERE canonical_key=?1 AND id<>?2 AND is_shadowed=0 LIMIT 1",
        params![canonical, event.id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );
    let mut shadowed = false;
    if let Ok((existing_id, existing_kind)) = existing {
        let existing_priority = SourceKind::parse(&existing_kind).priority();
        if existing_priority >= provenance.source_kind.priority() {
            shadowed = true;
        } else {
            db.execute(
                "UPDATE usage_events SET is_shadowed=1 WHERE id=?1",
                [existing_id],
            )?;
        }
    }
    db.execute(
        "INSERT OR REPLACE INTO usage_events(
            id,provider,model,at,input_tokens,output_tokens,cached_tokens,cost,task,
            source_id,source_kind,accuracy,account_id,collected_at,source_ref,
            price_version,currency,canonical_key,is_shadowed
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
        params![
            event.id,
            event.provider,
            event.model,
            event.at.to_rfc3339(),
            event.input,
            event.output,
            event.cached,
            event.cost,
            event.task,
            provenance.source_id,
            provenance.source_kind.as_str(),
            provenance.accuracy.as_str(),
            provenance.account_id,
            provenance.collected_at.to_rfc3339(),
            provenance.source_ref,
            provenance.price_version,
            provenance.currency,
            canonical,
            i64::from(shadowed),
        ],
    )?;
    Ok(!shadowed)
}

fn load_source(db: &Connection, source_id: &str) -> Result<SyncSourceConfig, String> {
    db.query_row(
        "SELECT id,provider,kind,mode,path,enabled,interval_seconds FROM usage_sources WHERE id=?1",
        [source_id],
        |row| {
            Ok(SyncSourceConfig {
                id: row.get(0)?,
                provider: row.get(1)?,
                kind: row.get(2)?,
                mode: row.get(3)?,
                path: row.get(4)?,
                enabled: row.get::<_, i64>(5)? != 0,
                interval_seconds: row.get::<_, i64>(6)?.clamp(5, 3600) as u32,
            })
        },
    )
    .map_err(|error| format!("未找到数据源 {source_id}：{error}"))
}

fn save_cursor(db: &Connection, source_id: &str, success: bool, detail: &str) {
    let now = Utc::now().to_rfc3339();
    let _ = db.execute(
        "INSERT INTO sync_cursors(source_id,cursor_json,last_success_at,last_error,updated_at)
         VALUES(?1,'{}',?2,?3,?4)
         ON CONFLICT(source_id) DO UPDATE SET
           last_success_at=CASE WHEN ?5 THEN excluded.last_success_at ELSE sync_cursors.last_success_at END,
           last_error=excluded.last_error,updated_at=excluded.updated_at",
        params![
            source_id,
            if success { Some(now.clone()) } else { None },
            if success { None::<String> } else { Some(detail.to_string()) },
            now,
            success,
        ],
    );
}

pub fn install_watcher(
    source: &SyncSourceConfig,
    app: &AppHandle,
    coordinator: &SyncCoordinator,
) -> Result<(), String> {
    let mut watchers = coordinator
        .watchers
        .lock()
        .map_err(|_| "文件监听器锁定失败")?;
    watchers.remove(&source.id);
    if !source.enabled || source.path.trim().is_empty() || !Path::new(&source.path).exists() {
        return Ok(());
    }
    let app_handle = app.clone();
    let source_id = source.id.clone();
    let gate = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(2)));
    let callback_gate = gate.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_err() {
            return;
        }
        let Ok(mut last) = callback_gate.lock() else {
            return;
        };
        if last.elapsed() < Duration::from_millis(500) {
            return;
        }
        *last = Instant::now();
        let _ = app_handle.emit(
            "local-source-changed",
            serde_json::json!({"source_id": source_id, "at": Utc::now().to_rfc3339()}),
        );
    })
    .map_err(|error| format!("创建文件监听器失败：{error}"))?;
    let path = PathBuf::from(&source.path);
    let mode = if path.is_dir() {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    watcher
        .watch(&path, mode)
        .map_err(|error| format!("监听 {} 失败：{error}", path.display()))?;
    watchers.insert(source.id.clone(), watcher);
    Ok(())
}

#[tauri::command]
pub fn configure_local_source(
    config: SyncSourceConfig,
    app: AppHandle,
    db_state: State<AppDb>,
    coordinator: State<SyncCoordinator>,
) -> Result<SyncSourceConfig, String> {
    if config.id.trim().is_empty() || config.provider.trim().is_empty() {
        return Err("数据源 ID 和平台不能为空".into());
    }
    if !matches!(config.mode.as_str(), "auto" | "sqlite" | "json") {
        return Err("本地数据源模式必须为 auto、sqlite 或 json".into());
    }
    let interval = config.interval_seconds.clamp(5, 3600);
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    db.execute(
        "INSERT INTO usage_sources(id,provider,kind,mode,path,enabled,interval_seconds,updated_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(id) DO UPDATE SET provider=excluded.provider,kind=excluded.kind,
         mode=excluded.mode,path=excluded.path,enabled=excluded.enabled,
         interval_seconds=excluded.interval_seconds,updated_at=excluded.updated_at",
        params![
            config.id,
            config.provider,
            config.kind,
            config.mode,
            config.path,
            config.enabled,
            interval,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|error| error.to_string())?;
    let saved = load_source(&db, &config.id)?;
    drop(db);
    install_watcher(&saved, &app, &coordinator)?;
    coordinator
        .health
        .lock()
        .map_err(|_| "同步状态锁定失败")?
        .insert(saved.id.clone(), SyncHealth::idle(&saved));
    Ok(saved)
}

#[tauri::command]
pub fn list_sync_sources(db_state: State<AppDb>) -> Result<Vec<SyncSourceConfig>, String> {
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    let mut statement = db
        .prepare("SELECT id,provider,kind,mode,path,enabled,interval_seconds FROM usage_sources ORDER BY provider,id")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(SyncSourceConfig {
                id: row.get(0)?,
                provider: row.get(1)?,
                kind: row.get(2)?,
                mode: row.get(3)?,
                path: row.get(4)?,
                enabled: row.get::<_, i64>(5)? != 0,
                interval_seconds: row.get::<_, i64>(6)?.clamp(5, 3600) as u32,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_sync_health(
    db_state: State<AppDb>,
    coordinator: State<SyncCoordinator>,
) -> Result<Vec<SyncHealth>, String> {
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    let mut statement = db
        .prepare(
            "SELECT s.id,s.provider,s.kind,s.mode,s.path,s.enabled,s.interval_seconds,
                    c.last_success_at,c.last_error
             FROM usage_sources s LEFT JOIN sync_cursors c ON c.source_id=s.id
             ORDER BY s.provider,s.id",
        )
        .map_err(|error| error.to_string())?;
    let sources = statement
        .query_map([], |row| {
            Ok((
                SyncSourceConfig {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    kind: row.get(2)?,
                    mode: row.get(3)?,
                    path: row.get(4)?,
                    enabled: row.get::<_, i64>(5)? != 0,
                    interval_seconds: row.get::<_, i64>(6)?.clamp(5, 3600) as u32,
                },
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let live = coordinator.health.lock().map_err(|_| "同步状态锁定失败")?;
    Ok(sources
        .into_iter()
        .map(|(source, last_success_at, last_error)| {
            live.get(&source.id).cloned().unwrap_or_else(|| {
                let mut health = SyncHealth::idle(&source);
                health.last_success_at = last_success_at;
                health.last_error = last_error;
                health
            })
        })
        .collect())
}

fn sync_one(
    source: &SyncSourceConfig,
    force: bool,
    db: &Connection,
    app: &AppHandle,
    coordinator: &SyncCoordinator,
) -> Result<SyncHealth, String> {
    let started = Instant::now();
    let mut health = SyncHealth::idle(source);
    health.status = "syncing".into();
    health.detail = "正在读取本地数据".into();
    coordinator
        .health
        .lock()
        .map_err(|_| "同步状态锁定失败")?
        .insert(source.id.clone(), health.clone());
    let _ = app.emit("sync-status-updated", &health);

    if !source.enabled {
        return Ok(health);
    }
    let result = if source.provider == "OpenCode Go" {
        opencode::sync(db, source, force)
    } else {
        Ok(opencode::LocalSyncOutcome {
            imported: 0,
            scanned_files: 0,
            source_kind: source.kind.clone(),
            source_path: source.path.clone(),
            detail: "由现有客户端日志或代理链路实时更新".into(),
        })
    };
    health.latency_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(outcome) => {
            health.status = "healthy".into();
            health.source_kind = outcome.source_kind.clone();
            health.imported = outcome.imported;
            health.detail = outcome.detail;
            health.last_success_at = Some(Utc::now().to_rfc3339());
            health.next_refresh_at = Some(
                (Utc::now() + chrono::Duration::seconds(source.interval_seconds as i64))
                    .to_rfc3339(),
            );
            health.last_error = None;
            if !outcome.source_path.is_empty()
                && (source.path != outcome.source_path || source.kind != outcome.source_kind)
            {
                let _ = db.execute(
                    "UPDATE usage_sources SET path=?1,kind=?2,updated_at=?3 WHERE id=?4",
                    params![
                        outcome.source_path,
                        outcome.source_kind,
                        Utc::now().to_rfc3339(),
                        source.id,
                    ],
                );
                if let Ok(updated) = load_source(db, &source.id) {
                    let _ = install_watcher(&updated, app, coordinator);
                }
            }
            save_cursor(db, &source.id, true, &health.detail);
            let _ = app.emit(
                "usage-updated",
                serde_json::json!({
                    "source_id": source.id,
                    "provider": source.provider,
                    "imported": health.imported,
                    "scanned_files": outcome.scanned_files,
                    "at": health.last_success_at,
                }),
            );
        }
        Err(error) => {
            health.status = "error".into();
            health.detail = "同步失败".into();
            health.last_error = Some(error.clone());
            save_cursor(db, &source.id, false, &error);
        }
    }
    coordinator
        .health
        .lock()
        .map_err(|_| "同步状态锁定失败")?
        .insert(source.id.clone(), health.clone());
    let _ = app.emit("sync-status-updated", &health);
    Ok(health)
}

#[tauri::command]
pub fn sync_source(
    source_id: String,
    force: Option<bool>,
    app: AppHandle,
    db_state: State<AppDb>,
    coordinator: State<SyncCoordinator>,
) -> Result<SyncHealth, String> {
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    let source = load_source(&db, &source_id)?;
    sync_one(&source, force.unwrap_or(false), &db, &app, &coordinator)
}

#[tauri::command]
pub fn sync_all_sources(
    force: Option<bool>,
    app: AppHandle,
    db_state: State<AppDb>,
    coordinator: State<SyncCoordinator>,
) -> Result<Vec<SyncHealth>, String> {
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    let mut statement = db
        .prepare("SELECT id FROM usage_sources WHERE enabled=1 ORDER BY provider,id")
        .map_err(|error| error.to_string())?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    let mut results = Vec::new();
    for id in ids {
        let source = load_source(&db, &id)?;
        results.push(sync_one(
            &source,
            force.unwrap_or(false),
            &db,
            &app,
            &coordinator,
        )?);
    }
    Ok(results)
}

#[tauri::command]
pub fn list_usage_v2(days: i64, db_state: State<AppDb>) -> Result<Vec<UsageEventV2>, String> {
    let since = (Utc::now() - chrono::Duration::days(days.clamp(1, 365))).to_rfc3339();
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    let mut statement = db
        .prepare(
            "SELECT id,provider,model,at,input_tokens,output_tokens,cached_tokens,cost,task,
                source_id,source_kind,accuracy,account_id,collected_at,source_ref,
                price_version,currency,canonical_key,is_shadowed
         FROM usage_events WHERE at>=?1 AND is_shadowed=0 ORDER BY at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([since], |row| {
            Ok(UsageEventV2 {
                usage: UsageEvent {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    model: row.get(2)?,
                    at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?
                        .with_timezone(&Utc),
                    input: row.get::<_, i64>(4)?.max(0) as u64,
                    output: row.get::<_, i64>(5)?.max(0) as u64,
                    cached: row.get::<_, i64>(6)?.max(0) as u64,
                    cost: row.get(7)?,
                    task: row.get(8)?,
                },
                source_id: row.get(9)?,
                source_kind: row.get(10)?,
                accuracy: row.get(11)?,
                account_id: row.get(12)?,
                collected_at: row.get(13)?,
                source_ref: row.get(14)?,
                price_version: row.get(15)?,
                currency: row.get(16)?,
                canonical_key: row.get(17)?,
                is_shadowed: row.get::<_, i64>(18)? != 0,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn restore_watchers(app: &AppHandle, db: &AppDb, coordinator: &SyncCoordinator) {
    let Ok(db) = db.0.lock() else { return };
    let Ok(mut statement) = db.prepare(
        "SELECT id,provider,kind,mode,path,enabled,interval_seconds FROM usage_sources WHERE enabled=1 AND path<>''",
    ) else { return };
    let Ok(rows) = statement.query_map([], |row| {
        Ok(SyncSourceConfig {
            id: row.get(0)?,
            provider: row.get(1)?,
            kind: row.get(2)?,
            mode: row.get(3)?,
            path: row.get(4)?,
            enabled: row.get::<_, i64>(5)? != 0,
            interval_seconds: row.get::<_, i64>(6)?.clamp(5, 3600) as u32,
        })
    }) else {
        return;
    };
    for source in rows.flatten() {
        let _ = install_watcher(&source, app, coordinator);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_usage_without_losing_rows() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE usage_events(id TEXT PRIMARY KEY,provider TEXT,model TEXT,at TEXT,input_tokens INTEGER,output_tokens INTEGER,cached_tokens INTEGER,cost REAL,task TEXT); INSERT INTO usage_events VALUES('evt-1','DeepSeek','deepseek-chat','2026-08-11T00:00:00Z',10,20,2,0.1,'其他');").unwrap();
        migrate_database(&db).unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM usage_events", [], |row| row.get(0))
            .unwrap();
        let kind: String = db
            .query_row("SELECT source_kind FROM usage_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(kind, "local_proxy");
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            DATABASE_VERSION
        );
    }

    #[test]
    fn higher_priority_event_shadows_duplicate() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE usage_events(id TEXT PRIMARY KEY,provider TEXT,model TEXT,at TEXT,input_tokens INTEGER,output_tokens INTEGER,cached_tokens INTEGER,cost REAL,task TEXT);").unwrap();
        migrate_database(&db).unwrap();
        let at = DateTime::parse_from_rfc3339("2026-08-11T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut event = UsageEvent {
            id: "json".into(),
            provider: "OpenCode Go".into(),
            model: "gpt".into(),
            at,
            input: 10,
            output: 20,
            cached: 0,
            cost: 0.1,
            task: "其他".into(),
        };
        let json = UsageProvenance::observed("opencode", SourceKind::LocalJson);
        assert!(upsert_usage(&db, &event, &json).unwrap());
        event.id = "proxy".into();
        let proxy = UsageProvenance::observed("proxy", SourceKind::LocalProxy);
        assert!(upsert_usage(&db, &event, &proxy).unwrap());
        let visible: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM usage_events WHERE is_shadowed=0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(visible, 1);
        let source: String = db
            .query_row(
                "SELECT source_kind FROM usage_events WHERE is_shadowed=0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, "local_proxy");
    }
}
