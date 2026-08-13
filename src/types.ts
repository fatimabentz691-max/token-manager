export type ProviderSourceKind = '官方账单' | '本地代理' | '账单导入' | '本地日志' | '不可用'

export interface Provider {
  id: string
  name: string
  source: ProviderSourceKind
  description: string
  configured: boolean
  models: string[]
}

export type UsageSourceKind =
  | 'official_api'
  | 'local_proxy'
  | 'local_log'
  | 'local_json'
  | 'local_sqlite'
  | 'bill_import'
  | 'estimate'

export type UsageAccuracy = 'official' | 'observed' | 'imported' | 'estimated'

export interface Usage {
  id: string
  provider: string
  model: string
  at: string
  input: number
  output: number
  cached: number
  cost: number
  task: '生成' | '调试' | '文档' | '其他' | '失败'
  source_id?: string
  source_kind?: UsageSourceKind
  accuracy?: UsageAccuracy
  account_id?: string | null
  collected_at?: string
  source_ref?: string | null
  price_version?: string
  currency?: string
  canonical_key?: string
  is_shadowed?: boolean
  request_count?: number
}

export interface SyncSourceConfig {
  id: string
  provider: string
  kind: UsageSourceKind
  mode: 'auto' | 'sqlite' | 'json' | 'jsonl' | 'log' | 'otel' | 'cache'
  path: string
  enabled: boolean
  interval_seconds: number
  agent_id: string
  collector_kind: string
  paths: string[]
  capabilities: string[]
  detected: boolean
  schema_version: number
  path_mode: 'auto' | 'manual'
  path_spec_ids: string[]
}

export interface AgentPathCandidate {
  id: string
  template: string
  resolved_path: string
  exists: boolean
  root_kind: 'home' | 'app_data' | 'local_app_data' | 'environment'
}

/** Rust 本地 Agent 注册表的只读定义；不含提示词、回复或认证数据。 */
export interface LocalAgentDefinition {
  id: string
  name: string
  provider: string
  collector_kind: string
  default_paths: string[]
  capabilities: string[]
  proxy_protocol: 'openai' | 'anthropic' | 'both' | null
  detected: boolean
  detected_paths: string[]
  schema_version: number
  detail: string
  brand_id: string
  official_url: string
  collector_mode: string
  path_specs: string[]
  usage_capability: 'official' | 'observed' | 'none'
  limit_capability: 'client' | 'official' | 'none'
  proxy_capability: 'openai' | 'anthropic' | 'both' | 'none'
  schema_fingerprint: string
  support_state: 'supported' | 'detected_only' | 'experimental'
  path_candidates: AgentPathCandidate[]
}

export interface AgentSourceModel {
  source_id: string
  model: string
  records: number
  last_seen_at: string
}

export interface SyncHealth {
  source_id: string
  provider: string
  source_kind: UsageSourceKind
  status: 'idle' | 'syncing' | 'healthy' | 'error' | 'disabled'
  last_success_at: string | null
  next_refresh_at: string | null
  last_error: string | null
  latency_ms: number
  imported: number
  detail: string
}

export interface SyncJob {
  id: string
  kind: 'all' | 'source'
  source_ids: string[]
  status: 'queued' | 'running' | 'completed' | 'cancelled'
  completed: number
  total: number
  started_at: string
  finished_at: string | null
  last_error: string | null
}

export interface Dashboard {
  today: number
  week: number
  balance: number | null
  input: number
  output: number
  codex5h: number
  codex7d: number
  resetAt: string
}
