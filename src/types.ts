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
}

export interface SyncSourceConfig {
  id: string
  provider: string
  kind: UsageSourceKind
  mode: 'auto' | 'sqlite' | 'json'
  path: string
  enabled: boolean
  interval_seconds: number
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
