use crate::{
    providers::{local_agents, opencode},
    AppDb, UsageEvent,
};
use chrono::{DateTime, Utc};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    panic::AssertUnwindSafe,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State};

pub const DATABASE_VERSION: i64 = 16;

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
            price_version: "bundled-v0.11.1".into(),
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
    /// 原始明细为 1；为保护 WebView 性能而生成的历史日聚合记录保存真实请求数。
    pub request_count: u64,
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
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub collector_kind: String,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub detected: bool,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_path_mode")]
    pub path_mode: String,
    #[serde(default)]
    pub path_spec_ids: Vec<String>,
}

fn default_schema_version() -> u32 {
    1
}

fn default_path_mode() -> String {
    "auto".into()
}

fn resolve_automatic_paths(mut source: SyncSourceConfig) -> SyncSourceConfig {
    if source.path_mode == "auto" && !source.agent_id.is_empty() {
        let resolved = local_agents::resolve_path_spec_ids(&source.agent_id, &source.path_spec_ids);
        if !resolved.is_empty() {
            source.path = resolved.first().cloned().unwrap_or_default();
            source.paths = resolved;
        }
    }
    source
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub id: String,
    pub kind: String,
    pub source_ids: Vec<String>,
    pub status: String,
    pub completed: usize,
    pub total: usize,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub last_error: Option<String>,
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
    jobs: Mutex<HashMap<String, SyncJob>>,
    active_sources: Mutex<HashSet<String>>,
    cancelled_jobs: Mutex<HashSet<String>>,
    source_slots: Arc<tokio::sync::Semaphore>,
    sequence: AtomicU64,
}

impl Default for SyncCoordinator {
    fn default() -> Self {
        Self {
            health: Mutex::new(HashMap::new()),
            watchers: Mutex::new(HashMap::new()),
            jobs: Mutex::new(HashMap::new()),
            active_sources: Mutex::new(HashSet::new()),
            cancelled_jobs: Mutex::new(HashSet::new()),
            source_slots: Arc::new(tokio::sync::Semaphore::new(2)),
            sequence: AtomicU64::new(1),
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
    let previous_version = db.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))?;
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
        ensure_column(db, "usage_sources", "agent_id", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(
            db,
            "usage_sources",
            "collector_kind",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        ensure_column(
            db,
            "usage_sources",
            "paths_json",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;
        ensure_column(
            db,
            "usage_sources",
            "capabilities_json",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;
        ensure_column(
            db,
            "usage_sources",
            "detected",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(
            db,
            "usage_sources",
            "schema_version",
            "INTEGER NOT NULL DEFAULT 1",
        )?;
        ensure_column(
            db,
            "usage_sources",
            "path_mode",
            "TEXT NOT NULL DEFAULT 'auto'",
        )?;
        ensure_column(
            db,
            "usage_sources",
            "path_spec_ids_json",
            "TEXT NOT NULL DEFAULT '[]'",
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
        for source in local_agents::default_source_rows() {
            db.execute(
                "INSERT INTO usage_sources(id,provider,kind,mode,path,enabled,interval_seconds,updated_at,agent_id,collector_kind,paths_json,capabilities_json,detected,schema_version,path_mode,path_spec_ids_json)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
                 ON CONFLICT(id) DO UPDATE SET
                   agent_id=CASE WHEN usage_sources.agent_id='' THEN excluded.agent_id ELSE usage_sources.agent_id END,
                   collector_kind=CASE WHEN usage_sources.collector_kind='' THEN excluded.collector_kind ELSE usage_sources.collector_kind END,
                   paths_json=CASE WHEN usage_sources.paths_json='[]' THEN excluded.paths_json ELSE usage_sources.paths_json END,
                   capabilities_json=excluded.capabilities_json,
                   detected=excluded.detected,
                   schema_version=excluded.schema_version,
                   path_mode=CASE WHEN usage_sources.path_spec_ids_json='[]' THEN excluded.path_mode ELSE usage_sources.path_mode END,
                   path_spec_ids_json=CASE WHEN usage_sources.path_spec_ids_json='[]' THEN excluded.path_spec_ids_json ELSE usage_sources.path_spec_ids_json END",
                params![source.id, source.provider, source.kind, source.mode, source.path, source.enabled, source.interval_seconds, now, source.agent_id, source.collector_kind, serde_json::to_string(&source.paths).unwrap_or_else(|_| "[]".into()), serde_json::to_string(&source.capabilities).unwrap_or_else(|_| "[]".into()), source.detected, source.schema_version, source.path_mode, serde_json::to_string(&source.path_spec_ids).unwrap_or_else(|_| "[]".into())],
            )?;
        }
        if previous_version < 14 {
            // 早期预览版曾把检测到的 Agent 默认启用。正式版改为先发现、再由用户确认接入，
            // 避免 Cursor、Copilot 等大型目录在启动时同时建立递归监听。
            db.execute(
                "UPDATE usage_sources
                 SET enabled=0
                 WHERE id LIKE 'agent-%'",
                [],
            )?;
        }
        if previous_version < 15 {
            // Codex 恢复为专属直读链路；保留旧记录，但不再由通用 Agent 同步器扫描。
            db.execute(
                "UPDATE usage_sources SET enabled=0,updated_at=?1 WHERE id='codex-local' OR agent_id='codex'",
                [now.clone()],
            )?;
        }
        if previous_version < 16 {
            let mut statement = db.prepare(
                "SELECT id,agent_id,path,paths_json FROM usage_sources WHERE agent_id<>'' AND path_mode='auto'",
            )?;
            let legacy = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            drop(statement);
            for (id, agent_id, path, paths_json) in legacy {
                let mut paths =
                    serde_json::from_str::<Vec<String>>(&paths_json).unwrap_or_default();
                if !path.is_empty() && !paths.iter().any(|item| item.eq_ignore_ascii_case(&path)) {
                    paths.insert(0, path);
                }
                let spec_ids = local_agents::infer_path_spec_ids(&agent_id, &paths);
                if !spec_ids.is_empty() {
                    db.execute(
                        "UPDATE usage_sources SET path_spec_ids_json=?1,path_mode='auto',updated_at=?2 WHERE id=?3",
                        params![serde_json::to_string(&spec_ids).unwrap_or_else(|_| "[]".into()), now, id],
                    )?;
                } else {
                    db.execute(
                        "UPDATE usage_sources SET path_mode='manual',updated_at=?1 WHERE id=?2",
                        params![now, id],
                    )?;
                }
            }
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
        "SELECT id,provider,kind,mode,path,enabled,interval_seconds,agent_id,collector_kind,paths_json,capabilities_json,detected,schema_version,path_mode,path_spec_ids_json FROM usage_sources WHERE id=?1",
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
                agent_id: row.get(7)?,
                collector_kind: row.get(8)?,
                paths: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                capabilities: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
                detected: row.get::<_, i64>(11)? != 0,
                schema_version: row.get::<_, i64>(12)?.max(1) as u32,
                path_mode: row.get(13)?,
                path_spec_ids: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
            })
        },
    )
    .map(resolve_automatic_paths)
    .map_err(|error| format!("未找到数据源 {source_id}：{error}"))
}

fn save_cursor(db: &Connection, source_id: &str, success: bool, detail: &str) {
    let now = Utc::now().to_rfc3339();
    let _ = db.execute(
        "INSERT INTO sync_cursors(source_id,cursor_json,last_success_at,last_error,updated_at)
         VALUES(?1,'{}',?2,?3,?4)
         ON CONFLICT(source_id) DO UPDATE SET
           cursor_json=sync_cursors.cursor_json,
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

fn append_sync_log(source: &SyncSourceConfig, health: &SyncHealth) {
    let path = crate::data_path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("logs")
        .join("sync-runtime.log");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::metadata(&path)
        .map(|meta| meta.len() > 2 * 1024 * 1024)
        .unwrap_or(false)
    {
        let _ = std::fs::rename(&path, path.with_extension("log.1"));
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let record = serde_json::json!({
            "at": Utc::now().to_rfc3339(),
            "source_id": source.id,
            "provider": source.provider,
            "status": health.status,
            "latency_ms": health.latency_ms,
            "imported": health.imported,
            "last_error": health.last_error,
            "detail": health.detail,
        });
        let _ = writeln!(file, "{record}");
    }
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
    if !source.enabled {
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
        // 文件变化直接进入 Rust 任务合并队列，避免主窗口和悬浮窗各自再触发一次同步。
        let event_app = app_handle.clone();
        let event_source = source_id.clone();
        tauri::async_runtime::spawn(async move {
            let _ = execute_source_task(event_source, false, event_app).await;
        });
    })
    .map_err(|error| format!("创建文件监听器失败：{error}"))?;
    let mut paths = source
        .paths
        .iter()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if !source.path.trim().is_empty() {
        paths.insert(0, PathBuf::from(&source.path));
    }
    paths.sort();
    paths.dedup();
    let mut watched = 0usize;
    for path in paths.into_iter().filter(|path| path.exists()) {
        let mode = if path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        watcher
            .watch(&path, mode)
            .map_err(|error| format!("监听 {} 失败：{error}", path.display()))?;
        watched += 1;
    }
    if watched == 0 {
        return Ok(());
    }
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
    if !matches!(
        config.mode.as_str(),
        "auto" | "sqlite" | "json" | "jsonl" | "log" | "otel" | "cache"
    ) {
        return Err("本地数据源模式必须为 auto、sqlite、json、jsonl、log、otel 或 cache".into());
    }
    let interval = config.interval_seconds.clamp(5, 3600);
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    db.execute(
        "INSERT INTO usage_sources(id,provider,kind,mode,path,enabled,interval_seconds,updated_at,agent_id,collector_kind,paths_json,capabilities_json,detected,schema_version,path_mode,path_spec_ids_json)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
         ON CONFLICT(id) DO UPDATE SET provider=excluded.provider,kind=excluded.kind,
         mode=excluded.mode,path=excluded.path,enabled=excluded.enabled,
         interval_seconds=excluded.interval_seconds,updated_at=excluded.updated_at,
         agent_id=excluded.agent_id,collector_kind=excluded.collector_kind,
         paths_json=excluded.paths_json,capabilities_json=excluded.capabilities_json,
         detected=excluded.detected,schema_version=excluded.schema_version,
         path_mode=excluded.path_mode,path_spec_ids_json=excluded.path_spec_ids_json",
        params![
            config.id,
            config.provider,
            config.kind,
            config.mode,
            config.path,
            config.enabled,
            interval,
            Utc::now().to_rfc3339(),
            config.agent_id,
            config.collector_kind,
            serde_json::to_string(&config.paths).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&config.capabilities).unwrap_or_else(|_| "[]".into()),
            config.detected,
            config.schema_version.max(1),
            config.path_mode,
            serde_json::to_string(&config.path_spec_ids).unwrap_or_else(|_| "[]".into()),
        ],
    )
    .map_err(|error| error.to_string())?;
    let saved = load_source(&db, &config.id)?;
    drop(db);
    coordinator
        .health
        .lock()
        .map_err(|_| "同步状态锁定失败")?
        .insert(saved.id.clone(), SyncHealth::idle(&saved));
    // Windows 递归监听注册在超大 workspaceStorage 上可能明显变慢。
    // 保存接入配置后立即返回，监听器由 Rust 后台安装，避免前端表现为未响应。
    let watcher_app = app.clone();
    let watcher_source = saved.clone();
    std::thread::spawn(move || {
        let watcher_coordinator = watcher_app.state::<SyncCoordinator>();
        let _ = install_watcher(&watcher_source, &watcher_app, &watcher_coordinator);
    });
    Ok(saved)
}

#[tauri::command]
pub fn list_local_agent_definitions() -> Vec<local_agents::AgentConnectorDefinition> {
    local_agents::definitions()
}

#[tauri::command]
pub fn detect_local_agents() -> Vec<local_agents::AgentConnectorDefinition> {
    local_agents::definitions()
}

#[tauri::command]
pub fn configure_agent_source(
    config: SyncSourceConfig,
    app: AppHandle,
    db_state: State<AppDb>,
    coordinator: State<SyncCoordinator>,
) -> Result<SyncSourceConfig, String> {
    if config.agent_id.trim().is_empty() {
        return Err("本地 Agent 数据源必须包含 agent_id".into());
    }
    configure_local_source(config, app, db_state, coordinator)
}

#[tauri::command]
pub async fn test_agent_source(source_id: String, app: AppHandle) -> Result<SyncHealth, String> {
    sync_source(source_id, Some(true), app).await
}

#[tauri::command]
pub fn list_agent_source_models(
    source_id: String,
    db_state: State<AppDb>,
) -> Result<Vec<local_agents::AgentSourceModel>, String> {
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    local_agents::models(&db, &source_id)
}

#[tauri::command]
pub fn list_sync_sources(db_state: State<AppDb>) -> Result<Vec<SyncSourceConfig>, String> {
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    let mut statement = db
        .prepare("SELECT id,provider,kind,mode,path,enabled,interval_seconds,agent_id,collector_kind,paths_json,capabilities_json,detected,schema_version,path_mode,path_spec_ids_json FROM usage_sources ORDER BY provider,id")
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
                agent_id: row.get(7)?,
                collector_kind: row.get(8)?,
                paths: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                capabilities: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
                detected: row.get::<_, i64>(11)? != 0,
                schema_version: row.get::<_, i64>(12)?.max(1) as u32,
                path_mode: row.get(13)?,
                path_spec_ids: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map(|sources| sources.into_iter().map(resolve_automatic_paths).collect())
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
                    s.agent_id,s.collector_kind,s.paths_json,s.capabilities_json,s.detected,s.schema_version,s.path_mode,s.path_spec_ids_json,
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
                    agent_id: row.get(7)?,
                    collector_kind: row.get(8)?,
                    paths: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                    capabilities: serde_json::from_str(&row.get::<_, String>(10)?)
                        .unwrap_or_default(),
                    detected: row.get::<_, i64>(11)? != 0,
                    schema_version: row.get::<_, i64>(12)?.max(1) as u32,
                    path_mode: row.get(13)?,
                    path_spec_ids: serde_json::from_str(&row.get::<_, String>(14)?)
                        .unwrap_or_default(),
                },
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<String>>(16)?,
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
    let result = if !source.agent_id.is_empty() {
        local_agents::sync(db, source, force)
    } else if source.provider == "OpenCode Go" {
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
            let _ = app.emit(
                "source-usage-updated",
                serde_json::json!({
                    "source_id": source.id,
                    "provider": source.provider,
                    "imported": health.imported,
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
    append_sync_log(source, &health);
    let _ = app.emit("sync-status-updated", &health);
    Ok(health)
}

fn sync_source_blocking(
    source_id: String,
    force: bool,
    app: &AppHandle,
) -> Result<SyncHealth, String> {
    let db_state = app.state::<AppDb>();
    let coordinator = app.state::<SyncCoordinator>();
    // 读取配置后立即释放主连接；目录扫描与写入使用独立 WAL 连接。
    let source = {
        let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
        load_source(&db, &source_id)?
    };
    if source.id == "codex-local" || source.agent_id == "codex" {
        return Err("Codex 已恢复为专属直读链路，不参与通用 Agent 同步".into());
    }
    let db = Connection::open(crate::data_path()).map_err(|error| error.to_string())?;
    db.busy_timeout(Duration::from_secs(3))
        .map_err(|error| error.to_string())?;
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
        .map_err(|error| error.to_string())?;
    sync_one(&source, force, &db, app, &coordinator)
}

async fn execute_source_task(
    source_id: String,
    force: bool,
    app: AppHandle,
) -> Result<SyncHealth, String> {
    let coordinator = app.state::<SyncCoordinator>();
    // 先登记来源，再等待并发槽位。否则大量相同请求会全部排队，
    // 在前一个任务结束后又逐个重复扫描同一目录。
    {
        let mut active = coordinator
            .active_sources
            .lock()
            .map_err(|_| "同步来源状态锁定失败".to_string())?;
        if !active.insert(source_id.clone()) {
            return coordinator
                .health
                .lock()
                .ok()
                .and_then(|rows| rows.get(&source_id).cloned())
                .ok_or_else(|| "相同来源已在同步，当前请求已合并".to_string());
        }
    }
    let permit = match coordinator.source_slots.clone().acquire_owned().await {
        Ok(permit) => permit,
        Err(_) => {
            if let Ok(mut active) = coordinator.active_sources.lock() {
                active.remove(&source_id);
            }
            return Err("同步队列已关闭".into());
        }
    };
    let task_source = source_id.clone();
    let task_app = app.clone();
    let work = tauri::async_runtime::spawn_blocking(move || {
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            sync_source_blocking(task_source, force, &task_app)
        }))
        .map_err(|_| "解析器发生异常，已隔离该来源".to_string())?
    });
    let result = match tokio::time::timeout(Duration::from_secs(22), work).await {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => Err(format!("同步工作线程异常：{error}")),
        Err(_) => Err("单一来源同步超过20秒，已标记为超时".into()),
    };
    if let Ok(mut active) = coordinator.active_sources.lock() {
        active.remove(&source_id);
    }
    drop(permit);
    result
}

#[tauri::command]
pub async fn sync_source(
    source_id: String,
    force: Option<bool>,
    app: AppHandle,
) -> Result<SyncHealth, String> {
    execute_source_task(source_id, force.unwrap_or(false), app).await
}

fn sync_all_sources_blocking(force: bool, app: &AppHandle) -> Result<Vec<SyncHealth>, String> {
    let db_state = app.state::<AppDb>();
    let ids = {
        let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
        let mut statement = db
            .prepare("SELECT id FROM usage_sources WHERE enabled=1 AND id<>'codex-local' AND agent_id<>'codex' ORDER BY provider,id")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        rows
    };
    let mut results = Vec::new();
    for id in ids {
        if let Ok(health) = sync_source_blocking(id, force, app) {
            results.push(health);
        }
    }
    Ok(results)
}

#[tauri::command]
pub async fn sync_all_sources(
    force: Option<bool>,
    app: AppHandle,
) -> Result<Vec<SyncHealth>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        sync_all_sources_blocking(force.unwrap_or(false), &app)
    })
    .await
    .map_err(|error| format!("本地同步线程异常：{error}"))?
}

fn emit_job(app: &AppHandle, job: &SyncJob) {
    let _ = app.emit("sync-job-updated", job);
}

fn snapshot_enabled_source_ids(app: &AppHandle) -> Result<Vec<String>, String> {
    let db_state = app.state::<AppDb>();
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    let mut statement = db
        .prepare("SELECT id FROM usage_sources WHERE enabled=1 AND id<>'codex-local' AND agent_id<>'codex' ORDER BY provider,id")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

async fn run_sync_job(job_id: String, force: bool, app: AppHandle) {
    let ids = app
        .state::<SyncCoordinator>()
        .jobs
        .lock()
        .ok()
        .and_then(|jobs| jobs.get(&job_id).map(|job| job.source_ids.clone()))
        .unwrap_or_default();
    for chunk in ids.chunks(2) {
        let cancelled = app
            .state::<SyncCoordinator>()
            .cancelled_jobs
            .lock()
            .map(|rows| rows.contains(&job_id))
            .unwrap_or(true);
        if cancelled {
            break;
        }
        let tasks = chunk
            .iter()
            .cloned()
            .map(|source_id| execute_source_task(source_id, force, app.clone()));
        let results = futures_util::future::join_all(tasks).await;
        let coordinator = app.state::<SyncCoordinator>();
        if let Ok(mut jobs) = coordinator.jobs.lock() {
            if let Some(job) = jobs.get_mut(&job_id) {
                job.completed = (job.completed + results.len()).min(job.total);
                for result in results {
                    if let Err(error) = result {
                        job.last_error = Some(error);
                    }
                }
                emit_job(&app, job);
            }
        };
    }
    let coordinator = app.state::<SyncCoordinator>();
    let cancelled = coordinator
        .cancelled_jobs
        .lock()
        .map(|rows| rows.contains(&job_id))
        .unwrap_or(true);
    if let Ok(mut jobs) = coordinator.jobs.lock() {
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = if cancelled { "cancelled" } else { "completed" }.into();
            job.finished_at = Some(Utc::now().to_rfc3339());
            emit_job(&app, job);
        }
    };
}

fn create_sync_job(
    kind: &str,
    source_ids: Vec<String>,
    app: &AppHandle,
) -> Result<(SyncJob, bool), String> {
    let coordinator = app.state::<SyncCoordinator>();
    let mut jobs = coordinator
        .jobs
        .lock()
        .map_err(|_| "同步任务状态锁定失败".to_string())?;
    if let Some(existing) = jobs.values().find(|job| {
        job.kind == kind
            && job.source_ids == source_ids
            && matches!(job.status.as_str(), "queued" | "running")
    }) {
        return Ok((existing.clone(), false));
    }
    let sequence = coordinator.sequence.fetch_add(1, Ordering::Relaxed);
    let job = SyncJob {
        id: format!("sync-{}-{sequence}", Utc::now().timestamp_millis()),
        kind: kind.into(),
        total: source_ids.len(),
        source_ids,
        status: "queued".into(),
        completed: 0,
        started_at: Utc::now().to_rfc3339(),
        finished_at: None,
        last_error: None,
    };
    jobs.insert(job.id.clone(), job.clone());
    drop(jobs);
    emit_job(app, &job);
    Ok((job, true))
}

#[tauri::command]
pub fn start_sync_all(force: Option<bool>, app: AppHandle) -> Result<SyncJob, String> {
    let source_ids = snapshot_enabled_source_ids(&app)?;
    let (job, is_new) = create_sync_job("all", source_ids, &app)?;
    if !is_new {
        return Ok(job);
    }
    let job_id = job.id.clone();
    let job_app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(mut jobs) = job_app.state::<SyncCoordinator>().jobs.lock() {
            if let Some(current) = jobs.get_mut(&job_id) {
                current.status = "running".into();
                emit_job(&job_app, current);
            }
        }
        run_sync_job(job_id, force.unwrap_or(false), job_app).await;
    });
    Ok(job)
}

#[tauri::command]
pub fn start_sync_source(
    source_id: String,
    force: Option<bool>,
    app: AppHandle,
) -> Result<SyncJob, String> {
    if source_id == "codex-local" {
        return Err("Codex 使用专属直读链路，无需加入通用同步队列".into());
    }
    let source_ids = vec![source_id];
    let (job, is_new) = create_sync_job("source", source_ids, &app)?;
    if !is_new {
        return Ok(job);
    }
    let job_id = job.id.clone();
    let job_app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(mut jobs) = job_app.state::<SyncCoordinator>().jobs.lock() {
            if let Some(current) = jobs.get_mut(&job_id) {
                current.status = "running".into();
                emit_job(&job_app, current);
            }
        }
        run_sync_job(job_id, force.unwrap_or(false), job_app).await;
    });
    Ok(job)
}

#[tauri::command]
pub fn cancel_sync(job_id: String, coordinator: State<SyncCoordinator>) -> Result<(), String> {
    coordinator
        .cancelled_jobs
        .lock()
        .map_err(|_| "同步任务状态锁定失败".to_string())?
        .insert(job_id);
    Ok(())
}

#[tauri::command]
pub fn get_sync_job(
    job_id: String,
    coordinator: State<SyncCoordinator>,
) -> Result<SyncJob, String> {
    coordinator
        .jobs
        .lock()
        .map_err(|_| "同步任务状态锁定失败".to_string())?
        .get(&job_id)
        .cloned()
        .ok_or_else(|| "未找到同步任务".into())
}

#[tauri::command]
pub fn list_usage_v2(days: i64, db_state: State<AppDb>) -> Result<Vec<UsageEventV2>, String> {
    let since = (Utc::now() - chrono::Duration::days(days.clamp(1, 365))).to_rfc3339();
    let db = db_state.0.lock().map_err(|_| "数据库锁定失败")?;
    // 仪表盘只需要最近明细；更早记录在下方按天聚合，完整逐条数据仍保留在 SQLite 和 Excel 导出中。
    const RAW_EVENT_LIMIT: i64 = 500;
    let mut statement = db
        .prepare(
            "SELECT id,provider,model,at,input_tokens,output_tokens,cached_tokens,cost,task,
                source_id,source_kind,accuracy,account_id,collected_at,source_ref,
                price_version,currency,canonical_key,is_shadowed
         FROM usage_events WHERE at>=?1 AND is_shadowed=0 ORDER BY at DESC LIMIT ?2",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![since, RAW_EVENT_LIMIT], |row| {
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
                request_count: 1,
            })
        })
        .map_err(|error| error.to_string())?;
    let mut events = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    if events.len() == RAW_EVENT_LIMIT as usize {
        let cutoff = events
            .last()
            .map(|event| event.usage.at.to_rfc3339())
            .unwrap_or_else(|| since.clone());
        let mut aggregate = db.prepare(
            "SELECT provider,model,substr(at,1,10),SUM(input_tokens),SUM(output_tokens),SUM(cached_tokens),SUM(cost),COUNT(*),
                    source_id,source_kind,accuracy,account_id,MAX(collected_at),price_version,currency
             FROM usage_events
             WHERE at>=?1 AND at<?2 AND is_shadowed=0
             GROUP BY provider,model,substr(at,1,10),source_id,source_kind,accuracy,account_id,price_version,currency
             ORDER BY substr(at,1,10) DESC"
        ).map_err(|error| error.to_string())?;
        let older = aggregate
            .query_map(params![since, cutoff], |row| {
                let provider: String = row.get(0)?;
                let model: String = row.get(1)?;
                let day: String = row.get(2)?;
                let source_id: String = row.get(8)?;
                let account_id: Option<String> = row.get(11)?;
                let id = format!(
                    "aggregate:{source_id}:{account}:{model}:{day}",
                    account = account_id.as_deref().unwrap_or("local")
                );
                let at = DateTime::parse_from_rfc3339(&format!("{day}T12:00:00Z"))
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc);
                Ok(UsageEventV2 {
                    usage: UsageEvent {
                        id: id.clone(),
                        provider,
                        model,
                        at,
                        input: row.get::<_, i64>(3)?.max(0) as u64,
                        output: row.get::<_, i64>(4)?.max(0) as u64,
                        cached: row.get::<_, i64>(5)?.max(0) as u64,
                        cost: row.get(6)?,
                        task: "历史日聚合".into(),
                    },
                    source_id,
                    source_kind: row.get(9)?,
                    accuracy: row.get(10)?,
                    account_id,
                    collected_at: row.get(12)?,
                    source_ref: Some("daily-aggregate".into()),
                    price_version: row.get(13)?,
                    currency: row.get(14)?,
                    canonical_key: id,
                    is_shadowed: false,
                    request_count: row.get::<_, i64>(7)?.max(1) as u64,
                })
            })
            .map_err(|error| error.to_string())?;
        events.extend(
            older
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?,
        );
    }
    Ok(events)
}

pub fn restore_watchers(app: &AppHandle, db: &AppDb, coordinator: &SyncCoordinator) {
    let Ok(db) = db.0.lock() else { return };
    let Ok(mut statement) = db.prepare(
        "SELECT id,provider,kind,mode,path,enabled,interval_seconds,agent_id,collector_kind,paths_json,capabilities_json,detected,schema_version,path_mode,path_spec_ids_json FROM usage_sources WHERE enabled=1 AND (path<>'' OR paths_json<>'[]' OR path_spec_ids_json<>'[]')",
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
            agent_id: row.get(7)?,
            collector_kind: row.get(8)?,
            paths: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
            capabilities: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
            detected: row.get::<_, i64>(11)? != 0,
            schema_version: row.get::<_, i64>(12)?.max(1) as u32,
            path_mode: row.get(13)?,
            path_spec_ids: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
        })
    }) else {
        return;
    };
    let sources = rows.flatten().collect::<Vec<_>>();
    drop(statement);
    drop(db);
    for source in sources.into_iter().map(resolve_automatic_paths) {
        let _ = install_watcher(&source, app, coordinator);
    }
}

/// 文件监听在休眠、网络盘或部分编辑器的原子替换场景下可能丢失事件。
/// 由 Rust 主进程统一执行 30 秒安全补扫，避免两个 Vue 窗口分别调度。
pub fn start_safety_rescan(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        if app.get_webview_window("main").is_none() {
            break;
        }
        let task_app = app.clone();
        tauri::async_runtime::spawn(async move {
            let Ok(ids) = snapshot_enabled_source_ids(&task_app) else {
                return;
            };
            for chunk in ids.chunks(2) {
                let tasks = chunk
                    .iter()
                    .cloned()
                    .map(|source_id| execute_source_task(source_id, false, task_app.clone()));
                let _ = futures_util::future::join_all(tasks).await;
            }
        });
    });
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

    #[test]
    fn one_source_is_coalesced_during_click_storm() {
        let coordinator = SyncCoordinator::default();
        let mut accepted = 0;
        for _ in 0..100 {
            if coordinator
                .active_sources
                .lock()
                .unwrap()
                .insert("agent-opencode".into())
            {
                accepted += 1;
            }
        }
        assert_eq!(accepted, 1);
    }
}
