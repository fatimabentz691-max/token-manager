mod providers;
mod reliability;

use axum::{
    body::Body,
    extract::State as AxumState,
    http::{Request, Response, StatusCode},
    routing::any,
    Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use rusqlite::{params, Connection};
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    collections::HashMap,
    fs,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageEvent {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub at: DateTime<Utc>,
    pub input: u64,
    pub output: u64,
    pub cached: u64,
    pub cost: f64,
    pub task: String,
}

#[derive(Debug, Serialize)]
pub struct OpenCodeLocalSyncResult {
    pub imported: usize,
    pub scanned_files: usize,
    pub source: String,
    pub detail: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FloatingRendererConfig {
    enabled: bool,
    quality: String,
    tone: String,
    transparency: f64,
    distortion: f64,
}

#[derive(Debug, Serialize, Clone)]
struct FloatingRenderStatus {
    backend: String,
    state: String,
    fps: u32,
    detail: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FloatingSurface {
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
    strength: f64,
}

struct FloatingWindowRuntime {
    renderer: Mutex<FloatingRenderStatus>,
    surfaces: Mutex<Vec<FloatingSurface>>,
    interaction: Mutex<String>,
    radius: Mutex<f64>,
    snap_to_edges: Mutex<bool>,
}

impl Default for FloatingWindowRuntime {
    fn default() -> Self {
        Self {
            renderer: Mutex::new(FloatingRenderStatus {
                backend: "translucent".into(),
                state: "stopped".into(),
                fps: 0,
                detail: "悬浮窗尚未显示".into(),
            }),
            surfaces: Mutex::new(Vec::new()),
            interaction: Mutex::new("interactive".into()),
            radius: Mutex::new(24.0),
            snap_to_edges: Mutex::new(true),
        }
    }
}

#[derive(Debug, Serialize)]
struct PreparedLiquidVideo {
    path: String,
    file_name: String,
    size_bytes: u64,
    extension: String,
}

/// 校验并仅向 Tauri asset 协议放行用户选中的单个本地视频。
/// 这里只保存路径，不读取视频正文，更不会把大文件写入 localStorage 或上传网络。
#[tauri::command]
fn prepare_liquid_background_video(
    app: AppHandle,
    path: String,
) -> Result<PreparedLiquidVideo, String> {
    const MAX_VIDEO_BYTES: u64 = 1024 * 1024 * 1024;
    let requested = PathBuf::from(path.trim());
    if path.trim().is_empty() {
        return Err("请选择视频文件".into());
    }
    let canonical =
        fs::canonicalize(&requested).map_err(|error| format!("无法读取所选视频文件：{error}"))?;
    let metadata =
        fs::metadata(&canonical).map_err(|error| format!("无法读取视频文件信息：{error}"))?;
    if !metadata.is_file() {
        return Err("所选路径不是文件".into());
    }
    if metadata.len() == 0 {
        return Err("视频文件为空".into());
    }
    if metadata.len() > MAX_VIDEO_BYTES {
        return Err("视频文件不能超过 1 GB".into());
    }
    let extension = canonical
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "mp4" | "webm" | "mov" | "m4v") {
        return Err("仅支持 MP4、WebM、MOV 或 M4V 视频".into());
    }
    app.asset_protocol_scope()
        .allow_file(&canonical)
        .map_err(|error| format!("无法授权本地视频播放：{error}"))?;
    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("本地视频")
        .to_string();
    Ok(PreparedLiquidVideo {
        path: canonical.to_string_lossy().into_owned(),
        file_name,
        size_bytes: metadata.len(),
        extension,
    })
}

/// 从 Codex 本地状态库读取的可验证汇总，不包含服务端会员额度。
#[derive(Debug, Serialize)]
pub struct CodexSnapshot {
    pub total_tokens: u64,
    pub active_thread_tokens: u64,
    pub updated_at_ms: i64,
    pub source: String,
}
#[derive(Debug, Serialize)]
pub struct CodexBudget {
    pub five_hour_limit: u64,
    pub seven_day_limit: u64,
}
#[derive(Debug, Serialize)]
pub struct CodexWindowUsage {
    pub five_hour_used: u64,
    pub seven_day_used: u64,
    pub budget: CodexBudget,
    pub source: String,
}
#[derive(Debug, Serialize)]
pub struct CodexUsagePoint {
    pub at: i64,
    pub tokens: u64,
    pub model: String,
    pub session_id: String,
    pub category: String,
    pub temperature: Option<f64>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodexQuotaWindow {
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub window_minutes: u64,
    pub resets_at: i64,
}
#[derive(Debug, Serialize, Clone)]
pub struct CodexQuotaSnapshot {
    pub five_hour: Option<CodexQuotaWindow>,
    pub seven_day: Option<CodexQuotaWindow>,
    pub plan_type: Option<String>,
    pub credit_balance: Option<String>,
    pub observed_at: i64,
    pub source: String,
}
#[derive(Debug, Serialize)]
pub struct AccountSummary {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub base_url: String,
    pub created_at: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountBalance {
    pub account_id: String,
    pub provider: String,
    pub currency: String,
    pub total_balance: f64,
    pub granted_balance: f64,
    pub topped_up_balance: f64,
    pub is_available: bool,
    pub synced_at: String,
}
#[derive(Debug, Serialize)]
pub struct BalancePoint {
    pub at: String,
    pub total_balance: f64,
}
#[derive(Debug, Serialize, Clone)]
pub struct ProxyEndpoint {
    pub account_id: String,
    pub provider: String,
    pub name: String,
    pub local_url: String,
    pub anthropic_url: Option<String>,
    pub port: u16,
}
#[derive(Debug, Serialize, Clone)]
pub struct ProxyTrafficEvent {
    pub account_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub at: String,
}
#[derive(Debug, Serialize)]
pub struct CcSwitchStatus {
    pub installed: bool,
    pub running: bool,
    pub local_routing: bool,
    pub detail: String,
    pub checked_at: String,
}
#[derive(Debug, Serialize)]
pub struct ClientIntegrationResult {
    pub connected: bool,
    pub account_name: String,
    pub openai_url: String,
    pub anthropic_url: Option<String>,
    pub changed: Vec<String>,
    pub restart_required: bool,
    pub detail: String,
}
#[derive(Debug, Serialize, Deserialize, Default)]
struct ClientIntegrationBackup {
    openai_base_url: Option<String>,
    openai_api_base: Option<String>,
    anthropic_base_url: Option<String>,
    claude_settings_path: Option<String>,
    claude_settings_original: Option<String>,
    claude_settings_existed: bool,
    applied_at: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct BackupAccount {
    id: String,
    provider: String,
    name: String,
    base_url: String,
    api_key: String,
    created_at: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct BackupBundle {
    version: u8,
    exported_at: String,
    accounts: Vec<BackupAccount>,
    usage: Vec<UsageEvent>,
    balances: Vec<AccountBalance>,
    five_hour_limit: u64,
    seven_day_limit: u64,
    ui_state: serde_json::Value,
}
#[derive(Debug, Serialize, Deserialize)]
struct BackupEnvelope {
    version: u8,
    salt: String,
    nonce: String,
    ciphertext: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CloudSessionSummary {
    pub email: String,
    pub base_url: String,
    pub expires_at: String,
}
#[derive(Debug, Deserialize)]
struct CloudUser {
    email: String,
}
#[derive(Debug, Deserialize)]
struct CloudAuthReply {
    access_token: String,
    expires_at: String,
    user: CloudUser,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CloudCodeReply {
    pub expires_in: u64,
    pub debug_code: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CloudTransferReply {
    pub id: String,
    pub link: String,
    pub expires_at: String,
    pub one_time: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteContentItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub action_label: String,
    pub action_url: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Deserialize)]
struct RemoteContentReply {
    items: Vec<RemoteContentItem>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdapterCapability {
    pub provider: String,
    pub billing: bool,
    pub proxy: bool,
    pub import: bool,
    pub note: String,
}
pub trait ProviderAdapter: Send + Sync {
    fn capability(&self) -> AdapterCapability;
    fn normalize(&self, value: &serde_json::Value) -> Option<UsageEvent>;
}
pub struct RegistryAdapter {
    id: &'static str,
    billing: bool,
}
impl ProviderAdapter for RegistryAdapter {
    fn capability(&self) -> AdapterCapability {
        AdapterCapability {
            provider: self.id.into(),
            billing: self.billing,
            proxy: true,
            import: true,
            note: if self.billing {
                "需使用对应官方账单接口及账户权限".into()
            } else {
                "使用本地代理或账单导入统计".into()
            },
        }
    }
    fn normalize(&self, value: &serde_json::Value) -> Option<UsageEvent> {
        Some(UsageEvent {
            id: uuid_like(),
            provider: self.id.into(),
            model: value.get("model")?.as_str()?.into(),
            at: Utc::now(),
            input: value
                .pointer("/usage/prompt_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            output: value
                .pointer("/usage/completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            cached: 0,
            cost: 0.0,
            task: "其他".into(),
        })
    }
}
fn uuid_like() -> String {
    format!("evt-{}", Utc::now().timestamp_micros())
}

#[derive(Debug, Serialize)]
struct UpdatePreflightResult {
    size_bytes: u64,
    sha256: String,
}

/// 在交给 Tauri Updater 做签名校验前，先从同一 HTTPS 地址流式复核大小与 SHA-256。
/// 数据只经过内存分块，不写入磁盘；失败时更新流程会立即终止。
#[tauri::command]
async fn verify_update_artifact(
    url: String,
    expected_sha256: String,
    expected_size_bytes: u64,
) -> Result<UpdatePreflightResult, String> {
    if !url.starts_with("https://") {
        return Err("更新包必须使用 HTTPS 地址".into());
    }
    let expected_hash = expected_sha256.trim().to_ascii_uppercase();
    if expected_hash.len() != 64 || !expected_hash.chars().all(|value| value.is_ascii_hexdigit()) {
        return Err("发布清单中的 SHA-256 无效".into());
    }
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10 * 60))
        .build()
        .map_err(|error| error.to_string())?
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("无法下载更新包进行校验：{error}"))?
        .error_for_status()
        .map_err(|error| format!("更新包地址不可用：{error}"))?;
    let mut stream = response.bytes_stream();
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("读取更新包失败：{error}"))?;
        size = size.saturating_add(chunk.len() as u64);
        if size > 1024 * 1024 * 1024 {
            return Err("更新包超过 1 GB 安全限制".into());
        }
        hasher.update(&chunk);
    }
    let actual_hash = format!("{:X}", hasher.finalize());
    if expected_size_bytes > 0 && size != expected_size_bytes {
        return Err(format!(
            "更新包大小不一致：期望 {expected_size_bytes}，实际 {size}"
        ));
    }
    if actual_hash != expected_hash {
        return Err("更新包 SHA-256 校验失败，已禁止安装".into());
    }
    Ok(UpdatePreflightResult {
        size_bytes: size,
        sha256: actual_hash,
    })
}
pub(crate) struct AppDb(pub(crate) Mutex<Connection>);
pub struct ProxyServerState(Mutex<HashMap<u16, String>>);
impl AppDb {
    fn open() -> Self {
        let path = data_path();
        let db = Connection::open(path).expect("无法打开本地数据库");
        db.execute_batch("CREATE TABLE IF NOT EXISTS usage_events(id TEXT PRIMARY KEY,provider TEXT,model TEXT,at TEXT,input_tokens INTEGER,output_tokens INTEGER,cached_tokens INTEGER,cost REAL,task TEXT); CREATE TABLE IF NOT EXISTS accounts(id TEXT PRIMARY KEY,provider TEXT,name TEXT,secret_cipher BLOB,created_at TEXT); CREATE TABLE IF NOT EXISTS account_configs(id TEXT PRIMARY KEY,provider TEXT NOT NULL,name TEXT NOT NULL,base_url TEXT NOT NULL,secret_cipher BLOB NOT NULL,created_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS codex_budget(id INTEGER PRIMARY KEY CHECK(id=1),five_hour_limit INTEGER NOT NULL,seven_day_limit INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS account_balances(account_id TEXT NOT NULL,provider TEXT NOT NULL,currency TEXT NOT NULL,total_balance REAL NOT NULL,granted_balance REAL NOT NULL,topped_up_balance REAL NOT NULL,is_available INTEGER NOT NULL,synced_at TEXT NOT NULL,PRIMARY KEY(account_id,currency)); CREATE TABLE IF NOT EXISTS balance_history(id INTEGER PRIMARY KEY AUTOINCREMENT,account_id TEXT NOT NULL,currency TEXT NOT NULL,total_balance REAL NOT NULL,at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS cloud_session(id INTEGER PRIMARY KEY CHECK(id=1),email TEXT NOT NULL,base_url TEXT NOT NULL,token_cipher BLOB NOT NULL,expires_at TEXT NOT NULL,updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS app_identity(id INTEGER PRIMARY KEY CHECK(id=1),install_id TEXT NOT NULL,created_at TEXT NOT NULL); CREATE INDEX IF NOT EXISTS idx_balance_history_account_at ON balance_history(account_id,at);").expect("无法初始化数据库");
        reliability::migrate_database(&db).expect("无法升级本地数据库到 v11");
        db.execute("INSERT OR IGNORE INTO codex_budget(id,five_hour_limit,seven_day_limit) VALUES(1,100000,1000000)",[]).expect("无法创建默认预算");
        Self(Mutex::new(db))
    }
}
fn data_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let root = base.join("Token Manager");
    fs::create_dir_all(&root).ok();
    let next = root.join("token-manager.db");
    let legacy = base.join("TokenLens").join("tokenlens.db");
    if !next.exists() && legacy.exists() {
        fs::copy(legacy, &next).ok();
    }
    next
}
#[derive(Clone)]
struct ProxyRuntime {
    upstream: String,
    provider: String,
    account_id: Option<String>,
    api_key: Option<String>,
    client: reqwest::Client,
    app: AppHandle,
}
fn deepseek_cost(model: &str, input: u64, output: u64, cached: u64) -> f64 {
    let pro = model.to_ascii_lowercase().starts_with("deepseek-v4-pro");
    let (hit, miss, out) = if pro {
        (0.025, 3.0, 6.0)
    } else {
        (0.02, 1.0, 2.0)
    };
    let uncached = input.saturating_sub(cached);
    (cached as f64 * hit + uncached as f64 * miss + output as f64 * out) / 1_000_000.0
}
/// OpenCode Go 官方套餐按美元价值核减额度。这里使用官方文档公开的每百万 Token 单价，
/// 只计算本机代理实际观察到的输入、输出与缓存读取；缓存写入未在响应 usage 中提供，因此不伪造。
fn opencode_go_cost_usd(model: &str, input: u64, output: u64, cached: u64) -> f64 {
    let normalized = model
        .trim()
        .trim_start_matches("opencode-go/")
        .to_ascii_lowercase();
    let (hit, miss, out) = match normalized.as_str() {
        "grok-4.5" => (0.30, 2.00, 6.00),
        "gpt-5.6-luna" => (0.02, 0.20, 1.20),
        "glm-5.2" | "glm-5.1" => (0.26, 1.40, 4.40),
        "kimi-k3" => (0.30, 3.00, 15.00),
        "kimi-k2.7-code" => (0.19, 0.95, 4.00),
        "kimi-k2.6" => (0.16, 0.95, 4.00),
        "mimo-v2.5" | "deepseek-v4-flash" => (0.0028, 0.14, 0.28),
        "mimo-v2.5-pro" | "deepseek-v4-pro" => (0.003625, 0.435, 0.87),
        "minimax-m3" | "minimax-m2.7" | "minimax-m2.5" => (0.06, 0.30, 1.20),
        "qwen3.8-max" => (0.25, 2.00, 6.00),
        "qwen3.7-max" => (0.50, 2.50, 7.50),
        "qwen3.7-plus" => (0.04, 0.40, 1.60),
        "qwen3.6-plus" => (0.05, 0.50, 3.00),
        "hy3" => (0.035, 0.14, 0.58),
        _ => return 0.0,
    };
    let uncached = input.saturating_sub(cached);
    (cached as f64 * hit + uncached as f64 * miss + output as f64 * out) / 1_000_000.0
}
/// DeepSeek 的 Anthropic 兼容接口会把 Claude 模型别名映射到 V4。
/// 统一归档名称，避免同一次调用落到 “claude-opus” 而不是 V4 Pro 仪表盘。
fn normalized_proxy_model(provider: &str, model: &str) -> String {
    if provider != "DeepSeek" {
        return model.to_string();
    }
    let lower = model.to_ascii_lowercase();
    if lower.starts_with("claude-opus") || lower.starts_with("deepseek-v4-pro") {
        "deepseek-v4-pro".into()
    } else if lower.starts_with("claude-sonnet")
        || lower.starts_with("claude-haiku")
        || lower.starts_with("deepseek-v4-flash")
        || lower == "deepseek-chat"
        || lower == "deepseek-reasoner"
    {
        "deepseek-v4-flash".into()
    } else {
        model.to_string()
    }
}
/// 离线版本化人民币参考价（元/百万 Token）；DeepSeek 使用已核验规则，其余平台明确标记为内置参考价。
fn provider_cost(provider: &str, model: &str, input: u64, output: u64, cached: u64) -> f64 {
    if provider == "DeepSeek" {
        return deepseek_cost(model, input, output, cached);
    }
    if provider == "OpenCode Go" {
        // 主账单统一以人民币展示；套餐卡片会按同一固定参考汇率还原美元额度占用。
        return opencode_go_cost_usd(model, input, output, cached) * 7.2;
    }
    let (hit, miss, out) = match provider {
        "腾讯混元" => (1.0, 4.0, 12.0),
        "豆包（火山方舟）" => (0.2, 0.8, 2.0),
        "文心千帆" => (1.0, 4.0, 16.0),
        "通义百炼" => (0.5, 2.0, 8.0),
        "智谱 AI" => (1.0, 4.0, 16.0),
        "Kimi" => (1.0, 4.0, 16.0),
        "小米 MiMo" => (0.1, 1.0, 3.0),
        "讯飞星火" => (1.0, 4.0, 12.0),
        "MiniMax" => (0.5, 2.0, 8.0),
        "阶跃星辰" => (0.8, 3.0, 12.0),
        "零一万物" => (0.5, 2.0, 8.0),
        "商汤日日新" => (1.0, 4.0, 12.0),
        "百川智能" => (1.0, 4.0, 12.0),
        "OpenAI" => (4.5, 18.0, 72.0),
        "Anthropic" => (2.16, 21.6, 108.0),
        "Google Gemini" => (2.25, 9.0, 72.0),
        _ => return 0.0,
    };
    let uncached = input.saturating_sub(cached);
    (cached as f64 * hit + uncached as f64 * miss + output as f64 * out) / 1_000_000.0
}
fn usage_values(body: &[u8]) -> Vec<serde_json::Value> {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
        return match value {
            serde_json::Value::Array(rows) => rows,
            _ => vec![value],
        };
    }
    let text = String::from_utf8_lossy(body);
    text.lines()
        .filter_map(|line| {
            let data = line.trim().strip_prefix("data:")?.trim();
            if data == "[DONE]" {
                None
            } else {
                serde_json::from_str::<serde_json::Value>(data).ok()
            }
        })
        .collect()
}
fn usage_counts(value: &serde_json::Value) -> Option<(u64, u64, u64)> {
    fn counts(usage: &serde_json::Value) -> (u64, u64, u64) {
        let input = usage
            .get("prompt_tokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let output = usage
            .get("completion_tokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cached = usage
            .get("prompt_cache_hit_tokens")
            .or_else(|| usage.get("cache_read_input_tokens"))
            .or_else(|| usage.pointer("/prompt_tokens_details/cached_tokens"))
            .or_else(|| usage.pointer("/input_tokens_details/cached_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        (input, output, cached)
    }
    if let Some(usage) = value.get("usage").filter(|usage| !usage.is_null()) {
        return Some(counts(usage));
    }
    // Anthropic SSE 的 message_start 把输入 Token 放在 message.usage 内，
    // message_delta 则把输出 Token 放在顶层 usage；两者后续按最大值合并。
    if let Some(usage) = value.pointer("/message/usage") {
        return Some(counts(usage));
    }
    if let Some(usage) = value.get("usageMetadata") {
        let input = usage
            .get("promptTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let output = usage
            .get("candidatesTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            + usage
                .get("thoughtsTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
        let cached = usage
            .get("cachedContentTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        return Some((input, output, cached));
    }
    None
}
fn response_usage(
    body: &[u8],
    model: &str,
    provider: &str,
    status: u16,
    account_id: Option<&str>,
) -> Option<String> {
    let mut input = 0;
    let mut output = 0;
    let mut cached = 0;
    for value in usage_values(body) {
        if let Some((next_input, next_output, next_cached)) = usage_counts(&value) {
            input = input.max(next_input);
            output = output.max(next_output);
            cached = cached.max(next_cached)
        }
    }
    let cost = provider_cost(provider, model, input, output, cached);
    let task = if status >= 400 { "失败" } else { "其他" };
    let Ok(db) = Connection::open(data_path()) else {
        return None;
    };
    let id = uuid_like();
    let event = UsageEvent {
        id: id.clone(),
        provider: provider.to_string(),
        model: model.to_string(),
        at: Utc::now(),
        input,
        output,
        cached,
        cost,
        task: task.to_string(),
    };
    let mut provenance = reliability::UsageProvenance::observed(
        account_id
            .map(|value| format!("proxy-{value}"))
            .unwrap_or_else(|| {
                format!("proxy-{}", provider.to_ascii_lowercase().replace(' ', "-"))
            }),
        reliability::SourceKind::LocalProxy,
    );
    provenance.account_id = account_id.map(str::to_string);
    provenance.source_ref = Some(id.clone());
    reliability::upsert_usage(&db, &event, &provenance).ok()?;
    Some(id)
}

async fn fetch_and_store_deepseek_balance(
    account_id: &str,
    base_url: &str,
    key: &str,
) -> Result<Vec<AccountBalance>, String> {
    let base = base_url.trim_end_matches('/').trim_end_matches("/v1");
    let value = reqwest::Client::new()
        .get(format!("{base}/user/balance"))
        .bearer_auth(key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("DeepSeek 余额请求失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("DeepSeek 余额接口返回错误：{e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("DeepSeek 余额数据无法解析：{e}"))?;
    let available = value
        .get("is_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let now = Utc::now().to_rfc3339();
    let mut rows = Vec::new();
    for item in value
        .get("balance_infos")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let currency = item
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("CNY")
            .to_string();
        let parse = |name: &str| {
            item.get(name)
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        rows.push(AccountBalance {
            account_id: account_id.into(),
            provider: "DeepSeek".into(),
            currency,
            total_balance: parse("total_balance"),
            granted_balance: parse("granted_balance"),
            topped_up_balance: parse("topped_up_balance"),
            is_available: available,
            synced_at: now.clone(),
        });
    }
    let db = Connection::open(data_path()).map_err(|e| e.to_string())?;
    for row in &rows {
        db.execute("INSERT OR REPLACE INTO account_balances(account_id,provider,currency,total_balance,granted_balance,topped_up_balance,is_available,synced_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![row.account_id,row.provider,row.currency,row.total_balance,row.granted_balance,row.topped_up_balance,row.is_available as i32,row.synced_at]).map_err(|e|e.to_string())?;
        db.execute("INSERT INTO balance_history(account_id,currency,total_balance,at) SELECT ?1,?2,?3,?4 WHERE NOT EXISTS(SELECT 1 FROM balance_history WHERE account_id=?1 AND currency=?2 AND total_balance=?3 AND at>=datetime(?4,'-30 seconds'))",params![row.account_id,row.currency,row.total_balance,row.synced_at]).map_err(|e|e.to_string())?;
    }
    Ok(rows)
}
async fn proxy_request(
    AxumState(runtime): AxumState<ProxyRuntime>,
    request: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let (parts, body) = request.into_parts();
    let payload = axum::body::to_bytes(body, 20 * 1024 * 1024)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let path = parts
        .uri
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or("/");
    let mut payload_value = serde_json::from_slice::<serde_json::Value>(&payload).ok();
    let raw_model = payload_value
        .as_ref()
        .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(str::to_owned))
        .or_else(|| {
            path.split("/models/")
                .nth(1)
                .and_then(|v| v.split([':', '?']).next())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".into());
    let model = normalized_proxy_model(&runtime.provider, &raw_model);
    // DeepSeek 的 OpenAI 流式接口只有 include_usage=true 才保证在 [DONE] 前返回总 Token。
    // 仅修改代理转发副本，不保存提示词或响应正文。
    if runtime.provider == "DeepSeek"
        && path.contains("/completions")
        && payload_value
            .as_ref()
            .and_then(|v| v.get("stream"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    {
        if let Some(value) = payload_value.as_mut().and_then(|v| v.as_object_mut()) {
            let options = value
                .entry("stream_options")
                .or_insert_with(|| serde_json::json!({}));
            if let Some(options) = options.as_object_mut() {
                options.insert("include_usage".into(), serde_json::Value::Bool(true));
            }
        }
    }
    let payload = payload_value
        .and_then(|value| serde_json::to_vec(&value).ok())
        .unwrap_or_else(|| payload.to_vec());
    runtime
        .app
        .emit(
            "proxy-request-started",
            ProxyTrafficEvent {
                account_id: runtime.account_id.clone(),
                provider: runtime.provider.clone(),
                model: model.clone(),
                at: Utc::now().to_rfc3339(),
            },
        )
        .ok();
    let base = runtime.upstream.trim_end_matches('/');
    let path = if ["/v1", "/v2", "/v3", "/v4"]
        .iter()
        .any(|suffix| base.ends_with(suffix))
    {
        path.strip_prefix("/v1").unwrap_or(path)
    } else {
        path
    };
    let url = format!("{base}{path}");
    let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let mut outbound = runtime.client.request(method, url).body(payload);
    for (name, value) in parts.headers.iter() {
        if name.as_str().eq_ignore_ascii_case("host")
            || name.as_str().eq_ignore_ascii_case("content-length")
            || (runtime.api_key.is_some()
                && (name.as_str().eq_ignore_ascii_case("authorization")
                    || name.as_str().eq_ignore_ascii_case("x-api-key")))
        {
            continue;
        }
        outbound = outbound.header(name, value);
    }
    if let Some(key) = &runtime.api_key {
        outbound = outbound.bearer_auth(key).header("x-api-key", key);
    }
    let mut remote = match outbound.send().await {
        Ok(response) => response,
        Err(error) => {
            let event_id = response_usage(
                &[],
                &model,
                &runtime.provider,
                StatusCode::BAD_GATEWAY.as_u16(),
                runtime.account_id.as_deref(),
            );
            runtime
                .app
                .emit(
                    "usage-updated",
                    serde_json::json!({
                        "event_id": event_id,
                        "provider": runtime.provider,
                        "account_id": runtime.account_id,
                        "at": Utc::now().to_rfc3339(),
                    }),
                )
                .ok();
            return Err((StatusCode::BAD_GATEWAY, format!("上游请求失败：{error}")));
        }
    };
    let status = remote.status();
    let headers = remote.headers().clone();
    let is_event_stream = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase().contains("text/event-stream"))
        .unwrap_or(false);
    let mut builder = Response::builder().status(status);
    for (name, value) in headers.iter() {
        if !name.as_str().eq_ignore_ascii_case("content-length") {
            builder = builder.header(name, value);
        }
    }
    if is_event_stream {
        let (sender, receiver) =
            tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::io::Error>>(24);
        let stream_runtime = runtime.clone();
        let stream_model = model.clone();
        tauri::async_runtime::spawn(async move {
            let mut captured = Vec::new();
            loop {
                match remote.chunk().await {
                    Ok(Some(bytes)) => {
                        captured.extend_from_slice(&bytes);
                        if sender.send(Ok(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        let _ = sender
                            .send(Err(std::io::Error::other(error.to_string())))
                            .await;
                        break;
                    }
                }
            }
            let event_id = response_usage(
                &captured,
                &stream_model,
                &stream_runtime.provider,
                status.as_u16(),
                stream_runtime.account_id.as_deref(),
            );
            if stream_runtime.provider == "DeepSeek" {
                if let (Some(id), Some(key)) = (&stream_runtime.account_id, &stream_runtime.api_key)
                {
                    let _ =
                        fetch_and_store_deepseek_balance(id, &stream_runtime.upstream, key).await;
                }
            }
            stream_runtime
                .app
                .emit(
                    "usage-updated",
                    serde_json::json!({
                        "event_id": event_id,
                        "provider": stream_runtime.provider,
                        "account_id": stream_runtime.account_id,
                        "at": Utc::now().to_rfc3339(),
                    }),
                )
                .ok();
        });
        let body_stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
            receiver.recv().await.map(|item| (item, receiver))
        });
        return builder
            .body(Body::from_stream(body_stream))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    }
    let bytes = remote
        .bytes()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    let event_id = response_usage(
        &bytes,
        &model,
        &runtime.provider,
        status.as_u16(),
        runtime.account_id.as_deref(),
    );
    if runtime.provider == "DeepSeek" {
        if let (Some(id), Some(key)) = (&runtime.account_id, &runtime.api_key) {
            let _ = fetch_and_store_deepseek_balance(id, &runtime.upstream, key).await;
        }
    }
    runtime
        .app
        .emit(
            "usage-updated",
            serde_json::json!({
                "event_id": event_id,
                "provider": runtime.provider,
                "account_id": runtime.account_id,
                "at": Utc::now().to_rfc3339(),
            }),
        )
        .ok();
    builder
        .body(Body::from(bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
#[tauri::command]
fn start_proxy(
    upstream: String,
    port: u16,
    account_id: Option<String>,
    state: State<ProxyServerState>,
    db_state: State<AppDb>,
    app: AppHandle,
) -> Result<String, String> {
    if !upstream.starts_with("https://")
        && !upstream.starts_with("http://127.0.0.1")
        && !upstream.starts_with("http://localhost")
    {
        return Err("上游地址必须使用 HTTPS，或为本机地址".into());
    }
    let mut running = state.0.lock().map_err(|_| "代理状态锁定")?;
    if running.contains_key(&port) {
        return Err(format!("端口 {port} 的本地代理已在运行"));
    }
    running.insert(port, account_id.clone().unwrap_or_else(|| "custom".into()));
    drop(running);
    let selected_account = account_id.clone();
    let (api_key, provider) = if let Some(id) = account_id {
        let db = db_state.0.lock().map_err(|_| "数据库锁定")?;
        let (cipher, provider): (Vec<u8>, String) = db
            .query_row(
                "SELECT secret_cipher,provider FROM account_configs WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| "未找到已配置账户")?;
        (Some(unprotect_secret(&cipher)?), provider)
    } else {
        (None, "自定义 OpenAI 兼容".into())
    };
    let runtime = ProxyRuntime {
        upstream,
        provider,
        account_id: selected_account,
        api_key,
        client: reqwest::Client::new(),
        app,
    };
    tauri::async_runtime::spawn(async move {
        let router = Router::new()
            .fallback(any(proxy_request))
            .with_state(runtime);
        if let Ok(listener) =
            tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await
        {
            let _ = axum::serve(listener, router).await;
        }
    });
    Ok(format!("http://127.0.0.1:{port}/v1"))
}
#[tauri::command]
fn start_all_proxies(
    state: State<ProxyServerState>,
    db_state: State<AppDb>,
    app: AppHandle,
) -> Result<Vec<ProxyEndpoint>, String> {
    let accounts = {
        let db = db_state.0.lock().map_err(|_| "数据库锁定")?;
        let mut query=db.prepare("SELECT id,provider,name,base_url,secret_cipher FROM account_configs ORDER BY created_at ASC").map_err(|e|e.to_string())?;
        let rows = query
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Vec<u8>>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };
    if accounts.is_empty() {
        return Err("请先在“账户与模型”中添加至少一个 API 账户".into());
    }
    let mut endpoints = Vec::new();
    let mut running = state.0.lock().map_err(|_| "代理状态锁定")?;
    for (index, (id, provider, name, upstream, cipher)) in accounts.into_iter().enumerate() {
        let port = 18765u16.saturating_add(index as u16);
        let local_url = format!("http://127.0.0.1:{port}/v1");
        if !running.contains_key(&port) {
            let key = unprotect_secret(&cipher)?;
            let runtime = ProxyRuntime {
                upstream,
                provider: provider.clone(),
                account_id: Some(id.clone()),
                api_key: Some(key),
                client: reqwest::Client::new(),
                app: app.clone(),
            };
            tauri::async_runtime::spawn(async move {
                let router = Router::new()
                    .fallback(any(proxy_request))
                    .with_state(runtime);
                if let Ok(listener) =
                    tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await
                {
                    let _ = axum::serve(listener, router).await;
                }
            });
            running.insert(port, id.clone());
        }
        endpoints.push(ProxyEndpoint {
            account_id: id,
            anthropic_url: (provider == "DeepSeek")
                .then(|| format!("http://127.0.0.1:{port}/anthropic")),
            provider,
            name,
            local_url,
            port,
        });
    }
    app.emit("proxy-state", endpoints.clone()).ok();
    Ok(endpoints)
}
#[cfg(windows)]
fn protect_secret(value: &str) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB},
    };
    let mut input = value.as_bytes().to_vec();
    let source = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let ok = unsafe {
        CryptProtectData(
            &source,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        return Err("DPAPI 加密失败".into());
    }
    let data =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(data)
}
#[cfg(windows)]
fn unprotect_secret(value: &[u8]) -> Result<String, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB},
    };
    let mut input = value.to_vec();
    let source = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let ok = unsafe {
        CryptUnprotectData(
            &source,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        return Err("DPAPI 解密失败".into());
    }
    let data =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { LocalFree(output.pbData.cast()) };
    String::from_utf8(data).map_err(|_| "密钥格式无效".into())
}
#[cfg(not(windows))]
fn protect_secret(_: &str) -> Result<Vec<u8>, String> {
    Err("仅支持 Windows DPAPI".into())
}
#[cfg(not(windows))]
fn unprotect_secret(_: &[u8]) -> Result<String, String> {
    Err("仅支持 Windows DPAPI".into())
}
#[tauri::command]
fn save_account_config(
    id: String,
    provider: String,
    name: String,
    base_url: String,
    api_key: String,
    state: State<AppDb>,
) -> Result<(), String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let cipher = if api_key.trim().is_empty() {
        db.query_row(
            "SELECT secret_cipher FROM account_configs WHERE id=?1",
            [&id],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .map_err(|_| "新账户必须输入 API Key")?
    } else {
        protect_secret(&api_key)?
    };
    db.execute("INSERT OR REPLACE INTO account_configs(id,provider,name,base_url,secret_cipher,created_at) VALUES(?1,?2,?3,?4,?5,COALESCE((SELECT created_at FROM account_configs WHERE id=?1),?6))",params![id,provider,name,base_url,cipher,Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
    Ok(())
}
#[tauri::command]
fn list_account_configs(state: State<AppDb>) -> Result<Vec<AccountSummary>, String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let mut query=db.prepare("SELECT id,provider,name,base_url,created_at FROM account_configs ORDER BY created_at DESC").map_err(|e|e.to_string())?;
    let result = query
        .query_map([], |r| {
            Ok(AccountSummary {
                id: r.get(0)?,
                provider: r.get(1)?,
                name: r.get(2)?,
                base_url: r.get(3)?,
                created_at: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    result
}
#[tauri::command]
fn delete_account_config(id: String, state: State<AppDb>) -> Result<(), String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    db.execute("DELETE FROM account_configs WHERE id=?1", [&id])
        .map_err(|e| e.to_string())?;
    db.execute("DELETE FROM account_balances WHERE account_id=?1", [&id])
        .map_err(|e| e.to_string())?;
    db.execute("DELETE FROM balance_history WHERE account_id=?1", [&id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
fn clear_account_configs(state: State<AppDb>) -> Result<(), String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    db.execute_batch(
        "DELETE FROM account_configs;DELETE FROM account_balances;DELETE FROM balance_history;",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
async fn sync_account_balances(state: State<'_, AppDb>) -> Result<Vec<AccountBalance>, String> {
    let accounts = {
        let db = state.0.lock().map_err(|_| "数据库锁定")?;
        let mut query = db
            .prepare(
                "SELECT id,base_url,secret_cipher FROM account_configs WHERE provider='DeepSeek'",
            )
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };
    let mut balances = Vec::new();
    let mut errors = Vec::new();
    for (id, base, cipher) in accounts {
        match unprotect_secret(&cipher) {
            Ok(key) => match fetch_and_store_deepseek_balance(&id, &base, &key).await {
                Ok(mut rows) => balances.append(&mut rows),
                Err(error) => errors.push(error),
            },
            Err(error) => errors.push(error),
        }
    }
    if balances.is_empty() && !errors.is_empty() {
        Err(errors.join("；"))
    } else {
        Ok(balances)
    }
}
#[tauri::command]
fn list_account_balances(state: State<AppDb>) -> Result<Vec<AccountBalance>, String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let mut query=db.prepare("SELECT account_id,provider,currency,total_balance,granted_balance,topped_up_balance,is_available,synced_at FROM account_balances ORDER BY synced_at DESC").map_err(|e|e.to_string())?;
    let rows = query
        .query_map([], |r| {
            Ok(AccountBalance {
                account_id: r.get(0)?,
                provider: r.get(1)?,
                currency: r.get(2)?,
                total_balance: r.get(3)?,
                granted_balance: r.get(4)?,
                topped_up_balance: r.get(5)?,
                is_available: r.get::<_, i64>(6)? != 0,
                synced_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}
#[tauri::command]
fn list_balance_history(
    account_id: String,
    days: i64,
    state: State<AppDb>,
) -> Result<Vec<BalancePoint>, String> {
    let since = (Utc::now() - chrono::Duration::days(days.clamp(1, 365))).to_rfc3339();
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let mut query=db.prepare("SELECT at,total_balance FROM balance_history WHERE account_id=?1 AND currency='CNY' AND at>=?2 ORDER BY at ASC").map_err(|e|e.to_string())?;
    let rows = query
        .query_map(params![account_id, since], |r| {
            Ok(BalancePoint {
                at: r.get(0)?,
                total_balance: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}
#[tauri::command]
async fn fetch_account_models(id: String, state: State<'_, AppDb>) -> Result<Vec<String>, String> {
    let (provider, base_url, cipher): (String, String, Vec<u8>) = {
        let db = state.0.lock().map_err(|_| "数据库锁定")?;
        db.query_row(
            "SELECT provider,base_url,secret_cipher FROM account_configs WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "未找到已配置账户")?
    };
    let key = unprotect_secret(&cipher)?;
    let base = base_url.trim_end_matches('/');
    let url = if provider == "Google Gemini" {
        format!("{base}/v1beta/models?key={key}")
    } else if base.ends_with("/v1") || base.ends_with("/v2") || base.ends_with("/v4") {
        format!("{base}/models")
    } else {
        format!("{base}/v1/models")
    };
    let client = reqwest::Client::new();
    let mut request = client.get(url);
    if provider == "Anthropic" {
        request = request
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01");
    } else if provider != "Google Gemini" {
        request = request.bearer_auth(key);
    }
    let value = request
        .send()
        .await
        .map_err(|e| format!("模型列表请求失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("模型列表接口返回错误：{e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("无法解析模型列表：{e}"))?;
    let mut models = if provider == "Google Gemini" {
        value
            .get("models")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| {
                v.get("name")
                    .and_then(|x| x.as_str())
                    .map(|s| s.trim_start_matches("models/").to_string())
            })
            .collect::<Vec<_>>()
    } else {
        value
            .get("data")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| v.get("id").and_then(|x| x.as_str()).map(str::to_string))
            .collect::<Vec<_>>()
    };
    models.sort();
    models.dedup();
    Ok(models)
}
/// CSS 圆角只能裁切 WebView 内容，无法裁掉 Windows Acrylic 的原生矩形合成面。
/// 使用 Win32 窗口区域同步裁切整个悬浮窗，彻底消除四角透明方框残留。
#[cfg(windows)]
fn apply_floating_rounded_region(
    window: &tauri::WebviewWindow,
    radius_points: f64,
) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::HWND,
        Graphics::Dwm::DwmSetWindowAttribute,
        Graphics::Gdi::{
            CreateRoundRectRgn, DeleteObject, RedrawWindow, SetWindowRgn, RDW_FRAME,
            RDW_INVALIDATE, RDW_UPDATENOW,
        },
    };

    let size = window.inner_size().map_err(|error| error.to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    // 胶囊模式传入一个极大半径，并在这里严格钳制为窗口高度的一半，
    // 从而保证左右端帽始终是完整半圆，而不是普通圆角矩形。
    let max_radius = size.height as f64 / 2.0;
    let radius = (radius_points * scale).min(max_radius).round().max(1.0) as i32;
    let region = unsafe {
        CreateRoundRectRgn(
            0,
            0,
            (size.width as i32).saturating_add(1),
            (size.height as i32).saturating_add(1),
            radius * 2,
            radius * 2,
        )
    };
    if region.is_null() {
        return Err("无法创建悬浮窗圆角区域".into());
    }
    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    // Windows 11 原生圆角偏好。Windows 10 返回失败时继续使用 SetWindowRgn 兜底。
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWA_BORDER_COLOR: u32 = 34;
    // SetWindowRgn 是唯一的最终裁切来源。关闭 DWM 的第二层圆角，避免两套曲线
    // 在高 DPI 下相差半个像素而留下四角细线或透明残片。
    const DWMWCP_DONOTROUND: u32 = 1;
    const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
    let preference = DWMWCP_DONOTROUND;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd.0 as HWND,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const u32 as *const core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
        // 禁用 Windows 11 为无边框窗口自动绘制的 1px 深色轮廓。
        // 否则它会与透明 WebView 的裁切边缘叠加，形成用户看到的黑边。
        let _ = DwmSetWindowAttribute(
            hwnd.0 as HWND,
            DWMWA_BORDER_COLOR,
            &DWMWA_COLOR_NONE as *const u32 as *const core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
    let applied = unsafe { SetWindowRgn(hwnd.0 as HWND, region, 1) };
    if applied == 0 {
        unsafe {
            DeleteObject(region);
        }
        return Err("Windows 未能应用悬浮窗圆角区域".into());
    }
    // 立即重绘非客户区和客户区，清掉形态切换前遗留的旧矩形边缘。
    unsafe {
        let _ = RedrawWindow(
            hwnd.0 as HWND,
            std::ptr::null(),
            std::ptr::null_mut(),
            RDW_FRAME | RDW_INVALIDATE | RDW_UPDATENOW,
        );
    }
    // SetWindowRgn 成功后区域所有权交给系统，不能再次 DeleteObject。
    Ok(())
}

#[tauri::command]
fn configure_floating_renderer(
    config: FloatingRendererConfig,
    state: State<'_, FloatingWindowRuntime>,
) -> Result<FloatingRenderStatus, String> {
    let mut status = state.renderer.lock().map_err(|_| "悬浮渲染状态锁定失败")?;
    if !config.enabled {
        *status = FloatingRenderStatus {
            backend: "translucent".into(),
            state: "stopped".into(),
            fps: 0,
            detail: "悬浮窗采用主题纯色材质，不启用液态玻璃或桌面捕获".into(),
        };
    } else {
        // DXGI/D3D11 捕获能力异常、远程桌面或设备丢失时，必须留在同一窗口安全降级。
        // 当前 Windows WebView 使用系统 Acrylic 背板，Vue 内容层仍保留分层折射边缘。
        let fps = if config.quality == "high" { 60 } else { 30 };
        *status = FloatingRenderStatus {
            backend: "acrylic".into(),
            state: "degraded".into(),
            fps,
            detail: format!(
                "同进程 Acrylic 安全背板 · {} · 通透度 {:.0}% · 扭曲 {:.0}%",
                if config.tone == "clear" {
                    "纯白"
                } else {
                    "深色"
                },
                config.transparency.clamp(0.0, 100.0),
                config.distortion.clamp(0.0, 100.0)
            ),
        };
    }
    Ok(status.clone())
}

#[tauri::command]
fn update_floating_surfaces(
    surfaces: Vec<FloatingSurface>,
    state: State<'_, FloatingWindowRuntime>,
) -> Result<(), String> {
    let mut target = state.surfaces.lock().map_err(|_| "悬浮材质区域锁定失败")?;
    *target = surfaces
        .into_iter()
        .take(128)
        .map(|mut item| {
            item.radius = item.radius.clamp(0.0, 64.0);
            item.strength = item.strength.clamp(0.0, 100.0);
            item
        })
        .collect();
    Ok(())
}

#[tauri::command]
fn set_floating_interaction_mode(
    _mode: String,
    app: AppHandle,
    state: State<'_, FloatingWindowRuntime>,
) -> Result<(), String> {
    // v0.10.4 起悬浮窗永久保持交互。即使旧前端仍传入 smart/passthrough，
    // 原生窗口也会立刻恢复鼠标事件，避免用户无法拖动、展开或关闭。
    *state
        .interaction
        .lock()
        .map_err(|_| "悬浮交互状态锁定失败")? = "interactive".into();
    if let Some(window) = app.get_webview_window("floating") {
        window
            .set_ignore_cursor_events(false)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn set_floating_window_mode(
    mode: String,
    radius: f64,
    width: Option<f64>,
    height: Option<f64>,
    always_on_top: bool,
    snap_to_edges: bool,
    app: AppHandle,
    state: State<'_, FloatingWindowRuntime>,
) -> Result<(), String> {
    if !matches!(mode.as_str(), "capsule" | "compact" | "full") {
        return Err("不支持的悬浮窗形态".into());
    }
    let resolved_radius = if mode == "capsule" {
        // v0.8.5 的折叠形态是 360×152 的圆角窗口，不是全高药丸形裁切。
        28.0
    } else {
        // v0.8.5 经典展开窗统一使用 24px 圆角，避免 DPI 下 CSS 与原生裁切不一致。
        radius.clamp(20.0, 24.0)
    };
    *state.radius.lock().map_err(|_| "悬浮圆角状态锁定失败")? = resolved_radius;
    *state.snap_to_edges.lock().map_err(|_| "贴边状态锁定失败")? = snap_to_edges;
    if let Some(window) = app.get_webview_window("floating") {
        // 形态和物理窗口尺寸由同一个主进程命令原子更新，避免只缩小 Vue 内容、
        // 却留下旧窗口透明矩形区域所形成的黑框或空白方框。
        if let (Some(width), Some(height)) = (width, height) {
            window
                .set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                    width.clamp(300.0, 960.0),
                    height.clamp(96.0, 1000.0),
                )))
                .map_err(|error| error.to_string())?;
        }
        window
            .set_always_on_top(always_on_top)
            .map_err(|error| error.to_string())?;
        #[cfg(windows)]
        apply_floating_rounded_region(&window, resolved_radius)?;
    }
    Ok(())
}

#[tauri::command]
fn get_floating_render_status(
    state: State<'_, FloatingWindowRuntime>,
) -> Result<FloatingRenderStatus, String> {
    state
        .renderer
        .lock()
        .map(|status| status.clone())
        .map_err(|_| "悬浮渲染状态锁定失败".into())
}

/// 智能穿透由主进程内的轻量轮询器实现，不创建后台服务或第二个进程。
/// 内容区空闲时允许鼠标落到桌面；顶部控制区、八方向边缘和按住 Alt 时恢复交互。
#[cfg(windows)]
fn start_floating_interaction_monitor(app: AppHandle) {
    std::thread::spawn(move || {
        use windows_sys::Win32::{
            Foundation::{HWND, POINT, RECT},
            UI::{
                Input::KeyboardAndMouse::GetAsyncKeyState,
                WindowsAndMessaging::{GetCursorPos, GetWindowRect},
            },
        };
        let mut last_ignored = false;
        loop {
            std::thread::sleep(Duration::from_millis(48));
            let Some(window) = app.get_webview_window("floating") else {
                break;
            };
            if !window.is_visible().unwrap_or(false) {
                if last_ignored {
                    let _ = window.set_ignore_cursor_events(false);
                    last_ignored = false;
                }
                continue;
            }
            let mode = app
                .state::<FloatingWindowRuntime>()
                .interaction
                .lock()
                .map(|value| value.clone())
                .unwrap_or_else(|_| "interactive".into());
            let should_ignore = if mode == "passthrough" {
                let alt_down = unsafe { (GetAsyncKeyState(0x12) as u16 & 0x8000) != 0 };
                !alt_down
            } else if mode == "interactive" {
                false
            } else {
                let Ok(hwnd) = window.hwnd() else { continue };
                let mut cursor = POINT { x: 0, y: 0 };
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                let captured = unsafe {
                    GetCursorPos(&mut cursor) != 0 && GetWindowRect(hwnd.0 as HWND, &mut rect) != 0
                };
                if !captured {
                    false
                } else {
                    let scale = window.scale_factor().unwrap_or(1.0);
                    let edge = (10.0 * scale).round() as i32;
                    let header = (58.0 * scale).round() as i32;
                    let inside = cursor.x >= rect.left
                        && cursor.x < rect.right
                        && cursor.y >= rect.top
                        && cursor.y < rect.bottom;
                    let in_control = inside
                        && (cursor.y <= rect.top + header
                            || cursor.x <= rect.left + edge
                            || cursor.x >= rect.right - edge
                            || cursor.y >= rect.bottom - edge);
                    let alt_down = unsafe { (GetAsyncKeyState(0x12) as u16 & 0x8000) != 0 };
                    inside && !in_control && !alt_down
                }
            };
            if should_ignore != last_ignored {
                if window.set_ignore_cursor_events(should_ignore).is_ok() {
                    last_ignored = should_ignore;
                }
            }
        }
    });
}

fn show_floating(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("floating")
        .ok_or("悬浮窗口尚未初始化")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_always_on_top(true).map_err(|e| e.to_string())?;
    // 每次显示都主动恢复鼠标事件，防止旧版本遗留的穿透状态继续生效。
    window
        .set_ignore_cursor_events(false)
        .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        let radius = app
            .state::<FloatingWindowRuntime>()
            .radius
            .lock()
            .map(|value| *value)
            .unwrap_or(24.0);
        apply_floating_rounded_region(&window, radius)?;
    }
    window.set_focus().ok();
    app.emit("floating-state", true).ok();
    Ok(())
}
#[tauri::command]
fn set_floating_window(
    enabled: bool,
    app: AppHandle,
    state: State<'_, FloatingWindowRuntime>,
) -> Result<(), String> {
    if enabled {
        show_floating(&app)
    } else {
        if let Some(window) = app.get_webview_window("floating") {
            window.hide().map_err(|e| e.to_string())?
        }
        if let Ok(mut status) = state.renderer.lock() {
            status.state = "stopped".into();
            status.fps = 0;
            status.detail = "悬浮窗已隐藏，桌面采样和材质刷新已停止".into();
        }
        app.emit("floating-state", false).ok();
        Ok(())
    }
}
#[tauri::command]
fn send_budget_alert(level: u8, remaining: u8, app: AppHandle) -> Result<(), String> {
    let title = if level <= 10 {
        "Token Manager · 预算余额严重不足"
    } else {
        "Token Manager · 预算余额提醒"
    };
    app.notification()
        .builder()
        .title(title)
        .body(format!(
            "Codex 个人预算余额约 {remaining}%，请合理安排后续任务。 "
        ))
        .show()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn providers() -> Vec<AdapterCapability> {
    let ids = [
        "腾讯混元",
        "豆包（火山方舟）",
        "文心千帆",
        "通义百炼",
        "智谱 AI",
        "DeepSeek",
        "Kimi",
        "小米 MiMo",
        "讯飞星火",
        "MiniMax",
        "阶跃星辰",
        "零一万物",
        "商汤日日新",
        "百川智能",
        "OpenCode Go",
        "OpenAI",
        "Anthropic",
        "Google Gemini",
        "自定义 OpenAI 兼容",
    ];
    ids.into_iter()
        .map(|id| {
            RegistryAdapter {
                id,
                billing: id == "DeepSeek",
            }
            .capability()
        })
        .collect()
}

/// 只检查 CC Switch 是否安装、进程是否存在，以及常用本地路由端口是否监听；不读取其密钥或供应商配置。
#[tauri::command]
fn cc_switch_status() -> CcSwitchStatus {
    let home = dirs::home_dir().unwrap_or_default();
    let local = dirs::data_local_dir().unwrap_or_default();
    let installed = [
        home.join(".cc-switch"),
        local.join("CC Switch"),
        local.join("cc-switch"),
    ]
    .iter()
    .any(|path| path.exists());
    #[cfg(windows)]
    let running = Command::new("tasklist")
        .creation_flags(0x0800_0000)
        .args(["/FO", "CSV", "/NH"])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).to_ascii_lowercase())
        .map(|text| {
            text.contains("cc-switch") || text.contains("cc_switch") || text.contains("ccswitch")
        })
        .unwrap_or(false);
    #[cfg(not(windows))]
    let running = false;
    let local_routing = [15721u16, 15722, 15723].into_iter().any(|port| {
        TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(90),
        )
        .is_ok()
    });
    let detail = if local_routing {
        "本地路由已连接"
    } else if running {
        "应用运行中，本地路由未检测到"
    } else if installed {
        "已安装，当前未运行"
    } else {
        "未检测到 CC Switch"
    }
    .to_string();
    CcSwitchStatus {
        installed: installed || running,
        running,
        local_routing,
        detail,
        checked_at: Utc::now().to_rfc3339(),
    }
}

#[cfg(windows)]
fn query_user_environment(name: &str) -> Option<String> {
    let output = Command::new("reg")
        .creation_flags(0x0800_0000)
        .args(["query", "HKCU\\Environment", "/v", name])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().find_map(|line| {
        let columns = line.split_whitespace().collect::<Vec<_>>();
        let index = columns
            .iter()
            .position(|value| value.eq_ignore_ascii_case(name))?;
        (columns.len() > index + 2).then(|| columns[index + 2..].join(" "))
    })
}

#[cfg(not(windows))]
fn query_user_environment(_name: &str) -> Option<String> {
    None
}

#[cfg(windows)]
fn write_user_environment(name: &str, value: Option<&str>) -> Result<(), String> {
    let mut command = Command::new("reg");
    command.creation_flags(0x0800_0000);
    if let Some(value) = value {
        command.args([
            "add",
            "HKCU\\Environment",
            "/v",
            name,
            "/t",
            "REG_SZ",
            "/d",
            value,
            "/f",
        ]);
    } else {
        command.args(["delete", "HKCU\\Environment", "/v", name, "/f"]);
    }
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() || value.is_none() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(not(windows))]
fn write_user_environment(_name: &str, _value: Option<&str>) -> Result<(), String> {
    Err("自动接入当前仅支持 Windows".into())
}

fn client_integration_backup_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join("client-integration-backup.json"))
}

/// 自动把常见 OpenAI 兼容工具与 Claude Code 指向本机代理。
/// 修改前保存原始配置，只写 Base URL 和 Claude Code 的本地占位令牌，不读取用户密钥。
#[tauri::command]
fn auto_connect_client(
    local_url: String,
    anthropic_url: Option<String>,
    account_name: String,
    app: AppHandle,
) -> Result<ClientIntegrationResult, String> {
    if !local_url.starts_with("http://127.0.0.1:") {
        return Err("自动接入仅允许使用 Token Manager 的 127.0.0.1 本机地址".into());
    }
    if let Some(url) = anthropic_url.as_deref() {
        if !url.starts_with("http://127.0.0.1:") {
            return Err("Claude Code 自动接入地址不是安全的本机地址".into());
        }
    }

    let backup_path = client_integration_backup_path(&app)?;
    if !backup_path.exists() {
        let claude_path = dirs::home_dir().map(|home| home.join(".claude").join("settings.json"));
        let backup = ClientIntegrationBackup {
            openai_base_url: query_user_environment("OPENAI_BASE_URL"),
            openai_api_base: query_user_environment("OPENAI_API_BASE"),
            anthropic_base_url: query_user_environment("ANTHROPIC_BASE_URL"),
            claude_settings_path: claude_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            claude_settings_original: claude_path
                .as_ref()
                .and_then(|path| fs::read_to_string(path).ok()),
            claude_settings_existed: claude_path.as_ref().is_some_and(|path| path.exists()),
            applied_at: Utc::now().to_rfc3339(),
        };
        fs::write(
            &backup_path,
            serde_json::to_vec_pretty(&backup).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    }

    write_user_environment("OPENAI_BASE_URL", Some(&local_url))?;
    write_user_environment("OPENAI_API_BASE", Some(&local_url))?;
    let mut changed = vec![
        "Windows OPENAI_BASE_URL".to_string(),
        "Windows OPENAI_API_BASE".to_string(),
    ];

    if let Some(anthropic_url) = anthropic_url.as_deref() {
        write_user_environment("ANTHROPIC_BASE_URL", Some(anthropic_url))?;
        let claude_path = dirs::home_dir()
            .ok_or("无法定位当前用户目录")?
            .join(".claude")
            .join("settings.json");
        if let Some(parent) = claude_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut settings = fs::read_to_string(&claude_path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        if !settings.is_object() {
            settings = serde_json::json!({});
        }
        let object = settings.as_object_mut().ok_or("Claude Code 设置格式无效")?;
        let env = object.entry("env").or_insert_with(|| serde_json::json!({}));
        if !env.is_object() {
            *env = serde_json::json!({});
        }
        if let Some(env) = env.as_object_mut() {
            env.insert(
                "ANTHROPIC_BASE_URL".into(),
                serde_json::Value::String(anthropic_url.to_string()),
            );
            env.entry("ANTHROPIC_AUTH_TOKEN")
                .or_insert_with(|| serde_json::Value::String("token-manager-local".into()));
        }
        fs::write(
            &claude_path,
            serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        changed.push("Claude Code ~/.claude/settings.json".into());
        changed.push("Windows ANTHROPIC_BASE_URL".into());
    }

    Ok(ClientIntegrationResult {
        connected: true,
        account_name,
        openai_url: local_url,
        anthropic_url,
        changed,
        restart_required: true,
        detail: "自动接入已完成；请重启正在运行的 Code、Cursor 或 Claude Code 后发起一次请求。"
            .into(),
    })
}

#[tauri::command]
fn restore_client_connection(app: AppHandle) -> Result<String, String> {
    let backup_path = client_integration_backup_path(&app)?;
    if !backup_path.exists() {
        return Ok("没有需要恢复的自动接入配置".into());
    }
    let backup: ClientIntegrationBackup =
        serde_json::from_slice(&fs::read(&backup_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    write_user_environment("OPENAI_BASE_URL", backup.openai_base_url.as_deref())?;
    write_user_environment("OPENAI_API_BASE", backup.openai_api_base.as_deref())?;
    write_user_environment("ANTHROPIC_BASE_URL", backup.anthropic_base_url.as_deref())?;
    if let Some(path) = backup.claude_settings_path.as_deref() {
        let path = PathBuf::from(path);
        if let Some(original) = backup.claude_settings_original {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(path, original).map_err(|error| error.to_string())?;
        } else if !backup.claude_settings_existed && path.exists() {
            fs::remove_file(path).map_err(|error| error.to_string())?;
        }
    }
    fs::remove_file(backup_path).map_err(|error| error.to_string())?;
    Ok("已恢复自动接入前的连接配置；重启调用工具后生效。".into())
}

fn backup_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 120_000, &mut key);
    key
}

fn build_backup_envelope(
    password: &str,
    ui_state: serde_json::Value,
    state: &State<AppDb>,
) -> Result<BackupEnvelope, String> {
    if password.chars().count() < 8 {
        return Err("迁移密码至少需要 8 个字符".into());
    }
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let mut account_query=db.prepare("SELECT id,provider,name,base_url,secret_cipher,created_at FROM account_configs ORDER BY created_at").map_err(|e|e.to_string())?;
    let accounts = account_query
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Vec<u8>>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .map(|row| {
            let (id, provider, name, base_url, cipher, created_at) =
                row.map_err(|e| e.to_string())?;
            Ok(BackupAccount {
                id,
                provider,
                name,
                base_url,
                api_key: unprotect_secret(&cipher)?,
                created_at,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut usage_query=db.prepare("SELECT id,provider,model,at,input_tokens,output_tokens,cached_tokens,cost,task FROM usage_events ORDER BY at").map_err(|e|e.to_string())?;
    let usage = usage_query
        .query_map([], |r| {
            Ok(UsageEvent {
                id: r.get(0)?,
                provider: r.get(1)?,
                model: r.get(2)?,
                at: DateTime::parse_from_rfc3339(&r.get::<_, String>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc),
                input: r.get::<_, i64>(4)?.max(0) as u64,
                output: r.get::<_, i64>(5)?.max(0) as u64,
                cached: r.get::<_, i64>(6)?.max(0) as u64,
                cost: r.get(7)?,
                task: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut balance_query=db.prepare("SELECT account_id,provider,currency,total_balance,granted_balance,topped_up_balance,is_available,synced_at FROM account_balances").map_err(|e|e.to_string())?;
    let balances = balance_query
        .query_map([], |r| {
            Ok(AccountBalance {
                account_id: r.get(0)?,
                provider: r.get(1)?,
                currency: r.get(2)?,
                total_balance: r.get(3)?,
                granted_balance: r.get(4)?,
                topped_up_balance: r.get(5)?,
                is_available: r.get::<_, i64>(6)? != 0,
                synced_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let budget = db
        .query_row(
            "SELECT five_hour_limit,seven_day_limit FROM codex_budget WHERE id=1",
            [],
            |r| {
                Ok(CodexBudget {
                    five_hour_limit: r.get::<_, i64>(0)?.max(1) as u64,
                    seven_day_limit: r.get::<_, i64>(1)?.max(1) as u64,
                })
            },
        )
        .map_err(|e| e.to_string())?;
    drop(balance_query);
    drop(usage_query);
    drop(account_query);
    drop(db);
    let bundle = BackupBundle {
        version: 1,
        exported_at: Utc::now().to_rfc3339(),
        accounts,
        usage,
        balances,
        five_hour_limit: budget.five_hour_limit,
        seven_day_limit: budget.seven_day_limit,
        ui_state,
    };
    let plaintext = serde_json::to_vec(&bundle).map_err(|e| e.to_string())?;
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);
    let key = backup_key(password, &salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|e| e.to_string())?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|_| "无法加密迁移包")?;
    Ok(BackupEnvelope {
        version: 1,
        salt: BASE64.encode(salt),
        nonce: BASE64.encode(nonce),
        ciphertext: BASE64.encode(ciphertext),
    })
}

fn restore_backup_envelope(
    envelope: BackupEnvelope,
    password: &str,
    state: &State<AppDb>,
) -> Result<serde_json::Value, String> {
    let salt = BASE64.decode(envelope.salt).map_err(|_| "迁移包盐值无效")?;
    let nonce = BASE64
        .decode(envelope.nonce)
        .map_err(|_| "迁移包随机数无效")?;
    if nonce.len() != 12 {
        return Err("迁移包随机数长度无效".into());
    }
    let ciphertext = BASE64
        .decode(envelope.ciphertext)
        .map_err(|_| "迁移包密文无效")?;
    let key = backup_key(password, &salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|e| e.to_string())?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| "迁移密码错误或迁移包已损坏")?;
    let bundle: BackupBundle =
        serde_json::from_slice(&plaintext).map_err(|_| "迁移数据无法解析")?;
    let mut db = state.0.lock().map_err(|_| "数据库锁定")?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    tx.execute_batch("DELETE FROM account_configs;DELETE FROM usage_events;DELETE FROM account_balances;DELETE FROM balance_history;").map_err(|e|e.to_string())?;
    for account in bundle.accounts {
        let cipher = protect_secret(&account.api_key)?;
        tx.execute("INSERT INTO account_configs(id,provider,name,base_url,secret_cipher,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![account.id,account.provider,account.name,account.base_url,cipher,account.created_at]).map_err(|e|e.to_string())?;
    }
    for event in bundle.usage {
        tx.execute(
            "INSERT INTO usage_events(id,provider,model,at,input_tokens,output_tokens,cached_tokens,cost,task) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                event.id,
                event.provider,
                event.model,
                event.at.to_rfc3339(),
                event.input,
                event.output,
                event.cached,
                event.cost,
                event.task
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    for row in bundle.balances {
        tx.execute("INSERT INTO account_balances(account_id,provider,currency,total_balance,granted_balance,topped_up_balance,is_available,synced_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![row.account_id,row.provider,row.currency,row.total_balance,row.granted_balance,row.topped_up_balance,row.is_available as i32,row.synced_at]).map_err(|e|e.to_string())?;
    }
    tx.execute(
        "UPDATE codex_budget SET five_hour_limit=?1,seven_day_limit=?2 WHERE id=1",
        params![bundle.five_hour_limit, bundle.seven_day_limit],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(bundle.ui_state)
}

/// 导出时先用当前用户 DPAPI 解密密钥，再把整个备份用用户密码重新加密，才能安全迁移到另一台 Windows 电脑。
#[tauri::command]
fn export_encrypted_backup(
    path: String,
    password: String,
    ui_state: serde_json::Value,
    state: State<AppDb>,
) -> Result<(), String> {
    let envelope = build_backup_envelope(&password, ui_state, &state)?;
    fs::write(
        path,
        serde_json::to_vec(&envelope).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn import_encrypted_backup(
    path: String,
    password: String,
    state: State<AppDb>,
) -> Result<serde_json::Value, String> {
    let envelope: BackupEnvelope =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|_| "迁移包格式无效")?;
    restore_backup_envelope(envelope, &password, &state)
}

fn validate_cloud_base(value: &str) -> Result<String, String> {
    let base = value.trim().trim_end_matches('/');
    if base.starts_with("https://")
        || base.starts_with("http://127.0.0.1")
        || base.starts_with("http://localhost")
    {
        Ok(base.into())
    } else {
        Err("后端地址必须使用 HTTPS，或为本机开发地址".into())
    }
}
fn install_identity(state: &State<'_, AppDb>) -> Result<String, String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    if let Ok(value) = db.query_row(
        "SELECT install_id FROM app_identity WHERE id=1",
        [],
        |row| row.get::<_, String>(0),
    ) {
        return Ok(value);
    }
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let value = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    db.execute(
        "INSERT OR REPLACE INTO app_identity(id,install_id,created_at) VALUES(1,?1,?2)",
        params![value, Utc::now().to_rfc3339()],
    )
    .map_err(|error| error.to_string())?;
    Ok(value)
}
fn save_cloud_auth(
    base_url: &str,
    auth: &CloudAuthReply,
    state: &State<AppDb>,
) -> Result<CloudSessionSummary, String> {
    let cipher = protect_secret(&auth.access_token)?;
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    db.execute("INSERT OR REPLACE INTO cloud_session(id,email,base_url,token_cipher,expires_at,updated_at) VALUES(1,?1,?2,?3,?4,?5)",params![auth.user.email,base_url,cipher,auth.expires_at,Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
    Ok(CloudSessionSummary {
        email: auth.user.email.clone(),
        base_url: base_url.into(),
        expires_at: auth.expires_at.clone(),
    })
}
fn cloud_credentials(state: &State<AppDb>) -> Result<(CloudSessionSummary, String), String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let row: (String, String, Vec<u8>, String) = db
        .query_row(
            "SELECT email,base_url,token_cipher,expires_at FROM cloud_session WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|_| "尚未登录云账户")?;
    if row.3 < Utc::now().to_rfc3339() {
        return Err("云账户登录已过期，请重新登录".into());
    }
    let token = unprotect_secret(&row.2)?;
    Ok((
        CloudSessionSummary {
            email: row.0,
            base_url: row.1,
            expires_at: row.3,
        },
        token,
    ))
}
#[tauri::command]
fn cloud_session(state: State<AppDb>) -> Result<Option<CloudSessionSummary>, String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let result = db.query_row(
        "SELECT email,base_url,expires_at FROM cloud_session WHERE id=1",
        [],
        |r| {
            Ok(CloudSessionSummary {
                email: r.get(0)?,
                base_url: r.get(1)?,
                expires_at: r.get(2)?,
            })
        },
    );
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}
#[tauri::command]
async fn cloud_request_code(
    base_url: String,
    email: String,
    purpose: String,
) -> Result<CloudCodeReply, String> {
    let base = validate_cloud_base(&base_url)?;
    let response = reqwest::Client::new()
        .post(format!("{base}/v1/auth/code/request"))
        .json(&serde_json::json!({"email":email,"purpose":purpose}))
        .send()
        .await
        .map_err(|e| format!("无法连接账户后端：{e}"))?;
    let status = response.status();
    let value = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("验证码发送失败")
            .into());
    }
    Ok(CloudCodeReply {
        expires_in: value
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(600),
        debug_code: value
            .get("debug_code")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
    })
}
async fn cloud_auth(
    base_url: String,
    path: &str,
    body: serde_json::Value,
    state: &State<'_, AppDb>,
) -> Result<CloudSessionSummary, String> {
    let base = validate_cloud_base(&base_url)?;
    let response = reqwest::Client::new()
        .post(format!("{base}{path}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("无法连接账户后端：{e}"))?;
    let status = response.status();
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let value = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or_default();
        return Err(value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("登录失败")
            .into());
    }
    let auth =
        serde_json::from_slice::<CloudAuthReply>(&bytes).map_err(|_| "后端登录响应格式无效")?;
    save_cloud_auth(&base, &auth, state)
}
#[tauri::command]
async fn cloud_password_login(
    base_url: String,
    email: String,
    password: String,
    state: State<'_, AppDb>,
) -> Result<CloudSessionSummary, String> {
    cloud_auth(
        base_url,
        "/v1/auth/password/login",
        serde_json::json!({"email":email,"password":password}),
        &state,
    )
    .await
}
#[tauri::command]
async fn cloud_password_register(
    base_url: String,
    email: String,
    password: String,
    code: String,
    state: State<'_, AppDb>,
) -> Result<CloudSessionSummary, String> {
    cloud_auth(
        base_url,
        "/v1/auth/password/register",
        serde_json::json!({"email":email,"password":password,"code":code}),
        &state,
    )
    .await
}
#[tauri::command]
async fn cloud_code_login(
    base_url: String,
    email: String,
    code: String,
    state: State<'_, AppDb>,
) -> Result<CloudSessionSummary, String> {
    cloud_auth(
        base_url,
        "/v1/auth/code/login",
        serde_json::json!({"email":email,"code":code}),
        &state,
    )
    .await
}
#[tauri::command]
async fn cloud_logout(state: State<'_, AppDb>) -> Result<(), String> {
    if let Ok((session, token)) = cloud_credentials(&state) {
        let _ = reqwest::Client::new()
            .post(format!("{}/v1/auth/logout", session.base_url))
            .bearer_auth(token)
            .send()
            .await;
    }
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    db.execute("DELETE FROM cloud_session WHERE id=1", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
async fn cloud_create_transfer(
    password: String,
    ui_state: serde_json::Value,
    ttl_hours: i64,
    one_time: bool,
    state: State<'_, AppDb>,
) -> Result<CloudTransferReply, String> {
    let (session, token) = cloud_credentials(&state)?;
    let envelope = build_backup_envelope(&password, ui_state, &state)?;
    let payload = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
    let response=reqwest::Client::new().post(format!("{}/v1/transfers",session.base_url)).bearer_auth(token).json(&serde_json::json!({"label":"Token Manager 加密迁移","payload":payload,"ttl_hours":ttl_hours,"one_time":one_time})).send().await.map_err(|e|format!("迁移链接创建失败：{e}"))?;
    let status = response.status();
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let value = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or_default();
        return Err(value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("迁移链接创建失败")
            .into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "后端迁移链接响应格式无效".into())
}
#[tauri::command]
async fn cloud_import_transfer(
    link: String,
    password: String,
    state: State<'_, AppDb>,
) -> Result<serde_json::Value, String> {
    let clean = link.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(clean).map_err(|_| "迁移链接格式无效")?;
    if parsed.scheme() != "https"
        && !(parsed.scheme() == "http"
            && matches!(parsed.host_str(), Some("127.0.0.1") | Some("localhost")))
    {
        return Err("迁移链接必须使用 HTTPS，或为本机开发地址".into());
    }
    if !parsed.path().contains("/v1/transfer/") {
        return Err("这不是 Token Manager 迁移链接".into());
    }
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{clean}/preview"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| format!("迁移链接读取失败：{e}"))?;
    let status = response.status();
    let value = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "迁移链接响应无法解析")?;
    if !status.is_success() {
        return Err(value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("迁移链接不可用")
            .into());
    }
    let payload = value
        .get("payload")
        .and_then(|v| v.as_str())
        .ok_or("迁移链接没有数据")?;
    let envelope =
        serde_json::from_str::<BackupEnvelope>(payload).map_err(|_| "云端迁移包格式无效")?;
    let ui_state = restore_backup_envelope(envelope, &password, &state)?;
    let _ = client
        .post(format!("{clean}/consume"))
        .json(&serde_json::json!({}))
        .send()
        .await;
    Ok(ui_state)
}
#[tauri::command]
async fn cloud_public_content(base_url: String) -> Result<Vec<RemoteContentItem>, String> {
    let base = validate_cloud_base(&base_url)?;
    let response = reqwest::Client::new()
        .get(format!("{base}/v1/content"))
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| format!("公告服务暂不可用：{e}"))?;
    if !response.status().is_success() {
        return Err(format!("公告服务返回 {}", response.status()));
    }
    response
        .json::<RemoteContentReply>()
        .await
        .map(|reply| reply.items)
        .map_err(|_| "公告服务响应格式无效".into())
}
/// 无需登录的匿名应用心跳。随机安装标识不包含邮箱、机器名、硬件信息或用户文件内容。
#[tauri::command]
async fn cloud_app_presence(
    base_url: String,
    app_version: String,
    event: String,
    state: State<'_, AppDb>,
) -> Result<(), String> {
    let base = validate_cloud_base(&base_url)?;
    let event = if event == "launch" {
        "launch"
    } else {
        "heartbeat"
    };
    let install_id = install_identity(&state)?;
    let response = reqwest::Client::new()
        .post(format!("{base}/v1/app/heartbeat"))
        .timeout(Duration::from_secs(8))
        .json(&serde_json::json!({"install_id":install_id,"app_version":app_version,"event":event}))
        .send()
        .await
        .map_err(|e| format!("匿名使用统计暂不可用：{e}"))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("匿名使用统计返回 {}", response.status()))
    }
}
#[tauri::command]
fn save_usage(event: UsageEvent, state: State<AppDb>) -> Result<(), String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let provenance = reliability::UsageProvenance::observed(
        format!(
            "manual-{}",
            event.provider.to_ascii_lowercase().replace(' ', "-")
        ),
        reliability::SourceKind::BillImport,
    );
    reliability::upsert_usage(&db, &event, &provenance).map_err(|e| e.to_string())?;
    Ok(())
}
/// 读取本机持久化的真实 API/代理用量事件，不读取任何请求内容或密钥。
#[tauri::command]
fn list_usage(days: i64, state: State<AppDb>) -> Result<Vec<UsageEvent>, String> {
    let since = (Utc::now() - chrono::Duration::days(days.clamp(1, 365))).to_rfc3339();
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let mut statement=db.prepare("SELECT id,provider,model,at,input_tokens,output_tokens,cached_tokens,cost,task FROM usage_events WHERE at>=?1 AND is_shadowed=0 ORDER BY at DESC").map_err(|e|e.to_string())?;
    let rows = statement
        .query_map([since], |r| {
            Ok(UsageEvent {
                id: r.get(0)?,
                provider: r.get(1)?,
                model: r.get(2)?,
                at: DateTime::parse_from_rfc3339(&r.get::<_, String>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc),
                input: r.get::<_, i64>(4)?.max(0) as u64,
                output: r.get::<_, i64>(5)?.max(0) as u64,
                cached: r.get::<_, i64>(6)?.max(0) as u64,
                cost: r.get(7)?,
                task: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
/// 按全部模型或单个平台/模型导出真正的 Excel 工作簿，不经过外部服务。
#[tauri::command]
fn export_usage_xlsx(
    path: String,
    provider: Option<String>,
    model: Option<String>,
    state: State<AppDb>,
) -> Result<String, String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    let mut statement=db.prepare("SELECT id,provider,model,at,input_tokens,output_tokens,cached_tokens,cost,task FROM usage_events WHERE is_shadowed=0 ORDER BY at DESC").map_err(|e|e.to_string())?;
    let events = statement
        .query_map([], |r| {
            Ok(UsageEvent {
                id: r.get(0)?,
                provider: r.get(1)?,
                model: r.get(2)?,
                at: DateTime::parse_from_rfc3339(&r.get::<_, String>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc),
                input: r.get::<_, i64>(4)?.max(0) as u64,
                output: r.get::<_, i64>(5)?.max(0) as u64,
                cached: r.get::<_, i64>(6)?.max(0) as u64,
                cost: r.get(7)?,
                task: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(statement);
    drop(db);
    let rows = events
        .into_iter()
        .filter(|item| {
            provider
                .as_ref()
                .map(|value| value == &item.provider)
                .unwrap_or(true)
                && model
                    .as_ref()
                    .map(|value| value == &item.model)
                    .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    let mut summary: HashMap<(String, String), (u64, u64, u64, u64, f64)> = HashMap::new();
    for item in &rows {
        let entry = summary
            .entry((item.provider.clone(), item.model.clone()))
            .or_insert((0, 0, 0, 0, 0.0));
        entry.0 += 1;
        entry.1 += item.input;
        entry.2 += item.output;
        entry.3 += item.cached;
        entry.4 += item.cost;
    }
    let mut workbook = Workbook::new();
    {
        let sheet = workbook.add_worksheet();
        sheet.set_name("模型汇总").map_err(|e| e.to_string())?;
        for (column, title) in [
            "平台",
            "模型",
            "请求次数",
            "输入 Token",
            "输出 Token",
            "缓存 Token",
            "总 Token",
            "累计消费（元）",
        ]
        .iter()
        .enumerate()
        {
            sheet
                .write_string(0, column as u16, *title)
                .map_err(|e| e.to_string())?;
        }
        let mut values = summary.into_iter().collect::<Vec<_>>();
        values.sort_by(|a, b| b.1 .4.total_cmp(&a.1 .4));
        for (index, ((provider, model), (calls, input, output, cached, cost))) in
            values.iter().enumerate()
        {
            let row = (index + 1) as u32;
            sheet
                .write_string(row, 0, provider)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(row, 1, model)
                .map_err(|e| e.to_string())?;
            for (column, value) in [
                (2, *calls as f64),
                (3, *input as f64),
                (4, *output as f64),
                (5, *cached as f64),
                (6, (*input + *output) as f64),
                (7, *cost),
            ] {
                sheet
                    .write_number(row, column, value)
                    .map_err(|e| e.to_string())?;
            }
        }
        sheet.autofit();
    }
    {
        let sheet = workbook.add_worksheet();
        sheet.set_name("调用明细").map_err(|e| e.to_string())?;
        for (column, title) in [
            "时间",
            "平台",
            "模型",
            "输入 Token",
            "输出 Token",
            "缓存 Token",
            "总 Token",
            "成本（元）",
            "任务/状态",
        ]
        .iter()
        .enumerate()
        {
            sheet
                .write_string(0, column as u16, *title)
                .map_err(|e| e.to_string())?;
        }
        for (index, item) in rows.iter().enumerate() {
            let row = (index + 1) as u32;
            sheet
                .write_string(
                    row,
                    0,
                    item.at
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string(),
                )
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(row, 1, &item.provider)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(row, 2, &item.model)
                .map_err(|e| e.to_string())?;
            for (column, value) in [
                (3, item.input as f64),
                (4, item.output as f64),
                (5, item.cached as f64),
                (6, (item.input + item.output) as f64),
                (7, item.cost),
            ] {
                sheet
                    .write_number(row, column, value)
                    .map_err(|e| e.to_string())?;
            }
            sheet
                .write_string(row, 8, &item.task)
                .map_err(|e| e.to_string())?;
        }
        sheet.autofit();
    }
    workbook
        .save(&path)
        .map_err(|e| format!("Excel 导出失败：{e}"))?;
    Ok(path)
}
#[tauri::command]
fn parse_codex_line(line: String) -> Option<UsageEvent> {
    let v: serde_json::Value = serde_json::from_str(&line).ok()?;
    let payload = v.get("payload").unwrap_or(&v);
    RegistryAdapter {
        id: "Codex",
        billing: false,
    }
    .normalize(payload)
    .map(|mut e| {
        e.task = "调试".into();
        e
    })
}
/// 只读访问 Codex 自己的状态库；仅提取计数及更新时间，绝不读取提示词或会话正文。
#[tauri::command]
fn codex_snapshot() -> Result<CodexSnapshot, String> {
    let path = dirs::home_dir()
        .ok_or("无法定位用户目录")?
        .join(".codex")
        .join("state_5.sqlite");
    if !path.exists() {
        return Err("未发现 Codex 本地状态库".into());
    }
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let (total, active, updated):(i64,i64,i64)=db.query_row(
  "SELECT COALESCE(SUM(tokens_used),0), COALESCE((SELECT tokens_used FROM threads ORDER BY updated_at_ms DESC LIMIT 1),0), COALESCE(MAX(NULLIF(updated_at_ms,0)),MAX(updated_at)*1000,0) FROM threads",
  [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))
 ).map_err(|e| format!("无法读取 Codex 用量：{e}"))?;
    Ok(CodexSnapshot {
        total_tokens: total.max(0) as u64,
        active_thread_tokens: active.max(0) as u64,
        updated_at_ms: updated,
        source: "Codex 本地状态库（threads.tokens_used）".into(),
    })
}
fn current_budget(state: &State<AppDb>) -> Result<CodexBudget, String> {
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    db.query_row(
        "SELECT five_hour_limit,seven_day_limit FROM codex_budget WHERE id=1",
        [],
        |r| {
            Ok(CodexBudget {
                five_hour_limit: r.get::<_, i64>(0)?.max(1) as u64,
                seven_day_limit: r.get::<_, i64>(1)?.max(1) as u64,
            })
        },
    )
    .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_codex_budget(
    five_hour_limit: u64,
    seven_day_limit: u64,
    state: State<AppDb>,
) -> Result<CodexBudget, String> {
    if five_hour_limit == 0 || seven_day_limit == 0 {
        return Err("预算必须大于 0".into());
    }
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    db.execute(
        "UPDATE codex_budget SET five_hour_limit=?1,seven_day_limit=?2 WHERE id=1",
        params![five_hour_limit as i64, seven_day_limit as i64],
    )
    .map_err(|e| e.to_string())?;
    Ok(CodexBudget {
        five_hour_limit,
        seven_day_limit,
    })
}
fn field(body: &str, marker: &str) -> Option<String> {
    let start = body.find(marker)? + marker.len();
    let value = &body[start..];
    Some(
        value
            .chars()
            .take_while(|c| {
                c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ':'
            })
            .collect(),
    )
}
fn recent_files(root: &Path, extension: &str, limit: usize) -> Vec<PathBuf> {
    fn collect(path: &Path, extension: &str, rows: &mut Vec<(std::time::SystemTime, PathBuf)>) {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(&path, extension, rows)
            } else if path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case(extension))
                .unwrap_or(false)
            {
                let modified = entry
                    .metadata()
                    .and_then(|value| value.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                rows.push((modified, path));
            }
        }
    }
    let mut rows = Vec::new();
    collect(root, extension, &mut rows);
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.into_iter().take(limit).map(|(_, path)| path).collect()
}
fn quota_window(value: &serde_json::Value) -> Option<CodexQuotaWindow> {
    let used = value.get("used_percent")?.as_f64()?.clamp(0.0, 100.0);
    Some(CodexQuotaWindow {
        used_percent: used,
        remaining_percent: (100.0 - used).clamp(0.0, 100.0),
        window_minutes: value.get("window_minutes")?.as_u64()?,
        resets_at: value
            .get("resets_at")
            .and_then(|item| item.as_i64())
            .unwrap_or(0),
    })
}
fn parse_codex_quota_line(line: &str) -> Option<CodexQuotaSnapshot> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    let payload = value.get("payload")?;
    let limits = payload.get("rate_limits")?;
    let windows = [limits.get("primary"), limits.get("secondary")]
        .into_iter()
        .flatten()
        .filter_map(quota_window)
        .collect::<Vec<_>>();
    let five_hour = windows
        .iter()
        .find(|item| item.window_minutes == 300)
        .cloned();
    let seven_day = windows
        .iter()
        .find(|item| item.window_minutes == 10_080)
        .cloned();
    if five_hour.is_none() && seven_day.is_none() {
        return None;
    }
    let observed_at = value
        .get("timestamp")
        .and_then(|item| item.as_str())
        .and_then(|item| DateTime::parse_from_rfc3339(item).ok())
        .map(|item| item.timestamp())
        .unwrap_or_else(|| Utc::now().timestamp());
    Some(CodexQuotaSnapshot {
        five_hour,
        seven_day,
        plan_type: limits
            .get("plan_type")
            .and_then(|item| item.as_str())
            .map(str::to_owned),
        credit_balance: limits
            .pointer("/credits/balance")
            .and_then(|item| item.as_str())
            .map(str::to_owned),
        observed_at,
        source: "Codex 客户端本地 rate_limits 事件".into(),
    })
}
/// 读取 Codex 已落盘的额度状态。仅处理百分比、窗口和重置时间，不读取提示词、回复正文或认证文件。
#[tauri::command]
fn codex_quota_snapshot() -> Result<CodexQuotaSnapshot, String> {
    let sessions = dirs::home_dir()
        .ok_or("无法定位用户目录")?
        .join(".codex")
        .join("sessions");
    if !sessions.exists() {
        return Err("未发现 Codex 本地会话目录".into());
    }
    for path in recent_files(&sessions, "jsonl", 96) {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for line in text.lines().rev() {
            if !line.contains("\"rate_limits\"") {
                continue;
            }
            if let Some(snapshot) = parse_codex_quota_line(line) {
                return Ok(snapshot);
            }
        }
    }
    Err("Codex 尚未写入可读取的额度状态；完成一次 Codex 对话后再刷新".into())
}
/// 解析 Claude Code 本地 JSONL 的 usage 元数据，不读取或保存会话正文。
#[tauri::command]
fn claude_code_usage_series(days: Option<u32>) -> Result<Vec<UsageEvent>, String> {
    let projects = dirs::home_dir()
        .ok_or("无法定位用户目录")?
        .join(".claude")
        .join("projects");
    if !projects.exists() {
        return Ok(Vec::new());
    }
    let since = Utc::now().timestamp() - days.unwrap_or(30).clamp(1, 90) as i64 * 86_400;
    let mut messages: HashMap<String, UsageEvent> = HashMap::new();
    for path in recent_files(&projects, "jsonl", 160) {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for line in text.lines() {
            if !line.contains("\"usage\"") || !line.contains("\"assistant\"") {
                continue;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if value.get("type").and_then(|item| item.as_str()) != Some("assistant") {
                continue;
            }
            let Some(message) = value.get("message") else {
                continue;
            };
            let Some(usage) = message.get("usage") else {
                continue;
            };
            let at = value
                .get("timestamp")
                .and_then(|item| item.as_str())
                .and_then(|item| DateTime::parse_from_rfc3339(item).ok())
                .map(|item| item.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            if at.timestamp() < since {
                continue;
            }
            let id = message
                .get("id")
                .and_then(|item| item.as_str())
                .unwrap_or_else(|| {
                    value
                        .get("uuid")
                        .and_then(|item| item.as_str())
                        .unwrap_or("unknown")
                })
                .to_string();
            let direct = usage
                .get("input_tokens")
                .and_then(|item| item.as_u64())
                .unwrap_or(0);
            let cache_write = usage
                .get("cache_creation_input_tokens")
                .and_then(|item| item.as_u64())
                .unwrap_or(0);
            let cache_read = usage
                .get("cache_read_input_tokens")
                .and_then(|item| item.as_u64())
                .unwrap_or(0);
            let output = usage
                .get("output_tokens")
                .and_then(|item| item.as_u64())
                .unwrap_or(0);
            let event = UsageEvent {
                id: id.clone(),
                provider: "Claude Code".into(),
                model: message
                    .get("model")
                    .and_then(|item| item.as_str())
                    .unwrap_or("Claude")
                    .into(),
                at,
                input: direct + cache_write + cache_read,
                output,
                cached: cache_read,
                cost: 0.0,
                task: "其他".into(),
            };
            messages.insert(id, event);
        }
    }
    let mut rows = messages.into_values().collect::<Vec<_>>();
    rows.sort_by(|a, b| b.at.cmp(&a.at));
    Ok(rows)
}

fn opencode_local_candidates(custom_path: Option<String>) -> Vec<PathBuf> {
    if let Some(path) = custom_path.filter(|value| !value.trim().is_empty()) {
        return vec![PathBuf::from(path.trim())];
    }
    let mut rows = Vec::new();
    if let Ok(value) = std::env::var("XDG_DATA_HOME") {
        rows.push(PathBuf::from(value).join("opencode").join("storage"));
    }
    if let Some(home) = dirs::home_dir() {
        rows.push(
            home.join(".local")
                .join("share")
                .join("opencode")
                .join("storage"),
        );
    }
    if let Some(roaming) = dirs::data_dir() {
        rows.push(roaming.join("opencode").join("storage"));
    }
    if let Some(local) = dirs::data_local_dir() {
        rows.push(local.join("opencode").join("storage"));
    }
    rows
}

fn collect_opencode_json_files(path: &Path, rows: &mut Vec<PathBuf>) {
    const MAX_FILES: usize = 2_000;
    if rows.len() >= MAX_FILES {
        return;
    }
    if path.is_file() {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if extension.eq_ignore_ascii_case("json") || extension.eq_ignore_ascii_case("jsonl") {
            rows.push(path.to_path_buf());
        }
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        if rows.len() >= MAX_FILES {
            break;
        }
        let next = entry.path();
        if next.is_dir() {
            collect_opencode_json_files(&next, rows);
        } else {
            collect_opencode_json_files(&next, rows);
        }
    }
}

fn opencode_u64(value: &serde_json::Value, pointers: &[&str]) -> u64 {
    pointers
        .iter()
        .find_map(|pointer| {
            let item = value.pointer(pointer)?;
            item.as_u64().or_else(|| {
                item.as_i64()
                    .map(|number| number.max(0) as u64)
                    .or_else(|| item.as_str()?.parse::<u64>().ok())
            })
        })
        .unwrap_or(0)
}

fn opencode_time(value: &serde_json::Value) -> DateTime<Utc> {
    for pointer in [
        "/time/completed",
        "/time/updated",
        "/time/created",
        "/updated_at",
        "/created_at",
        "/timestamp",
        "/at",
    ] {
        let Some(item) = value.pointer(pointer) else {
            continue;
        };
        if let Some(text) = item.as_str() {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(text) {
                return parsed.with_timezone(&Utc);
            }
            if let Ok(number) = text.parse::<i64>() {
                if let Some(parsed) = DateTime::<Utc>::from_timestamp(
                    if number > 10_000_000_000 {
                        number / 1_000
                    } else {
                        number
                    },
                    0,
                ) {
                    return parsed;
                }
            }
        }
        if let Some(number) = item
            .as_i64()
            .or_else(|| item.as_u64().map(|value| value as i64))
        {
            if let Some(parsed) = DateTime::<Utc>::from_timestamp(
                if number > 10_000_000_000 {
                    number / 1_000
                } else {
                    number
                },
                0,
            ) {
                return parsed;
            }
        }
    }
    Utc::now()
}

fn opencode_model(value: &serde_json::Value) -> String {
    let raw = ["/modelID", "/model/id", "/model", "/model_id"]
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(|item| item.as_str()))
        .unwrap_or("opencode-local");
    raw.rsplit('/').next().unwrap_or(raw).to_string()
}

fn opencode_event(value: &serde_json::Value, identity: &str) -> Option<UsageEvent> {
    let role = value.get("role").and_then(|item| item.as_str());
    let has_usage = value.get("tokens").is_some() || value.get("usage").is_some();
    if !has_usage || matches!(role, Some(role) if role != "assistant") {
        return None;
    }
    let cache_read = opencode_u64(
        value,
        &[
            "/tokens/cache/read",
            "/usage/cache_read_input_tokens",
            "/usage/prompt_cache_hit_tokens",
        ],
    );
    let cache_write = opencode_u64(
        value,
        &["/tokens/cache/write", "/usage/cache_creation_input_tokens"],
    );
    let direct_input = opencode_u64(
        value,
        &[
            "/tokens/input",
            "/usage/input_tokens",
            "/usage/prompt_tokens",
        ],
    );
    let output = opencode_u64(
        value,
        &[
            "/tokens/output",
            "/usage/output_tokens",
            "/usage/completion_tokens",
        ],
    ) + opencode_u64(value, &["/tokens/reasoning", "/usage/reasoning_tokens"]);
    if direct_input == 0 && output == 0 && cache_read == 0 && cache_write == 0 {
        return None;
    }
    let model = opencode_model(value);
    let source_id = ["/id", "/messageID", "/sessionID", "/session_id"]
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(|item| item.as_str()))
        .unwrap_or(identity);
    let digest = Sha256::digest(format!("{identity}:{source_id}").as_bytes());
    let raw_cost = value
        .get("cost")
        .and_then(|item| item.as_f64())
        .or_else(|| value.pointer("/usage/cost").and_then(|item| item.as_f64()))
        .unwrap_or(0.0);
    let input = direct_input + cache_read + cache_write;
    let cost = if raw_cost > 0.0 {
        raw_cost * 7.2
    } else {
        opencode_go_cost_usd(&model, input, output, cache_read) * 7.2
    };
    Some(UsageEvent {
        id: format!("opencode-json:{:x}", digest),
        provider: "OpenCode Go".into(),
        model,
        at: opencode_time(value),
        input,
        output,
        cached: cache_read,
        cost,
        task: "其他".into(),
    })
}

fn collect_opencode_events(value: &serde_json::Value, identity: &str, rows: &mut Vec<UsageEvent>) {
    if let Some(items) = value.as_array() {
        for (index, item) in items.iter().enumerate() {
            collect_opencode_events(item, &format!("{identity}:{index}"), rows);
        }
        return;
    }
    if let Some(event) = opencode_event(value, identity) {
        rows.push(event);
        return;
    }
    for key in ["messages", "events", "items", "data"] {
        if let Some(items) = value.get(key).and_then(|item| item.as_array()) {
            for (index, item) in items.iter().enumerate() {
                collect_opencode_events(item, &format!("{identity}:{key}:{index}"), rows);
            }
        }
    }
}

pub(crate) fn sync_opencode_json_into(
    target: &Connection,
    custom_path: Option<String>,
    days: u32,
    source_id: &str,
    force: bool,
) -> Result<providers::opencode::LocalSyncOutcome, String> {
    let source = opencode_local_candidates(custom_path)
        .into_iter()
        .find(|path| path.exists())
        .ok_or("未发现 OpenCode JSON/JSONL 本地记录")?;
    let message_root = if source.is_dir() && source.join("message").exists() {
        source.join("message")
    } else if source.is_dir() && source.join("storage").join("message").exists() {
        source.join("storage").join("message")
    } else {
        source.clone()
    };
    let mut files = Vec::new();
    collect_opencode_json_files(&message_root, &mut files);
    let since = Utc::now().timestamp() - days.clamp(1, 365) as i64 * 86_400;
    let mut events = Vec::new();
    let previous = if force {
        serde_json::Map::new()
    } else {
        target
            .query_row(
                "SELECT cursor_json FROM sync_cursors WHERE source_id=?1",
                [source_id],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|value| {
                value
                    .get("json_files")
                    .and_then(|value| value.as_object())
                    .cloned()
            })
            .unwrap_or_default()
    };
    let mut next_cursor = previous.clone();
    for path in &files {
        let Ok(metadata) = fs::metadata(path) else {
            continue;
        };
        if metadata.len() == 0 || metadata.len() > 8 * 1024 * 1024 {
            continue;
        }
        let modified = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|value| value.as_millis() as u64)
            .unwrap_or_default();
        let key = path.to_string_lossy().into_owned();
        let fingerprint = format!("{}:{modified}", metadata.len());
        if !force && previous.get(&key).and_then(|value| value.as_str()) == Some(&fingerprint) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let identity = path.to_string_lossy();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            collect_opencode_events(&value, &identity, &mut events);
        } else {
            for (index, line) in text.lines().enumerate() {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                    collect_opencode_events(&value, &format!("{identity}:{index}"), &mut events);
                }
            }
        }
        next_cursor.insert(key, serde_json::Value::String(fingerprint));
    }
    events.retain(|event| event.at.timestamp() >= since);
    let mut imported = 0usize;
    for event in &events {
        let mut provenance =
            reliability::UsageProvenance::observed(source_id, reliability::SourceKind::LocalJson);
        provenance.source_ref = Some(event.id.clone());
        if reliability::upsert_usage(target, event, &provenance)
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
            params![
                source_id,
                serde_json::json!({"json_files":next_cursor}).to_string(),
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(providers::opencode::LocalSyncOutcome {
        imported,
        scanned_files: files.len(),
        source_kind: reliability::SourceKind::LocalJson.as_str().into(),
        source_path: source.to_string_lossy().into_owned(),
        detail: if events.is_empty() {
            "已读取目录，但最近周期内没有包含 Token 的 OpenCode JSON 记录".into()
        } else {
            format!("已从本地 JSON 同步 {imported} 条 OpenCode 用量")
        },
    })
}

/// 读取 OpenCode 官方旧版 storage/message JSON，或用户指定的 JSON/JSONL 导出目录。
/// 只提取模型、时间、Token、缓存和成本，不保存提示词、回复正文或认证信息。
#[tauri::command]
fn sync_opencode_local_usage(
    custom_path: Option<String>,
    days: Option<u32>,
    state: State<AppDb>,
) -> Result<OpenCodeLocalSyncResult, String> {
    let source = opencode_local_candidates(custom_path)
        .into_iter()
        .find(|path| path.exists())
        .ok_or("未发现 OpenCode 本地 JSON；请在“账户与模型”中选择 JSON 记录目录")?;
    let message_root = if source.is_dir() && source.join("message").exists() {
        source.join("message")
    } else if source.is_dir() && source.join("storage").join("message").exists() {
        source.join("storage").join("message")
    } else {
        source.clone()
    };
    let mut files = Vec::new();
    collect_opencode_json_files(&message_root, &mut files);
    let since = Utc::now().timestamp() - days.unwrap_or(30).clamp(1, 365) as i64 * 86_400;
    let mut events = Vec::new();
    for path in &files {
        let Ok(metadata) = fs::metadata(path) else {
            continue;
        };
        if metadata.len() == 0 || metadata.len() > 8 * 1024 * 1024 {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let identity = path.to_string_lossy();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            collect_opencode_events(&value, &identity, &mut events);
        } else {
            for (index, line) in text.lines().enumerate() {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                    collect_opencode_events(&value, &format!("{identity}:{index}"), &mut events);
                }
            }
        }
    }
    events.retain(|event| event.at.timestamp() >= since);
    let db = state.0.lock().map_err(|_| "数据库锁定")?;
    for event in &events {
        let mut provenance = reliability::UsageProvenance::observed(
            "opencode-local",
            reliability::SourceKind::LocalJson,
        );
        provenance.source_ref = Some(event.id.clone());
        reliability::upsert_usage(&db, event, &provenance).map_err(|error| error.to_string())?;
    }
    Ok(OpenCodeLocalSyncResult {
        imported: events.len(),
        scanned_files: files.len(),
        source: source.to_string_lossy().into_owned(),
        detail: if events.is_empty() {
            "已读取目录，但最近周期内没有包含 Token 的 OpenCode JSON 记录".into()
        } else {
            format!("已从本地 JSON 同步 {} 条 OpenCode 用量", events.len())
        },
    })
}
/// 返回最近七天每个 Codex turn 的最终 token 计数，供本地柱状图使用。
#[tauri::command]
fn codex_usage_series() -> Result<Vec<CodexUsagePoint>, String> {
    let path = dirs::home_dir()
        .ok_or("无法定位用户目录")?
        .join(".codex")
        .join("logs_2.sqlite");
    if !path.exists() {
        return Err("未发现 Codex 日志数据库".into());
    }
    let since = Utc::now().timestamp() - 7 * 24 * 3600;
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let mut query=db.prepare("SELECT ts,feedback_log_body FROM logs WHERE ts>=?1 AND feedback_log_body LIKE '%total_usage_tokens=%'").map_err(|e|e.to_string())?;
    let rows = query
        .query_map([since], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut turns: HashMap<String, CodexUsagePoint> = HashMap::new();
    for row in rows {
        let (ts, body) = row.map_err(|e| e.to_string())?;
        let Some(body) = body else { continue };
        let Some(id) = field(&body, "turn_id=") else {
            continue;
        };
        let Some(tokens) = field(&body, "total_usage_tokens=").and_then(|v| v.parse::<u64>().ok())
        else {
            continue;
        };
        let model = field(&body, "model=").unwrap_or_else(|| "Codex".into());
        let lower = body.to_ascii_lowercase();
        let category = if lower.contains("task_type=generation") || lower.contains("intent=create")
        {
            "全新生成"
        } else if lower.contains("task_type=explain") || lower.contains("intent=explain") {
            "问答解释"
        } else if lower.contains("task_type=debug") || lower.contains("intent=fix") {
            "修改调试"
        } else if tokens >= 30_000 {
            "全新生成"
        } else if tokens <= 8_000 {
            "问答解释"
        } else {
            "修改调试"
        }
        .to_string();
        let temperature = field(&body, "temperature=").and_then(|v| v.parse::<f64>().ok());
        let replace = turns
            .get(&id)
            .map(|old| tokens > old.tokens || ts > old.at)
            .unwrap_or(true);
        if replace {
            turns.insert(
                id.clone(),
                CodexUsagePoint {
                    at: ts,
                    tokens,
                    model,
                    session_id: id,
                    category,
                    temperature,
                },
            );
        }
    }
    let mut points = turns.into_values().collect::<Vec<_>>();
    points.sort_by_key(|point| point.at);
    Ok(points)
}
/// 每个 turn 取最终的最大 total_usage_tokens，避免流式日志多次累计造成重复计数。
#[tauri::command]
fn codex_window_usage(state: State<AppDb>) -> Result<CodexWindowUsage, String> {
    let path = dirs::home_dir()
        .ok_or("无法定位用户目录")?
        .join(".codex")
        .join("logs_2.sqlite");
    if !path.exists() {
        return Err("未发现 Codex 日志数据库".into());
    }
    let now = Utc::now().timestamp();
    let week_start = now - 7 * 24 * 3600;
    let short_start = now - 5 * 3600;
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let mut query=db.prepare("SELECT ts,feedback_log_body FROM logs WHERE ts>=?1 AND feedback_log_body LIKE '%post sampling token usage%'").map_err(|e|e.to_string())?;
    let rows = query
        .query_map([week_start], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut turns: HashMap<String, (i64, u64)> = HashMap::new();
    for row in rows {
        let (ts, body) = row.map_err(|e| e.to_string())?;
        let Some(body) = body else { continue };
        let Some(id) = field(&body, "turn_id=") else {
            continue;
        };
        let Some(tokens) = field(&body, "total_usage_tokens=").and_then(|v| v.parse::<u64>().ok())
        else {
            continue;
        };
        let entry = turns.entry(id).or_insert((ts, tokens));
        if tokens > entry.1 || ts > entry.0 {
            *entry = (ts, tokens)
        }
    }
    let mut five = 0;
    let mut seven = 0;
    for (_, (ts, tokens)) in turns {
        seven += tokens;
        if ts >= short_start {
            five += tokens
        }
    }
    Ok(CodexWindowUsage {
        five_hour_used: five,
        seven_day_used: seven,
        budget: current_budget(&state)?,
        source: "Codex 本地日志（每个 turn 最终 token 计数）".into(),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ArenaModel {
    rank: u32,
    model: String,
    provider: String,
    license: String,
    score: Option<f64>,
    score_lower: Option<f64>,
    score_upper: Option<f64>,
    votes: Option<u64>,
    category: String,
    published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArenaSnapshot {
    board: String,
    updated_at: String,
    #[serde(default)]
    checked_at: String,
    official_published_at: String,
    source_url: String,
    source_state: String,
    fresh: bool,
    status: String,
    total_models: u64,
    models: Vec<ArenaModel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ArenaModelProfileDimension {
    id: String,
    label: String,
    #[serde(default)]
    sync_state: String,
    #[serde(default)]
    error: Option<String>,
    rank: Option<u32>,
    total_models: Option<u64>,
    score: Option<f64>,
    score_lower: Option<f64>,
    score_upper: Option<f64>,
    votes: Option<u64>,
    percentile: Option<f64>,
    published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArenaModelProfile {
    model: String,
    updated_at: String,
    #[serde(default)]
    checked_at: String,
    source_url: String,
    source_state: String,
    fresh: bool,
    status: String,
    dimensions: Vec<ArenaModelProfileDimension>,
}

#[derive(Debug, Clone, Copy)]
struct ArenaModelProfileDimensionSpec {
    id: &'static str,
    label: &'static str,
}

const ARENA_MODEL_PROFILE_DIMENSIONS: [ArenaModelProfileDimensionSpec; 6] = [
    ArenaModelProfileDimensionSpec {
        id: "coding",
        label: "编程",
    },
    ArenaModelProfileDimensionSpec {
        id: "math",
        label: "数学",
    },
    ArenaModelProfileDimensionSpec {
        id: "instruction_following",
        label: "指令遵循",
    },
    ArenaModelProfileDimensionSpec {
        id: "multi_turn",
        label: "多轮对话",
    },
    ArenaModelProfileDimensionSpec {
        id: "creative_writing",
        label: "创意写作",
    },
    ArenaModelProfileDimensionSpec {
        id: "longer_query",
        label: "长问题处理",
    },
];

#[derive(Debug, Clone, Copy)]
struct ArenaBoardSpec {
    id: &'static str,
    config: &'static str,
    category: &'static str,
    source_url: &'static str,
}

fn arena_board_spec(board: &str) -> ArenaBoardSpec {
    match board {
        "coding" => ArenaBoardSpec {
            id: "coding",
            config: "text_style_control",
            category: "coding",
            source_url: "https://arena.ai/leaderboard/text/coding",
        },
        "webdev" => ArenaBoardSpec {
            id: "webdev",
            config: "webdev",
            category: "overall",
            source_url: "https://arena.ai/leaderboard/code/webdev",
        },
        "vision" => ArenaBoardSpec {
            id: "vision",
            config: "vision_style_control",
            category: "overall",
            source_url: "https://arena.ai/leaderboard/vision",
        },
        "search" => ArenaBoardSpec {
            id: "search",
            config: "search_style_control",
            category: "overall",
            source_url: "https://arena.ai/leaderboard/search",
        },
        "document" => ArenaBoardSpec {
            id: "document",
            config: "document_style_control",
            category: "overall",
            source_url: "https://arena.ai/leaderboard/document",
        },
        _ => ArenaBoardSpec {
            id: "overall",
            config: "text_style_control",
            category: "overall",
            source_url: "https://arena.ai/leaderboard/text",
        },
    }
}

fn arena_provider(organization: &str) -> String {
    match organization.trim().to_ascii_lowercase().as_str() {
        "anthropic" => "Anthropic",
        "openai" => "OpenAI",
        "google" | "google deepmind" => "Google Gemini",
        "deepseek" => "DeepSeek",
        "moonshot" | "moonshot ai" => "Kimi",
        "xiaomi" => "小米 MiMo",
        "z.ai" | "z-ai" | "zai" | "zhipu ai" => "智谱 AI",
        "alibaba" | "qwen" => "通义百炼",
        "bytedance" | "byteplus" => "豆包",
        "tencent" => "腾讯混元",
        "baidu" => "文心千帆",
        "minimax" => "MiniMax",
        "xai" | "spacexai" => "xAI",
        "meta" => "Meta",
        "mistral" | "mistral ai" => "Mistral AI",
        "nvidia" => "NVIDIA",
        "perplexity" | "perplexity ai" => "Perplexity",
        "huawei" | "huawei noah's ark lab" => "华为",
        "stepfun" | "step fun" => "阶跃星辰",
        "01.ai" | "01 ai" | "lingyiwanwu" => "零一万物",
        "baichuan" | "baichuan intelligence" => "百川智能",
        value if value.is_empty() => "未知实验室",
        _ => organization.trim(),
    }
    .to_string()
}

fn arena_cache_timestamp(checked_at: &str, updated_at: &str) -> Option<i64> {
    [checked_at, updated_at].into_iter().find_map(|value| {
        (!value.trim().is_empty())
            .then(|| DateTime::parse_from_rfc3339(value).ok())
            .flatten()
            .map(|date| date.timestamp())
    })
}

fn arena_cache_is_fresh_at(
    checked_at: &str,
    updated_at: &str,
    now_timestamp: i64,
    ttl_seconds: i64,
) -> bool {
    arena_cache_timestamp(checked_at, updated_at)
        .map(|timestamp| {
            let age = now_timestamp.saturating_sub(timestamp);
            age >= 0 && age < ttl_seconds
        })
        .unwrap_or(false)
}

fn arena_snapshot_data_equal(left: &ArenaSnapshot, right: &ArenaSnapshot) -> bool {
    left.board == right.board
        && left.official_published_at == right.official_published_at
        && left.total_models == right.total_models
        && left.models == right.models
}

fn latest_arena_published_at(models: &[ArenaModel]) -> String {
    models
        .iter()
        .filter_map(|item| {
            let value = item.published_at.trim();
            (!value.is_empty()).then_some(value)
        })
        .max()
        .unwrap_or_default()
        .to_string()
}

fn arena_rank_percentile(rank: u64, total_models: u64) -> Option<f64> {
    if rank == 0 || total_models == 0 || rank > total_models {
        return None;
    }
    if total_models == 1 {
        return Some(100.0);
    }
    Some(((total_models - rank) as f64 / (total_models - 1) as f64 * 100.0).clamp(0.0, 100.0))
}

fn load_arena_cache(path: &Path) -> Option<ArenaSnapshot> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ArenaSnapshot>(&bytes).ok())
}

fn load_arena_model_profile_cache(path: &Path) -> Option<ArenaModelProfile> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ArenaModelProfile>(&bytes).ok())
}

fn arena_model_profile_cache_path(model: &str) -> PathBuf {
    let digest = Sha256::digest(model.trim().to_ascii_lowercase().as_bytes());
    let key = digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    data_path().with_file_name(format!("arena-model-profile-{key}.json"))
}

fn arena_model_profile_data_equal(left: &ArenaModelProfile, right: &ArenaModelProfile) -> bool {
    left.model.eq_ignore_ascii_case(&right.model) && left.dimensions == right.dimensions
}

async fn fetch_arena_official(spec: ArenaBoardSpec) -> Result<ArenaSnapshot, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(18))
        .build()
        .map_err(|e| e.to_string())?;
    let mut offset = 0usize;
    let mut total_models = 0u64;
    let mut models = Vec::new();
    loop {
        let mut url = reqwest::Url::parse("https://datasets-server.huggingface.co/filter")
            .map_err(|e| e.to_string())?;
        url.query_pairs_mut()
            .append_pair("dataset", "lmarena-ai/leaderboard-dataset")
            .append_pair("config", spec.config)
            .append_pair("split", "latest")
            .append_pair("where", &format!("\"category\"='{}'", spec.category))
            .append_pair("orderby", "\"rank\" ASC")
            .append_pair("offset", &offset.to_string())
            .append_pair("length", "100");
        let page = client
            .get(url)
            .header(
                reqwest::header::USER_AGENT,
                "Token Manager/0.7 (+local desktop usage monitor)",
            )
            .send()
            .await
            .map_err(|e| format!("无法连接 Arena 官方数据集：{e}"))?
            .error_for_status()
            .map_err(|e| format!("Arena 官方数据集返回错误状态：{e}"))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("无法解析 Arena 官方数据集：{e}"))?;
        total_models = page
            .get("num_rows_total")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(total_models);
        let rows = page
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "Arena 官方数据集缺少 rows 字段".to_string())?;
        if rows.is_empty() {
            break;
        }
        for item in rows {
            let Some(row) = item.get("row") else {
                continue;
            };
            let Some(model) = row.get("model_name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let Some(rank) = arena_json_u64(row, "rank")
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value > 0)
            else {
                // 排名缺失时不能用数组顺序伪造官方名次；直接忽略该条不完整记录。
                continue;
            };
            let organization = row
                .get("organization")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let published_at = row
                .get("leaderboard_publish_date")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            models.push(ArenaModel {
                rank,
                model: model.to_string(),
                provider: arena_provider(organization),
                license: row
                    .get("license")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("未标注")
                    .to_string(),
                score: arena_json_f64(row, "rating"),
                score_lower: arena_json_f64(row, "rating_lower"),
                score_upper: arena_json_f64(row, "rating_upper"),
                votes: arena_json_u64(row, "vote_count"),
                category: spec.category.to_string(),
                published_at,
            });
        }
        offset += rows.len();
        if offset as u64 >= total_models || offset >= 1_000 {
            break;
        }
    }
    if models.len() < 5 {
        return Err("Arena 官方数据集返回的有效模型不足，已保留本地缓存".into());
    }
    models.sort_by_key(|item| item.rank);
    let official_published_at = latest_arena_published_at(&models);
    let checked_at = Utc::now().to_rfc3339();
    let snapshot = ArenaSnapshot {
        board: spec.id.into(),
        updated_at: checked_at.clone(),
        checked_at,
        official_published_at,
        source_url: spec.source_url.into(),
        source_state: "live".into(),
        fresh: true,
        status: format!("已同步 Arena 官方公开数据集 · {} 个模型", models.len()),
        total_models: total_models.max(models.len() as u64),
        models,
    };
    Ok(snapshot)
}

/// 从 Arena 官方发布的结构化排行榜数据集读取名次与置信区间。
/// 30 分钟内复用本机缓存；联网失败时返回最后一次成功快照并明确标记为离线缓存。
#[tauri::command]
async fn fetch_arena_rankings(board: Option<String>, force: bool) -> Result<ArenaSnapshot, String> {
    let spec = arena_board_spec(board.as_deref().unwrap_or("overall"));
    let cache_path = data_path().with_file_name(format!("arena-rankings-{}.json", spec.id));
    let cached = load_arena_cache(&cache_path);
    if !force {
        if let Some(mut snapshot) = cached.clone() {
            let retrying_stale_snapshot = snapshot.source_state == "stale";
            let cache_ttl_seconds = if retrying_stale_snapshot { 300 } else { 1_800 };
            if arena_cache_is_fresh_at(
                &snapshot.checked_at,
                &snapshot.updated_at,
                Utc::now().timestamp(),
                cache_ttl_seconds,
            ) {
                snapshot.fresh = false;
                if retrying_stale_snapshot {
                    snapshot.status = "上次官方同步失败 · 显示本机快照，5 分钟后自动重试".into();
                } else {
                    snapshot.source_state = "cache".into();
                    snapshot.status = "使用本机缓存 · 30 分钟内无需重复联网检查".into();
                }
                return Ok(snapshot);
            }
        }
    }

    let mut snapshot = match fetch_arena_official(spec).await {
        Ok(snapshot) => snapshot,
        Err(error) => {
            if let Some(mut snapshot) = cached {
                snapshot.checked_at = Utc::now().to_rfc3339();
                snapshot.fresh = false;
                snapshot.source_state = "stale".into();
                snapshot.status = format!("官方同步失败，显示上次成功快照：{error}");
                // 保存失败检查时间，避免每次切换榜单都立刻重复请求；5 分钟后再重试。
                if let Ok(content) = serde_json::to_vec_pretty(&snapshot) {
                    let _ = fs::write(&cache_path, content);
                }
                return Ok(snapshot);
            }
            return Err(error);
        }
    };
    if let Some(previous) = cached.as_ref() {
        if arena_snapshot_data_equal(previous, &snapshot) {
            if !previous.updated_at.trim().is_empty() {
                snapshot.updated_at = previous.updated_at.clone();
            }
            snapshot.status = format!(
                "Arena 官方榜单暂无变化 · 已核对 {} 个模型",
                snapshot.models.len()
            );
        }
    }
    fs::write(
        &cache_path,
        serde_json::to_vec_pretty(&snapshot).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Arena 缓存写入失败：{e}"))?;
    Ok(snapshot)
}

fn arena_json_u64(value: &serde_json::Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|item| {
        item.as_u64().or_else(|| {
            item.as_f64()
                .filter(|number| number.is_finite() && *number >= 0.0)
                .map(|number| number.round() as u64)
        })
    })
}

fn arena_json_f64(value: &serde_json::Value, key: &str) -> Option<f64> {
    value
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .filter(|number| number.is_finite())
}

async fn fetch_arena_dataset_page(
    client: &reqwest::Client,
    where_clause: &str,
    length: usize,
) -> Result<serde_json::Value, String> {
    let mut url = reqwest::Url::parse("https://datasets-server.huggingface.co/filter")
        .map_err(|error| error.to_string())?;
    url.query_pairs_mut()
        .append_pair("dataset", "lmarena-ai/leaderboard-dataset")
        .append_pair("config", "text_style_control")
        .append_pair("split", "latest")
        .append_pair("where", where_clause)
        .append_pair("orderby", "\"rank\" ASC")
        .append_pair("offset", "0")
        .append_pair("length", &length.to_string());
    client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "Token Manager/0.7 (+local desktop usage monitor)",
        )
        .send()
        .await
        .map_err(|error| format!("无法连接 Arena 官方六维数据：{error}"))?
        .error_for_status()
        .map_err(|error| format!("Arena 官方六维数据返回错误状态：{error}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("无法解析 Arena 官方六维数据：{error}"))
}

async fn fetch_arena_profile_dimension(
    client: &reqwest::Client,
    spec: ArenaModelProfileDimensionSpec,
    model: &str,
) -> Result<ArenaModelProfileDimension, String> {
    let category_where = format!("\"category\"='{}'", spec.id);
    let category_page = fetch_arena_dataset_page(client, &category_where, 1).await?;
    let total_models = category_page
        .get("num_rows_total")
        .and_then(serde_json::Value::as_u64)
        .filter(|total| *total > 0);

    let escaped_model = model.replace('\'', "''");
    let model_where = format!(
        "\"category\"='{}' AND \"model_name\"='{}'",
        spec.id, escaped_model
    );
    let model_page = fetch_arena_dataset_page(client, &model_where, 10).await?;
    let row = model_page
        .get("rows")
        .and_then(serde_json::Value::as_array)
        .and_then(|rows| {
            rows.iter().find_map(|item| {
                let row = item.get("row")?;
                let row_model = row.get("model_name")?.as_str()?;
                row_model.eq_ignore_ascii_case(model).then_some(row)
            })
        });

    let rank = row
        .and_then(|value| arena_json_u64(value, "rank"))
        .and_then(|value| u32::try_from(value).ok());
    let published_at = row
        .and_then(|value| value.get("leaderboard_publish_date"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    Ok(ArenaModelProfileDimension {
        id: spec.id.into(),
        label: spec.label.into(),
        sync_state: "live".into(),
        error: None,
        rank,
        total_models,
        score: row.and_then(|value| arena_json_f64(value, "rating")),
        score_lower: row.and_then(|value| arena_json_f64(value, "rating_lower")),
        score_upper: row.and_then(|value| arena_json_f64(value, "rating_upper")),
        votes: row.and_then(|value| arena_json_u64(value, "vote_count")),
        percentile: rank
            .zip(total_models)
            .and_then(|(rank, total)| arena_rank_percentile(rank as u64, total)),
        published_at,
    })
}

async fn fetch_arena_model_profile_official(model: &str) -> Result<ArenaModelProfile, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(18))
        .build()
        .map_err(|error| error.to_string())?;
    let futures = ARENA_MODEL_PROFILE_DIMENSIONS
        .iter()
        .copied()
        .map(|spec| fetch_arena_profile_dimension(&client, spec, model));
    let results = futures_util::future::join_all(futures).await;
    let mut failed_dimensions = Vec::new();
    let dimensions = ARENA_MODEL_PROFILE_DIMENSIONS
        .iter()
        .copied()
        .zip(results)
        .map(|(spec, result)| match result {
            Ok(dimension) => dimension,
            Err(error) => {
                failed_dimensions.push(spec.label);
                ArenaModelProfileDimension {
                    id: spec.id.into(),
                    label: spec.label.into(),
                    sync_state: "error".into(),
                    error: Some(error),
                    rank: None,
                    total_models: None,
                    score: None,
                    score_lower: None,
                    score_upper: None,
                    votes: None,
                    percentile: None,
                    published_at: None,
                }
            }
        })
        .collect::<Vec<_>>();
    if failed_dimensions.len() == ARENA_MODEL_PROFILE_DIMENSIONS.len() {
        return Err("Arena 官方六维分类暂时全部无法连接".into());
    }
    let available = dimensions
        .iter()
        .filter(|dimension| dimension.rank.is_some())
        .count();
    let checked_at = Utc::now().to_rfc3339();
    Ok(ArenaModelProfile {
        model: model.to_string(),
        updated_at: checked_at.clone(),
        checked_at,
        source_url: "https://arena.ai/leaderboard/text".into(),
        source_state: if failed_dimensions.is_empty() {
            "live".into()
        } else {
            "partial".into()
        },
        fresh: failed_dimensions.is_empty(),
        status: if !failed_dimensions.is_empty() {
            format!(
                "已同步可用的官方六维分类 · {available}/6 项有排名 · {} 项暂时连接失败",
                failed_dimensions.len()
            )
        } else if available > 0 {
            format!("已同步 Arena 官方六维分类 · {available}/6 项有排名")
        } else {
            "Arena 官方六维分类中暂未找到该模型".into()
        },
        dimensions,
    })
}

/// 读取一个模型在 Arena Text 榜单六个独立官方分类中的真实位置。
/// 缺失分类保持为 None，不根据综合分推算；30 分钟内复用本机缓存。
#[tauri::command]
async fn fetch_arena_model_profile(
    model: String,
    force: bool,
) -> Result<ArenaModelProfile, String> {
    let model = model.trim();
    if model.is_empty() {
        return Err("模型名称不能为空".into());
    }
    let cache_path = arena_model_profile_cache_path(model);
    let cached = load_arena_model_profile_cache(&cache_path)
        .filter(|snapshot| snapshot.model.eq_ignore_ascii_case(model));
    if !force {
        if let Some(mut snapshot) = cached.clone() {
            let retrying_stale_snapshot = snapshot.source_state == "stale";
            let cache_ttl_seconds = if retrying_stale_snapshot { 300 } else { 1_800 };
            if arena_cache_is_fresh_at(
                &snapshot.checked_at,
                &snapshot.updated_at,
                Utc::now().timestamp(),
                cache_ttl_seconds,
            ) {
                snapshot.fresh = false;
                if retrying_stale_snapshot {
                    snapshot.status =
                        "上次官方六维同步失败 · 显示本机快照，5 分钟后自动重试".into();
                } else {
                    snapshot.source_state = "cache".into();
                    snapshot.status = "使用本机六维缓存 · 30 分钟内无需重复联网检查".into();
                }
                return Ok(snapshot);
            }
        }
    }

    let mut snapshot = match fetch_arena_model_profile_official(model).await {
        Ok(snapshot) => snapshot,
        Err(error) => {
            if let Some(mut snapshot) = cached {
                snapshot.checked_at = Utc::now().to_rfc3339();
                snapshot.fresh = false;
                snapshot.source_state = "stale".into();
                snapshot.status = format!("官方六维同步失败，显示上次成功快照：{error}");
                // 与排行榜缓存保持相同的 5 分钟退避，避免选择同一模型时反复联网。
                if let Ok(content) = serde_json::to_vec_pretty(&snapshot) {
                    let _ = fs::write(&cache_path, content);
                }
                return Ok(snapshot);
            }
            return Err(error);
        }
    };
    if let Some(previous) = cached.as_ref() {
        if arena_model_profile_data_equal(previous, &snapshot) {
            if !previous.updated_at.trim().is_empty() {
                snapshot.updated_at = previous.updated_at.clone();
            }
            let available = snapshot
                .dimensions
                .iter()
                .filter(|dimension| dimension.rank.is_some())
                .count();
            snapshot.status = format!("Arena 官方六维数据暂无变化 · {available}/6 项有排名");
        }
    }
    fs::write(
        &cache_path,
        serde_json::to_vec_pretty(&snapshot).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("Arena 六维缓存写入失败：{error}"))?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_arena_boards_and_official_providers() {
        let coding = arena_board_spec("coding");
        assert_eq!(coding.config, "text_style_control");
        assert_eq!(coding.category, "coding");
        assert_eq!(
            coding.source_url,
            "https://arena.ai/leaderboard/text/coding"
        );
        assert_eq!(arena_provider("Moonshot AI"), "Kimi");
        assert_eq!(arena_provider("Xiaomi"), "小米 MiMo");
        assert_eq!(arena_provider("Z.AI"), "智谱 AI");
        assert_eq!(arena_provider("zai"), "智谱 AI");
        assert_eq!(arena_provider("Mistral AI"), "Mistral AI");
        assert_eq!(arena_provider("NVIDIA"), "NVIDIA");
        assert_eq!(arena_provider("StepFun"), "阶跃星辰");
        assert_eq!(arena_provider("01.AI"), "零一万物");
    }

    #[test]
    fn calculates_arena_rank_percentile_boundaries() {
        assert_eq!(arena_rank_percentile(1, 10), Some(100.0));
        assert_eq!(arena_rank_percentile(10, 10), Some(0.0));
        assert_eq!(arena_rank_percentile(5, 9), Some(50.0));
        assert_eq!(arena_rank_percentile(1, 1), Some(100.0));
        assert_eq!(arena_rank_percentile(0, 10), None);
        assert_eq!(arena_rank_percentile(11, 10), None);
        assert_eq!(arena_rank_percentile(1, 0), None);
    }

    #[test]
    fn exposes_six_official_arena_profile_dimensions() {
        let dimensions = ARENA_MODEL_PROFILE_DIMENSIONS
            .iter()
            .map(|dimension| (dimension.id, dimension.label))
            .collect::<Vec<_>>();
        assert_eq!(
            dimensions,
            vec![
                ("coding", "编程"),
                ("math", "数学"),
                ("instruction_following", "指令遵循"),
                ("multi_turn", "多轮对话"),
                ("creative_writing", "创意写作"),
                ("longer_query", "长问题处理"),
            ]
        );
    }

    #[test]
    fn keeps_missing_arena_metrics_unknown_instead_of_faking_zero() {
        let missing = serde_json::json!({
            "rank": 8,
            "rating": null,
            "rating_lower": null,
            "rating_upper": null,
            "vote_count": null
        });
        assert_eq!(arena_json_f64(&missing, "rating"), None);
        assert_eq!(arena_json_f64(&missing, "rating_lower"), None);
        assert_eq!(arena_json_f64(&missing, "rating_upper"), None);
        assert_eq!(arena_json_u64(&missing, "vote_count"), None);

        let published = serde_json::json!({
            "rating": 1234.56,
            "vote_count": 9876.4
        });
        assert_eq!(arena_json_f64(&published, "rating"), Some(1234.56));
        assert_eq!(arena_json_u64(&published, "vote_count"), Some(9876));
    }

    #[test]
    fn arena_cache_ttl_prefers_checked_at_and_falls_back_to_updated_at() {
        let now = DateTime::parse_from_rfc3339("2026-07-30T12:30:00Z")
            .unwrap()
            .timestamp();
        assert!(arena_cache_is_fresh_at(
            "2026-07-30T12:15:01Z",
            "2026-07-20T00:00:00Z",
            now,
            1_800
        ));
        assert!(!arena_cache_is_fresh_at(
            "2026-07-30T11:59:59Z",
            "2026-07-30T12:29:59Z",
            now,
            1_800
        ));
        assert!(arena_cache_is_fresh_at(
            "",
            "2026-07-30T12:29:59Z",
            now,
            1_800
        ));
    }

    #[test]
    fn parses_deepseek_stream_usage() {
        let body=b"data: {\"choices\":[{\"delta\":{}}],\"usage\":{\"prompt_tokens\":1000,\"completion_tokens\":200,\"prompt_cache_hit_tokens\":800}}\n\ndata: [DONE]\n";
        let values = usage_values(body);
        assert_eq!(usage_counts(&values[0]), Some((1000, 200, 800)));
    }
    #[test]
    fn parses_anthropic_and_gemini_usage() {
        let anthropic = serde_json::json!({"usage":{"input_tokens":120,"output_tokens":30,"cache_read_input_tokens":80}});
        assert_eq!(usage_counts(&anthropic), Some((120, 30, 80)));
        let anthropic_start = serde_json::json!({"type":"message_start","message":{"usage":{"input_tokens":10240,"cache_read_input_tokens":8192}}});
        assert_eq!(usage_counts(&anthropic_start), Some((10240, 0, 8192)));
        let gemini = serde_json::json!({"usageMetadata":{"promptTokenCount":500,"candidatesTokenCount":70,"thoughtsTokenCount":20,"cachedContentTokenCount":300}});
        assert_eq!(usage_counts(&gemini), Some((500, 90, 300)));
    }
    #[test]
    fn normalizes_deepseek_v4_anthropic_aliases() {
        assert_eq!(
            normalized_proxy_model("DeepSeek", "deepseek-v4-pro[1m]"),
            "deepseek-v4-pro"
        );
        assert_eq!(
            normalized_proxy_model("DeepSeek", "claude-opus-4-6"),
            "deepseek-v4-pro"
        );
        assert_eq!(
            normalized_proxy_model("DeepSeek", "claude-sonnet-4-6"),
            "deepseek-v4-flash"
        );
    }
    #[test]
    fn calculates_v4_pro_cny_cost() {
        let cost = deepseek_cost("deepseek-v4-pro", 1_000_000, 1_000_000, 200_000);
        assert!((cost - 8.405).abs() < 0.000001);
    }
    #[test]
    fn calculates_versioned_provider_cost() {
        let cost = provider_cost("小米 MiMo", "mimo-v2.5-pro", 1_000_000, 500_000, 200_000);
        assert!((cost - 2.32).abs() < 0.000001);
        assert_eq!(
            provider_cost("自定义 OpenAI 兼容", "unknown", 10, 10, 0),
            0.0
        );
    }
    #[test]
    fn calculates_opencode_go_observed_cost() {
        let usd = opencode_go_cost_usd("deepseek-v4-pro", 1_000_000, 500_000, 200_000);
        assert!((usd - 0.783725).abs() < 0.000001);
        let cny = provider_cost(
            "OpenCode Go",
            "deepseek-v4-pro",
            1_000_000,
            500_000,
            200_000,
        );
        assert!((cny - usd * 7.2).abs() < 0.000001);
    }
    #[test]
    fn parses_opencode_local_json_usage_without_message_content() {
        let value = serde_json::json!({
            "id":"msg_test",
            "role":"assistant",
            "providerID":"opencode",
            "modelID":"deepseek-v4-pro",
            "time":{"completed":1786300800000_i64},
            "cost":0.125,
            "tokens":{"input":1200,"output":340,"reasoning":60,"cache":{"read":800,"write":100}},
            "parts":[{"type":"text","text":"这段正文不能进入 Token Manager 数据库"}]
        });
        let event = opencode_event(&value, "fixture").expect("应解析 OpenCode usage");
        assert_eq!(event.provider, "OpenCode Go");
        assert_eq!(event.model, "deepseek-v4-pro");
        assert_eq!(event.input, 2100);
        assert_eq!(event.output, 400);
        assert_eq!(event.cached, 800);
        assert!((event.cost - 0.9).abs() < 0.000001);
        assert!(!event.id.contains("正文"));
    }
    #[test]
    fn parses_codex_client_rate_limits() {
        let line = r#"{"timestamp":"2026-07-22T05:52:09.665Z","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":12.0,"window_minutes":300,"resets_at":1785300000},"secondary":{"used_percent":34.0,"window_minutes":10080,"resets_at":1785900000},"credits":{"balance":"0"},"plan_type":"plus"}}}"#;
        let snapshot = parse_codex_quota_line(line).expect("应解析额度事件");
        assert_eq!(snapshot.five_hour.unwrap().remaining_percent, 88.0);
        assert_eq!(snapshot.seven_day.unwrap().remaining_percent, 66.0);
        assert_eq!(snapshot.plan_type.as_deref(), Some("plus"));
    }
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppDb::open())
        .manage(reliability::SyncCoordinator::default())
        .manage(ProxyServerState(Mutex::new(HashMap::new())))
        .manage(FloatingWindowRuntime::default())
        .setup(|app| {
            reliability::restore_watchers(
                app.handle(),
                &app.state::<AppDb>(),
                &app.state::<reliability::SyncCoordinator>(),
            );
            // 显式把打包图标应用到主窗口与系统托盘，确保 Windows 任务栏不回退到默认图标。
            let app_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;
            let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let floating = MenuItem::with_id(app, "floating", "开启悬浮窗", true, None::<&str>)?;
            let close_floating =
                MenuItem::with_id(app, "close_floating", "关闭悬浮窗", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "最小化到托盘", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &floating, &close_floating, &hide, &quit])?;
            TrayIconBuilder::with_id("tray")
                .icon(app_icon.clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "floating" => {
                        let _ = show_floating(app);
                    }
                    "close_floating" => {
                        if let Some(w) = app.get_webview_window("floating") {
                            let _ = w.hide();
                        }
                        if let Ok(mut status) = app.state::<FloatingWindowRuntime>().renderer.lock()
                        {
                            status.state = "stopped".into();
                            status.fps = 0;
                            status.detail = "悬浮窗已从托盘隐藏，材质刷新已停止".into();
                        }
                        app.emit("floating-state", false).ok();
                    }
                    "hide" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            if let Some(main) = app.get_webview_window("main") {
                main.set_icon(app_icon)?;
                let main_copy = main.clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_copy.hide();
                    }
                });
            }
            #[cfg(windows)]
            if let Some(floating_window) = app.get_webview_window("floating") {
                apply_floating_rounded_region(&floating_window, 24.0)?;
                let floating_copy = floating_window.clone();
                floating_window.on_window_event(move |event| {
                    if let WindowEvent::Resized(_) = event {
                        let radius = floating_copy
                            .app_handle()
                            .state::<FloatingWindowRuntime>()
                            .radius
                            .lock()
                            .map(|value| *value)
                            .unwrap_or(24.0);
                        let _ = apply_floating_rounded_region(&floating_copy, radius);
                    }
                });
                start_floating_interaction_monitor(app.handle().clone());
            }
            if std::env::args().any(|arg| arg == "--floating") {
                show_floating(app.handle())?;
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.hide();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            providers,
            save_usage,
            list_usage,
            export_usage_xlsx,
            parse_codex_line,
            codex_snapshot,
            codex_quota_snapshot,
            codex_window_usage,
            codex_usage_series,
            claude_code_usage_series,
            sync_opencode_local_usage,
            reliability::list_usage_v2,
            reliability::list_sync_sources,
            reliability::get_sync_health,
            reliability::configure_local_source,
            reliability::sync_source,
            reliability::sync_all_sources,
            save_codex_budget,
            save_account_config,
            list_account_configs,
            delete_account_config,
            clear_account_configs,
            sync_account_balances,
            list_account_balances,
            list_balance_history,
            fetch_account_models,
            cc_switch_status,
            auto_connect_client,
            restore_client_connection,
            export_encrypted_backup,
            import_encrypted_backup,
            cloud_session,
            cloud_request_code,
            cloud_password_login,
            cloud_password_register,
            cloud_code_login,
            cloud_logout,
            cloud_create_transfer,
            cloud_import_transfer,
            cloud_public_content,
            cloud_app_presence,
            set_floating_window,
            configure_floating_renderer,
            update_floating_surfaces,
            set_floating_interaction_mode,
            set_floating_window_mode,
            get_floating_render_status,
            send_budget_alert,
            start_proxy,
            start_all_proxies,
            fetch_arena_rankings,
            fetch_arena_model_profile,
            prepare_liquid_background_video,
            verify_update_artifact
        ])
        .run(tauri::generate_context!())
        .expect("运行 Token Manager 失败");
}
