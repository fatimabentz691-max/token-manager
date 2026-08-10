//! Token Manager 可独立部署的账户与加密迁移链接服务。
//! 服务端只保存客户端已经加密的迁移包，永远不会接触 API Key 明文或迁移密码。

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use pbkdf2::pbkdf2_hmac;
use rand::{Rng, RngCore};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

const PASSWORD_ROUNDS: u32 = 310_000;
const SESSION_DAYS: i64 = 30;
const MAX_PAYLOAD_BYTES: usize = 25 * 1024 * 1024;

#[derive(Clone)]
struct ServerState {
    db: Arc<Mutex<Connection>>,
    client: reqwest::Client,
    public_base_url: String,
    email_webhook_url: Option<String>,
    email_webhook_token: Option<String>,
    code_secret: String,
    production: bool,
    admin_key: Option<String>,
}

#[derive(Debug)]
struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}
type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(Deserialize)]
struct EmailRequest {
    email: String,
    purpose: Option<String>,
}
#[derive(Deserialize)]
struct PasswordRegister {
    email: String,
    password: String,
    code: String,
}
#[derive(Deserialize)]
struct PasswordLogin {
    email: String,
    password: String,
}
#[derive(Deserialize)]
struct CodeLogin {
    email: String,
    code: String,
}
#[derive(Serialize)]
struct AuthReply {
    access_token: String,
    expires_at: String,
    user: UserReply,
}
#[derive(Serialize)]
struct UserReply {
    id: String,
    email: String,
}
#[derive(Deserialize)]
struct TransferCreate {
    label: Option<String>,
    payload: String,
    ttl_hours: Option<i64>,
    one_time: Option<bool>,
}
#[derive(Deserialize)]
struct AdminTransferCreate {
    email: String,
    label: Option<String>,
    payload: String,
    ttl_hours: Option<i64>,
    one_time: Option<bool>,
}
#[derive(Serialize)]
struct TransferReply {
    id: String,
    link: String,
    expires_at: String,
    one_time: bool,
}
#[derive(Debug, Serialize, Deserialize)]
struct ContentItem {
    id: String,
    kind: String,
    title: String,
    body: String,
    action_label: String,
    action_url: String,
    starts_at: String,
    ends_at: Option<String>,
    enabled: bool,
    created_at: String,
    updated_at: String,
}
#[derive(Deserialize)]
struct ContentInput {
    id: Option<String>,
    kind: String,
    title: String,
    body: String,
    action_label: Option<String>,
    action_url: Option<String>,
    starts_at: Option<String>,
    ends_at: Option<String>,
    enabled: Option<bool>,
}

#[tokio::main]
async fn main() {
    let state = init_state().expect("无法初始化 Token Manager 后端");
    let app = Router::new()
        .route("/", get(landing_page))
        .route("/admin", get(admin_page))
        .route("/health", get(health))
        .route("/v1/content", get(public_content))
        .route("/v1/auth/code/request", post(request_code))
        .route("/v1/auth/password/register", post(register_password))
        .route("/v1/auth/password/login", post(login_password))
        .route("/v1/auth/code/login", post(login_code))
        .route("/v1/auth/logout", post(logout))
        .route("/v1/me", get(me))
        .route("/v1/transfers", post(create_transfer).get(list_transfers))
        .route("/v1/transfers/:id", delete(revoke_transfer))
        .route("/v1/transfer/:token", get(transfer_meta))
        .route("/v1/transfer/:token/preview", post(preview_transfer))
        .route("/v1/transfer/:token/consume", post(consume_transfer))
        .route("/v1/admin/transfers", post(admin_create_transfer))
        .route("/v1/admin/metrics", get(admin_metrics))
        .route(
            "/v1/admin/content",
            get(admin_list_content).post(admin_save_content),
        )
        .route("/v1/admin/content/:id", delete(admin_delete_content))
        .with_state(state.clone());
    let port = env::var("TOKEN_MANAGER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8787);
    let bind = env::var("TOKEN_MANAGER_BIND").unwrap_or_else(|_| "127.0.0.1".into());
    let address = format!("{bind}:{port}")
        .parse::<SocketAddr>()
        .expect("TOKEN_MANAGER_BIND 地址无效");
    println!("Token Manager 后端已启动：http://{address}");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("端口监听失败");
    axum::serve(listener, app).await.expect("后端服务异常退出");
}

fn init_state() -> Result<ServerState, String> {
    let path = env::var("TOKEN_MANAGER_SERVER_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("token-manager-server.db"));
    let db = Connection::open(path).map_err(|e| e.to_string())?;
    db.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
         CREATE TABLE IF NOT EXISTS users(id TEXT PRIMARY KEY,email TEXT NOT NULL UNIQUE,password_salt BLOB,password_hash BLOB,created_at TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS login_codes(email TEXT NOT NULL,purpose TEXT NOT NULL,code_hash TEXT NOT NULL,expires_at TEXT NOT NULL,attempts INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL,PRIMARY KEY(email,purpose));
         CREATE TABLE IF NOT EXISTS sessions(id TEXT PRIMARY KEY,user_id TEXT NOT NULL,token_hash TEXT NOT NULL UNIQUE,expires_at TEXT NOT NULL,created_at TEXT NOT NULL,last_seen TEXT,FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE);
         CREATE TABLE IF NOT EXISTS transfer_links(id TEXT PRIMARY KEY,user_id TEXT NOT NULL,token_hash TEXT NOT NULL UNIQUE,label TEXT NOT NULL,payload TEXT NOT NULL,expires_at TEXT NOT NULL,max_downloads INTEGER NOT NULL,download_count INTEGER NOT NULL DEFAULT 0,revoked_at TEXT,created_at TEXT NOT NULL,FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE);
         CREATE TABLE IF NOT EXISTS auth_limits(key TEXT PRIMARY KEY,attempts INTEGER NOT NULL,window_start TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS login_events(id TEXT PRIMARY KEY,user_id TEXT NOT NULL,method TEXT NOT NULL,created_at TEXT NOT NULL,FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE);
         CREATE TABLE IF NOT EXISTS content_items(id TEXT PRIMARY KEY,kind TEXT NOT NULL,title TEXT NOT NULL,body TEXT NOT NULL,action_label TEXT NOT NULL,action_url TEXT NOT NULL,starts_at TEXT NOT NULL,ends_at TEXT,enabled INTEGER NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token_hash);
         CREATE INDEX IF NOT EXISTS idx_login_events_created ON login_events(created_at);
         CREATE INDEX IF NOT EXISTS idx_transfer_token ON transfer_links(token_hash);"
    ).map_err(|e| e.to_string())?;
    let has_last_seen = {
        let mut query = db
            .prepare("PRAGMA table_info(sessions)")
            .map_err(|e| e.to_string())?;
        let found = query
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .any(|name| name == "last_seen");
        found
    };
    if !has_last_seen {
        db.execute("ALTER TABLE sessions ADD COLUMN last_seen TEXT", [])
            .map_err(|e| e.to_string())?;
    }
    db.execute(
        "UPDATE sessions SET last_seen=created_at WHERE last_seen IS NULL",
        [],
    )
    .map_err(|e| e.to_string())?;
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_last_seen ON sessions(last_seen)",
        [],
    )
    .map_err(|e| e.to_string())?;
    let production = env::var("TOKEN_MANAGER_ENV")
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false);
    let admin_key = env::var("TOKEN_MANAGER_ADMIN_KEY")
        .ok()
        .or_else(|| (!production).then(|| "token-manager-local-admin".into()));
    Ok(ServerState {
        db: Arc::new(Mutex::new(db)),
        client: reqwest::Client::new(),
        public_base_url: env::var("TOKEN_MANAGER_PUBLIC_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8787".into())
            .trim_end_matches('/')
            .into(),
        email_webhook_url: env::var("TOKEN_MANAGER_EMAIL_WEBHOOK_URL").ok(),
        email_webhook_token: env::var("TOKEN_MANAGER_EMAIL_WEBHOOK_TOKEN").ok(),
        code_secret: env::var("TOKEN_MANAGER_CODE_SECRET").unwrap_or_else(|_| random_token(32)),
        production,
        admin_key,
    })
}

async fn health() -> Json<Value> {
    Json(json!({"ok":true,"service":"token-manager-server","time":Utc::now()}))
}
async fn landing_page() -> Html<&'static str> {
    Html(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Token Manager 后端</title><style>body{margin:0;background:#fff;color:#1d1d1f;font-family:"PingFang SC","Microsoft YaHei UI",sans-serif}.wrap{min-height:100vh;display:grid;place-items:center;padding:24px}.card{width:min(560px,calc(100vw - 48px));padding:34px;border:1px solid #e5e5ea;border-radius:22px;box-sizing:border-box}span{display:inline-flex;padding:7px 11px;border-radius:99px;background:#f2f2f7;font-size:12px}h1{margin:22px 0 10px;font-size:32px}p{margin:0;color:#6e6e73;line-height:1.75}.actions{display:flex;gap:18px;flex-wrap:wrap}a{display:inline-block;margin-top:22px;color:#1d1d1f;font-weight:600}</style></head><body><main class="wrap"><section class="card"><span>● 服务运行正常</span><h1>Token Manager 后端</h1><p>邮箱登录、验证码、用户活跃统计、公告、广告与加密迁移链接服务已经启动。</p><div class="actions"><a href="/admin">进入管理后台 →</a><a href="/health">健康检查 JSON →</a></div></section></main></body></html>"#,
    )
}
async fn admin_page() -> Html<&'static str> {
    Html(include_str!("../../admin/index.html"))
}
#[allow(dead_code)]
async fn home() -> Html<&'static str> {
    Html(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Token Manager 后端</title><style>body{margin:0;background:#fff;color:#1d1d1f;font-family:"PingFang SC","Microsoft YaHei UI",sans-serif}.wrap{min-height:100vh;display:grid;place-items:center;padding:24px}.card{width:min(560px,calc(100vw - 48px));padding:34px;border:1px solid #e5e5ea;border-radius:22px;box-sizing:border-box}span{display:inline-flex;padding:7px 11px;border-radius:99px;background:#f2f2f7;font-size:12px}h1{margin:22px 0 10px;font-size:32px}p{margin:0;color:#6e6e73;line-height:1.75}a{display:inline-block;margin-top:22px;color:#1d1d1f;font-weight:600}</style></head><body><main class="wrap"><section class="card"><span>● 服务运行正常</span><h1>Token Manager 后端</h1><p>邮箱登录、验证码与加密迁移链接服务已经启动。该页面不展示用户数据。</p><a href="/health">查看健康检查 JSON →</a></section></main></body></html>"#,
    )
}

fn normalize_email(value: &str) -> Result<String, ApiError> {
    let email = value.trim().to_ascii_lowercase();
    if email.len() > 254 || !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(ApiError(StatusCode::BAD_REQUEST, "邮箱格式无效".into()));
    }
    Ok(email)
}
fn random_token(bytes: usize) -> String {
    let mut raw = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut raw);
    URL_SAFE_NO_PAD.encode(raw)
}
fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn code_digest(state: &ServerState, email: &str, purpose: &str, code: &str) -> String {
    digest(&format!("{}:{email}:{purpose}:{code}", state.code_secret))
}
fn secure_equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}
fn password_hash(password: &str, salt: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PASSWORD_ROUNDS, &mut out);
    out
}
fn rate_limit(
    state: &ServerState,
    key: &str,
    max_attempts: i64,
    window_seconds: i64,
) -> Result<(), ApiError> {
    let now = Utc::now();
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let row = db
        .query_row(
            "SELECT attempts,window_start FROM auth_limits WHERE key=?1",
            [key],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(internal)?;
    let (attempts, start) = row
        .and_then(|(attempts, start)| {
            chrono::DateTime::parse_from_rfc3339(&start)
                .ok()
                .map(|start| (attempts, start.with_timezone(&Utc)))
        })
        .unwrap_or((0, now));
    if now - start > Duration::seconds(window_seconds) {
        db.execute(
            "INSERT OR REPLACE INTO auth_limits(key,attempts,window_start) VALUES(?1,1,?2)",
            params![key, now.to_rfc3339()],
        )
        .map_err(internal)?;
        return Ok(());
    }
    if attempts >= max_attempts {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "操作过于频繁，请稍后再试".into(),
        ));
    }
    db.execute(
        "INSERT OR REPLACE INTO auth_limits(key,attempts,window_start) VALUES(?1,?2,?3)",
        params![key, attempts + 1, start.to_rfc3339()],
    )
    .map_err(internal)?;
    Ok(())
}

async fn request_code(
    State(state): State<ServerState>,
    Json(input): Json<EmailRequest>,
) -> ApiResult<Value> {
    let email = normalize_email(&input.email)?;
    let purpose = input.purpose.unwrap_or_else(|| "login".into());
    if purpose != "login" && purpose != "register" {
        return Err(ApiError(StatusCode::BAD_REQUEST, "验证码用途无效".into()));
    }
    rate_limit(&state, &format!("code:{email}"), 5, 600)?;
    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let now = Utc::now();
    let expires = now + Duration::minutes(10);
    let mut debug_code = None;
    if let Some(url) = &state.email_webhook_url {
        let mut req=state.client.post(url).json(&json!({"to":email,"subject":"Token Manager 登录验证码","text":format!("你的验证码是 {code}，10 分钟内有效。"),"code":code}));
        if let Some(token) = &state.email_webhook_token {
            req = req.bearer_auth(token)
        }
        req.send()
            .await
            .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("验证码邮件发送失败：{e}")))?
            .error_for_status()
            .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("邮件服务返回错误：{e}")))?;
    } else if state.production {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "生产环境尚未配置邮件发送服务".into(),
        ));
    } else {
        debug_code = Some(code.clone());
    }
    {
        let db = state
            .db
            .lock()
            .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
        db.execute("INSERT OR REPLACE INTO login_codes(email,purpose,code_hash,expires_at,attempts,created_at) VALUES(?1,?2,?3,?4,0,?5)",params![email,purpose,code_digest(&state,&email,&purpose,&code),expires.to_rfc3339(),now.to_rfc3339()]).map_err(internal)?;
    }
    Ok(Json(
        json!({"ok":true,"expires_in":600,"debug_code":debug_code}),
    ))
}

fn consume_code(
    state: &ServerState,
    email: &str,
    purpose: &str,
    code: &str,
) -> Result<(), ApiError> {
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let row = db
        .query_row(
            "SELECT code_hash,expires_at,attempts FROM login_codes WHERE email=?1 AND purpose=?2",
            params![email, purpose],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "验证码无效或已过期".into()))?;
    if row.2 >= 5 || row.1 < Utc::now().to_rfc3339() {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "验证码无效或已过期".into(),
        ));
    }
    if row.0 != code_digest(state, email, purpose, code) {
        db.execute(
            "UPDATE login_codes SET attempts=attempts+1 WHERE email=?1 AND purpose=?2",
            params![email, purpose],
        )
        .ok();
        return Err(ApiError(StatusCode::UNAUTHORIZED, "验证码错误".into()));
    }
    db.execute(
        "DELETE FROM login_codes WHERE email=?1 AND purpose=?2",
        params![email, purpose],
    )
    .map_err(internal)?;
    Ok(())
}

async fn register_password(
    State(state): State<ServerState>,
    Json(input): Json<PasswordRegister>,
) -> ApiResult<AuthReply> {
    let email = normalize_email(&input.email)?;
    if input.password.chars().count() < 8 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "密码至少需要 8 个字符".into(),
        ));
    }
    consume_code(&state, &email, "register", &input.code)?;
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let hash = password_hash(&input.password, &salt);
    let id = format!("usr_{}", random_token(12));
    {
        let db = state
            .db
            .lock()
            .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
        db.execute("INSERT INTO users(id,email,password_salt,password_hash,created_at) VALUES(?1,?2,?3,?4,?5)",params![id,email,salt.to_vec(),hash,Utc::now().to_rfc3339()]).map_err(|e|if e.to_string().contains("UNIQUE"){ApiError(StatusCode::CONFLICT,"该邮箱已注册".into())}else{internal(e)})?;
    }
    Ok(Json(new_session(&state, &id, &email, "password_register")?))
}
async fn login_password(
    State(state): State<ServerState>,
    Json(input): Json<PasswordLogin>,
) -> ApiResult<AuthReply> {
    let email = normalize_email(&input.email)?;
    rate_limit(&state, &format!("password:{email}"), 20, 900)?;
    let row = {
        let db = state
            .db
            .lock()
            .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
        db.query_row(
            "SELECT id,password_salt,password_hash FROM users WHERE email=?1",
            [&email],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<Vec<u8>>>(1)?,
                    r.get::<_, Option<Vec<u8>>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(internal)?
    }
    .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "邮箱或密码错误".into()))?;
    let (salt, expected) = match (row.1, row.2) {
        (Some(s), Some(h)) => (s, h),
        _ => {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "该账户尚未设置密码，请使用验证码登录".into(),
            ))
        }
    };
    if !secure_equal(&password_hash(&input.password, &salt), &expected) {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "邮箱或密码错误".into()));
    }
    Ok(Json(new_session(&state, &row.0, &email, "password")?))
}
async fn login_code(
    State(state): State<ServerState>,
    Json(input): Json<CodeLogin>,
) -> ApiResult<AuthReply> {
    let email = normalize_email(&input.email)?;
    consume_code(&state, &email, "login", &input.code)?;
    let id = {
        let db = state
            .db
            .lock()
            .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
        if let Some(id) = db
            .query_row("SELECT id FROM users WHERE email=?1", [&email], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(internal)?
        {
            id
        } else {
            let id = format!("usr_{}", random_token(12));
            db.execute(
                "INSERT INTO users(id,email,created_at) VALUES(?1,?2,?3)",
                params![id, email, Utc::now().to_rfc3339()],
            )
            .map_err(internal)?;
            id
        }
    };
    Ok(Json(new_session(&state, &id, &email, "code")?))
}
fn new_session(
    state: &ServerState,
    user_id: &str,
    email: &str,
    method: &str,
) -> Result<AuthReply, ApiError> {
    let token = random_token(32);
    let now = Utc::now();
    let expires = now + Duration::days(SESSION_DAYS);
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    db.execute("INSERT INTO sessions(id,user_id,token_hash,expires_at,created_at,last_seen) VALUES(?1,?2,?3,?4,?5,?5)",params![format!("ses_{}",random_token(10)),user_id,digest(&token),expires.to_rfc3339(),now.to_rfc3339()]).map_err(internal)?;
    db.execute(
        "INSERT INTO login_events(id,user_id,method,created_at) VALUES(?1,?2,?3,?4)",
        params![
            format!("login_{}", random_token(10)),
            user_id,
            method,
            now.to_rfc3339()
        ],
    )
    .map_err(internal)?;
    Ok(AuthReply {
        access_token: token,
        expires_at: expires.to_rfc3339(),
        user: UserReply {
            id: user_id.into(),
            email: email.into(),
        },
    })
}

fn bearer(headers: &HeaderMap) -> Result<String, ApiError> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "需要登录".into()))
}
fn current_user(state: &ServerState, headers: &HeaderMap) -> Result<(String, String), ApiError> {
    let token = bearer(headers)?;
    let token_hash = digest(&token);
    let now = Utc::now().to_rfc3339();
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let user=db.query_row("SELECT users.id,users.email FROM sessions JOIN users ON users.id=sessions.user_id WHERE sessions.token_hash=?1 AND sessions.expires_at>?2",params![token_hash,now],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(internal)?.ok_or_else(||ApiError(StatusCode::UNAUTHORIZED,"登录已过期".into()))?;
    db.execute(
        "UPDATE sessions SET last_seen=?1 WHERE token_hash=?2",
        params![now, token_hash],
    )
    .map_err(internal)?;
    Ok(user)
}
async fn me(State(state): State<ServerState>, headers: HeaderMap) -> ApiResult<UserReply> {
    let (id, email) = current_user(&state, &headers)?;
    Ok(Json(UserReply { id, email }))
}
async fn logout(State(state): State<ServerState>, headers: HeaderMap) -> ApiResult<Value> {
    let token = bearer(&headers)?;
    state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?
        .execute("DELETE FROM sessions WHERE token_hash=?1", [digest(&token)])
        .map_err(internal)?;
    Ok(Json(json!({"ok":true})))
}

fn create_transfer_row(
    state: &ServerState,
    user_id: &str,
    input: TransferCreate,
) -> Result<TransferReply, ApiError> {
    if input.payload.len() > MAX_PAYLOAD_BYTES {
        return Err(ApiError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "迁移包超过 25MB".into(),
        ));
    }
    serde_json::from_str::<Value>(&input.payload).map_err(|_| {
        ApiError(
            StatusCode::BAD_REQUEST,
            "迁移包必须是有效 JSON 密文信封".into(),
        )
    })?;
    let token = random_token(32);
    let id = format!("tr_{}", random_token(10));
    let hours = input.ttl_hours.unwrap_or(24).clamp(1, 168);
    let expires = Utc::now() + Duration::hours(hours);
    let one_time = input.one_time.unwrap_or(true);
    let max_downloads = if one_time { 1 } else { 20 };
    let label = input
        .label
        .unwrap_or_else(|| "Token Manager 数据迁移".into());
    state.db.lock().map_err(|_|ApiError(StatusCode::INTERNAL_SERVER_ERROR,"数据库忙".into()))?.execute("INSERT INTO transfer_links(id,user_id,token_hash,label,payload,expires_at,max_downloads,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![id,user_id,digest(&token),label,input.payload,expires.to_rfc3339(),max_downloads,Utc::now().to_rfc3339()]).map_err(internal)?;
    Ok(TransferReply {
        id,
        link: format!("{}/v1/transfer/{}", state.public_base_url, token),
        expires_at: expires.to_rfc3339(),
        one_time,
    })
}
async fn create_transfer(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<TransferCreate>,
) -> ApiResult<TransferReply> {
    let (user_id, _) = current_user(&state, &headers)?;
    Ok(Json(create_transfer_row(&state, &user_id, input)?))
}
async fn list_transfers(State(state): State<ServerState>, headers: HeaderMap) -> ApiResult<Value> {
    let (user_id, _) = current_user(&state, &headers)?;
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let mut query=db.prepare("SELECT id,label,expires_at,max_downloads,download_count,revoked_at,created_at FROM transfer_links WHERE user_id=?1 ORDER BY created_at DESC LIMIT 50").map_err(internal)?;
    let rows=query.query_map([user_id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"label":r.get::<_,String>(1)?,"expires_at":r.get::<_,String>(2)?,"one_time":r.get::<_,i64>(3)?==1,"download_count":r.get::<_,i64>(4)?,"revoked":r.get::<_,Option<String>>(5)?.is_some(),"created_at":r.get::<_,String>(6)?}))).map_err(internal)?.collect::<Result<Vec<_>,_>>().map_err(internal)?;
    Ok(Json(json!({"items":rows})))
}
async fn revoke_transfer(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    let (user_id, _) = current_user(&state, &headers)?;
    let changed=state.db.lock().map_err(|_|ApiError(StatusCode::INTERNAL_SERVER_ERROR,"数据库忙".into()))?.execute("UPDATE transfer_links SET revoked_at=?1 WHERE id=?2 AND user_id=?3 AND revoked_at IS NULL",params![Utc::now().to_rfc3339(),id,user_id]).map_err(internal)?;
    if changed == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "迁移链接不存在".into()));
    }
    Ok(Json(json!({"ok":true})))
}
fn transfer_row(
    state: &ServerState,
    token: &str,
) -> Result<(String, String, String, i64, i64), ApiError> {
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    db.query_row("SELECT id,label,expires_at,max_downloads,download_count FROM transfer_links WHERE token_hash=?1 AND revoked_at IS NULL",[digest(token)],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(internal)?.filter(|row|row.2>Utc::now().to_rfc3339()&&row.4<row.3).ok_or_else(||ApiError(StatusCode::GONE,"迁移链接已过期、撤销或使用完毕".into()))
}
async fn transfer_meta(
    State(state): State<ServerState>,
    Path(token): Path<String>,
) -> ApiResult<Value> {
    let (row_id, label, expires, max, count) = transfer_row(&state, &token)?;
    Ok(Json(
        json!({"id":row_id,"label":label,"expires_at":expires,"one_time":max==1,"remaining":max-count}),
    ))
}
async fn preview_transfer(
    State(state): State<ServerState>,
    Path(token): Path<String>,
) -> ApiResult<Value> {
    let (id, label, expires, _, _) = transfer_row(&state, &token)?;
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let payload = db
        .query_row(
            "SELECT payload FROM transfer_links WHERE id=?1",
            [&id],
            |r| r.get::<_, String>(0),
        )
        .map_err(internal)?;
    Ok(Json(
        json!({"id":id,"label":label,"expires_at":expires,"payload":payload}),
    ))
}
async fn consume_transfer(
    State(state): State<ServerState>,
    Path(token): Path<String>,
) -> ApiResult<Value> {
    let (id, label, expires, _, _) = transfer_row(&state, &token)?;
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let payload = db
        .query_row(
            "SELECT payload FROM transfer_links WHERE id=?1",
            [&id],
            |r| r.get::<_, String>(0),
        )
        .map_err(internal)?;
    db.execute(
        "UPDATE transfer_links SET download_count=download_count+1 WHERE id=?1",
        [&id],
    )
    .map_err(internal)?;
    Ok(Json(
        json!({"id":id,"label":label,"expires_at":expires,"payload":payload}),
    ))
}
fn require_admin(state: &ServerState, headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = state
        .admin_key
        .as_ref()
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "管理接口未启用".into()))?;
    let supplied = headers
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !secure_equal(supplied.as_bytes(), expected.as_bytes()) {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "管理密钥错误".into()));
    }
    Ok(())
}

fn content_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContentItem> {
    Ok(ContentItem {
        id: row.get(0)?,
        kind: row.get(1)?,
        title: row.get(2)?,
        body: row.get(3)?,
        action_label: row.get(4)?,
        action_url: row.get(5)?,
        starts_at: row.get(6)?,
        ends_at: row.get(7)?,
        enabled: row.get::<_, i64>(8)? != 0,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

async fn public_content(State(state): State<ServerState>) -> ApiResult<Value> {
    let now = Utc::now().to_rfc3339();
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let mut query=db.prepare("SELECT id,kind,title,body,action_label,action_url,starts_at,ends_at,enabled,created_at,updated_at FROM content_items WHERE enabled=1 AND starts_at<=?1 AND (ends_at IS NULL OR ends_at>?1) ORDER BY CASE kind WHEN 'announcement' THEN 0 ELSE 1 END,updated_at DESC").map_err(internal)?;
    let items = query
        .query_map([now], content_from_row)
        .map_err(internal)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal)?;
    Ok(Json(json!({"items":items})))
}

async fn admin_metrics(State(state): State<ServerState>, headers: HeaderMap) -> ApiResult<Value> {
    require_admin(&state, &headers)?;
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let since_24 = (now - Duration::hours(24)).to_rfc3339();
    let since_7 = (now - Duration::days(7)).to_rfc3339();
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let scalar = |sql: &str, values: &[&dyn rusqlite::ToSql]| -> Result<i64, ApiError> {
        db.query_row(sql, values, |r| r.get(0)).map_err(internal)
    };
    let total_users = scalar("SELECT COUNT(*) FROM users", &[])?;
    let active_sessions = scalar(
        "SELECT COUNT(*) FROM sessions WHERE expires_at>?1",
        &[&now_text],
    )?;
    let active_users_24h = scalar(
        "SELECT COUNT(DISTINCT user_id) FROM sessions WHERE expires_at>?1 AND last_seen>=?2",
        &[&now_text, &since_24],
    )?;
    let active_users_7d = scalar(
        "SELECT COUNT(DISTINCT user_id) FROM sessions WHERE expires_at>?1 AND last_seen>=?2",
        &[&now_text, &since_7],
    )?;
    let logins_24h = scalar(
        "SELECT COUNT(*) FROM login_events WHERE created_at>=?1",
        &[&since_24],
    )?;
    let active_transfers=scalar("SELECT COUNT(*) FROM transfer_links WHERE revoked_at IS NULL AND expires_at>?1 AND download_count<max_downloads",&[&now_text])?;
    let active_announcements=scalar("SELECT COUNT(*) FROM content_items WHERE kind='announcement' AND enabled=1 AND starts_at<=?1 AND (ends_at IS NULL OR ends_at>?1)",&[&now_text])?;
    let active_ads=scalar("SELECT COUNT(*) FROM content_items WHERE kind='ad' AND enabled=1 AND starts_at<=?1 AND (ends_at IS NULL OR ends_at>?1)",&[&now_text])?;
    let mut query=db.prepare("SELECT substr(created_at,1,10),COUNT(*) FROM login_events WHERE created_at>=?1 GROUP BY substr(created_at,1,10) ORDER BY 1").map_err(internal)?;
    let daily_logins = query
        .query_map([since_7], |r| {
            Ok(json!({"date":r.get::<_,String>(0)?,"count":r.get::<_,i64>(1)?}))
        })
        .map_err(internal)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal)?;
    Ok(Json(
        json!({"total_users":total_users,"active_sessions":active_sessions,"active_users_24h":active_users_24h,"active_users_7d":active_users_7d,"logins_24h":logins_24h,"active_transfer_links":active_transfers,"active_announcements":active_announcements,"active_ads":active_ads,"daily_logins":daily_logins,"generated_at":now_text}),
    ))
}

async fn admin_list_content(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> ApiResult<Value> {
    require_admin(&state, &headers)?;
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let mut query=db.prepare("SELECT id,kind,title,body,action_label,action_url,starts_at,ends_at,enabled,created_at,updated_at FROM content_items ORDER BY updated_at DESC").map_err(internal)?;
    let items = query
        .query_map([], content_from_row)
        .map_err(internal)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal)?;
    Ok(Json(json!({"items":items})))
}

fn parse_content_time(
    value: Option<String>,
    fallback: &str,
    field: &str,
) -> Result<String, ApiError> {
    let value = value.unwrap_or_default();
    let value = value.trim();
    if value.is_empty() {
        return Ok(fallback.into());
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|v| v.with_timezone(&Utc).to_rfc3339())
        .map_err(|_| {
            ApiError(
                StatusCode::BAD_REQUEST,
                format!("{field} 必须是 RFC3339 时间"),
            )
        })
}

async fn admin_save_content(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ContentInput>,
) -> ApiResult<ContentItem> {
    require_admin(&state, &headers)?;
    if input.kind != "announcement" && input.kind != "ad" {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "类型只能是 announcement 或 ad".into(),
        ));
    }
    let title = input.title.trim();
    let body = input.body.trim();
    if title.is_empty() || title.chars().count() > 80 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "标题需为 1 至 80 个字符".into(),
        ));
    }
    if body.is_empty() || body.chars().count() > 500 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "内容需为 1 至 500 个字符".into(),
        ));
    }
    let action_url = input.action_url.unwrap_or_default().trim().to_string();
    if !action_url.is_empty() && !action_url.starts_with("https://") {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "跳转地址必须使用 HTTPS".into(),
        ));
    }
    let now = Utc::now().to_rfc3339();
    let starts_at = parse_content_time(input.starts_at, &now, "开始时间")?;
    let ends_at = match input.ends_at {
        Some(value) if !value.trim().is_empty() => {
            Some(parse_content_time(Some(value), &now, "结束时间")?)
        }
        _ => None,
    };
    if ends_at.as_ref().is_some_and(|end| end <= &starts_at) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "结束时间必须晚于开始时间".into(),
        ));
    }
    let id = input
        .id
        .unwrap_or_else(|| format!("content_{}", random_token(10)));
    let action_label = input
        .action_label
        .unwrap_or_default()
        .trim()
        .chars()
        .take(30)
        .collect::<String>();
    let enabled = input.enabled.unwrap_or(true);
    let db = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
    let created_at = db
        .query_row(
            "SELECT created_at FROM content_items WHERE id=?1",
            [&id],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(internal)?
        .unwrap_or_else(|| now.clone());
    db.execute("INSERT INTO content_items(id,kind,title,body,action_label,action_url,starts_at,ends_at,enabled,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(id) DO UPDATE SET kind=excluded.kind,title=excluded.title,body=excluded.body,action_label=excluded.action_label,action_url=excluded.action_url,starts_at=excluded.starts_at,ends_at=excluded.ends_at,enabled=excluded.enabled,updated_at=excluded.updated_at",params![id,input.kind,title,body,action_label,action_url,starts_at,ends_at,if enabled{1}else{0},created_at,now]).map_err(internal)?;
    let item=db.query_row("SELECT id,kind,title,body,action_label,action_url,starts_at,ends_at,enabled,created_at,updated_at FROM content_items WHERE id=?1",[id],content_from_row).map_err(internal)?;
    Ok(Json(item))
}

async fn admin_delete_content(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    require_admin(&state, &headers)?;
    let changed = state
        .db
        .lock()
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?
        .execute("DELETE FROM content_items WHERE id=?1", [id])
        .map_err(internal)?;
    if changed == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "内容不存在".into()));
    }
    Ok(Json(json!({"ok":true})))
}

async fn admin_create_transfer(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AdminTransferCreate>,
) -> ApiResult<TransferReply> {
    require_admin(&state, &headers)?;
    let email = normalize_email(&input.email)?;
    let user_id = {
        let db = state
            .db
            .lock()
            .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库忙".into()))?;
        db.query_row("SELECT id FROM users WHERE email=?1", [email], |r| {
            r.get::<_, String>(0)
        })
        .optional()
        .map_err(internal)?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "用户不存在".into()))?
    };
    Ok(Json(create_transfer_row(
        &state,
        &user_id,
        TransferCreate {
            label: input.label,
            payload: input.payload,
            ttl_hours: input.ttl_hours,
            one_time: input.one_time,
        },
    )?))
}
fn internal<E: std::fmt::Display>(error: E) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
