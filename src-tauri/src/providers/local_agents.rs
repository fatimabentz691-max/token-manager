use crate::{
    reliability::{self, SourceKind, SyncSourceConfig, UsageProvenance},
    UsageEvent,
};
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{types::Value as SqlValue, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, UNIX_EPOCH},
};

use super::opencode::LocalSyncOutcome;

const MAX_FILES: usize = 1_500;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SCAN_DEPTH: usize = 6;
const SOURCE_DEADLINE: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConnectorDefinition {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub collector_kind: String,
    pub default_paths: Vec<String>,
    pub capabilities: Vec<String>,
    pub proxy_protocol: Option<String>,
    pub detected: bool,
    pub detected_paths: Vec<String>,
    pub schema_version: u32,
    pub detail: String,
    pub brand_id: String,
    pub official_url: String,
    pub collector_mode: String,
    pub path_specs: Vec<String>,
    pub usage_capability: String,
    pub limit_capability: String,
    pub proxy_capability: String,
    pub schema_fingerprint: String,
    pub support_state: String,
    pub path_candidates: Vec<AgentPathCandidate>,
}

/// 可跨 Windows 账户和电脑迁移的 Agent 路径描述。
/// 数据库只保存 `id`/`template`，`resolved_path` 每次启动按当前电脑重新计算。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPathCandidate {
    pub id: String,
    pub template: String,
    pub resolved_path: String,
    pub exists: bool,
    pub root_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSourceModel {
    pub source_id: String,
    pub model: String,
    pub records: u64,
    pub last_seen_at: String,
}

#[derive(Clone, Copy)]
struct AgentSpec {
    id: &'static str,
    name: &'static str,
    provider: &'static str,
    collector_kind: &'static str,
    capabilities: &'static [&'static str],
    proxy_protocol: Option<&'static str>,
}

const SPECS: &[AgentSpec] = &[
    AgentSpec {
        id: "claude-code",
        name: "Claude Code",
        provider: "Claude Code",
        collector_kind: "jsonl",
        capabilities: &["本地日志", "客户端额度", "会话明细", "实时代理"],
        proxy_protocol: Some("anthropic"),
    },
    AgentSpec {
        id: "opencode",
        name: "OpenCode CLI",
        provider: "OpenCode Go",
        collector_kind: "sqlite",
        capabilities: &["本地 SQLite", "本地 JSON", "会话明细", "实时代理"],
        proxy_protocol: Some("both"),
    },
    AgentSpec {
        id: "cursor",
        name: "Cursor",
        provider: "Cursor",
        collector_kind: "cache",
        capabilities: &["本地缓存", "账户同步"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "github-copilot",
        name: "GitHub Copilot",
        provider: "GitHub Copilot",
        collector_kind: "otel",
        capabilities: &["OTel", "本地数据库", "会话明细"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "gemini-cli",
        name: "Gemini CLI",
        provider: "Gemini CLI",
        collector_kind: "json",
        capabilities: &["本地 JSON", "会话明细"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "cline",
        name: "Cline",
        provider: "Cline",
        collector_kind: "json",
        capabilities: &["本地日志", "会话明细", "实时代理"],
        proxy_protocol: Some("both"),
    },
    AgentSpec {
        id: "kilo-code",
        name: "Kilo Code",
        provider: "Kilo Code",
        collector_kind: "json",
        capabilities: &["本地日志", "会话明细", "实时代理"],
        proxy_protocol: Some("both"),
    },
    AgentSpec {
        id: "kimi-cli",
        name: "Kimi CLI / Kimi Code",
        provider: "Kimi CLI",
        collector_kind: "jsonl",
        capabilities: &["本地日志", "会话明细", "实时代理"],
        proxy_protocol: Some("openai"),
    },
    AgentSpec {
        id: "qwen-cli",
        name: "Qwen CLI",
        provider: "Qwen CLI",
        collector_kind: "jsonl",
        capabilities: &["本地日志", "会话明细", "实时代理"],
        proxy_protocol: Some("openai"),
    },
    AgentSpec {
        id: "openclaw",
        name: "OpenClaw",
        provider: "OpenClaw",
        collector_kind: "jsonl",
        capabilities: &["本地日志", "会话明细"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "hermes",
        name: "Hermes Agent",
        provider: "Hermes Agent",
        collector_kind: "sqlite",
        capabilities: &["本地数据库", "会话明细"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "antigravity",
        name: "Antigravity",
        provider: "Antigravity",
        collector_kind: "cache",
        capabilities: &["本地缓存", "会话明细"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "grok-build",
        name: "Grok Build",
        provider: "Grok Build",
        collector_kind: "jsonl",
        capabilities: &["本地日志", "会话明细"],
        proxy_protocol: Some("openai"),
    },
    AgentSpec {
        id: "pi",
        name: "Pi",
        provider: "Pi",
        collector_kind: "jsonl",
        capabilities: &["本地日志", "会话明细"],
        proxy_protocol: Some("both"),
    },
    AgentSpec {
        id: "zed",
        name: "Zed",
        provider: "Zed",
        collector_kind: "sqlite",
        capabilities: &["本地数据库", "会话明细"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "kiro",
        name: "Kiro",
        provider: "Kiro",
        collector_kind: "sqlite",
        capabilities: &["本地数据库", "本地日志"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "proma",
        name: "Proma",
        provider: "Proma",
        collector_kind: "jsonl",
        capabilities: &["本地日志", "会话明细"],
        proxy_protocol: Some("openai"),
    },
    AgentSpec {
        id: "reasonix",
        name: "Reasonix",
        provider: "Reasonix",
        collector_kind: "json",
        capabilities: &["本地统计", "会话明细"],
        proxy_protocol: Some("openai"),
    },
    AgentSpec {
        id: "mimo-code",
        name: "MiMo Code",
        provider: "MiMo Code",
        collector_kind: "sqlite",
        capabilities: &["本地 SQLite", "模型明细", "会话明细"],
        proxy_protocol: Some("both"),
    },
    AgentSpec {
        id: "zcode",
        name: "ZCode / GLM",
        provider: "ZCode",
        collector_kind: "sqlite",
        capabilities: &["本地 SQLite", "本地日志", "模型排行"],
        proxy_protocol: Some("both"),
    },
    AgentSpec {
        id: "codebuddy",
        name: "CodeBuddy",
        provider: "CodeBuddy",
        collector_kind: "jsonl",
        capabilities: &["本地日志", "会话明细"],
        proxy_protocol: Some("both"),
    },
    AgentSpec {
        id: "workbuddy",
        name: "WorkBuddy",
        provider: "WorkBuddy",
        collector_kind: "sqlite",
        capabilities: &["本地 SQLite", "本地日志"],
        proxy_protocol: Some("both"),
    },
    AgentSpec {
        id: "trae-agent",
        name: "Trae Agent",
        provider: "Trae Agent",
        collector_kind: "json",
        capabilities: &["用户导出的 trajectory", "会话明细"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "trae-ide",
        name: "Trae IDE",
        provider: "Trae IDE",
        collector_kind: "detected",
        capabilities: &["安装检测", "暂不支持本地用量"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "qoder",
        name: "Qoder",
        provider: "Qoder",
        collector_kind: "detected",
        capabilities: &["安装检测", "暂不支持本地用量"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "lingma",
        name: "通义灵码",
        provider: "通义灵码",
        collector_kind: "detected",
        capabilities: &["安装检测", "暂不支持本地用量"],
        proxy_protocol: None,
    },
    AgentSpec {
        id: "comate",
        name: "百度 Comate",
        provider: "百度 Comate",
        collector_kind: "detected",
        capabilities: &["安装检测", "暂不支持本地用量"],
        proxy_protocol: None,
    },
];

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn roaming() -> PathBuf {
    dirs::data_dir().unwrap_or_else(home)
}

fn local_data() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(home)
}

fn expand_template(template: &str) -> PathBuf {
    let replacements = [
        ("%USERPROFILE%", home()),
        ("%APPDATA%", roaming()),
        ("%LOCALAPPDATA%", local_data()),
    ];
    for (prefix, root) in replacements {
        if let Some(rest) = template.strip_prefix(prefix) {
            return rest
                .trim_start_matches(['\\', '/'])
                .split(['\\', '/'])
                .filter(|part| !part.is_empty())
                .fold(root, |path, part| path.join(part));
        }
    }
    if let Some(name) = template
        .strip_prefix('%')
        .and_then(|value| value.split_once('%').map(|(name, _)| name))
    {
        if let Ok(root) = std::env::var(name) {
            let suffix = template
                .strip_prefix(&format!("%{name}%"))
                .unwrap_or_default();
            return suffix
                .trim_start_matches(['\\', '/'])
                .split(['\\', '/'])
                .filter(|part| !part.is_empty())
                .fold(PathBuf::from(root), |path, part| path.join(part));
        }
    }
    PathBuf::from(template)
}

fn candidate_templates(id: &str) -> Vec<(&'static str, &'static str)> {
    match id {
        "claude-code" => vec![
            ("home-projects", "%USERPROFILE%\\.claude\\projects"),
            ("home-transcripts", "%USERPROFILE%\\.claude\\transcripts"),
        ],
        "opencode" => vec![
            ("home-xdg", "%USERPROFILE%\\.local\\share\\opencode"),
            ("roaming", "%APPDATA%\\opencode"),
        ],
        "cursor" => vec![
            ("workspace", "%APPDATA%\\Cursor\\User\\workspaceStorage"),
            ("global", "%APPDATA%\\Cursor\\User\\globalStorage"),
        ],
        "github-copilot" => vec![
            ("otel", "%USERPROFILE%\\.copilot\\otel"),
            ("database", "%USERPROFILE%\\.copilot\\data.db"),
            ("vscode", "%APPDATA%\\Code\\User\\workspaceStorage"),
        ],
        "gemini-cli" => vec![("chats", "%USERPROFILE%\\.gemini\\tmp")],
        "cline" => vec![
            (
                "vscode",
                "%APPDATA%\\Code\\User\\globalStorage\\saoudrizwan.claude-dev\\tasks",
            ),
            ("home", "%USERPROFILE%\\.cline\\data\\sessions"),
        ],
        "kilo-code" => vec![
            (
                "vscode",
                "%APPDATA%\\Code\\User\\globalStorage\\kilocode.kilo-code\\tasks",
            ),
            ("home", "%USERPROFILE%\\.kilocode\\sessions"),
        ],
        "kimi-cli" => vec![
            ("kimi", "%USERPROFILE%\\.kimi\\sessions"),
            ("kimi-code", "%USERPROFILE%\\.kimi-code\\sessions"),
        ],
        "qwen-cli" => vec![("projects", "%USERPROFILE%\\.qwen\\projects")],
        "openclaw" => vec![("agents", "%USERPROFILE%\\.openclaw\\agents")],
        "hermes" => vec![
            ("custom-home", "%HERMES_HOME%\\state.db"),
            ("home", "%USERPROFILE%\\.hermes\\state.db"),
        ],
        "antigravity" => vec![
            (
                "workspace",
                "%APPDATA%\\Antigravity\\User\\workspaceStorage",
            ),
            ("global", "%APPDATA%\\Antigravity\\User\\globalStorage"),
        ],
        "grok-build" => vec![
            ("sessions", "%USERPROFILE%\\.grok\\sessions"),
            ("log", "%USERPROFILE%\\.grok\\logs\\unified.jsonl"),
        ],
        "pi" => vec![
            ("pi", "%USERPROFILE%\\.pi\\agent\\sessions"),
            ("omp", "%USERPROFILE%\\.omp\\agent\\sessions"),
        ],
        "zed" => vec![("threads", "%LOCALAPPDATA%\\Zed\\threads\\threads.db")],
        "kiro" => vec![
            ("sessions", "%USERPROFILE%\\.kiro\\sessions\\cli"),
            (
                "database",
                "%USERPROFILE%\\.local\\share\\kiro-cli\\data.sqlite3",
            ),
        ],
        "proma" => vec![("sessions", "%USERPROFILE%\\.proma\\agent-sessions")],
        "reasonix" => vec![
            ("stats", "%USERPROFILE%\\.reasonix\\stats"),
            ("sessions", "%USERPROFILE%\\.reasonix\\sessions"),
            ("projects", "%USERPROFILE%\\.reasonix\\projects"),
        ],
        "mimo-code" => vec![
            ("local", "%LOCALAPPDATA%\\mimocode"),
            ("local-title", "%LOCALAPPDATA%\\MiMo Code"),
        ],
        "zcode" => vec![
            ("projects", "%USERPROFILE%\\.zcode\\projects"),
            ("database", "%USERPROFILE%\\.zcode\\cli\\db\\db.sqlite"),
        ],
        "codebuddy" => vec![
            ("projects", "%USERPROFILE%\\.codebuddy\\projects"),
            ("roaming", "%APPDATA%\\CodeBuddy"),
        ],
        "workbuddy" => vec![
            ("projects", "%USERPROFILE%\\.workbuddy\\projects"),
            ("database", "%USERPROFILE%\\.workbuddy\\workbuddy.db"),
        ],
        "trae-agent" => vec![
            ("trajectories", "%USERPROFILE%\\.trae-agent\\trajectories"),
            ("exports", "%USERPROFILE%\\trae-agent-trajectories"),
        ],
        "trae-ide" => vec![("roaming", "%APPDATA%\\Trae")],
        "qoder" => vec![
            ("roaming", "%APPDATA%\\Qoder"),
            ("local", "%LOCALAPPDATA%\\Qoder"),
        ],
        "lingma" => vec![
            ("roaming", "%APPDATA%\\Lingma"),
            ("home", "%USERPROFILE%\\.lingma"),
        ],
        "comate" => vec![
            ("roaming", "%APPDATA%\\Baidu Comate"),
            ("home", "%USERPROFILE%\\.comate"),
        ],
        _ => Vec::new(),
    }
}

pub fn path_candidates(id: &str) -> Vec<AgentPathCandidate> {
    candidate_templates(id)
        .into_iter()
        .map(|(id, template)| {
            let resolved = expand_template(template);
            AgentPathCandidate {
                id: id.into(),
                template: template.into(),
                resolved_path: resolved.to_string_lossy().into_owned(),
                exists: resolved.exists(),
                root_kind: if template.starts_with("%APPDATA%") {
                    "app_data"
                } else if template.starts_with("%LOCALAPPDATA%") {
                    "local_app_data"
                } else if template.starts_with("%USERPROFILE%") {
                    "home"
                } else {
                    "environment"
                }
                .into(),
            }
        })
        .collect()
}

pub fn resolve_path_spec_ids(agent_id: &str, ids: &[String]) -> Vec<String> {
    let candidates = path_candidates(agent_id);
    let selected = if ids.is_empty() {
        candidates
    } else {
        candidates
            .into_iter()
            .filter(|item| ids.contains(&item.id))
            .collect()
    };
    selected
        .into_iter()
        .map(|item| item.resolved_path)
        .collect()
}

/// 只将与已知标准目录完全相同或尾部结构完全相同的旧路径迁移为模板，
/// 其余路径视为用户手选目录并原样保留。
pub fn infer_path_spec_ids(agent_id: &str, paths: &[String]) -> Vec<String> {
    let candidates = path_candidates(agent_id);
    candidates
        .into_iter()
        .filter(|candidate| {
            paths.iter().any(|path| {
                path.eq_ignore_ascii_case(&candidate.resolved_path)
                    || candidate.template.split('%').last().is_some_and(|suffix| {
                        let suffix = suffix.trim_start_matches(['\\', '/']);
                        !suffix.is_empty()
                            && path
                                .replace('/', "\\")
                                .to_ascii_lowercase()
                                .ends_with(&suffix.to_ascii_lowercase())
                    })
            })
        })
        .map(|candidate| candidate.id)
        .collect()
}

fn candidate_paths(id: &str) -> Vec<PathBuf> {
    let templated = path_candidates(id);
    if !templated.is_empty() {
        return templated
            .into_iter()
            .map(|item| PathBuf::from(item.resolved_path))
            .collect();
    }
    let home = home();
    let roaming = roaming();
    let local = local_data();
    match id {
        "claude-code" => vec![
            home.join(".claude").join("projects"),
            home.join(".claude").join("transcripts"),
        ],
        "opencode" => vec![
            home.join(".local").join("share").join("opencode"),
            roaming.join("opencode"),
        ],
        "cursor" => vec![
            roaming.join("Cursor").join("User").join("workspaceStorage"),
            roaming.join("Cursor").join("User").join("globalStorage"),
        ],
        "github-copilot" => vec![
            home.join(".copilot").join("otel"),
            home.join(".copilot").join("data.db"),
            roaming.join("Code").join("User").join("workspaceStorage"),
        ],
        "gemini-cli" => vec![home.join(".gemini").join("tmp")],
        "cline" => vec![
            roaming
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("saoudrizwan.claude-dev")
                .join("tasks"),
            home.join(".cline").join("data").join("sessions"),
        ],
        "kilo-code" => vec![
            roaming
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("kilocode.kilo-code")
                .join("tasks"),
            home.join(".kilocode").join("sessions"),
        ],
        "kimi-cli" => vec![
            home.join(".kimi").join("sessions"),
            home.join(".kimi-code").join("sessions"),
        ],
        "qwen-cli" => vec![home.join(".qwen").join("projects")],
        "openclaw" => vec![home.join(".openclaw").join("agents")],
        "hermes" => vec![std::env::var_os("HERMES_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".hermes"))
            .join("state.db")],
        "antigravity" => vec![
            roaming
                .join("Antigravity")
                .join("User")
                .join("workspaceStorage"),
            roaming
                .join("Antigravity")
                .join("User")
                .join("globalStorage"),
        ],
        "grok-build" => vec![
            home.join(".grok").join("sessions"),
            home.join(".grok").join("logs").join("unified.jsonl"),
        ],
        "pi" => vec![
            home.join(".pi").join("agent").join("sessions"),
            home.join(".omp").join("agent").join("sessions"),
        ],
        "zed" => vec![local.join("Zed").join("threads").join("threads.db")],
        "kiro" => vec![
            home.join(".kiro").join("sessions").join("cli"),
            home.join(".local")
                .join("share")
                .join("kiro-cli")
                .join("data.sqlite3"),
        ],
        "proma" => vec![home.join(".proma").join("agent-sessions")],
        "reasonix" => vec![
            home.join(".reasonix").join("stats"),
            home.join(".reasonix").join("sessions"),
            home.join(".reasonix").join("projects"),
        ],
        "mimo-code" => vec![local.join("mimocode"), local.join("MiMo Code")],
        "zcode" => vec![
            home.join(".zcode").join("projects"),
            home.join(".zcode").join("cli").join("db").join("db.sqlite"),
        ],
        "codebuddy" => vec![
            home.join(".codebuddy").join("projects"),
            roaming.join("CodeBuddy"),
        ],
        "workbuddy" => vec![
            home.join(".workbuddy").join("projects"),
            home.join(".workbuddy").join("workbuddy.db"),
        ],
        "trae-agent" => vec![
            home.join(".trae-agent").join("trajectories"),
            home.join("trae-agent-trajectories"),
        ],
        "trae-ide" => vec![roaming.join("Trae")],
        "qoder" => vec![roaming.join("Qoder"), local.join("Qoder")],
        "lingma" => vec![roaming.join("Lingma"), home.join(".lingma")],
        "comate" => vec![roaming.join("Baidu Comate"), home.join(".comate")],
        _ => Vec::new(),
    }
}

fn spec(id: &str) -> Option<&'static AgentSpec> {
    SPECS.iter().find(|item| item.id == id)
}

fn official_url(id: &str) -> &'static str {
    match id {
        "claude-code" => "https://github.com/anthropics/claude-code",
        "opencode" => "https://github.com/anomalyco/opencode",
        "cursor" => "https://www.cursor.com/",
        "github-copilot" => "https://github.com/features/copilot",
        "gemini-cli" => "https://github.com/google-gemini/gemini-cli",
        "cline" => "https://github.com/cline/cline",
        "kilo-code" => "https://github.com/Kilo-Org/kilocode",
        "kimi-cli" => "https://github.com/MoonshotAI/kimi-cli",
        "qwen-cli" => "https://github.com/QwenLM/qwen-code",
        "openclaw" => "https://github.com/openclaw/openclaw",
        "hermes" => "https://github.com/NousResearch/hermes-agent",
        "antigravity" => "https://antigravity.google/",
        "grok-build" => "https://x.ai/",
        "pi" => "https://pi.dev/",
        "zed" => "https://github.com/zed-industries/zed",
        "kiro" => "https://kiro.dev/",
        "proma" => "https://github.com/tokscale-ai/tokscale",
        "reasonix" => "https://github.com/tokscale-ai/tokscale",
        "mimo-code" => "https://github.com/XiaomiMiMo/MiMo-Code",
        "zcode" => "https://zcode.z.ai/",
        "codebuddy" => "https://www.codebuddy.cn/docs/cli/",
        "workbuddy" => "https://www.codebuddy.cn/",
        "trae-agent" | "trae-ide" => "https://www.trae.ai/",
        "qoder" => "https://qoder.com/",
        "lingma" => "https://lingma.aliyun.com/",
        "comate" => "https://comate.baidu.com/",
        _ => "",
    }
}

fn schema_fingerprint(id: &str) -> &'static str {
    match id {
        "opencode" => "opencode.sqlite.message.parts.v1|opencode.jsonl.usage.v1",
        "github-copilot" => "copilot.otel.usage.v1|copilot.data.db.v1",
        "hermes" => "hermes.state.db.v1",
        "mimo-code" => "mimocode.sqlite.usage.v1",
        "zcode" => "zcode.db.usage.v1|zcode.projects.jsonl.v1",
        "zed" => "zed.threads.db.v1",
        "kiro" => "kiro.data.sqlite3.v1|kiro.sessions.v1",
        value => match spec(value).map(|item| item.collector_kind) {
            Some("detected") => "installation-detection-only",
            Some("sqlite") => "agent-specific-sqlite-usage-v1",
            Some("otel") => "agent-specific-otel-usage-v1",
            _ => "agent-specific-json-usage-v1",
        },
    }
}

/// 只有已经确认了产品官方站点、品牌来源和可验证用量结构的连接器，
/// 才能在界面标记为“正式支持”。其余连接器保留检测或实验状态，
/// 避免把路径参考误导成稳定的正式兼容能力。
fn support_state(id: &str, collector_kind: &str) -> &'static str {
    if collector_kind == "detected" {
        return "detected_only";
    }
    match id {
        "claude-code" | "opencode" | "cursor" | "github-copilot" | "gemini-cli" | "cline"
        | "kilo-code" | "kimi-cli" | "qwen-cli" | "openclaw" | "hermes" | "pi" | "zed" | "kiro"
        | "mimo-code" | "zcode" | "codebuddy" | "trae-agent" => "supported",
        _ => "experimental",
    }
}

pub fn definitions() -> Vec<AgentConnectorDefinition> {
    SPECS
        .iter()
        .map(|item| {
            let candidates = path_candidates(item.id);
            let defaults = candidates
                .iter()
                .map(|item| PathBuf::from(&item.resolved_path))
                .collect::<Vec<_>>();
            let detected_paths = defaults
                .iter()
                .filter(|path| path.exists())
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            AgentConnectorDefinition {
                id: item.id.into(),
                name: item.name.into(),
                provider: item.provider.into(),
                collector_kind: item.collector_kind.into(),
                default_paths: defaults
                    .iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect(),
                capabilities: item
                    .capabilities
                    .iter()
                    .map(|value| (*value).into())
                    .collect(),
                proxy_protocol: item.proxy_protocol.map(str::to_string),
                detected: !detected_paths.is_empty(),
                detected_paths,
                schema_version: 1,
                detail: if item.id == "cursor" {
                    "Cursor 本地缓存只有在包含可靠用量字段时才会导入；不会把缺失字段显示为 0".into()
                } else {
                    "仅读取本地用量元数据，不读取提示词、回复或代码正文".into()
                },
                brand_id: item.id.into(),
                official_url: official_url(item.id).into(),
                collector_mode: item.collector_kind.into(),
                path_specs: candidates
                    .iter()
                    .map(|item| item.template.clone())
                    .collect(),
                usage_capability: if item.collector_kind == "detected" {
                    "none"
                } else {
                    "observed"
                }
                .into(),
                limit_capability: "none".into(),
                proxy_capability: item.proxy_protocol.unwrap_or("none").into(),
                schema_fingerprint: schema_fingerprint(item.id).into(),
                support_state: support_state(item.id, item.collector_kind).into(),
                path_candidates: candidates,
            }
        })
        .collect()
}

pub fn default_source_rows() -> Vec<SyncSourceConfig> {
    definitions()
        .into_iter()
        .map(|item| {
            let paths = if item.detected_paths.is_empty() {
                item.default_paths.clone()
            } else {
                item.detected_paths.clone()
            };
            SyncSourceConfig {
                id: match item.id.as_str() {
                    "codex" => "codex-local".into(),
                    "claude-code" => "claude-local".into(),
                    "opencode" => "opencode-local".into(),
                    _ => format!("agent-{}", item.id),
                },
                provider: item.provider,
                kind: if item.collector_kind == "sqlite" {
                    SourceKind::LocalSqlite.as_str().into()
                } else if item.collector_kind == "json"
                    || item.collector_kind == "jsonl"
                    || item.collector_kind == "otel"
                    || item.collector_kind == "cache"
                {
                    SourceKind::LocalJson.as_str().into()
                } else {
                    SourceKind::LocalLog.as_str().into()
                },
                mode: "auto".into(),
                path: paths.first().cloned().unwrap_or_default(),
                // 自动发现只负责提示可接入来源；大型编辑器目录必须由用户确认后再建立递归监听。
                enabled: false,
                interval_seconds: 30,
                agent_id: item.id,
                collector_kind: item.collector_kind,
                paths,
                capabilities: item.capabilities,
                detected: item.detected,
                schema_version: item.schema_version,
                path_mode: "auto".into(),
                path_spec_ids: item
                    .path_candidates
                    .iter()
                    .map(|candidate| candidate.id.clone())
                    .collect(),
            }
        })
        .collect()
}

fn number(value: &Value, paths: &[&str]) -> u64 {
    paths
        .iter()
        .find_map(|path| {
            value.pointer(path).and_then(|item| {
                item.as_u64()
                    .or_else(|| item.as_i64().map(|v| v.max(0) as u64))
            })
        })
        .unwrap_or(0)
}

fn text(value: &Value, paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| {
            value
                .pointer(path)
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|value| !value.trim().is_empty())
}

fn timestamp(value: &Value, fallback_ms: i64) -> DateTime<Utc> {
    let raw = [
        "/timestamp",
        "/created_at",
        "/createdAt",
        "/time",
        "/time/created",
        "/time/updated",
        "/message/timestamp",
    ]
    .iter()
    .find_map(|path| value.pointer(path));
    if let Some(Value::String(raw)) = raw {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
            return parsed.with_timezone(&Utc);
        }
        if let Ok(number) = raw.parse::<i64>() {
            return numeric_time(number);
        }
    }
    if let Some(number) = raw.and_then(Value::as_i64) {
        return numeric_time(number);
    }
    Utc.timestamp_millis_opt(fallback_ms)
        .single()
        .unwrap_or_else(Utc::now)
}

fn numeric_time(value: i64) -> DateTime<Utc> {
    if value.abs() > 10_000_000_000 {
        Utc.timestamp_millis_opt(value)
            .single()
            .unwrap_or_else(Utc::now)
    } else {
        Utc.timestamp_opt(value, 0)
            .single()
            .unwrap_or_else(Utc::now)
    }
}

fn file_modified_ms(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis() as i64)
        .unwrap_or_else(|| Utc::now().timestamp_millis())
}

fn agent_token_paths(agent_id: &str, field: &str) -> &'static [&'static str] {
    const INPUT_STANDARD: &[&str] = &[
        "/usage/input_tokens",
        "/usage/prompt_tokens",
        "/tokens/input",
        "/input_tokens",
    ];
    const OUTPUT_STANDARD: &[&str] = &[
        "/usage/output_tokens",
        "/usage/completion_tokens",
        "/tokens/output",
        "/output_tokens",
    ];
    const CACHE_STANDARD: &[&str] = &[
        "/usage/cache_read_input_tokens",
        "/usage/cached_tokens",
        "/tokens/cached",
        "/cache_read_tokens",
    ];
    const INPUT_CAMEL: &[&str] = &[
        "/usage/inputTokens",
        "/tokens/input_tokens",
        "/inputTokens",
        "/prompt_tokens",
    ];
    const OUTPUT_CAMEL: &[&str] = &[
        "/usage/outputTokens",
        "/tokens/output_tokens",
        "/outputTokens",
        "/completion_tokens",
    ];
    const CACHE_CAMEL: &[&str] = &[
        "/usage/cacheReadInputTokens",
        "/tokens/cache_read",
        "/cached_tokens",
    ];
    const INPUT_GEMINI: &[&str] = &[
        "/usageMetadata/promptTokenCount",
        "/tokens/input",
        "/usage/input_tokens",
    ];
    const OUTPUT_GEMINI: &[&str] = &[
        "/usageMetadata/candidatesTokenCount",
        "/tokens/output",
        "/usage/output_tokens",
    ];
    const CACHE_GEMINI: &[&str] = &["/usageMetadata/cachedContentTokenCount", "/tokens/cached"];
    const INPUT_OTEL: &[&str] = &[
        "/attributes/gen_ai.usage.input_tokens",
        "/usage/input_tokens",
        "/tokens/input",
    ];
    const OUTPUT_OTEL: &[&str] = &[
        "/attributes/gen_ai.usage.output_tokens",
        "/usage/output_tokens",
        "/tokens/output",
    ];
    const CACHE_OTEL: &[&str] = &[
        "/attributes/gen_ai.usage.cache_read_input_tokens",
        "/usage/cached_tokens",
    ];
    let profile = if agent_id == "gemini-cli" {
        "gemini"
    } else if agent_id == "github-copilot" {
        "otel"
    } else if matches!(
        agent_id,
        "cline"
            | "kilo-code"
            | "kimi-cli"
            | "qwen-cli"
            | "mimo-code"
            | "zcode"
            | "codebuddy"
            | "workbuddy"
    ) {
        "camel"
    } else {
        "standard"
    };
    match (profile, field) {
        ("gemini", "input") => INPUT_GEMINI,
        ("gemini", "output") => OUTPUT_GEMINI,
        ("gemini", "cached") => CACHE_GEMINI,
        ("otel", "input") => INPUT_OTEL,
        ("otel", "output") => OUTPUT_OTEL,
        ("otel", "cached") => CACHE_OTEL,
        ("camel", "input") => INPUT_CAMEL,
        ("camel", "output") => OUTPUT_CAMEL,
        ("camel", "cached") => CACHE_CAMEL,
        (_, "input") => INPUT_STANDARD,
        (_, "output") => OUTPUT_STANDARD,
        _ => CACHE_STANDARD,
    }
}

fn has_usage(agent_id: &str, value: &Value) -> bool {
    number(value, agent_token_paths(agent_id, "input")) > 0
        || number(value, agent_token_paths(agent_id, "output")) > 0
        || number(value, agent_token_paths(agent_id, "cached")) > 0
}

fn collect_usage_values<'a>(
    agent_id: &str,
    value: &'a Value,
    rows: &mut Vec<&'a Value>,
    depth: usize,
) {
    if depth > 10 {
        return;
    }
    if has_usage(agent_id, value) {
        rows.push(value);
        return;
    }
    match value {
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_usage_values(agent_id, item, rows, depth + 1)),
        Value::Object(map) => map
            .values()
            .for_each(|item| collect_usage_values(agent_id, item, rows, depth + 1)),
        _ => {}
    }
}

fn event_from_value(
    source: &SyncSourceConfig,
    path: &Path,
    value: &Value,
    index: usize,
) -> Option<(UsageEvent, UsageProvenance)> {
    let direct_input = number(value, agent_token_paths(&source.agent_id, "input"));
    let output = number(value, agent_token_paths(&source.agent_id, "output"));
    let cached = number(value, agent_token_paths(&source.agent_id, "cached"));
    let cache_write = number(
        value,
        &[
            "/usage/cache_creation_input_tokens",
            "/usage/cache_write_tokens",
            "/tokens/cache_write",
            "/cache_write_tokens",
        ],
    );
    let reasoning = number(
        value,
        &[
            "/usage/reasoning_tokens",
            "/usage/thoughts_tokens",
            "/tokens/reasoning",
            "/tokens/thoughts",
            "/reasoning_tokens",
        ],
    );
    if direct_input + output + cached + cache_write + reasoning == 0 {
        return None;
    }
    let modified = file_modified_ms(path);
    let at = timestamp(value, modified);
    let model = text(
        value,
        &[
            "/model",
            "/model_id",
            "/modelID",
            "/message/model",
            "/request/model",
            "/provider/model",
        ],
    )
    .unwrap_or_else(|| {
        spec(&source.agent_id)
            .map(|item| item.name.to_string())
            .unwrap_or_else(|| source.provider.clone())
    });
    let stable = text(
        value,
        &[
            "/id",
            "/request_id",
            "/requestId",
            "/session_id",
            "/sessionId",
            "/message/id",
        ],
    )
    .unwrap_or_else(|| {
        format!(
            "{}:{index}:{}",
            path.to_string_lossy(),
            at.timestamp_millis()
        )
    });
    let digest = Sha256::digest(
        format!(
            "{}:{stable}:{direct_input}:{output}:{cached}:{cache_write}:{reasoning}",
            source.id
        )
        .as_bytes(),
    );
    let mut provenance = UsageProvenance::observed(
        &source.id,
        if source.collector_kind == "json"
            || source.collector_kind == "jsonl"
            || source.collector_kind == "otel"
            || source.collector_kind == "cache"
        {
            SourceKind::LocalJson
        } else {
            SourceKind::LocalLog
        },
    );
    provenance.source_ref = Some(stable);
    provenance.price_version = "local-agent-observed-v1".into();
    Some((
        UsageEvent {
            id: format!("{}:{:x}", source.id, digest),
            provider: source.provider.clone(),
            model,
            at,
            input: direct_input + cached + cache_write,
            output: output + reasoning,
            cached,
            cost: 0.0,
            task: "其他".into(),
        },
        provenance,
    ))
}

fn collect_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let started = Instant::now();
    let mut stack = roots
        .iter()
        .map(|path| (path.clone(), 0usize))
        .collect::<Vec<_>>();
    while let Some((path, depth)) = stack.pop() {
        if files.len() >= MAX_FILES
            || depth > MAX_SCAN_DEPTH
            || started.elapsed() >= SOURCE_DEADLINE
        {
            break;
        }
        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if matches!(ext.as_str(), "json" | "jsonl" | "ndjson") {
                files.push(path);
            }
            continue;
        }
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            if files.len() >= MAX_FILES || started.elapsed() >= SOURCE_DEADLINE {
                break;
            }
            let child = entry.path();
            if child.is_dir() {
                stack.push((child, depth + 1));
            } else {
                let ext = child
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if matches!(ext.as_str(), "json" | "jsonl" | "ndjson") {
                    files.push(child);
                }
            }
        }
    }
    files
}

fn roots(source: &SyncSourceConfig) -> Vec<PathBuf> {
    let mut rows = source
        .paths
        .iter()
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if !source.path.trim().is_empty() {
        rows.insert(0, PathBuf::from(&source.path));
    }
    if rows.is_empty() {
        rows = candidate_paths(&source.agent_id);
    }
    let mut seen = HashSet::new();
    rows.into_iter()
        .filter(|path| path.exists() && seen.insert(path.to_string_lossy().to_ascii_lowercase()))
        .collect()
}

fn cursor_map(target: &Connection, source_id: &str, _force: bool) -> HashMap<String, i64> {
    // 普通手动同步也必须沿用增量游标；全量重建只允许由诊断中心的独立操作触发。
    target
        .query_row(
            "SELECT cursor_json FROM sync_cursors WHERE source_id=?1",
            [source_id],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|raw| serde_json::from_str::<HashMap<String, i64>>(&raw).ok())
        .unwrap_or_default()
}

fn save_file_cursor(
    target: &Connection,
    source_id: &str,
    cursor: &HashMap<String, i64>,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    target.execute(
        "INSERT INTO sync_cursors(source_id,cursor_json,last_success_at,last_error,updated_at) VALUES(?1,?2,?3,NULL,?3)
         ON CONFLICT(source_id) DO UPDATE SET cursor_json=excluded.cursor_json,last_success_at=excluded.last_success_at,last_error=NULL,updated_at=excluded.updated_at",
        rusqlite::params![source_id, serde_json::to_string(cursor).map_err(|error| error.to_string())?, now],
    ).map_err(|error| error.to_string())?;
    Ok(())
}

fn flush_usage_batch(
    target: &Connection,
    pending: &mut Vec<(UsageEvent, UsageProvenance)>,
) -> Result<usize, String> {
    if pending.is_empty() {
        return Ok(0);
    }
    let transaction = target
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let mut imported = 0usize;
    for (event, provenance) in pending.drain(..) {
        if reliability::upsert_usage(&transaction, &event, &provenance)
            .map_err(|error| error.to_string())?
        {
            imported += 1;
        }
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(imported)
}

fn sync_json(
    target: &Connection,
    source: &SyncSourceConfig,
    force: bool,
) -> Result<LocalSyncOutcome, String> {
    let started = Instant::now();
    let source_roots = roots(source);
    if source_roots.is_empty() {
        return Err(format!(
            "未发现 {} 本地记录；可在设置中手动选择路径",
            source.provider
        ));
    }
    let files = collect_files(&source_roots);
    if files.is_empty() {
        return Err(format!(
            "已发现 {}，但没有可解析的 JSON/JSONL 用量文件",
            source_roots[0].display()
        ));
    }
    let old_cursor = cursor_map(target, &source.id, force);
    let mut next_cursor = old_cursor.clone();
    let mut imported = 0usize;
    let mut scanned = 0usize;
    let mut pending = Vec::with_capacity(250);
    for path in files {
        if started.elapsed() >= SOURCE_DEADLINE {
            return Err(format!(
                "{} 扫描达到20秒安全截止时间，已保留当前游标",
                source.provider
            ));
        }
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > MAX_FILE_BYTES {
            continue;
        }
        let modified = file_modified_ms(&path);
        let key = path.to_string_lossy().into_owned();
        if old_cursor.get(&key).is_some_and(|value| *value >= modified) {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        scanned += 1;
        let values = if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                value.eq_ignore_ascii_case("jsonl") || value.eq_ignore_ascii_case("ndjson")
            }) {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        } else {
            serde_json::from_str::<Value>(&raw)
                .ok()
                .into_iter()
                .collect::<Vec<_>>()
        };
        for root in &values {
            let mut rows = Vec::new();
            collect_usage_values(&source.agent_id, root, &mut rows, 0);
            for (index, value) in rows.into_iter().enumerate() {
                if let Some((event, provenance)) = event_from_value(source, &path, value, index) {
                    pending.push((event, provenance));
                    if pending.len() >= 250 {
                        imported += flush_usage_batch(target, &mut pending)?;
                    }
                }
            }
        }
        next_cursor.insert(key, modified);
    }
    imported += flush_usage_batch(target, &mut pending)?;
    save_file_cursor(target, &source.id, &next_cursor)?;
    Ok(LocalSyncOutcome {
        imported,
        scanned_files: scanned,
        source_kind: if source.collector_kind == "json"
            || source.collector_kind == "jsonl"
            || source.collector_kind == "otel"
            || source.collector_kind == "cache"
        {
            SourceKind::LocalJson.as_str().into()
        } else {
            SourceKind::LocalLog.as_str().into()
        },
        source_path: source_roots[0].to_string_lossy().into_owned(),
        detail: format!(
            "已扫描 {scanned} 个 {} 本地文件，新增 {imported} 条用量；未读取对话或代码正文",
            source.provider
        ),
    })
}

fn sql_i64(value: SqlValue) -> i64 {
    match value {
        SqlValue::Integer(v) => v,
        SqlValue::Real(v) => v as i64,
        SqlValue::Text(v) => v.parse().unwrap_or(0),
        _ => 0,
    }
}
fn sql_text(value: SqlValue) -> String {
    match value {
        SqlValue::Text(v) => v,
        SqlValue::Integer(v) => v.to_string(),
        SqlValue::Real(v) => v.to_string(),
        _ => String::new(),
    }
}
fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn collect_sqlite_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let started = Instant::now();
    let mut stack = roots
        .iter()
        .map(|path| (path.clone(), 0usize))
        .collect::<Vec<_>>();
    while let Some((path, depth)) = stack.pop() {
        if files.len() >= 256 || depth > MAX_SCAN_DEPTH || started.elapsed() >= SOURCE_DEADLINE {
            break;
        }
        if path.is_file() {
            if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| {
                    matches!(
                        value.to_ascii_lowercase().as_str(),
                        "db" | "sqlite" | "sqlite3"
                    )
                })
            {
                files.push(path);
            }
            continue;
        }
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if files.len() + stack.len() >= 256 || started.elapsed() >= SOURCE_DEADLINE {
                    break;
                }
                stack.push((entry.path(), depth + 1));
            }
        }
    }
    files.sort_by_key(|path| std::cmp::Reverse(file_modified_ms(path)));
    files
}

fn sync_sqlite(target: &Connection, source: &SyncSourceConfig) -> Result<LocalSyncOutcome, String> {
    let started = Instant::now();
    let db_path = collect_sqlite_files(&roots(source))
        .into_iter()
        .next()
        .ok_or_else(|| format!("未发现 {} SQLite 数据库", source.provider))?;
    let external = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("无法只读打开 {}：{error}", db_path.display()))?;
    external
        .busy_timeout(std::time::Duration::from_millis(800))
        .map_err(|error| error.to_string())?;
    let mut tables_stmt = external
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|error| error.to_string())?;
    let tables = tables_stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .flatten()
        .collect::<Vec<_>>();
    for table in tables {
        let mut pragma = external
            .prepare(&format!("PRAGMA table_info({})", quote_identifier(&table)))
            .map_err(|error| error.to_string())?;
        let columns = pragma
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|error| error.to_string())?
            .flatten()
            .collect::<Vec<_>>();
        let find = |names: &[&str]| {
            columns
                .iter()
                .find(|column| names.iter().any(|name| column.eq_ignore_ascii_case(name)))
                .cloned()
        };
        let Some(input_col) = find(&[
            "input_tokens",
            "tokens_input",
            "prompt_tokens",
            "inputTokens",
        ]) else {
            continue;
        };
        let Some(output_col) = find(&[
            "output_tokens",
            "tokens_output",
            "completion_tokens",
            "outputTokens",
        ]) else {
            continue;
        };
        let cached_col = find(&["cached_tokens", "tokens_cache_read", "cache_read_tokens"]);
        let model_col = find(&["model", "model_id", "modelID"]);
        let time_col = find(&[
            "timestamp",
            "created_at",
            "time_updated",
            "updated_at",
            "createdAt",
        ]);
        let id_col = find(&["id", "session_id", "request_id", "message_id"]);
        let select = [
            id_col.as_ref(),
            model_col.as_ref(),
            time_col.as_ref(),
            Some(&input_col),
            Some(&output_col),
            cached_col.as_ref(),
        ]
        .into_iter()
        .map(|column| {
            column
                .map(|value| quote_identifier(value))
                .unwrap_or_else(|| "NULL".into())
        })
        .collect::<Vec<_>>()
        .join(",");
        let mut statement = external
            .prepare(&format!(
                "SELECT {select} FROM {}",
                quote_identifier(&table)
            ))
            .map_err(|error| error.to_string())?;
        let mut rows = statement.query([]).map_err(|error| error.to_string())?;
        let mut imported = 0usize;
        let mut row_index = 0usize;
        let mut pending = Vec::with_capacity(250);
        while let Some(row) = rows.next().map_err(|error| error.to_string())? {
            if started.elapsed() >= SOURCE_DEADLINE {
                return Err(format!(
                    "{} SQLite 同步达到20秒安全截止时间",
                    source.provider
                ));
            }
            if row_index % 128 == 0 && !db_path.exists() {
                return Err("来源数据库在同步过程中消失".into());
            }
            row_index += 1;
            let input = sql_i64(row.get::<_, SqlValue>(3).unwrap_or(SqlValue::Null)).max(0) as u64;
            let output = sql_i64(row.get::<_, SqlValue>(4).unwrap_or(SqlValue::Null)).max(0) as u64;
            let cached = sql_i64(row.get::<_, SqlValue>(5).unwrap_or(SqlValue::Null)).max(0) as u64;
            if input + output + cached == 0 {
                continue;
            }
            let stable = sql_text(row.get::<_, SqlValue>(0).unwrap_or(SqlValue::Null));
            let model = sql_text(row.get::<_, SqlValue>(1).unwrap_or(SqlValue::Null));
            let time_raw = sql_text(row.get::<_, SqlValue>(2).unwrap_or(SqlValue::Null));
            let at = DateTime::parse_from_rfc3339(&time_raw)
                .map(|value| value.with_timezone(&Utc))
                .or_else(|_| time_raw.parse::<i64>().map(numeric_time))
                .unwrap_or_else(|_| Utc::now());
            let stable = if stable.is_empty() {
                format!("{table}:{row_index}")
            } else {
                stable
            };
            let digest = Sha256::digest(
                format!("{}:{stable}:{input}:{output}:{cached}", source.id).as_bytes(),
            );
            let event = UsageEvent {
                id: format!("{}:{:x}", source.id, digest),
                provider: source.provider.clone(),
                model: if model.is_empty() {
                    source.provider.clone()
                } else {
                    model
                },
                at,
                input,
                output,
                cached,
                cost: 0.0,
                task: "其他".into(),
            };
            let mut provenance = UsageProvenance::observed(&source.id, SourceKind::LocalSqlite);
            provenance.source_ref = Some(stable);
            provenance.price_version = "local-agent-observed-v1".into();
            pending.push((event, provenance));
            if pending.len() >= 250 {
                imported += flush_usage_batch(target, &mut pending)?;
            }
        }
        imported += flush_usage_batch(target, &mut pending)?;
        return Ok(LocalSyncOutcome {
            imported,
            scanned_files: 1,
            source_kind: SourceKind::LocalSqlite.as_str().into(),
            source_path: db_path.to_string_lossy().into_owned(),
            detail: format!(
                "已从 {} 的 {table} 表只读同步 {imported} 条记录；未读取正文",
                source.provider
            ),
        });
    }
    Err(format!(
        "当前 {} 数据库版本暂未适配：没有发现包含输入/输出 Token 的表",
        source.provider
    ))
}

pub fn sync(
    target: &Connection,
    source: &SyncSourceConfig,
    force: bool,
) -> Result<LocalSyncOutcome, String> {
    if source.collector_kind == "detected" {
        return Err(format!(
            "{} 已检测安装，但当前版本没有可验证的本地 Token 字段",
            source.provider
        ));
    }
    if source.agent_id == "opencode" {
        return super::opencode::sync(target, source, force);
    }
    if source.collector_kind == "sqlite" {
        sync_sqlite(target, source)
    } else {
        sync_json(target, source, force)
    }
}

pub fn models(target: &Connection, source_id: &str) -> Result<Vec<AgentSourceModel>, String> {
    let mut statement = target.prepare("SELECT model,COUNT(*),MAX(at) FROM usage_events WHERE source_id=?1 AND is_shadowed=0 GROUP BY model ORDER BY MAX(at) DESC").map_err(|error| error.to_string())?;
    let models = statement
        .query_map([source_id], |row| {
            Ok(AgentSourceModel {
                source_id: source_id.into(),
                model: row.get(0)?,
                records: row.get::<_, i64>(1)?.max(0) as u64,
                last_seen_at: row.get(2)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_requested_agent_matrix() {
        let definitions = definitions();
        assert!(definitions.len() >= 27);
        assert!(definitions.iter().any(|item| item.id == "github-copilot"));
        assert!(definitions.iter().any(|item| item.id == "hermes"));
        assert!(definitions.iter().any(|item| item.id == "mimo-code"));
        assert!(definitions.iter().any(|item| item.id == "zcode"));
        assert!(!definitions.iter().any(|item| item.id == "codex"));
    }

    #[test]
    fn extracts_usage_without_persisting_message_body() {
        let source = SyncSourceConfig {
            id: "agent-gemini-cli".into(),
            provider: "Gemini CLI".into(),
            kind: "local_json".into(),
            mode: "auto".into(),
            path: String::new(),
            enabled: true,
            interval_seconds: 30,
            agent_id: "gemini-cli".into(),
            collector_kind: "json".into(),
            paths: vec![],
            capabilities: vec!["本地 JSON".into()],
            detected: true,
            schema_version: 1,
            path_mode: "manual".into(),
            path_spec_ids: vec![],
        };
        let value = serde_json::json!({"model":"gemini-2.5-pro","tokens":{"input":120,"output":30,"cached":20},"content":"private prompt"});
        let path = std::env::temp_dir().join("tm-agent-fixture.json");
        fs::write(&path, value.to_string()).unwrap();
        let (event, provenance) = event_from_value(&source, &path, &value, 0).unwrap();
        assert_eq!((event.input, event.output, event.cached), (140, 30, 20));
        assert!(!serde_json::to_string(&event)
            .unwrap()
            .contains("private prompt"));
        assert_eq!(provenance.source_id, "agent-gemini-cli");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn every_connector_has_brand_and_official_source_metadata() {
        for definition in definitions() {
            assert!(
                !definition.brand_id.trim().is_empty(),
                "{} 缺少 brand_id",
                definition.id
            );
            assert!(
                !definition.official_url.trim().is_empty(),
                "{} 缺少官方来源",
                definition.id
            );
            assert!(!definition.schema_fingerprint.trim().is_empty());
        }
    }

    #[test]
    fn corrupt_and_oversized_json_are_isolated() {
        let root = std::env::temp_dir().join(format!(
            "tm-agent-corrupt-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("broken.jsonl"), "{not-json}\n[]\n").unwrap();
        let oversized = fs::File::create(root.join("oversized.json")).unwrap();
        oversized.set_len(MAX_FILE_BYTES + 1).unwrap();
        let target = Connection::open_in_memory().unwrap();
        target.execute_batch("CREATE TABLE usage_events(id TEXT PRIMARY KEY,provider TEXT,model TEXT,at TEXT,input_tokens INTEGER,output_tokens INTEGER,cached_tokens INTEGER,cost REAL,task TEXT);").unwrap();
        reliability::migrate_database(&target).unwrap();
        let source = SyncSourceConfig {
            id: "agent-gemini-corrupt".into(),
            provider: "Gemini CLI".into(),
            kind: "local_json".into(),
            mode: "json".into(),
            path: root.to_string_lossy().into_owned(),
            enabled: true,
            interval_seconds: 30,
            agent_id: "gemini-cli".into(),
            collector_kind: "json".into(),
            paths: vec![],
            capabilities: vec!["本地 JSON".into()],
            detected: true,
            schema_version: 1,
            path_mode: "manual".into(),
            path_spec_ids: vec![],
        };
        let outcome = sync_json(&target, &source, false).unwrap();
        assert_eq!(outcome.imported, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn manual_force_does_not_discard_incremental_cursor() {
        let target = Connection::open_in_memory().unwrap();
        target.execute_batch("CREATE TABLE usage_events(id TEXT PRIMARY KEY,provider TEXT,model TEXT,at TEXT,input_tokens INTEGER,output_tokens INTEGER,cached_tokens INTEGER,cost REAL,task TEXT);").unwrap();
        reliability::migrate_database(&target).unwrap();
        target.execute("INSERT INTO sync_cursors(source_id,cursor_json,updated_at) VALUES('agent-test','{\"file\":42}','now')", []).unwrap();
        assert_eq!(
            cursor_map(&target, "agent-test", true).get("file"),
            Some(&42)
        );
    }

    #[test]
    fn file_storm_and_unicode_paths_respect_scan_cap() {
        let root = std::env::temp_dir().join(format!(
            "令牌监控-文件风暴-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&root).unwrap();
        for index in 0..(MAX_FILES + 100) {
            fs::write(root.join(format!("事件-{index:04}.json")), "{broken-json}").unwrap();
        }
        let target = Connection::open_in_memory().unwrap();
        target.execute_batch("CREATE TABLE usage_events(id TEXT PRIMARY KEY,provider TEXT,model TEXT,at TEXT,input_tokens INTEGER,output_tokens INTEGER,cached_tokens INTEGER,cost REAL,task TEXT);").unwrap();
        reliability::migrate_database(&target).unwrap();
        let source = SyncSourceConfig {
            id: "agent-file-storm".into(),
            provider: "Gemini CLI".into(),
            kind: "local_json".into(),
            mode: "json".into(),
            path: root.to_string_lossy().into_owned(),
            enabled: true,
            interval_seconds: 30,
            agent_id: "gemini-cli".into(),
            collector_kind: "json".into(),
            paths: vec![],
            capabilities: vec!["本地 JSON".into()],
            detected: true,
            schema_version: 1,
            path_mode: "manual".into(),
            path_spec_ids: vec![],
        };
        let outcome = sync_json(&target, &source, false).unwrap();
        assert!(
            outcome.scanned_files <= MAX_FILES,
            "扫描数量必须受硬上限保护"
        );
        assert_eq!(outcome.imported, 0);
        let _ = fs::remove_dir_all(root);
    }
}
