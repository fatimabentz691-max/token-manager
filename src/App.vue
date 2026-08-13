<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'
import { emit, listen } from '@tauri-apps/api/event'
import { currentMonitor, getCurrentWindow, LogicalSize, PhysicalPosition } from '@tauri-apps/api/window'
import { openUrl } from '@tauri-apps/plugin-opener'
import { open, save } from '@tauri-apps/plugin-dialog'
import {
  ChartNoAxesCombined,
  Command,
  FileChartColumn,
  KeyRound,
  LibraryBig,
  Maximize2,
  Minus,
  PictureInPicture2,
  Settings,
  Trophy,
  X,
} from '@lucide/vue'
import ProviderMark from './components/ProviderMark.vue'
import CodexRetentionPanel from './components/CodexRetentionPanel.vue'
import AnalyticsCharts from './components/AnalyticsCharts.vue'
import ReportCenter from './components/ReportCenter.vue'
import SettingsCenter from './components/SettingsCenter.vue'
import AnimatedNumber from './components/AnimatedNumber.vue'
import SuccessBurst from './components/SuccessBurst.vue'
import ProxyGatewayIcon from './components/ProxyGatewayIcon.vue'
import MiniProgressRing from './components/MiniProgressRing.vue'
import CommandCenter from './components/CommandCenter.vue'
import PromptCenter from './components/PromptCenter.vue'
import ArenaCenter from './components/ArenaCenter.vue'
import ThemeSwitcher from './components/ThemeSwitcher.vue'
import InteractionEffects from './components/InteractionEffects.vue'
import LiquidGlassEnvironment from './components/LiquidGlassEnvironment.vue'
import EdgeScrollRail from './components/EdgeScrollRail.vue'
import DataSourceBadge from './components/DataSourceBadge.vue'
import SyncHealthPanel from './components/SyncHealthPanel.vue'
import OnboardingFlow from './components/OnboardingFlow.vue'
import SupplementalHelp from './components/SupplementalHelp.vue'
import UpdateNotice from './components/UpdateNotice.vue'
import AgentDrilldown from './components/AgentDrilldown.vue'
import FloatingWindowV3, { type FloatingDashboardView, type FloatingMetric, type FloatingRenderStatusView } from './components/FloatingWindowV3.vue'
import tokenManagerLogo from './assets/token-manager-logo.png?url'
import codexOfficialLogo from './assets/brands/codex-official-blue.png?url'
import ccSwitchOfficialLogo from './assets/brands/cc-switch-official.png?url'
import { useChartPreferences } from './features/chartPreferences'
import { chartColorStyle, useChartColorPreferences } from './features/chartColorPreferences'
import { themeStyle, useThemePreferences } from './features/themePreferences'
import { useVisualPreferences, type GlassDistortionSettings, type GlassQuality } from './features/visualPreferences'
import { useMotionPreferences } from './features/motionPreferences'
import { buildCodexInsights, loadLocal, priceCatalog } from './features/retention'
import { currentCloudBaseUrl } from './features/cloudConfig'
import {
  defaultFloatingSizes,
  loadFloatingConfigV3,
  saveFloatingConfigV3,
  type FloatingConfigV3,
  type FloatingInteractionMode,
  type FloatingMode,
  type FloatingModuleId,
} from './features/floatingPreferences'
import { demoDashboard, providers } from './data'
import type { AgentSourceModel, LocalAgentDefinition, Provider, SyncHealth, SyncJob, SyncSourceConfig, Usage, UsageAccuracy, UsageSourceKind } from './types'
type AppPage = 'command'|'dashboard'|'prompts'|'arena'|'accounts'|'reports'|'floating'|'settings'
const page = ref<AppPage>('command')
const arenaCenter = ref<{ refresh: (force?: boolean) => Promise<void> } | null>(null)
const navItems = [
  { id: 'command', label: '控制中心', icon: Command },
  { id: 'dashboard', label: '模型用量', icon: ChartNoAxesCombined },
  { id: 'prompts', label: 'Prompt 中心', icon: LibraryBig },
  { id: 'arena', label: 'Arena 排行榜', icon: Trophy },
  { id: 'accounts', label: '账户与模型', icon: KeyRound },
  { id: 'reports', label: '报告中心', icon: FileChartColumn },
  { id: 'floating', label: '悬浮窗', icon: PictureInPicture2 },
  { id: 'settings', label: '设置', icon: Settings },
] as const
const appVersion = ref('')
const accounts = ref<Provider[]>(providers)
interface SavedAccount { id:string; provider:string; name:string; base_url:string; created_at:string }
interface ModelDashboard {
 key:string
 kind:'account_model'|'local_agent'|'local_agent_model'
 accountId:string
 sourceId:string
 agentId:string
 provider:string
 model:string
 label:string
}
interface CodexUsagePoint { at:number; tokens:number; model:string; session_id?:string; category?:string; temperature?:number|null }
interface CodexQuotaWindow { used_percent:number; remaining_percent:number; window_minutes:number; resets_at:number }
interface CodexQuotaSnapshot { five_hour:CodexQuotaWindow|null; seven_day:CodexQuotaWindow|null; plan_type?:string|null; credit_balance?:string|null; observed_at:number; source:string }
interface AccountBalance { account_id:string; provider:string; currency:string; total_balance:number; granted_balance:number; topped_up_balance:number; is_available:boolean; synced_at:string }
interface BalancePoint { at:string; total_balance:number }
interface ProxyEndpoint { account_id:string; provider:string; name:string; local_url:string; anthropic_url?:string|null; port:number }
interface ProxyJob { id:string; status:'queued'|'running'|'completed'|'error'; stage:string; account_ids:string[]; agent_ids:string[]; endpoints:ProxyEndpoint[]; detail:string; started_at:string; finished_at?:string|null }
interface ProxyTrafficEvent { account_id?:string|null; provider:string; model:string; at:string }
interface CcSwitchStatus { installed:boolean; running:boolean; local_routing:boolean; detail:string; checked_at:string; coexistence:string; conflicting_variables:string[]; safe_repair_available:boolean }
interface ClientIntegrationResult { connected:boolean; account_name:string; openai_url:string; anthropic_url?:string|null; changed:string[]; restart_required:boolean; detail:string }
interface OpenCodeLocalSyncResult { imported:number; scanned_files:number; source:string; detail:string }
interface CloudSession { email:string; base_url:string; expires_at:string }
interface CloudTransfer { id:string; link:string; expires_at:string; one_time:boolean }
interface RemoteContentItem { id:string; kind:'announcement'|'ad'; title:string; body:string; action_label:string; action_url:string; starts_at:string; ends_at?:string|null; enabled:boolean; created_at:string; updated_at:string }
interface PresenceReleaseSummary { version:string; title?:string; size_bytes:number; notes:string; highlights:string[]; fixes:string[]; published_at:string; download_url:string }
interface PresenceAckV2 { ok:boolean; server_time:string; heartbeat_interval:number; latest_release?:PresenceReleaseSummary|null }
const floatingModuleCatalog:{id:FloatingModuleId;title:string;description:string}[]=[
 {id:'globalRemaining',title:'全局总剩余额度',description:'综合显示当前个人预算剩余比例'},
 {id:'todayTokens',title:'今日总消耗 Token',description:'Codex 与 API 今日 Token 合计'},
 {id:'estimatedDuration',title:'剩余开发预估时长',description:'按本地历史速率换算完整生成与局部调试时长'},
 {id:'taskCapacity',title:'可执行任务数量',description:'换算小脚本、页面重构和 Bug 修复次数'},
 {id:'weekCost',title:'本周总消耗金额',description:'最近 7 天人民币计价成本'},
 {id:'codexQuota',title:'Codex 滚动额度',description:'5 小时余额进度和 7 天重置倒计时'},
 {id:'modelBalances',title:'各模型分项余额',description:'按已添加模型展示余额或本地用量状态'},
 {id:'warning',title:'消费预警提示条',description:'20% / 10% 两档余额提醒'},
 {id:'todayCalls',title:'今日调用总次数',description:'今日 API 调用与 Codex turn 数量'},
 {id:'cacheChart',title:'缓存命中环',description:'当前模型缓存命中率迷你环图'},
 {id:'costChart',title:'7 天消费图',description:'当前模型每日消费金额迷你柱图'},
 {id:'requestChart',title:'7 天请求图',description:'当前模型每日 API 请求次数迷你柱图'},
 {id:'tokenTrendChart',title:'7 天 Token 趋势',description:'与仪表盘真实数据一致的迷你折线图'}
]
const floatingIds=floatingModuleCatalog.map(item=>item.id)
const { mode: globalMode, setMode: setGlobalMode } = useChartPreferences()
const {
  theme: appTheme,
  themeId: appThemeId,
  liquidTone,
  liquidTransparency,
  effectiveLiquidBackgroundImage,
  liquidBackgroundVideoPath,
  syncTheme,
  syncLiquidAppearance,
} = useThemePreferences()
const { chartColorTheme, chartColorThemeId, syncChartColorTheme } = useChartColorPreferences()
const { glassQuality, glassDistortion, syncGlassQuality, syncGlassDistortion } = useVisualPreferences()
const { spotlightEnabled, syncSpotlightEnabled } = useMotionPreferences()
const appThemeStyle=computed(()=>({...themeStyle(appTheme.value),...chartColorStyle(chartColorTheme.value)}))
const appThemeDark=computed(()=>appTheme.value.material==='liquid'?liquidTone.value==='dark':appTheme.value.dark)
for(const [next,legacy] of [['token-manager-selected-dashboard','tokenlens-selected-dashboard'],['token-manager-model-dashboards-v2','tokenlens-model-dashboards-v2'],['token-manager-floating','tokenlens-floating']] as const){if(localStorage.getItem(next)===null&&localStorage.getItem(legacy)!==null)localStorage.setItem(next,localStorage.getItem(legacy)!)}
const savedAccounts = ref<SavedAccount[]>([])
const accountModels = ref<Record<string,string[]>>({})
const modelsLoading = ref(false)
const usage = ref<Usage[]>([])
const claudeUsage = ref<Usage[]>([])
const accountBalances=ref<AccountBalance[]>([])
const deepseekBalanceHistory=ref<BalancePoint[]>([])
const balanceError=ref('')
const codexSeries = ref<CodexUsagePoint[]>([])
const dashboard = ref(demoDashboard)
const now = ref(Date.now())
const lastRefreshed = ref(Date.now())
const isSyncing = ref(false)
const successBurstKey = ref(0)
let usageRefreshQueued = false
const refreshIntervalMs = 30_000
const codexTokens = ref<number|null>(null)
const codexActiveTokens = ref<number|null>(null)
const codexUpdatedAt = ref<number|null>(null)
const codexError = ref('')
const codexQuota = ref<CodexQuotaSnapshot|null>(null)
const budget5h = ref(100000)
const budget7d = ref(1000000)
const used5h = ref(0)
const used7d = ref(0)
const budgetSaved = ref('')
const proxyUpstream = ref('https://api.openai.com')
const proxyPort = ref(18765)
const proxyStatus = ref('未启动')
const proxyEndpoints=ref<ProxyEndpoint[]>([])
const allProxyStatus=ref('')
const lastProxyRequestAt=ref(0)
const lastProxyTraffic=ref<ProxyTrafficEvent|null>(null)
const autoConnectBusy=ref(false)
const autoConnectStatus=ref('')
const autoConnectedAccountId=ref(localStorage.getItem('token-manager-auto-connected-account')||'')
const unattributedSpend=ref('')
const balanceSyncing=ref(false)
const ccSwitch=ref<CcSwitchStatus>({installed:false,running:false,local_routing:false,detail:'正在检测…',checked_at:'',coexistence:'unknown',conflicting_variables:[],safe_repair_available:false})
const selectedDashboard = ref(localStorage.getItem('token-manager-selected-dashboard')||'__all__')
type TokenChartRange = 7 | 30
const tokenChartRange = ref<TokenChartRange>(localStorage.getItem('token-manager-token-chart-range')==='30'?30:7)
const storedDashboards=JSON.parse(localStorage.getItem('token-manager-model-dashboards-v2')||'[]')
const modelDashboards = ref<ModelDashboard[]>(Array.isArray(storedDashboards)?storedDashboards.map((item:Partial<ModelDashboard>)=>({
 key:item.key||'',kind:item.kind||'account_model',accountId:item.accountId||'',sourceId:item.sourceId||'',agentId:item.agentId||'',provider:item.provider||'',model:item.model||'',label:item.label||item.model||'模型仪表盘'
})).filter((item:ModelDashboard)=>item.key):[])
const dashboardModal = ref(false)
const newDashboardKey = ref('')
const dashboardPickerTab=ref<'api'|'agent'>('api')
const configuring = ref<Provider|null>(null)
const configName = ref('')
const configUrl = ref('')
const configOpenCodeVariant = ref<'go'|'zen'>('go')
const configKey = ref('')
const configStatus = ref('')
const opencodeLocalPath=ref(localStorage.getItem('token-manager-opencode-json-path')||'')
const opencodePathManual=ref(false)
const opencodeLocalMode=ref<'auto'|'sqlite'|'json'>((localStorage.getItem('token-manager-opencode-local-mode') as 'auto'|'sqlite'|'json')||'auto')
const opencodeLocalStatus=ref('自动模式优先读取 OpenCode SQLite，找不到时回退 JSON/JSONL。')
const opencodeLocalSyncing=ref(false)
const syncHealthOpen=ref(false)
const syncHealthItems=ref<SyncHealth[]>([])
const syncSources=ref<SyncSourceConfig[]>([])
const localAgents=ref<LocalAgentDefinition[]>([])
const agentSourceModels=ref<Record<string,AgentSourceModel[]>>({})
const agentActionStatus=ref<Record<string,string>>({})
const onboardingOpen=ref(false)
const backupPassword=ref('')
const backupStatus=ref('')
const cloudSession=ref<CloudSession|null>(null)
const cloudStatus=ref('')
const cloudTransferLink=ref('')
const remoteContent=ref<RemoteContentItem[]>([])
const lastContentRefresh=ref(0)
const dismissedRemoteContent=ref<Set<string>>(new Set(JSON.parse(sessionStorage.getItem('token-manager-dismissed-content')||'[]')))
const activeAnnouncement=computed(()=>remoteContent.value.find(item=>item.kind==='announcement'&&!dismissedRemoteContent.value.has(item.id)))
const activeAd=computed(()=>remoteContent.value.find(item=>item.kind==='ad'&&!dismissedRemoteContent.value.has(item.id)))
const activeAccountId = ref('')
const editingAccountId = ref('')
// 优先使用 Tauri 原生窗口标签识别悬浮窗；浏览器预览退回查询参数，便于发布前做无白屏截图验收。
const tauriRuntime=Boolean((window as Window&{__TAURI_INTERNALS__?:unknown}).__TAURI_INTERNALS__)
const floatingMode = (tauriRuntime&&getCurrentWindow().label==='floating')||new URLSearchParams(location.search).get('floating')==='1'
document.documentElement.classList.toggle('floating-window',floatingMode)
document.body.classList.toggle('floating-window',floatingMode)

/** 悬浮窗回归 v0.8.5 的主题纯色材质，不再启用 Acrylic 或液态玻璃。 */
let floatingNativeGlassRevision=0
async function syncFloatingNativeGlass() {
 const revision=++floatingNativeGlassRevision
 if(!tauriRuntime||!floatingMode)return
 const root=document.documentElement
 try{
   await getCurrentWindow().clearEffects()
   if(revision!==floatingNativeGlassRevision)return
   root.dataset.nativeGlass='off'
   root.dataset.nativeGlassTone='none'
 }catch(error){
  if(revision!==floatingNativeGlassRevision)return
  root.dataset.nativeGlass='fallback'
  root.dataset.nativeGlassError=String(error)
 }
}
watch([appThemeId,liquidTone,liquidTransparency,glassQuality,()=>glassDistortion.value.edgeBend,effectiveLiquidBackgroundImage,liquidBackgroundVideoPath],()=>{void syncFloatingNativeGlass();void configureFloatingRenderer();void reportFloatingSurfaces()},{immediate:true})
 // 主窗口只恢复上次显式启用状态；独立的 --floating 验收入口本身视为已启用。
 const floatingEnabled = ref(floatingMode||localStorage.getItem('token-manager-floating')==='1')
 const floatingConfig=ref<FloatingConfigV3>(loadFloatingConfigV3(floatingIds))
 const floatingCollapsed = ref(floatingConfig.value.mode==='capsule')
const floatingCustomizing=ref(false)
 const floatingChartMetric=ref<FloatingMetric>((localStorage.getItem('token-manager-floating-chart') as FloatingMetric)||'tokens')
const draggedFloatingModule=ref<FloatingModuleId|null>(null)
 const floatingRenderStatus=ref<FloatingRenderStatusView>({backend:'translucent',state:'stopped',fps:0,detail:'等待悬浮窗渲染器启动'})
const providerModels:Record<string,string[]>={
 '腾讯混元':['hunyuan-turbos-latest','hunyuan-pro'],'豆包（火山方舟）':['doubao-seed-1-6','doubao-1-5-pro-32k'],'文心千帆':['ernie-4.5-8k-preview','ernie-speed-128k'],'通义百炼':['qwen3-max','qwen-plus','qwen-turbo'],'智谱 AI':['glm-4.5','glm-4-flash'],'DeepSeek':['deepseek-v4-pro','deepseek-v4-flash','deepseek-chat','deepseek-reasoner'],Kimi:['kimi-k2-0711-preview','moonshot-v1-32k'],'小米 MiMo':['mimo-v2.5-pro'],'讯飞星火':['spark-x1','generalv3.5'],'MiniMax':['MiniMax-M2','MiniMax-Text-01'],'阶跃星辰':['step-2-16k','step-1-32k'],'零一万物':['yi-lightning','yi-large'],'商汤日日新':['SenseChat-5','SenseNova-V6'],'百川智能':['Baichuan4','Baichuan3-Turbo'],'OpenCode Go':['grok-4.5','gpt-5.6-luna','glm-5.2','glm-5.1','kimi-k3','kimi-k2.7-code','kimi-k2.6','mimo-v2.5','mimo-v2.5-pro','minimax-m3','minimax-m2.7','minimax-m2.5','qwen3.8-max','qwen3.7-max','qwen3.7-plus','qwen3.6-plus','deepseek-v4-pro','deepseek-v4-flash','hy3'],OpenAI:['gpt-5.4','gpt-5.4-mini','gpt-4.1'],Anthropic:['claude-opus-4-1','claude-sonnet-4'],'Google Gemini':['gemini-2.5-pro','gemini-2.5-flash'],'自定义 OpenAI 兼容':['自定义模型']
}
const providerPortals: Record<string,string> = {
 '腾讯混元':'https://console.cloud.tencent.com/hunyuan', '豆包（火山方舟）':'https://console.volcengine.com/ark', '文心千帆':'https://console.bce.baidu.com/qianfan', '通义百炼':'https://bailian.console.aliyun.com/', '智谱 AI':'https://open.bigmodel.cn/', DeepSeek:'https://platform.deepseek.com/', Kimi:'https://platform.moonshot.cn/', '小米 MiMo':'https://mimo.mi.com/', '讯飞星火':'https://console.xfyun.cn/', MiniMax:'https://platform.minimaxi.com/', '阶跃星辰':'https://platform.stepfun.com/', '零一万物':'https://platform.lingyiwanwu.com/', '商汤日日新':'https://console.sensenova.cn/', '百川智能':'https://platform.baichuan-ai.com/', 'OpenCode Go':'https://opencode.ai/go', OpenAI:'https://platform.openai.com/', Anthropic:'https://console.anthropic.com/', 'Google Gemini':'https://aistudio.google.com/', '自定义 OpenAI 兼容':'https://platform.openai.com/docs/api-reference'
}
const providerApiBases:Record<string,string>={'腾讯混元':'https://api.hunyuan.cloud.tencent.com/v1','豆包（火山方舟）':'https://ark.cn-beijing.volces.com/api/v3','文心千帆':'https://qianfan.baidubce.com/v2','通义百炼':'https://dashscope.aliyuncs.com/compatible-mode/v1','智谱 AI':'https://open.bigmodel.cn/api/paas/v4',DeepSeek:'https://api.deepseek.com',Kimi:'https://api.moonshot.cn/v1','小米 MiMo':'https://api.xiaomimimo.com/v1','讯飞星火':'https://spark-api-open.xf-yun.com/v1',MiniMax:'https://api.minimax.chat/v1','阶跃星辰':'https://api.stepfun.com/v1','零一万物':'https://api.lingyiwanwu.com/v1','商汤日日新':'https://api.sensenova.cn/compatible-mode/v1','百川智能':'https://api.baichuan-ai.com/v1','OpenCode Go':'https://opencode.ai/zen/go/v1',OpenAI:'https://api.openai.com',Anthropic:'https://api.anthropic.com','Google Gemini':'https://generativelanguage.googleapis.com','自定义 OpenAI 兼容':''}
let timer:number
let lastPresencePing=0
const presenceSessionId=crypto.randomUUID()
let presenceEventSeq=0
const presenceHealth=ref({lastSuccess:'',lastError:'',httpStatus:'',consecutiveFailures:0})
function onWindowFocus(){if(!floatingMode&&appVersion.value)void sendAppPresence('heartbeat')}
let lastBalancePoll=0
let lastUsagePoll=0
// 首屏只读取已经落库的数据，避免窗口创建时同时触发两次全量文件扫描。
// 真实代理事件仍会通过 usage-updated 在 1 秒内刷新；本地来源由 30 秒安全补扫兜底。
let lastFullSync=Date.now()
let unlistenFloating:(()=>void)|undefined
let unlistenSelection:(()=>void)|undefined
let unlistenFloatingConfig:(()=>void)|undefined
let unlistenFloatingResize:(()=>void)|undefined
let unlistenUsageUpdated:(()=>void)|undefined
let unlistenSyncStatus:(()=>void)|undefined
let unlistenSyncJob:(()=>void)|undefined
let unlistenProxyState:(()=>void)|undefined
let unlistenProxyJob:(()=>void)|undefined
let unlistenProxyTraffic:(()=>void)|undefined
let unlistenThemeSync:(()=>void)|undefined
let unlistenVisualSync:(()=>void)|undefined
let unlistenDistortionSync:(()=>void)|undefined
let unlistenSpotlightSync:(()=>void)|undefined
let unlistenLiquidAppearanceSync:(()=>void)|undefined
let unlistenChartColorSync:(()=>void)|undefined
let floatingSnapTimer:number|undefined
let floatingResizeSaveTimer:number|undefined
let floatingMoveSaveTimer:number|undefined
function broadcastTheme(event:Event){
 const id=(event as CustomEvent<string>).detail
 if(id)void emit('theme-sync',{id,source:floatingMode?'floating':'main'})
}
function broadcastVisualPreference(event:Event){
 const quality=(event as CustomEvent<GlassQuality>).detail
 if(quality)void emit('visual-preference-sync',{quality,source:floatingMode?'floating':'main'})
}
function broadcastGlassDistortion(event:Event){
 const distortion=(event as CustomEvent<GlassDistortionSettings>).detail
 if(distortion)void emit('glass-distortion-sync',{distortion,source:floatingMode?'floating':'main'})
}
function broadcastSpotlightPreference(event:Event){
 const enabled=(event as CustomEvent<boolean>).detail
 void emit('spotlight-preference-sync',{enabled,source:floatingMode?'floating':'main'})
}
function broadcastLiquidAppearance(event:Event){
 const revision=Number((event as CustomEvent<number>).detail)||Date.now()
 void emit('liquid-appearance-sync',{revision,source:floatingMode?'floating':'main'})
}
function broadcastChartColor(event:Event){
 const id=(event as CustomEvent<string>).detail
 if(id)void emit('chart-color-sync',{id,source:floatingMode?'floating':'main'})
}
onMounted(()=>{
 window.addEventListener('token-manager-theme-change',broadcastTheme)
 window.addEventListener('token-manager-visual-change',broadcastVisualPreference)
 window.addEventListener('token-manager-glass-distortion-change',broadcastGlassDistortion)
 window.addEventListener('token-manager-spotlight-change',broadcastSpotlightPreference)
 window.addEventListener('token-manager-liquid-appearance-change',broadcastLiquidAppearance)
 window.addEventListener('token-manager-chart-color-change',broadcastChartColor)
 window.addEventListener('focus',onWindowFocus)
 void listen<{id:string;source:string}>('theme-sync',event=>{
   if(event.payload?.id&&event.payload.id!==appThemeId.value)syncTheme(event.payload.id)
 }).then(unlisten=>{unlistenThemeSync=unlisten})
 void listen<{quality:GlassQuality;source:string}>('visual-preference-sync',event=>{
   if(event.payload?.quality&&event.payload.quality!==glassQuality.value)syncGlassQuality(event.payload.quality)
 }).then(unlisten=>{unlistenVisualSync=unlisten})
 void listen<{distortion:GlassDistortionSettings;source:string}>('glass-distortion-sync',event=>{
   if(event.payload?.distortion)syncGlassDistortion(event.payload.distortion)
 }).then(unlisten=>{unlistenDistortionSync=unlisten})
 void listen<{enabled:boolean;source:string}>('spotlight-preference-sync',event=>{
   if(typeof event.payload?.enabled==='boolean'&&event.payload.enabled!==spotlightEnabled.value)syncSpotlightEnabled(event.payload.enabled)
 }).then(unlisten=>{unlistenSpotlightSync=unlisten})
 void listen<{revision:number;source:string}>('liquid-appearance-sync',event=>{
   if(event.payload?.revision)syncLiquidAppearance()
 }).then(unlisten=>{unlistenLiquidAppearanceSync=unlisten})
 void listen<{id:string;source:string}>('chart-color-sync',event=>{
   if(event.payload?.id&&event.payload.id!==chartColorThemeId.value)syncChartColorTheme(event.payload.id)
 }).then(unlisten=>{unlistenChartColorSync=unlisten})
 void getVersion().then(version=>{appVersion.value=version;if(!floatingMode)void sendAppPresence('launch')})
  void listen<boolean>('floating-state',event=>{
    floatingEnabled.value=event.payload
    localStorage.setItem('token-manager-floating',event.payload?'1':'0')
    if(floatingMode){if(event.payload){void configureFloatingRenderer();void reportFloatingSurfaces()}else void refreshFloatingRenderStatus()}
 }).then(unlisten=>{unlistenFloating=unlisten})
 void listen<string>('dashboard-selection',event=>{selectedDashboard.value=event.payload}).then(unlisten=>{unlistenSelection=unlisten})
 void listen<FloatingConfigV3>('floating-config',event=>{
   const value=event.payload?.version===3?event.payload:loadFloatingConfigV3(floatingIds)
   floatingConfig.value=value
   floatingCollapsed.value=value.mode==='capsule'
   saveFloatingConfigV3(value)
   // 发送端已经完成原生尺寸调整。接收端只同步视图，禁止再次 setSize 或 emit，
   // 否则主窗口与悬浮窗会互相触发 Resized，形成无限抖动反馈回路。
   if(floatingMode){
     document.documentElement.style.setProperty('--floating-window-radius',value.mode==='capsule'?'28px':'24px')
     void invoke('set_floating_interaction_mode',{mode:'interactive'}).catch(()=>{})
     void reportFloatingSurfaces()
   }
 }).then(unlisten=>{unlistenFloatingConfig=unlisten})
 void listen<string|{provider?:string}>('usage-updated',event=>{const provider=typeof event.payload==='string'?event.payload:event.payload?.provider||'';void refreshLiveUsage(provider);void refreshSyncHealth()}).then(unlisten=>{unlistenUsageUpdated=unlisten})
 void listen<SyncHealth>('sync-status-updated',event=>{const index=syncHealthItems.value.findIndex(item=>item.source_id===event.payload.source_id);if(index>=0)syncHealthItems.value.splice(index,1,event.payload);else syncHealthItems.value.push(event.payload)}).then(unlisten=>{unlistenSyncStatus=unlisten})
 void listen<SyncJob>('sync-job-updated',event=>{
   if(event.payload.status==='completed'||event.payload.status==='cancelled'){
     void refreshLiveUsage()
     if(!floatingMode)void refreshSyncHealth()
   }
 }).then(unlisten=>{unlistenSyncJob=unlisten})
 void listen<ProxyEndpoint[]>('proxy-state',event=>{proxyEndpoints.value=event.payload;allProxyStatus.value=`数据通道已就绪 · ${event.payload.length} 个账户`}).then(unlisten=>{unlistenProxyState=unlisten})
 void listen<ProxyJob>('proxy-job-updated',event=>{
   allProxyStatus.value=event.payload.detail
   if(event.payload.status==='completed'){
     proxyEndpoints.value=event.payload.endpoints
     if(event.payload.agent_ids.length&&event.payload.stage==='ready'&&event.payload.endpoints[0]){
       autoConnectedAccountId.value=event.payload.endpoints[0].account_id
       localStorage.setItem('token-manager-auto-connected-account',autoConnectedAccountId.value)
     }
   }
 }).then(unlisten=>{unlistenProxyJob=unlisten})
 void listen<ProxyTrafficEvent>('proxy-request-started',event=>{
   lastProxyTraffic.value=event.payload
   lastProxyRequestAt.value=Date.parse(event.payload.at)||Date.now()
   allProxyStatus.value=`正在接收 ${event.payload.provider} · ${event.payload.model}`
 }).then(unlisten=>{unlistenProxyTraffic=unlisten})
  timer=window.setInterval(()=>{
    now.value=Date.now()
    if (!isSyncing.value && now.value-lastFullSync >= refreshIntervalMs) void syncCurrentView(true)
    if (proxyEndpoints.value.length && !isSyncing.value && now.value-lastUsagePoll >= 5_000) {
      lastUsagePoll=now.value
      void refreshLiveUsage(lastProxyTraffic.value?.provider||'')
    }
    if (!floatingMode && !balanceSyncing.value && savedAccounts.value.some(item=>item.provider==='DeepSeek') && now.value-lastBalancePoll >= 10_000) {
      lastBalancePoll=now.value
      void refreshBalanceData(true)
    }
    if (!floatingMode && now.value-lastContentRefresh.value >= 60_000) void refreshRemoteContent()
    if (!floatingMode && appVersion.value && now.value-lastPresencePing >= 60_000) void sendAppPresence('heartbeat')
  },1000)
  onboardingOpen.value=!floatingMode
   &&localStorage.getItem('token-manager-onboarding-v011')!=='done'
   &&localStorage.getItem('token-manager-onboarding-v0113')!=='done'
  void initializeMonitoring()
  if(!floatingMode)void refreshSyncHealth()
 if(!floatingMode) void refreshRemoteContent()
 void invoke<CloudSession|null>('cloud_session').then(value=>{cloudSession.value=value}).catch(()=>{})
 if(floatingMode) void initializeFloatingWindow()
 if(!floatingMode&&floatingEnabled.value) void invoke('set_floating_window',{enabled:true})
})
onUnmounted(()=>{clearInterval(timer);if(floatingSnapTimer)clearTimeout(floatingSnapTimer);if(floatingResizeSaveTimer)clearTimeout(floatingResizeSaveTimer);if(floatingMoveSaveTimer)clearTimeout(floatingMoveSaveTimer);window.removeEventListener('focus',onWindowFocus);window.removeEventListener('token-manager-theme-change',broadcastTheme);window.removeEventListener('token-manager-visual-change',broadcastVisualPreference);window.removeEventListener('token-manager-glass-distortion-change',broadcastGlassDistortion);window.removeEventListener('token-manager-spotlight-change',broadcastSpotlightPreference);window.removeEventListener('token-manager-liquid-appearance-change',broadcastLiquidAppearance);window.removeEventListener('token-manager-chart-color-change',broadcastChartColor);unlistenFloating?.();unlistenSelection?.();unlistenFloatingConfig?.();unlistenFloatingResize?.();unlistenUsageUpdated?.();unlistenSyncStatus?.();unlistenSyncJob?.();unlistenProxyState?.();unlistenProxyJob?.();unlistenProxyTraffic?.();unlistenThemeSync?.();unlistenVisualSync?.();unlistenDistortionSync?.();unlistenSpotlightSync?.();unlistenLiquidAppearanceSync?.();unlistenChartColorSync?.()})
const resetText=computed(()=>{const seconds=Math.max(0,Math.floor((new Date(dashboard.value.resetAt).getTime()-now.value)/1000)); return `${Math.floor(seconds/60)}分${String(seconds%60).padStart(2,'0')}秒`})
const totalCost=computed(()=>usage.value.reduce((s,x)=>s+x.cost,0))
const selectedDashboardDef=computed(()=>modelDashboards.value.find(item=>item.key===selectedDashboard.value))
const combinedUsage=computed(()=>[...usage.value,...claudeUsage.value])
const proxyRecentlyActive=computed(()=>Boolean(proxyEndpoints.value.length&&lastProxyRequestAt.value&&now.value-lastProxyRequestAt.value<90_000))
const clientIntegrationConnected=computed(()=>Boolean(autoConnectedAccountId.value&&proxyEndpoints.value.some(endpoint=>endpoint.account_id===autoConnectedAccountId.value)))
const proxyConnectionLabel=computed(()=>proxyRecentlyActive.value?'正在接收':clientIntegrationConnected.value?'已自动接入':proxyEndpoints.value.length?'可自动接入':'未启用')
const proxyConnectionDetail=computed(()=>proxyRecentlyActive.value&&lastProxyTraffic.value
 ? `${lastProxyTraffic.value.provider} · ${lastProxyTraffic.value.model} · 请求已进入本机代理`
 : clientIntegrationConnected.value
 ? autoConnectStatus.value||'所选 Agent 已指向本机代理；没有改写其他工具的全局连接配置。'
 : proxyEndpoints.value.length
 ? '代理端口已经监听，点击“自动接入”即可配置常见 Code、SDK 与 Claude Code。'
 : '开启后会生成本机代理地址并自动接入常见开发工具。')
const commandActivities=computed(()=>combinedUsage.value
 .map(item=>({id:item.id,provider:item.provider,model:item.model,tokens:item.input+item.output,cost:item.cost,at:item.at}))
 .sort((a,b)=>new Date(b.at).getTime()-new Date(a.at).getTime()))
const dashboardUsage=computed(()=>{
 if(selectedDashboard.value==='__all__')return combinedUsage.value
 if(selectedDashboard.value==='__codex__')return[]
 if(selectedDashboard.value==='__claude__')return claudeUsage.value
 const target=selectedDashboardDef.value
 if(!target)return[]
 if(target.kind==='local_agent')return usage.value.filter(item=>item.source_id===target.sourceId)
 if(target.kind==='local_agent_model')return usage.value.filter(item=>item.source_id===target.sourceId&&item.model===target.model)
 return usage.value.filter(item=>item.account_id===target.accountId&&item.model===target.model)
})
const dashboardSourceMeta=computed<{kind:UsageSourceKind;accuracy:UsageAccuracy;collectedAt:string}>(()=>{
 if(selectedDashboard.value==='__codex__')return codexQuota.value?.five_hour||codexQuota.value?.seven_day
  ?{kind:'local_log',accuracy:'observed',collectedAt:codexQuota.value?.observed_at?new Date(codexQuota.value.observed_at*1000).toISOString():''}
  :{kind:'estimate',accuracy:'estimated',collectedAt:codexUpdatedAt.value?new Date(codexUpdatedAt.value).toISOString():''}
 if(selectedDashboard.value==='__claude__')return{kind:'local_log',accuracy:'observed',collectedAt:claudeUsage.value[0]?.at||''}
 const latest=dashboardUsage.value[0]||combinedUsage.value[0]
 return{kind:latest?.source_kind||'local_log',accuracy:latest?.accuracy||'observed',collectedAt:latest?.collected_at||latest?.at||''}
})
const dashboardTitle=computed(()=>selectedDashboard.value==='__all__'?'全部模型':selectedDashboard.value==='__codex__'?'Codex 专属':selectedDashboard.value==='__claude__'?'Claude Code 专属':selectedDashboardDef.value?.label||'模型仪表盘')
const dashboardAdvice=computed(()=>selectedDashboard.value==='__codex__'
 ? ['数据来自 Codex 本地状态库和日志，不需要 API Key。','优先显示 Codex 客户端写入的真实 rate_limits 百分比与重置时间；客户端未下发的窗口才使用个人预算估算。']
 : selectedDashboard.value==='__claude__'
 ? ['数据来自 Claude Code 本地 JSONL 的 usage 元数据，不读取提示词或回复正文。','Token、请求和缓存命中会随本地日志每 30 秒刷新；账户余额仍取决于 Claude 官方账户权限。']
 : selectedDashboard.value==='__all__'
 ? ['这里汇总所有已接入模型，可通过上方仪表盘标签查看单个模型。','独立模型视图中的数字、图表、调用记录和提示均不会混入其他模型。']
 : selectedDashboardDef.value?.kind==='local_agent'||selectedDashboardDef.value?.kind==='local_agent_model'
 ? [`数据只来自 ${selectedDashboardDef.value.label} 对应的本地数据源，不读取提示词、回复正文、代码或认证信息。`,`当前页面按 source_id${selectedDashboardDef.value.kind==='local_agent_model'?' 与模型':' '}隔离；无真实记录时不会生成模拟图表。`]
 : selectedDashboardDef.value?.provider==='OpenCode Go'
  ? ['支持 API Key 本机代理和 OpenCode 本地 JSON 两种监测方式；JSON 只提取模型、时间、Token、缓存和成本，不读取会话正文。','官方公开接口未提供套餐剩余百分比；额度卡仅展示本机已观测消耗，官方精确值请以 OpenCode 控制台为准。']
 : dashboardUsage.value.length
   ? [`${selectedDashboardDef.value?.model} 今日已记录 ${dashboardToday.value.length} 次调用，成本 ¥${dashboardCost.value.toFixed(2)}。`,`该页仅分析 ${selectedDashboardDef.value?.provider} / ${selectedDashboardDef.value?.model} 的本地真实记录。`]
   : [`${selectedDashboardDef.value?.model} 暂无真实调用记录。`,`通过已配置账户同步或本地代理产生调用后，此处将只显示该模型数据。`])
const dashboardToday=computed(()=>dashboardUsage.value.filter(x=>new Date(x.at).toDateString()===new Date().toDateString()))
const usageRequestCount=(item:Usage)=>item.request_count??1
const dashboardTodayCalls=computed(()=>dashboardToday.value.reduce((sum,item)=>sum+usageRequestCount(item),0))
const dashboardInput=computed(()=>dashboardToday.value.reduce((sum,x)=>sum+x.input,0))
const dashboardOutput=computed(()=>dashboardToday.value.reduce((sum,x)=>sum+x.output,0))
const dashboardCached=computed(()=>dashboardToday.value.reduce((sum,x)=>sum+x.cached,0))
const dashboardCost=computed(()=>dashboardToday.value.reduce((sum,x)=>sum+x.cost,0))
const dashboardFailedCalls=computed(()=>dashboardToday.value.filter(x=>x.task==='失败').length)
const dashboardCacheRate=computed(()=>dashboardInput.value?Math.round(dashboardCached.value/dashboardInput.value*100):0)
const dashboardWeekTotal=computed(()=>dashboardUsage.value.filter(item=>new Date(item.at).getTime()>=now.value-7*86_400_000).reduce((sum,item)=>sum+item.input+item.output,0))
/** 每个平台使用稳定的数据色；品牌 Logo 保持官网原始彩色版本。 */
const providerColors:Record<string,string>={
 'Codex':'#007AFF','Claude Code':'#d97757','OpenAI':'#10a37f','Anthropic':'#d97757','Google Gemini':'#4285f4',
 'DeepSeek':'#4d6bfe','腾讯混元':'#00a870','豆包（火山方舟）':'#2e90fa','文心千帆':'#2563eb',
 '通义百炼':'#6c5ce7','智谱 AI':'#245bdb','Kimi':'#7b61ff','小米 MiMo':'#ff6900',
 '讯飞星火':'#f59e0b','MiniMax':'#e5484d','阶跃星辰':'#0ea5e9','零一万物':'#1f9d74',
 '商汤日日新':'#df3b3b','百川智能':'#8b5cf6','OpenCode Go':'#131010','自定义 OpenAI 兼容':'#52525b'
}
const fallbackChartColors=['#0f766e','#c2410c','#0369a1','#7e22ce','#be123c','#3f6212','#4338ca','#a16207']
function providerColor(provider:string){let hash=0;for(const char of provider)hash=(hash*31+char.charCodeAt(0))>>>0;const colors=chartColorTheme.value.series;return colors[hash%colors.length]||providerColors[provider]||fallbackChartColors[hash%fallbackChartColors.length]}
const providerBreakdown=computed(()=>{const totals=new Map<string,{tokens:number;calls:number;cost:number}>();for(const item of combinedUsage.value){const row=totals.get(item.provider)||{tokens:0,calls:0,cost:0};row.tokens+=item.input+item.output;row.calls+=usageRequestCount(item);row.cost+=item.cost;totals.set(item.provider,row)}const rows=[...totals.entries()].map(([provider,value])=>({provider,...value})).sort((a,b)=>b.tokens-a.tokens);const total=Math.max(1,rows.reduce((sum,row)=>sum+row.tokens,0));return rows.map(row=>({...row,percent:row.tokens/total*100}))})
const monthlyProviderCosts=computed(()=>providerBreakdown.value.filter(item=>item.cost>0).sort((a,b)=>b.cost-a.cost))
const monthlyCostTotal=computed(()=>monthlyProviderCosts.value.reduce((sum,item)=>sum+item.cost,0))
const monthlyCostGradient=computed(()=>{let cursor=0;const stops=monthlyProviderCosts.value.map(item=>{const next=cursor+(monthlyCostTotal.value?item.cost/monthlyCostTotal.value*100:0);const stop=`${providerColor(item.provider)} ${cursor}% ${next}%`;cursor=next;return stop});return stops.length?`conic-gradient(${stops.join(',')})`:'#f2f2f7'})
const monthlyCostTrend=computed(()=>{const daily=Array.from({length:30},(_,i)=>{const date=new Date(now.value-(29-i)*86_400_000);return{key:localDayKey(date),cost:0}});for(const item of usage.value){const row=daily.find(day=>day.key===localDayKey(new Date(item.at)));if(row)row.cost+=item.cost}const max=Math.max(.01,...daily.map(day=>day.cost));return daily.map((day,index)=>`${index/(daily.length-1)*600},${135-day.cost/max*110}`).join(' ')})
const balanceStatusText=computed(()=>isDeepSeekDashboard.value?(dashboardBalance.value?`官方余额 · ${dashboardBalance.value.currency}`:balanceError.value||'等待官方余额接口同步'):isOpenCodeGoDashboard.value?'Go 套餐精确额度请查看 OpenCode 控制台':isOpenCodeZenDashboard.value?'Zen 网关仅显示代理实测用量，不套用 Go 套餐额度':selectedDashboardDef.value?.provider==='OpenAI'||selectedDashboardDef.value?.provider==='Anthropic'?'模型 API Key 不含组织账单权限':selectedDashboardDef.value?.provider==='Google Gemini'?'需单独配置 Google Cloud Billing IAM':selectedDashboardDef.value?.provider==='自定义 OpenAI 兼容'?'由中转服务决定，当前使用代理统计':'需厂商云账户签名或账单导入')
const codexTodayTokens=computed(()=>codexSeries.value.filter(item=>localDayKey(new Date(item.at*1000))===localDayKey(new Date(now.value))).reduce((sum,item)=>sum+item.tokens,0))
const activeModelCount=computed(()=>new Set(combinedUsage.value.filter(item=>localDayKey(new Date(item.at))===localDayKey(new Date(now.value))).map(item=>`${item.provider}::${item.model}`)).size+(codexTodayTokens.value?1:0))
const todayUsedProviders=computed(()=>{const rows=new Map<string,{provider:string,tokens:number,calls:number}>();for(const item of combinedUsage.value.filter(item=>localDayKey(new Date(item.at))===localDayKey(new Date(now.value)))){const row=rows.get(item.provider)||{provider:item.provider,tokens:0,calls:0};row.tokens+=item.input+item.output;row.calls+=usageRequestCount(item);rows.set(item.provider,row)}if(codexTodayTokens.value)rows.set('Codex',{provider:'Codex',tokens:codexTodayTokens.value,calls:todayCodexTurns.value});return[...rows.values()].sort((a,b)=>b.tokens-a.tokens).slice(0,8)})
const dashboardAccount=computed(()=>selectedDashboardDef.value?.kind==='account_model'?savedAccounts.value.find(item=>item.id===selectedDashboardDef.value?.accountId):undefined)
const isDeepSeekDashboard=computed(()=>selectedDashboardDef.value?.provider==='DeepSeek')
const selectedDashboardAccount=computed(()=>savedAccounts.value.find(account=>account.id===selectedDashboardDef.value?.accountId))
const isOpenCodeGoDashboard=computed(()=>selectedDashboardDef.value?.provider==='OpenCode Go'&&Boolean(selectedDashboardAccount.value?.base_url.includes('/zen/go/')))
const isOpenCodeZenDashboard=computed(()=>selectedDashboardDef.value?.provider==='OpenCode Go'&&!isOpenCodeGoDashboard.value)
const isClaudeDashboard=computed(()=>selectedDashboard.value==='__claude__')
const isApiDashboard=computed(()=>!['__all__','__codex__','__claude__'].includes(selectedDashboard.value))
const isLocalAgentDashboard=computed(()=>selectedDashboardDef.value?.kind==='local_agent'||selectedDashboardDef.value?.kind==='local_agent_model')
const selectedAgentSource=computed(()=>syncSources.value.find(item=>item.id===selectedDashboardDef.value?.sourceId))
const dashboardBalance=computed(()=>accountBalances.value.find(item=>item.account_id===dashboardAccount.value?.id&&item.currency==='CNY'))
const balanceChange=computed(()=>deepseekBalanceHistory.value.length>1?deepseekBalanceHistory.value[deepseekBalanceHistory.value.length-1].total_balance-deepseekBalanceHistory.value[0].total_balance:0)
const deepseekBalancePoints=computed(()=>{const rows=deepseekBalanceHistory.value;if(!rows.length)return'';const min=Math.min(...rows.map(x=>x.total_balance));const max=Math.max(...rows.map(x=>x.total_balance));return rows.map((item,index)=>`${rows.length===1?300:index/(rows.length-1)*600},${140-(item.total_balance-min)/Math.max(.01,max-min)*115}`).join(' ')})
const dashboardLastUsage=computed(()=>dashboardUsage.value[0])
const opencodeGoDiscoveredModels=computed(()=>dashboardAccount.value?accountModels.value[dashboardAccount.value.id]||[]:[])
/** OpenCode Go 套餐以美元价值限制；本机账单按固定参考汇率 7.2 换算回美元，仅作已观测用量参考。 */
const opencodeGoQuotaWindows=computed(()=>{
 const providerUsage=usage.value.filter(item=>item.provider==='OpenCode Go')
 const build=(label:string,hours:number,limit:number)=>{
  const since=now.value-hours*3_600_000
  const observedUsd=providerUsage.filter(item=>new Date(item.at).getTime()>=since).reduce((sum,item)=>sum+item.cost/7.2,0)
  const usedPercent=Math.min(100,observedUsd/limit*100)
  return{label,limit,observedUsd,usedPercent,remainingPercent:Math.max(0,100-usedPercent)}
 }
 return[build('5 小时滚动',5,12),build('近 7 天',24*7,30),build('近 30 天',24*30,60)]
})
const availableModels=computed(()=>{
 const configured=savedAccounts.value.flatMap(account=>(accountModels.value[account.id]?.length?accountModels.value[account.id]:providerModels[account.provider]||['自定义模型']).map(model=>({key:`${account.id}::${model}`,kind:'account_model' as const,accountId:account.id,sourceId:'',agentId:'',provider:account.provider,model,label:`${account.name} · ${model}`})))
 const existing=new Set(configured.map(item=>`${item.provider}::${item.model}`))
 const local=[...new Set(usage.value.filter(item=>item.provider==='OpenCode Go'&&item.source_id==='opencode-local').map(item=>item.model))].filter(model=>!existing.has(`OpenCode Go::${model}`)).map(model=>({key:`opencode-local::${model}`,kind:'local_agent_model' as const,accountId:'',sourceId:'opencode-local',agentId:'opencode',provider:'OpenCode Go',model,label:`OpenCode 本地 · ${model}`}))
 return[...configured,...local]
})
const availableAgentDashboards=computed<ModelDashboard[]>(()=>localAgents.value.flatMap(agent=>{
 const source=syncSources.value.find(item=>item.agent_id===agent.id)
 if(!source||(!agent.detected&&!source.detected))return[]
 const overview:ModelDashboard={key:`agent:${source.id}:*`,kind:'local_agent',accountId:'',sourceId:source.id,agentId:agent.id,provider:agent.provider,model:'全部模型',label:`${agent.name} · 本地总览`}
 const models=(agentSourceModels.value[source.id]||[]).map(item=>({key:`agent:${source.id}:${item.model}`,kind:'local_agent_model' as const,accountId:'',sourceId:source.id,agentId:agent.id,provider:agent.provider,model:item.model,label:`${agent.name} · ${item.model}`}))
 return[overview,...models]
}))
const refreshedText=computed(()=>new Intl.DateTimeFormat('zh-CN',{hour:'2-digit',minute:'2-digit',second:'2-digit'}).format(lastRefreshed.value))
const nextRefreshText=computed(()=>{
 const seconds=Math.max(0,Math.ceil((refreshIntervalMs-(now.value-(lastFullSync||lastRefreshed.value)))/1000))
 return `下次自动刷新 ${seconds} 秒后`
})
const remaining5h=computed(()=>Math.round(codexQuota.value?.five_hour?.remaining_percent??Math.max(0,100-used5h.value/budget5h.value*100)))
const remaining7d=computed(()=>Math.round(codexQuota.value?.seven_day?.remaining_percent??Math.max(0,100-used7d.value/budget7d.value*100)))
const codex5hIsReported=computed(()=>Boolean(codexQuota.value?.five_hour))
const codex7dIsReported=computed(()=>Boolean(codexQuota.value?.seven_day))
const codexQuotaObservedText=computed(()=>codexQuota.value?.observed_at?new Date(codexQuota.value.observed_at*1000).toLocaleString('zh-CN'):'尚未读取')
const monthlyBudget=computed(()=>loadLocal('token-manager-monthly-budget',200))
const codexInsights=computed(()=>buildCodexInsights(codexSeries.value,used7d.value,budget7d.value,monthlyBudget.value))
const used5hPercent=computed(()=>Math.min(100,100-remaining5h.value))
const used7dPercent=computed(()=>Math.min(100,100-remaining7d.value))
function localDayKey(value:Date){return `${value.getFullYear()}-${String(value.getMonth()+1).padStart(2,'0')}-${String(value.getDate()).padStart(2,'0')}`}
function setTokenChartRange(range:TokenChartRange){tokenChartRange.value=range;localStorage.setItem('token-manager-token-chart-range',String(range))}
const allTokenChart=computed(()=>{
 const range=tokenChartRange.value
 const days=Array.from({length:range},(_,index)=>{const date=new Date(now.value);date.setHours(0,0,0,0);date.setDate(date.getDate()-(range-1-index));return{key:localDayKey(date),day:date.toLocaleDateString('zh-CN',{month:'2-digit',day:'2-digit'}),providers:new Map<string,number>(),total:0}})
 const byDay=new Map(days.map(day=>[day.key,day]))
 const add=(key:string,provider:string,tokens:number)=>{if(!tokens)return;const day=byDay.get(key);if(!day)return;day.providers.set(provider,(day.providers.get(provider)||0)+tokens);day.total+=tokens}
 for(const item of combinedUsage.value)add(localDayKey(new Date(item.at)),item.provider,item.input+item.output)
 for(const point of codexSeries.value)add(localDayKey(new Date(point.at*1000)),'Codex',point.tokens)
 const providerTotals=new Map<string,number>()
 for(const day of days)for(const [provider,tokens] of day.providers)providerTotals.set(provider,(providerTotals.get(provider)||0)+tokens)
 const order=[...providerTotals.entries()].sort((a,b)=>b[1]-a[1]).map(([provider])=>provider)
 const max=Math.max(1,...days.map(day=>day.total))
 const rows=days.map((day,index)=>({
  ...day,
  label:range===7||index===0||index===range-1||index%5===4?day.day:'',
  height:day.total?Math.max(2,day.total/max*100):0,
  segments:order.map(provider=>({provider,tokens:day.providers.get(provider)||0,color:providerColor(provider),height:(day.providers.get(provider)||0)/max*100})).filter(item=>item.tokens>0)
 }))
 return{rows,legend:order.map(provider=>({provider,tokens:providerTotals.get(provider)||0,color:providerColor(provider)})),total:rows.reduce((sum,row)=>sum+row.total,0),peak:rows.reduce((peak,row)=>row.total>peak.total?row:peak,rows[0]),activeProviders:order.length}
})
const tokenChartDailyAverage=computed(()=>Math.round(allTokenChart.value.total/tokenChartRange.value))
const chartBars=computed(()=>{
 const daily=new Map<string,{input:number;output:number;cached:number;codex:number;cost:number;calls:number}>()
 for(let i=6;i>=0;i--){const date=new Date(now.value);date.setDate(date.getDate()-i);daily.set(localDayKey(date),{input:0,output:0,cached:0,codex:0,cost:0,calls:0})}
 for(const item of dashboardUsage.value){const bucket=daily.get(localDayKey(new Date(item.at)));if(bucket){bucket.input+=item.input;bucket.output+=item.output;bucket.cached+=item.cached;bucket.cost+=item.cost;bucket.calls+=usageRequestCount(item)}}
 if(selectedDashboard.value==='__all__'||selectedDashboard.value==='__codex__')for(const point of codexSeries.value){const bucket=daily.get(localDayKey(new Date(point.at*1000)));if(bucket)bucket.codex+=point.tokens}
 const values=[...daily.entries()].map(([day,value])=>({day,label:day.slice(5),...value,total:value.input+value.output+value.codex}))
 const max=Math.max(1,...values.map(item=>item.total));return values.map(item=>({...item,height:item.total?Math.max(6,item.total/max*100):2}))
})
const modelChartSeries=computed(()=>{let cumulative=0;const source=chartBars.value.map(item=>({...item,cumulative:cumulative+=item.input+item.output}));const costMax=Math.max(1,...source.map(item=>item.cost));const tokenMax=Math.max(1,...source.map(item=>item.cumulative));const callsMax=Math.max(1,...source.map(item=>item.calls));const cacheMax=Math.max(1,...source.map(item=>item.cached));return source.map(item=>({...item,costHeight:item.cost?Math.max(7,item.cost/costMax*100):2,tokenHeight:item.cumulative?Math.max(7,item.cumulative/tokenMax*100):2,callsHeight:item.calls?Math.max(7,item.calls/callsMax*100):2,cacheHeight:item.cached?Math.max(7,item.cached/cacheMax*100):2}))})
const floatingTokenPlot=computed(()=>{const values=modelChartSeries.value.map(item=>item.input+item.output);const max=Math.max(1,...values);return modelChartSeries.value.map((item,index)=>({...item,value:values[index],x:values.length<2?50:index/(values.length-1)*100,y:90-values[index]/max*76}))})
const floatingTokenPoints=computed(()=>floatingTokenPlot.value.map(point=>`${point.x/100*160},${point.y/100*56}`).join(' '))
const floatingTokenTotal7d=computed(()=>floatingTokenPlot.value.reduce((sum,point)=>sum+point.value,0))
const floatingBars=computed(()=>{
 const hour=Math.floor(now.value/3_600_000)*3_600_000
 const buckets=Array.from({length:5},(_,index)=>{const at=hour-(4-index)*3_600_000;return{at,label:new Date(at).toLocaleTimeString('zh-CN',{hour:'2-digit',minute:'2-digit'}),tokens:0}})
 if(selectedDashboard.value==='__all__'||selectedDashboard.value==='__codex__'){for(const point of codexSeries.value){const pointHour=Math.floor(point.at*1000/3_600_000)*3_600_000;const bucket=buckets.find(item=>item.at===pointHour);if(bucket)bucket.tokens+=point.tokens}}
 else{for(const item of dashboardUsage.value){const pointHour=Math.floor(new Date(item.at).getTime()/3_600_000)*3_600_000;const bucket=buckets.find(value=>value.at===pointHour);if(bucket)bucket.tokens+=item.input+item.output}}
 const max=Math.max(1,...buckets.map(item=>item.tokens));return buckets.map(item=>({...item,height:item.tokens?Math.max(8,item.tokens/max*100):3}))
})
const balanceLevel=computed(()=>{const remaining=Math.min(remaining5h.value,remaining7d.value);return remaining<=10?'critical':remaining<=20?'warning':'normal'})
const balanceNotice=computed(()=>codexTokens.value===null?'尚未读取 Codex 日志':balanceLevel.value==='critical'?'余额低于 10%，请尽快切换任务':balanceLevel.value==='warning'?'余额低于 20%，请控制高消耗任务':'余额充足，监控正常')
const floatingModelNotice=computed(()=>selectedDashboard.value==='__all__'||selectedDashboard.value==='__codex__'?balanceNotice.value:isDeepSeekDashboard.value&&dashboardBalance.value?`DeepSeek 官方余额 ¥${dashboardBalance.value.total_balance.toFixed(2)} · ${dashboardBalance.value.is_available?'可正常调用':'余额不可用'}`:dashboardUsage.value.length?`已记录 ${dashboardUsage.value.length} 次 ${selectedDashboardDef.value?.model} 调用`:`${selectedDashboardDef.value?.model} 暂无调用数据`)
const currentTimeText=computed(()=>new Intl.DateTimeFormat('zh-CN',{year:'numeric',month:'2-digit',day:'2-digit',hour:'2-digit',minute:'2-digit',second:'2-digit',hour12:false}).format(now.value))
const globalRemaining=computed(()=>Math.min(remaining5h.value,remaining7d.value))
const todayTotalTokens=computed(()=>codexTodayTokens.value+combinedUsage.value.filter(item=>localDayKey(new Date(item.at))===localDayKey(new Date(now.value))).reduce((sum,item)=>sum+item.input+item.output,0))
const weekCost=computed(()=>{const since=now.value-7*86_400_000;return usage.value.filter(item=>new Date(item.at).getTime()>=since).reduce((sum,item)=>sum+item.cost,0)})
const todayCodexTurns=computed(()=>codexSeries.value.filter(item=>localDayKey(new Date(item.at*1000))===localDayKey(new Date(now.value))).length)
const todayCalls=computed(()=>todayCodexTurns.value+combinedUsage.value.filter(item=>localDayKey(new Date(item.at))===localDayKey(new Date(now.value))).reduce((sum,item)=>sum+usageRequestCount(item),0))
const floatingTodayTokens=computed(()=>selectedDashboard.value==='__all__'?todayTotalTokens.value:selectedDashboard.value==='__codex__'?codexTodayTokens.value:dashboardInput.value+dashboardOutput.value)
const floatingWeekCost=computed(()=>selectedDashboard.value==='__all__'?weekCost.value:dashboardUsage.value.filter(item=>new Date(item.at).getTime()>=now.value-7*86_400_000).reduce((sum,item)=>sum+item.cost,0))
const floatingTodayCalls=computed(()=>selectedDashboard.value==='__all__'?todayCalls.value:selectedDashboard.value==='__codex__'?todayCodexTurns.value:dashboardTodayCalls.value)
function countdownTo(timestamp:number){let seconds=Math.max(0,Math.floor(timestamp-now.value/1000));const days=Math.floor(seconds/86_400);seconds%=86_400;const hours=Math.floor(seconds/3600);seconds%=3600;const minutes=Math.floor(seconds/60);const tail=`${String(seconds%60).padStart(2,'0')} 秒`;return days?`${days} 天 ${hours} 小时 ${minutes} 分 ${tail}`:`${hours} 小时 ${minutes} 分 ${tail}`}
const codex5hResetText=computed(()=>codexQuota.value?.five_hour?.resets_at?countdownTo(codexQuota.value.five_hour.resets_at):'等待客户端下发')
const codex7dResetText=computed(()=>{if(codexQuota.value?.seven_day?.resets_at)return countdownTo(codexQuota.value.seven_day.resets_at);const cutoff=now.value/1000-7*86_400;const oldest=codexSeries.value.filter(item=>item.at>=cutoff).sort((a,b)=>a.at-b.at)[0];return oldest?countdownTo(oldest.at+7*86_400):'暂无滚动记录'})
const modelBalanceRows=computed(()=>modelDashboards.value.map(model=>{const balance=model.kind==='account_model'?accountBalances.value.find(item=>item.account_id===model.accountId&&item.currency==='CNY'):null;const rows=model.kind==='local_agent'?usage.value.filter(item=>item.source_id===model.sourceId):model.kind==='local_agent_model'?usage.value.filter(item=>item.source_id===model.sourceId&&item.model===model.model):usage.value.filter(item=>item.account_id===model.accountId&&item.model===model.model);const tokens=rows.reduce((sum,item)=>sum+item.input+item.output,0);return{key:model.key,label:model.label,value:balance?`¥${balance.total_balance.toFixed(2)}`:rows.length?`${tokens.toLocaleString()} tokens`:'暂无调用'}}).slice(0,4))
type FloatingDashboardKind='all'|'codex'|'claude'|'model'|'agent'
interface FloatingDashboardRow {key:string;title:string;subtitle:string;provider:string;kind:FloatingDashboardKind;todayTokens:number;calls:number;ringPercent:number;ringLabel:string}
function compactFloatingNumber(value:number){if(value>=1_000_000)return`${(value/1_000_000).toFixed(value>=10_000_000?0:1)}M`;if(value>=1_000)return`${(value/1_000).toFixed(value>=10_000?0:1)}K`;return value.toLocaleString()}
function floatingUsageFor(key:string){if(key==='__all__')return combinedUsage.value;if(key==='__claude__')return claudeUsage.value;if(key==='__codex__')return[];const target=modelDashboards.value.find(item=>item.key===key);if(!target)return[];if(target.kind==='local_agent')return usage.value.filter(item=>item.source_id===target.sourceId);if(target.kind==='local_agent_model')return usage.value.filter(item=>item.source_id===target.sourceId&&item.model===target.model);return usage.value.filter(item=>item.account_id===target.accountId&&item.model===target.model)}
function todayTokenFor(key:string){if(key==='__all__')return todayTotalTokens.value;if(key==='__codex__')return codexTodayTokens.value;return floatingUsageFor(key).filter(item=>localDayKey(new Date(item.at))===localDayKey(new Date(now.value))).reduce((sum,item)=>sum+item.input+item.output,0)}
const floatingDashboardRows=computed<FloatingDashboardRow[]>(()=>{
 const definitions=[{key:'__all__',title:'全部模型',provider:'Token Manager',kind:'all' as const},{key:'__codex__',title:'Codex 专属',provider:'Codex',kind:'codex' as const},{key:'__claude__',title:'Claude Code 专属',provider:'Claude Code',kind:'claude' as const},...modelDashboards.value.map(item=>({key:item.key,title:item.kind==='local_agent'?item.label:item.model,provider:item.provider,kind:(item.kind==='account_model'?'model':'agent') as 'model'|'agent'}))]
 const tokens=definitions.map(item=>todayTokenFor(item.key));const max=Math.max(1,...tokens.filter((_,index)=>definitions[index].key!=='__codex__'))
 return definitions.map((item,index)=>{const source=floatingUsageFor(item.key);const todaySource=source.filter(event=>localDayKey(new Date(event.at))===localDayKey(new Date(now.value)));const calls=item.key==='__all__'?todayCalls.value:item.key==='__codex__'?todayCodexTurns.value:todaySource.reduce((sum,event)=>sum+usageRequestCount(event),0);const ringPercent=item.key==='__codex__'?remaining7d.value:Math.min(100,Math.round(tokens[index]/max*100));const subtitle=item.key==='__codex__'?`${codex7dIsReported.value?'客户端额度':'个人估算'} · 7 天`:item.key==='__claude__'?'Claude Code 本地日志':item.key==='__all__'?`${activeModelCount.value} 个今日活跃模型`:`${item.provider} · ${calls} 次`;return{...item,todayTokens:tokens[index],calls,ringPercent,ringLabel:`${ringPercent}%`,subtitle}})
})
function floatingDailyFor(key:string){
 const days=Array.from({length:7},(_,index)=>{const date=new Date(now.value);date.setHours(0,0,0,0);date.setDate(date.getDate()-(6-index));return{key:localDayKey(date),label:date.toLocaleDateString('zh-CN',{month:'2-digit',day:'2-digit'}),tokens:0,calls:0,cached:0,cost:0}})
 const map=new Map(days.map(item=>[item.key,item]))
 if(key==='__codex__'){for(const point of codexSeries.value){const bucket=map.get(localDayKey(new Date(point.at*1000)));if(bucket){bucket.tokens+=point.tokens;bucket.calls++}}}
 else{
  for(const item of floatingUsageFor(key)){const bucket=map.get(localDayKey(new Date(item.at)));if(bucket){bucket.tokens+=item.input+item.output;bucket.calls+=usageRequestCount(item);bucket.cached+=item.cached;bucket.cost+=item.cost}}
  // “全部模型”图表必须与顶部总 Token 一致：Codex 独立链路的总量和 turn 数在此合并，费用和缓存仍保持未知。
  if(key==='__all__')for(const point of codexSeries.value){const bucket=map.get(localDayKey(new Date(point.at*1000)));if(bucket){bucket.tokens+=point.tokens;bucket.calls++}}
 }
 return days
}
function floatingSeriesFor(key:string,metric:FloatingMetric){
 const days=floatingDailyFor(key)
 const values=days.map(item=>metric==='tokens'?item.tokens:metric==='calls'?item.calls:metric==='cached'?item.cached:item.cost)
 const max=Math.max(.0001,...values)
 return days.map((item,index)=>({...item,value:values[index],height:values[index]?Math.max(7,values[index]/max*100):2}))
}
const floatingDashboardViews=computed<FloatingDashboardView[]>(()=>floatingDashboardRows.value.map(row=>{
 const source=floatingUsageFor(row.key)
 const recent=source.filter(item=>new Date(item.at).getTime()>=now.value-7*86_400_000)
 const weekCostValue=recent.reduce((sum,item)=>sum+item.cost,0)
 const input=recent.reduce((sum,item)=>sum+item.input,0)
 const output=recent.reduce((sum,item)=>sum+item.output,0)
 const cached=recent.reduce((sum,item)=>sum+item.cached,0)
 const cacheRate=input?Math.min(100,Math.round(cached/input*100)):0
 const dashboardDef=modelDashboards.value.find(item=>item.key===row.key)
 const balance=dashboardDef?accountBalances.value.find(item=>item.account_id===dashboardDef.accountId&&item.currency==='CNY'):null
 const dataSource=row.kind==='codex'?(codex7dIsReported.value?'Codex 客户端 rate_limits':'个人预算估算'):row.kind==='claude'?'Claude Code 本地日志':row.kind==='all'?'主程序本地统一统计':row.kind==='agent'?`${dashboardDef?.label||row.provider} · 本地观测`:balance?'官方账户余额 + 本地代理':'本地代理 / 日志统计'
 const balanceText=row.kind==='codex'?row.ringLabel:balance?`官方余额 ¥${balance.total_balance.toFixed(2)}`:row.todayTokens?`${row.ringPercent}% 今日 Token 占比`:'暂无真实数据'
 const ringCaption=row.kind==='codex'?(codex7dIsReported.value?'7 天客户端额度':'7 天个人估算'):'今日 Token 占比'
 const codexLatest=row.kind==='codex'||row.kind==='all'?codexSeries.value.reduce((latest,item)=>Math.max(latest,item.at*1000),0):0
 const usageLatest=source.reduce((latest,item)=>Math.max(latest,new Date(item.collected_at||item.at).getTime()||0),0)
 const lastUpdated=Math.max(codexLatest,usageLatest)
 const daily=floatingDailyFor(row.key)
 const hasData=daily.some(item=>item.tokens>0||item.calls>0||item.cached>0||item.cost>0)
 return{...row,weekCost:weekCostValue,weekInput:input,weekOutput:output,weekCached:cached,cacheRate,ringCaption,dataSource,balanceText,lastUpdated:lastUpdated?new Intl.DateTimeFormat('zh-CN',{hour:'2-digit',minute:'2-digit',second:'2-digit'}).format(lastUpdated):'尚未同步',hasData,daily}
}))
const floatingVisibleViews=computed(()=>{
  const keys=floatingConfig.value.visibleKeys.length?floatingConfig.value.visibleKeys:[floatingConfig.value.selectedKey]
  const rows=floatingDashboardViews.value.filter(row=>keys.includes(row.key))
  const ordered=rows.length?rows:floatingDashboardViews.value.slice(0,1)
  return [...ordered].sort((left,right)=>{
   const leftPinned=floatingConfig.value.pinnedKeys.includes(left.key)?0:1
   const rightPinned=floatingConfig.value.pinnedKeys.includes(right.key)?0:1
   if(leftPinned!==rightPinned)return leftPinned-rightPinned
   return keys.indexOf(left.key)-keys.indexOf(right.key)
  })
})
const visibleFloatingDashboards=computed(()=>floatingCollapsed.value?floatingDashboardRows.value.slice(0,2):floatingDashboardRows.value)
const selectedFloatingRow=computed(()=>floatingDashboardRows.value.find(item=>item.key===selectedDashboard.value)||floatingDashboardRows.value[0])
const availableFloatingMetrics=computed<Array<'tokens'|'calls'|'cached'|'cost'>>(()=>{const metrics:Array<'tokens'|'calls'|'cached'|'cost'>=[];if(floatingConfig.value.enabled.some(id=>['todayTokens','tokenTrendChart'].includes(id)))metrics.push('tokens');if(floatingConfig.value.enabled.some(id=>['todayCalls','requestChart'].includes(id)))metrics.push('calls');if(floatingConfig.value.enabled.includes('cacheChart'))metrics.push('cached');if(floatingConfig.value.enabled.some(id=>['weekCost','costChart'].includes(id)))metrics.push('cost');return metrics.length?metrics:['tokens']})
 const floatingExpandedSeries=computed(()=>floatingSeriesFor(selectedDashboard.value,floatingChartMetric.value))
const floatingExpandedHasData=computed(()=>floatingExpandedSeries.value.some(item=>item.value>0))
const floatingExpandedTotal=computed(()=>floatingExpandedSeries.value.reduce((sum,item)=>sum+item.value,0))
async function toggleFloatingDashboard(key:string){
  const shouldExpand=!floatingConfig.value.expandedKeys.includes(key)
  selectDashboard(key)
  floatingConfig.value.expandedKeys=shouldExpand
   ? floatingConfig.value.allowMultipleExpanded?[...new Set([...floatingConfig.value.expandedKeys,key])]:[key]
   : floatingConfig.value.expandedKeys.filter(item=>item!==key)
  persistFloatingConfig()
 if(!shouldExpand)return
 await nextTick()
 requestAnimationFrame(()=>{
  const item=Array.from(document.querySelectorAll<HTMLElement>('[data-floating-dashboard-key]')).find(element=>element.dataset.floatingDashboardKey===key)
  const list=item?.closest<HTMLElement>('.floating-dashboard-list')
  if(!item||!list)return
  item.scrollTop=0
  const listRect=list.getBoundingClientRect()
  const itemRect=item.getBoundingClientRect()
  const itemTop=itemRect.top-listRect.top+list.scrollTop
  const itemBottom=itemTop+item.offsetHeight
  const visibleTop=list.scrollTop
  const visibleBottom=visibleTop+list.clientHeight
  const target=item.offsetHeight<=list.clientHeight?itemTop:itemTop<visibleTop?itemTop:itemBottom>visibleBottom?itemBottom-list.clientHeight+8:list.scrollTop
  if(target!==list.scrollTop)list.scrollTo({top:Math.max(0,target),behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'auto':'smooth'})
 })
}
function setFloatingChartMetric(metric:'tokens'|'calls'|'cached'|'cost'){floatingChartMetric.value=metric;localStorage.setItem('token-manager-floating-chart',metric)}
function floatingMetricTitle(metric:'tokens'|'calls'|'cached'|'cost'){return{tokens:'Token',calls:'请求',cached:'缓存',cost:'消费'}[metric]}
function floatingMetricText(value:number){return floatingChartMetric.value==='cost'?`¥${value.toFixed(4)}`:floatingChartMetric.value==='calls'?`${value.toLocaleString()} 次`:`${value.toLocaleString()} Token`}
const enabledFloatingModules=computed(()=>floatingConfig.value.order.filter(id=>floatingConfig.value.enabled.includes(id)).filter(id=>selectedDashboard.value==='__codex__'?!['weekCost','modelBalances'].includes(id):isApiDashboard.value||isClaudeDashboard.value?!['codexQuota','estimatedDuration','taskCapacity'].includes(id):true))
const floatingSummaryModules=computed(()=>effectiveFloatingModules.value.filter(id=>!['cacheChart','costChart','requestChart','tokenTrendChart'].includes(id)))
const effectiveFloatingModules=computed(()=>floatingCustomizing.value?enabledFloatingModules.value:globalMode.value==='simple'?enabledFloatingModules.value.filter(id=>['globalRemaining','todayTokens','warning','todayCalls'].includes(id)).slice(0,4):enabledFloatingModules.value)
const disabledFloatingModules=computed(()=>floatingConfig.value.order.filter(id=>!floatingConfig.value.enabled.includes(id)))
const miniFloatingModules=computed(()=>{const allowed=floatingConfig.value.mini.filter(id=>effectiveFloatingModules.value.includes(id));return [...allowed,...effectiveFloatingModules.value.filter(id=>!allowed.includes(id))].slice(0,2)})
function floatingModuleValue(id:FloatingModuleId){return id==='globalRemaining'?(isApiDashboard.value?(dashboardBalance.value?`¥${dashboardBalance.value.total_balance.toFixed(2)}`:'—'):`${globalRemaining.value}%`):id==='todayTokens'?floatingTodayTokens.value.toLocaleString():id==='estimatedDuration'?`${codexInsights.value.debugHours.toFixed(1)} 小时`:id==='taskCapacity'?`${codexInsights.value.bugFixes} 次修复`:id==='weekCost'?`¥${floatingWeekCost.value.toFixed(2)}`:id==='codexQuota'?`${remaining5h.value}% / ${remaining7d.value}%`:id==='modelBalances'?`${modelBalanceRows.value.length} 个模型`:id==='warning'?(isDeepSeekDashboard.value?(dashboardBalance.value?.is_available?'正常':'余额不可用'):balanceLevel.value==='normal'?'正常':balanceLevel.value==='warning'?'低于20%':'低于10%'):floatingTodayCalls.value.toLocaleString()}
function floatingModuleTitle(id:FloatingModuleId){return floatingModuleCatalog.find(item=>item.id===id)?.title||id}
function copyFloatingSizes(){return{capsule:{...floatingConfig.value.sizes.capsule},compact:{...floatingConfig.value.sizes.compact},full:{...floatingConfig.value.sizes.full}}}
function persistFloatingConfig(){const value:FloatingConfigV3={...floatingConfig.value,version:3,enabled:[...floatingConfig.value.enabled],order:[...floatingConfig.value.order],mini:[...floatingConfig.value.mini] as [FloatingModuleId,FloatingModuleId],visibleKeys:[...floatingConfig.value.visibleKeys],pinnedKeys:[...floatingConfig.value.pinnedKeys],expandedKeys:[...floatingConfig.value.expandedKeys],sizes:copyFloatingSizes(),position:floatingConfig.value.position?{...floatingConfig.value.position}:null};try{saveFloatingConfigV3(value)}catch{}void emit('floating-config',value)}
function toggleFloatingModule(id:FloatingModuleId,event:Event){const checked=(event.target as HTMLInputElement).checked;floatingConfig.value.enabled=checked?[...new Set([...floatingConfig.value.enabled,id])]:floatingConfig.value.enabled.filter(item=>item!==id);persistFloatingConfig()}
function toggleFloatingModuleValue(id:string,checked:boolean){const module=id as FloatingModuleId;floatingConfig.value.enabled=checked?[...new Set([...floatingConfig.value.enabled,module])]:floatingConfig.value.enabled.filter(item=>item!==module);persistFloatingConfig()}
function startFloatingModuleDrag(id:FloatingModuleId){draggedFloatingModule.value=id}
function moveFloatingModuleTo(source:FloatingModuleId,target:FloatingModuleId){if(source===target)return;const order=[...floatingConfig.value.order];order.splice(order.indexOf(source),1);order.splice(order.indexOf(target),0,source);floatingConfig.value.order=order;persistFloatingConfig()}
function dropFloatingModule(target:FloatingModuleId){const source=draggedFloatingModule.value;if(!source)return;moveFloatingModuleTo(source,target);draggedFloatingModule.value=null}
function moveFloatingModule(id:FloatingModuleId,delta:-1|1){const order=[...floatingConfig.value.order];const from=order.indexOf(id);const to=Math.max(0,Math.min(order.length-1,from+delta));if(from===to)return;order.splice(from,1);order.splice(to,0,id);floatingConfig.value.order=order;persistFloatingConfig()}
function updateMiniModule(index:0|1,event:Event){const value=(event.target as HTMLSelectElement).value as FloatingModuleId;const next=[...floatingConfig.value.mini] as [FloatingModuleId,FloatingModuleId];next[index]=value;if(next[0]===next[1])next[index===0?1:0]=floatingIds.find(id=>id!==value)||'todayTokens';floatingConfig.value.mini=next;persistFloatingConfig()}
function updateMiniModuleValue(index:0|1,value:string){const next=[...floatingConfig.value.mini] as [FloatingModuleId,FloatingModuleId];next[index]=value as FloatingModuleId;if(next[0]===next[1])next[index===0?1:0]=floatingIds.find(id=>id!==value)||'todayTokens';floatingConfig.value.mini=next;persistFloatingConfig()}
function setFloatingLayout(layout:'grid'|'list'){floatingConfig.value.layout=layout;persistFloatingConfig()}
function setFloatingVisible(key:string,checked:boolean){const current=[...floatingConfig.value.visibleKeys];if(checked){if(!current.includes(key))current.push(key)}else if(current.length>1){const index=current.indexOf(key);if(index>=0)current.splice(index,1)}floatingConfig.value.visibleKeys=current;if(!current.includes(floatingConfig.value.selectedKey))floatingConfig.value.selectedKey=current[0]||'__all__';persistFloatingConfig()}
function reorderFloatingRows(source:string,target:string){const keys=[...floatingConfig.value.visibleKeys];const sourceIndex=keys.indexOf(source),targetIndex=keys.indexOf(target);if(sourceIndex<0||targetIndex<0||sourceIndex===targetIndex)return;keys.splice(sourceIndex,1);keys.splice(targetIndex,0,source);floatingConfig.value.visibleKeys=keys;persistFloatingConfig()}
function toggleFloatingPin(key:string){floatingConfig.value.pinnedKeys=floatingConfig.value.pinnedKeys.includes(key)?floatingConfig.value.pinnedKeys.filter(item=>item!==key):[...floatingConfig.value.pinnedKeys,key];persistFloatingConfig()}
async function setFloatingSize(mode:FloatingMode,width:number,height:number){const minimum=floatingMinimumSize(mode);floatingConfig.value.sizes[mode]=mode==='capsule'?{...defaultFloatingSizes.capsule}:{width:Math.max(minimum.width,Math.min(960,Math.round(Number(width)||minimum.width))),height:Math.max(minimum.height,Math.min(1000,Math.round(Number(height)||minimum.height)))};persistFloatingConfig();if(floatingConfig.value.mode===mode)await applyFloatingModeSize()}
async function setFloatingMode(mode:FloatingMode){floatingConfig.value.mode=mode;floatingCollapsed.value=mode==='capsule';document.documentElement.style.setProperty('--floating-window-radius',mode==='capsule'?'28px':'24px');persistFloatingConfig();await applyFloatingModeSize();if(floatingMode){await nextTick();void reportFloatingSurfaces()}}
 function floatingMinimumSize(mode:FloatingMode){return mode==='capsule'?{width:360,height:152}:mode==='full'?{width:500,height:520}:{width:400,height:300}}
 async function rememberFloatingSize(){if(!floatingMode)return;try{const window=getCurrentWindow();const [size,scale]=await Promise.all([window.innerSize(),window.scaleFactor()]);const mode=floatingConfig.value.mode;const minimum=floatingMinimumSize(mode);floatingConfig.value.sizes[mode]=mode==='capsule'?{...defaultFloatingSizes.capsule}:{width:Math.max(minimum.width,Math.round(size.width/scale)),height:Math.max(minimum.height,Math.round(size.height/scale))};try{saveFloatingConfigV3(floatingConfig.value)}catch{}}catch{}}
async function applyFloatingModeSize(){const mode=floatingConfig.value.mode;const size=floatingConfig.value.sizes[mode];const radius=mode==='capsule'?28:24;try{await invoke('set_floating_window_mode',{mode,radius,width:size.width,height:size.height,alwaysOnTop:floatingConfig.value.alwaysOnTop,snapToEdges:floatingConfig.value.snapToEdges})}catch{if(floatingMode)try{const window=getCurrentWindow();await window.setSize(new LogicalSize(size.width,size.height));await window.setAlwaysOnTop(floatingConfig.value.alwaysOnTop)}catch{}}}
async function refreshFloatingRenderStatus(){if(!floatingMode||!tauriRuntime)return;try{floatingRenderStatus.value=await invoke<FloatingRenderStatusView>('get_floating_render_status')}catch{floatingRenderStatus.value={backend:'translucent',state:'degraded',fps:0,detail:'原生状态接口不可用，使用半透明材质'}}}
async function configureFloatingRenderer(){if(!floatingMode||!tauriRuntime)return;try{floatingRenderStatus.value=await invoke<FloatingRenderStatusView>('configure_floating_renderer',{config:{enabled:false,quality:'performance',tone:'solid',transparency:0,distortion:0}})}catch{floatingRenderStatus.value={backend:'translucent',state:'stopped',fps:0,detail:'悬浮窗使用主题纯色材质'}}}
async function reportFloatingSurfaces(){if(!floatingMode||!tauriRuntime)return;await invoke('update_floating_surfaces',{surfaces:[]}).catch(()=>{})}
async function snapFloatingToMonitor(){if(!floatingMode||!floatingConfig.value.snapToEdges)return;try{const current=getCurrentWindow();const monitor=await currentMonitor();if(!monitor)return;const [position,size,scale]=await Promise.all([current.outerPosition(),current.outerSize(),current.scaleFactor()]);const area=monitor.workArea;const threshold=Math.round(14*scale);let x=position.x;let y=position.y;const left=area.position.x;const top=area.position.y;const right=left+area.size.width-size.width;const bottom=top+area.size.height-size.height;if(Math.abs(x-left)<=threshold)x=left;else if(Math.abs(x-right)<=threshold)x=right;if(Math.abs(y-top)<=threshold)y=top;else if(Math.abs(y-bottom)<=threshold)y=bottom;if(x!==position.x||y!==position.y)await current.setPosition(new PhysicalPosition(x,y))}catch{}}
async function initializeFloatingWindow(){
 if(!floatingMode)return
 const window=getCurrentWindow()
 if(floatingConfig.value.position)try{await window.setPosition(new PhysicalPosition(floatingConfig.value.position.x,floatingConfig.value.position.y));if(!await currentMonitor())await window.center()}catch{await window.center().catch(()=>{})}
 await applyFloatingModeSize()
 if(await window.isVisible())await configureFloatingRenderer()
 await refreshFloatingRenderStatus()
 // 初始化只设置一次原生交互状态，避免通过 setFloatingInteraction 再广播配置。
 await invoke('set_floating_interaction_mode',{mode:'interactive'}).catch(()=>{})
 await reportFloatingSurfaces()
 try{
   unlistenFloatingResize=await window.onResized(async event=>{
     const scale=await window.scaleFactor()
     const mode=floatingConfig.value.mode
     const minimum=floatingMinimumSize(mode)
     const next=mode==='capsule'?{...defaultFloatingSizes.capsule}:{width:Math.max(minimum.width,Math.round(event.payload.width/scale)),height:Math.max(minimum.height,Math.round(event.payload.height/scale))}
     const current=floatingConfig.value.sizes[mode]
     if(current.width===next.width&&current.height===next.height)return
     floatingConfig.value.sizes[mode]=next
     // Windows 拖拽缩放和圆角重裁会连续产生 Resized；稳定后只保存一次，
     // 回调内绝不再次调用 set_floating_window_mode / setSize。
     if(floatingResizeSaveTimer)clearTimeout(floatingResizeSaveTimer)
     floatingResizeSaveTimer=globalThis.setTimeout(()=>{
       floatingResizeSaveTimer=undefined
       persistFloatingConfig()
       void reportFloatingSurfaces()
     },220)
   })
 }catch{}
 try{
   await window.onMoved(event=>{
     floatingConfig.value.position={x:event.payload.x,y:event.payload.y}
     if(floatingMoveSaveTimer)clearTimeout(floatingMoveSaveTimer)
     floatingMoveSaveTimer=globalThis.setTimeout(()=>{
       floatingMoveSaveTimer=undefined
       persistFloatingConfig()
     },180)
     if(floatingSnapTimer)clearTimeout(floatingSnapTimer)
     floatingSnapTimer=globalThis.setTimeout(()=>void snapFloatingToMonitor(),120)
   })
 }catch{}
}
async function startFloatingDrag(event:MouseEvent){if(!floatingMode||event.button!==0||(event.target as HTMLElement).closest('button,select'))return;try{await getCurrentWindow().startDragging()}catch{}}
 async function startFloatingResizeDirection(direction:string,event:MouseEvent){if(!floatingMode||event.button!==0)return;event.preventDefault();event.stopPropagation();try{await getCurrentWindow().startResizeDragging(direction as never)}catch{}}
 async function setFloatingInteraction(_mode:FloatingInteractionMode){const mode:FloatingInteractionMode='interactive';floatingConfig.value.interaction=mode;persistFloatingConfig();if(tauriRuntime)await invoke('set_floating_interaction_mode',{mode}).catch(()=>{})}
 async function toggleFloatingAlwaysOnTop(){floatingConfig.value.alwaysOnTop=!floatingConfig.value.alwaysOnTop;persistFloatingConfig();if(floatingMode)await getCurrentWindow().setAlwaysOnTop(floatingConfig.value.alwaysOnTop).catch(()=>{})}
// 主窗口使用应用内标题栏，避免 Windows 原生白色标题栏破坏主题连续性。
async function minimizeMainWindow(){if(floatingMode)return;try{await getCurrentWindow().minimize()}catch{}}
async function toggleMainWindowMaximize(){if(floatingMode)return;try{await getCurrentWindow().toggleMaximize()}catch{}}
async function closeMainWindow(){if(floatingMode)return;try{await getCurrentWindow().close()}catch{}}
 async function toggleMiniMode(event:Event){await setFloatingMode((event.target as HTMLInputElement).checked?'capsule':'compact')}
function syncCurrentView(background=false){return floatingMode?syncFloatingData():syncData(false,!background)}
async function refreshCcSwitch(){try{ccSwitch.value=await invoke<CcSwitchStatus>('cc_switch_status')}catch{ccSwitch.value={installed:false,running:false,local_routing:false,detail:'检测不可用',checked_at:new Date().toISOString(),coexistence:'unknown',conflicting_variables:[],safe_repair_available:false}}}
async function repairCcSwitch(){
 try{const result=await invoke<{detail:string}>('repair_cc_switch_conflicts');autoConnectStatus.value=result.detail;await refreshCcSwitch()}
 catch(error){autoConnectStatus.value=String(error)}
}
async function refreshBalanceData(sync=true){
 if(balanceSyncing.value)return
 balanceSyncing.value=true
 const previous=new Map(accountBalances.value.filter(item=>item.provider==='DeepSeek').map(item=>[item.account_id,item.total_balance]))
 try{
  const next=sync?await invoke<AccountBalance[]>('sync_account_balances'):await invoke<AccountBalance[]>('list_account_balances')
  accountBalances.value=next
  balanceError.value=''
  for(const row of next.filter(item=>item.provider==='DeepSeek')){
   const before=previous.get(row.account_id)
   if(before!==undefined&&row.total_balance<before-.000001){
    const drop=before-row.total_balance
    const sameAccountRequest=lastProxyTraffic.value?.account_id===row.account_id&&Boolean(lastProxyRequestAt.value)&&Date.now()-lastProxyRequestAt.value<=20_000
    if(!sameAccountRequest){
     unattributedSpend.value=`DeepSeek 余额减少 ¥${drop.toFixed(4)}，但没有请求进入本机代理；本次 Token 无法反推，请点击“自动接入”后重启调用工具。`
    }else{
     unattributedSpend.value=''
    }
   }
  }
  const accountId=selectedDashboardDef.value?.provider==='DeepSeek'?selectedDashboardDef.value.accountId:''
  deepseekBalanceHistory.value=accountId?await invoke<BalancePoint[]>('list_balance_history',{accountId,days:30}):[]
 }catch(error){
  balanceError.value=String(error)
  try{accountBalances.value=await invoke<AccountBalance[]>('list_account_balances')}catch{}
 }finally{balanceSyncing.value=false}
}
/** 代理每完成一次请求就走这条轻量链路，只读取本地落库结果，不重复请求所有厂商。 */
async function refreshLiveUsage(provider=''){
 if(isSyncing.value){usageRefreshQueued=true;return}
 isSyncing.value=true
 let success=false
 try{
  usage.value=await invoke<Usage[]>('list_usage_v2',{days:floatingMode?7:30})
  accountBalances.value=await invoke<AccountBalance[]>('list_account_balances')
  if(provider==='DeepSeek'&&selectedDashboardDef.value?.provider==='DeepSeek')deepseekBalanceHistory.value=await invoke<BalancePoint[]>('list_balance_history',{accountId:selectedDashboardDef.value.accountId,days:30})
  lastRefreshed.value=Date.now()
  success=true
 }catch(error){balanceError.value=String(error)}
 finally{
  isSyncing.value=false
  if(success)successBurstKey.value++
  if(usageRefreshQueued){usageRefreshQueued=false;queueMicrotask(()=>void refreshLiveUsage(provider))}
 }
}
async function syncFloatingData(){
 if(isSyncing.value)return
 isSyncing.value=true
 let success=false
 try{
   const [, , windowUsage,series,events,quota,claudeEvents]=await Promise.all([
    refreshCcSwitch(),
    refreshBalanceData(false),
    invoke<{five_hour_used:number;seven_day_used:number;budget:{five_hour_limit:number;seven_day_limit:number}}>('codex_window_usage').catch(()=>null),
   invoke<CodexUsagePoint[]>('codex_usage_series').catch(()=>[]),
   invoke<Usage[]>('list_usage_v2',{days:7}),
    invoke<CodexQuotaSnapshot>('codex_quota_snapshot').catch(()=>null),
    invoke<Usage[]>('claude_code_usage_series',{days:7}).catch(()=>[])
  ])
  if(windowUsage){used5h.value=windowUsage.five_hour_used;used7d.value=windowUsage.seven_day_used
  budget5h.value=windowUsage.budget.five_hour_limit;budget7d.value=windowUsage.budget.seven_day_limit}
   codexSeries.value=series;usage.value=events;claudeUsage.value=claudeEvents;codexQuota.value=quota;codexTokens.value=series.reduce((sum,item)=>sum+item.tokens,0);codexError.value='';success=true
 }catch(error){codexError.value=String(error);codexTokens.value=null}
 finally{lastRefreshed.value=Date.now();lastFullSync=lastRefreshed.value;isSyncing.value=false;if(success)successBurstKey.value++;if(usageRefreshQueued){usageRefreshQueued=false;queueMicrotask(()=>void refreshLiveUsage())}}
}
async function maybeSendBudgetAlert(){
 const remaining=Math.min(remaining5h.value,remaining7d.value);const level=remaining<=10?10:remaining<=20?20:0
 if(!level)return
 const key=`token-manager-budget-alert-${level}-${localDayKey(new Date())}`
 if(localStorage.getItem(key))return
 try{await invoke('send_budget_alert',{level,remaining});localStorage.setItem(key,'1')}catch{}
}
async function syncData(forceArena = false, queueSources = true){
 if(isSyncing.value) return
 isSyncing.value=true
 let success=false
 try {
   // 通用 Agent 同步只提交后台任务；Codex 专属数据继续直接读取旧版状态库和会话库。
   if(queueSources)void syncReliabilitySource(undefined,true)
   // 桌面端读取本机 Codex 状态库；浏览器预览没有 Tauri 后端时保持“未连接”。
    const [, , , events,snapshot,windowUsage,series,quota,claudeEvents]=await Promise.all([
     refreshCcSwitch(),
     refreshAccounts(),
     refreshBalanceData(true),
    invoke<Usage[]>('list_usage_v2',{days:30}),
    invoke<{total_tokens:number;active_thread_tokens:number;updated_at_ms:number}>('codex_snapshot').catch(()=>null),
   invoke<{five_hour_used:number;seven_day_used:number;budget:{five_hour_limit:number;seven_day_limit:number}}>('codex_window_usage').catch(()=>null),
    invoke<CodexUsagePoint[]>('codex_usage_series').catch(()=>[]),
     invoke<CodexQuotaSnapshot>('codex_quota_snapshot').catch(()=>null),
     invoke<Usage[]>('claude_code_usage_series',{days:30}).catch(()=>[]),
     page.value==='arena' && forceArena === true ? arenaCenter.value?.refresh(true) : Promise.resolve()
   ])
   usage.value=events
  if(snapshot){codexTokens.value=snapshot.total_tokens
  codexActiveTokens.value=snapshot.active_thread_tokens
  codexUpdatedAt.value=snapshot.updated_at_ms>0&&snapshot.updated_at_ms<1_000_000_000_000?snapshot.updated_at_ms*1000:snapshot.updated_at_ms}
   claudeUsage.value=claudeEvents
   codexQuota.value=quota
   if(windowUsage){used5h.value=windowUsage.five_hour_used
  used7d.value=windowUsage.seven_day_used
  codexSeries.value=series
  budget5h.value=windowUsage.budget.five_hour_limit
  budget7d.value=windowUsage.budget.seven_day_limit}
  codexError.value=''
  await maybeSendBudgetAlert()
  success=true
 } catch (error) { codexError.value=String(error) }
 finally { lastRefreshed.value=Date.now(); lastFullSync=lastRefreshed.value; isSyncing.value=false; if(success)successBurstKey.value++; if(usageRefreshQueued){usageRefreshQueued=false;queueMicrotask(()=>void refreshLiveUsage())} }
}
async function initializeMonitoring(){
 // 先让 WebView 完成首屏绘制，再恢复代理和后台同步，避免启动阶段阻塞窗口消息循环。
 try{
  usage.value=await invoke<Usage[]>('list_usage_v2',{days:floatingMode?7:30})
  accountBalances.value=await invoke<AccountBalance[]>('list_account_balances')
  lastRefreshed.value=Date.now()
  lastFullSync=lastRefreshed.value
 }catch(error){
  codexError.value=String(error)
 }
 if(floatingMode)return
 try{
  await refreshAccounts()
  if(!savedAccounts.value.length)return
  // 账户永久恢复不代表用户授权本次启动代理；避免启动即占用端口或改写调用工具连接。
  allProxyStatus.value=`已恢复 ${savedAccounts.value.length} 个加密账户 · 点击“开启并自动接入”开始实时代理监控`
 }catch(error){
  allProxyStatus.value=`自动恢复监控失败 · ${String(error)}`
 }
}
async function saveBudget(){
 budgetSaved.value=''
 try { await invoke('save_codex_budget',{fiveHourLimit:budget5h.value,sevenDayLimit:budget7d.value}); budgetSaved.value='预算已保存'; await syncData() }
 catch(error){ budgetSaved.value=`保存失败：${String(error)}` }
}
async function startProxy(){
 proxyStatus.value='正在建立数据通道…'
 try { const url=await invoke<string>('start_proxy',{upstream:proxyUpstream.value,port:proxyPort.value,accountId:activeAccountId.value||null}); proxyStatus.value=`数据通道已建立 · ${url}` }
 catch(error){ proxyStatus.value=`无法建立数据通道 · ${String(error)}` }
}
async function autoConnectEndpoint(endpoint:ProxyEndpoint,agentId?:string){
 if(autoConnectBusy.value)return
 autoConnectBusy.value=true
 autoConnectStatus.value=`正在自动接入 ${endpoint.name}…`
 try{
  if(!agentId)throw new Error('请先选择需要接入的本地 Agent，避免改写所有工具的全局连接配置')
  const result=await invoke<ClientIntegrationResult>('connect_agent_proxy',{agentId,endpointId:endpoint.account_id})
  autoConnectedAccountId.value=endpoint.account_id
  localStorage.setItem('token-manager-auto-connected-account',endpoint.account_id)
  autoConnectStatus.value=result.detail
  allProxyStatus.value=`${endpoint.name} 已自动接入 · 重启正在运行的调用工具后生效`
  successBurstKey.value++
 }catch(error){
  autoConnectStatus.value=`自动接入失败：${String(error)}`
  allProxyStatus.value=autoConnectStatus.value
 }finally{autoConnectBusy.value=false}
}
async function autoConnectPreferred(agentId=selectedDashboardDef.value?.agentId){
 if(!agentId){allProxyStatus.value='代理端口已就绪 · 请在本地 Agent 卡片中选择“接入代理”';return}
 const endpoint=proxyEndpoints.value.find(item=>item.account_id===activeAccountId.value)||proxyEndpoints.value[0]
 if(!endpoint){await startAllProxies();return}
 await autoConnectEndpoint(endpoint,agentId)
}
async function restoreClientConnection(){
 autoConnectBusy.value=true
 try{
  const message=await invoke<string>('restore_client_connection')
  autoConnectedAccountId.value=''
  localStorage.removeItem('token-manager-auto-connected-account')
  autoConnectStatus.value=message
  allProxyStatus.value=message
 }catch(error){autoConnectStatus.value=`恢复失败：${String(error)}`}
 finally{autoConnectBusy.value=false}
}
async function monitorPrimaryAction(){
 if(!proxyEndpoints.value.length){await startAllProxies();return}
 if(!proxyRecentlyActive.value){await autoConnectPreferred();return}
 await syncData()
}
async function startAllProxies(requestedAgentId?:string){
 allProxyStatus.value='正在建立本地数据通道…'
 try{
  await refreshAccounts()
  const selectedAccountId=activeAccountId.value||savedAccounts.value[0]?.id
  if(!selectedAccountId)throw new Error('请先添加并选择一个 API 账户')
  const selectedAgentId=typeof requestedAgentId==='string'?requestedAgentId:selectedDashboardDef.value?.agentId
  const job=await invoke<ProxyJob>('start_proxy_group',{accountIds:[selectedAccountId],agentIds:selectedAgentId?[selectedAgentId]:[]})
  lastProxyRequestAt.value=0
  lastProxyTraffic.value=null
  allProxyStatus.value=`代理任务已提交 · ${job.id}`
 }catch(error){
  allProxyStatus.value=`无法建立数据通道 · ${String(error)}`
  if(!savedAccounts.value.length)page.value='accounts'
 }
}
async function copyProxyUrl(url:string){
 try{await navigator.clipboard.writeText(url);allProxyStatus.value=`已复制本机接入地址 · ${url}`;successBurstKey.value++}
 catch{allProxyStatus.value=`请手动复制：${url}`}
}
async function openProviderPortal(p:Provider){
 const url=providerPortals[p.name]
 if(!url) return
 await openUrl(url)
}
async function refreshSyncHealth(){
 try{
  const [sources,health,agents]=await Promise.all([
   invoke<SyncSourceConfig[]>('list_sync_sources'),
   invoke<SyncHealth[]>('get_sync_health'),
   invoke<LocalAgentDefinition[]>('detect_local_agents'),
  ])
  syncSources.value=sources
  syncHealthItems.value=health
  localAgents.value=agents
  const activeAgentSourceIds=new Set(modelDashboards.value.filter(item=>item.sourceId).map(item=>item.sourceId))
  const modelEntries=await Promise.all(sources.filter(item=>item.agent_id&&(item.enabled||activeAgentSourceIds.has(item.id))).map(async source=>{
   try{return[source.id,await invoke<AgentSourceModel[]>('list_agent_source_models',{sourceId:source.id})] as const}catch{return[source.id,[]] as const}
  }))
  agentSourceModels.value=Object.fromEntries(modelEntries)
  const opencode=sources.find(item=>item.id==='opencode-local')
  if(opencode){
   if(['auto','sqlite','json'].includes(opencode.mode))opencodeLocalMode.value=opencode.mode as 'auto'|'sqlite'|'json'
   opencodePathManual.value=opencode.path_mode==='manual'
   if(opencode.path){opencodeLocalPath.value=opencode.path;localStorage.setItem('token-manager-opencode-json-path',opencode.path)}
  }
 }catch(error){
  if(syncHealthOpen.value)opencodeLocalStatus.value=`无法读取监控健康状态：${String(error)}`
 }
}
async function configureOpenCodeSource(){
 const kind:UsageSourceKind=opencodeLocalMode.value==='json'?'local_json':'local_sqlite'
 const previous=syncSources.value.find(item=>item.id==='opencode-local')
 const manual=Boolean(opencodeLocalPath.value&&opencodePathManual.value)
 const config:SyncSourceConfig={id:'opencode-local',provider:'OpenCode Go',kind,mode:opencodeLocalMode.value,path:opencodeLocalPath.value,enabled:true,interval_seconds:30,agent_id:'opencode',collector_kind:previous?.collector_kind||'sqlite_json',paths:manual?[opencodeLocalPath.value]:(previous?.paths||[]),capabilities:previous?.capabilities||['本地 SQLite','本地 JSON','实时代理'],detected:previous?.detected||Boolean(opencodeLocalPath.value),schema_version:previous?.schema_version||1,path_mode:manual?'manual':'auto',path_spec_ids:manual?[]:(previous?.path_spec_ids||['home-xdg','roaming'])}
 const saved=await invoke<SyncSourceConfig>('configure_local_source',{config})
 if(['auto','sqlite','json'].includes(saved.mode))opencodeLocalMode.value=saved.mode as 'auto'|'sqlite'|'json'
 opencodeLocalPath.value=saved.path
 localStorage.setItem('token-manager-opencode-local-mode',saved.mode)
 localStorage.setItem('token-manager-opencode-json-path',saved.path)
}
function agentSourceFor(agent:LocalAgentDefinition){return syncSources.value.find(item=>item.agent_id===agent.id)}
function agentKind(agent:LocalAgentDefinition):UsageSourceKind{return agent.collector_kind==='sqlite'?'local_sqlite':agent.collector_kind==='json'||agent.collector_kind==='jsonl'?'local_json':'local_log'}
function agentMode(agent:LocalAgentDefinition):SyncSourceConfig['mode']{return agent.collector_kind==='sqlite'?'sqlite':agent.collector_kind==='jsonl'?'jsonl':agent.collector_kind==='otel'?'otel':agent.collector_kind==='cache'?'cache':'auto'}
async function saveAgentSource(agent:LocalAgentDefinition,path?:string,enabled=true){
 const existing=agentSourceFor(agent)
 const manual=typeof path==='string'||existing?.path_mode==='manual'
 const selectedPath=path??existing?.path??agent.detected_paths[0]??agent.default_paths[0]??''
 const specIds=manual?[]:(existing?.path_spec_ids?.length?existing.path_spec_ids:agent.path_candidates.map(item=>item.id))
 const config:SyncSourceConfig={id:existing?.id||`agent-${agent.id}`,provider:agent.provider,kind:agentKind(agent),mode:agentMode(agent),path:selectedPath,enabled,interval_seconds:30,agent_id:agent.id,collector_kind:agent.collector_kind,paths:manual&&selectedPath?[selectedPath]:(agent.detected_paths.length?agent.detected_paths:agent.default_paths),capabilities:agent.capabilities,detected:Boolean(selectedPath||agent.detected),schema_version:agent.schema_version,path_mode:manual?'manual':'auto',path_spec_ids:specIds}
 agentActionStatus.value={...agentActionStatus.value,[agent.id]:'正在保存本地数据源…'}
 try{
  await invoke<SyncSourceConfig>('configure_agent_source',{config})
  const job=enabled?await invoke<SyncJob>('start_sync_source',{sourceId:config.id,force:false}):null
  agentActionStatus.value={...agentActionStatus.value,[agent.id]:job?`后台接入任务已提交 · ${job.id}`:'已停用'}
  await refreshSyncHealth()
 }catch(error){agentActionStatus.value={...agentActionStatus.value,[agent.id]:String(error)}}
}
async function chooseAgentSource(agent:LocalAgentDefinition){
 const path=await open({directory:true,multiple:false,title:`选择 ${agent.name} 本地数据目录`})
 if(typeof path==='string')await saveAgentSource(agent,path,true)
}
async function toggleAgentSource(agent:LocalAgentDefinition){const source=agentSourceFor(agent);await saveAgentSource(agent,undefined,!source?.enabled)}
async function syncReliabilitySource(sourceId?:string,silent=false){
 try{
  const job=sourceId
   ?await invoke<SyncJob>('start_sync_source',{sourceId,force:false})
   :await invoke<SyncJob>('start_sync_all',{force:false})
  if(!silent)opencodeLocalStatus.value=`同步任务已提交 · ${job.completed}/${job.total}`
  return job
 }catch(error){
  if(!silent)opencodeLocalStatus.value=String(error)
  return null
 }
}
async function syncOpenCodeLocalUsage(silent=false){
 if(opencodeLocalSyncing.value)return
 opencodeLocalSyncing.value=true
 if(!silent)opencodeLocalStatus.value='正在读取 OpenCode 本地数据…'
 try{
  await configureOpenCodeSource()
  const result=await invoke<SyncJob>('start_sync_source',{sourceId:'opencode-local',force:false})
  opencodeLocalStatus.value=`OpenCode 后台同步已提交 · ${result.id}`
  await refreshSyncHealth()
  if(!silent)successBurstKey.value++
 }catch(error){
  if(!silent)opencodeLocalStatus.value=String(error)
 }finally{opencodeLocalSyncing.value=false}
}
async function chooseOpenCodeLocalSource(){
 const path=await open({directory:true,multiple:false,title:'选择 OpenCode 数据目录（可包含 opencode.db、JSON 或 JSONL）'})
 if(typeof path!=='string')return
 opencodeLocalPath.value=path
 opencodePathManual.value=true
 localStorage.setItem('token-manager-opencode-json-path',path)
 await syncOpenCodeLocalUsage(false)
}
async function changeOpenCodeLocalMode(){
 localStorage.setItem('token-manager-opencode-local-mode',opencodeLocalMode.value)
 await configureOpenCodeSource()
 await syncOpenCodeLocalUsage(false)
}
function finishOnboarding(target:'finish'|'skip'|'accounts'){
 localStorage.setItem('token-manager-onboarding-v0113','done')
 onboardingOpen.value=false
 if(target==='accounts')page.value='accounts'
}
function setOpenCodeVariant(variant:'go'|'zen'){
 configOpenCodeVariant.value=variant
 configUrl.value=variant==='go'?'https://opencode.ai/zen/go/v1':'https://opencode.ai/zen/v1'
}
function openConfig(p:Provider,account?:SavedAccount){configuring.value=p;editingAccountId.value=account?.id||'';configName.value=account?.name||`${p.name} 账户`;configUrl.value=account?.base_url||providerApiBases[p.name]||'';configKey.value='';if(p.name==='OpenCode Go'){configOpenCodeVariant.value=configUrl.value.includes('/zen/v1')?'zen':'go';configStatus.value=`粘贴 ${configOpenCodeVariant.value==='go'?'Go':'Zen'} API Key 后验证模型目录。`}else configStatus.value=''}
async function saveConfig(){
 if(!configuring.value)return
 const provider=configuring.value.name
 configStatus.value='正在加密保存…'
 try{
  const id=editingAccountId.value||`${configuring.value.id}-${Date.now()}`
  await invoke('save_account_config',{id,provider,name:configName.value,baseUrl:configUrl.value,apiKey:configKey.value})
  activeAccountId.value=id
  proxyUpstream.value=configUrl.value
  await refreshAccounts()
  if(provider==='OpenCode Go'){
   configStatus.value='密钥已加密保存，正在验证 OpenCode Go 模型目录…'
   try{
    const models=await invoke<string[]>('fetch_account_models',{id})
    if(!models.length)throw new Error('官方接口没有返回可用模型')
    accountModels.value[id]=models
    configStatus.value=`连接成功 · 已读取 ${models.length} 个 Go 模型 · 可开启代理监控`
   }catch(error){
    configStatus.value=`密钥已保存，但连接验证失败：${String(error)}`
   }
  }else configStatus.value='已永久保存并设为代理账户。'
  successBurstKey.value++
 }catch(e){configStatus.value=`保存失败：${String(e)}`}
}
async function refreshAccounts(){
 savedAccounts.value=await invoke<SavedAccount[]>('list_account_configs')
 for(const p of accounts.value){
  const count=savedAccounts.value.filter(a=>a.provider===p.name).length
  p.configured=count>0
  p.description=count?`已保存 ${count} 个加密账户`:'尚未配置'
 }
 if(!activeAccountId.value&&savedAccounts.value[0]){
  activeAccountId.value=savedAccounts.value[0].id
  proxyUpstream.value=savedAccounts.value[0].base_url
 }
 // Go API Key 在应用重启后自动恢复验证结果；同一会话只请求一次模型目录，避免 30 秒同步反复访问官网。
 await Promise.all(savedAccounts.value.filter(account=>account.provider==='OpenCode Go'&&!accountModels.value[account.id]?.length).map(async account=>{
  try{
   const models=await invoke<string[]>('fetch_account_models',{id:account.id})
   if(models.length)accountModels.value[account.id]=models
  }catch{}
 }))
}
async function deleteSavedAccount(id:string){await invoke('delete_account_config',{id});modelDashboards.value=modelDashboards.value.filter(d=>d.accountId!==id);persistDashboards();await refreshAccounts()}
async function clearSavedAccounts(){if(!confirm('确定清空全部已保存 API 密钥吗？此操作不可恢复。'))return;await invoke('clear_account_configs');modelDashboards.value=[];persistDashboards();await refreshAccounts()}
function persistDashboards(){localStorage.setItem('token-manager-model-dashboards-v2',JSON.stringify(modelDashboards.value))}
function selectDashboard(key:string){selectedDashboard.value=key;floatingConfig.value.selectedKey=key;persistFloatingConfig();localStorage.setItem('token-manager-selected-dashboard',key);void emit('dashboard-selection',key);void refreshBalanceData(false)}
function scrollDashboardSwitcher(event:WheelEvent){const element=event.currentTarget as HTMLElement;const delta=Math.abs(event.deltaY)>=Math.abs(event.deltaX)?event.deltaY:event.deltaX;if(!delta||element.scrollWidth<=element.clientWidth)return;const canMove=delta>0?element.scrollLeft+element.clientWidth<element.scrollWidth-1:element.scrollLeft>0;if(!canMove)return;event.preventDefault();element.scrollBy({left:delta,behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'auto':'smooth'})}
function changeFloatingDashboard(event:Event){selectDashboard((event.target as HTMLSelectElement).value)}
async function toggleFloating(){floatingEnabled.value=!floatingEnabled.value;try{await invoke('set_floating_window',{enabled:floatingEnabled.value});localStorage.setItem('token-manager-floating',floatingEnabled.value?'1':'0')}catch(e){floatingEnabled.value=!floatingEnabled.value;alert(String(e))}}
async function closeFloating(){try{await invoke('set_floating_window',{enabled:false});floatingEnabled.value=false;localStorage.setItem('token-manager-floating','0')}catch(e){alert(String(e))}}
async function toggleFloatingCollapsed(){
 await setFloatingMode(floatingConfig.value.mode==='capsule'?'compact':'capsule')
}
async function openDashboardPicker(){dashboardModal.value=true;modelsLoading.value=true;await Promise.all([refreshSyncHealth(),...savedAccounts.value.map(async account=>{try{const models=await invoke<string[]>('fetch_account_models',{id:account.id});if(models.length)accountModels.value[account.id]=models}catch{}})]);if(!savedAccounts.value.length&&availableAgentDashboards.value.length)dashboardPickerTab.value='agent';modelsLoading.value=false}
function addDashboard(){const choices=dashboardPickerTab.value==='api'?availableModels.value:availableAgentDashboards.value;const choice=choices.find(item=>item.key===newDashboardKey.value);if(!choice)return;if(!modelDashboards.value.some(item=>item.key===choice.key))modelDashboards.value.push(choice);persistDashboards();selectDashboard(choice.key);newDashboardKey.value='';dashboardModal.value=false}
function removeDashboard(key:string){
 const target=modelDashboards.value.find(item=>item.key===key)
 if(!target||!confirm(`确定删除“${target.label}”仪表盘吗？\n仅删除仪表盘布局，不会删除账户、密钥或历史用量。`))return
 modelDashboards.value=modelDashboards.value.filter(item=>item.key!==key)
 persistDashboards()
 if(selectedDashboard.value===key)selectDashboard('__all__')
}
function addAccount(p:Provider){ p.configured=true;p.description='本地安全存储，等待同步'; alert(`${p.name} 已启用。请在 Tauri 桌面端的账户抽屉中输入密钥。`) }
async function exportBill(value?:{provider:string;model:string}|Event){const filter=value instanceof Event?undefined:value;const safe=filter?.model.replace(/[\\/:*?"<>|]/g,'-')||'全部模型';const path=await save({defaultPath:`Token Manager-${safe}-${new Date().toISOString().slice(0,10)}.xlsx`,filters:[{name:'Excel 工作簿',extensions:['xlsx']}]});if(!path)return;await invoke('export_usage_xlsx',{path,provider:filter?.provider||null,model:filter?.model||null});successBurstKey.value++}
function saveBudgetFromSettings(five:number,seven:number){budget5h.value=five;budget7d.value=seven;void saveBudget()}
function startProxyFromSettings(upstream:string,port:number){proxyUpstream.value=upstream;proxyPort.value=port;void startProxy()}
function setMiniModeValue(checked:boolean){void toggleMiniMode({target:{checked}} as unknown as Event)}
function exportBackupFromSettings(password:string){backupPassword.value=password;void exportBackup()}
function importBackupFromSettings(password:string){backupPassword.value=password;void importBackup()}
async function requestCloudCode(baseUrl:string,email:string,purpose:'login'|'register'){cloudStatus.value='正在发送验证码…';try{const result=await invoke<{expires_in:number;debug_code?:string}>('cloud_request_code',{baseUrl,email,purpose});cloudStatus.value=result.debug_code?`开发环境验证码：${result.debug_code}`:`验证码已发送，${Math.round(result.expires_in/60)} 分钟内有效`}catch(error){cloudStatus.value=`发送失败：${String(error)}`}}
async function cloudPasswordLogin(baseUrl:string,email:string,password:string){cloudStatus.value='正在登录…';try{cloudSession.value=await invoke<CloudSession>('cloud_password_login',{baseUrl,email,password});cloudStatus.value='登录成功'}catch(error){cloudStatus.value=`登录失败：${String(error)}`}}
async function cloudPasswordRegister(baseUrl:string,email:string,password:string,code:string){cloudStatus.value='正在创建账户…';try{cloudSession.value=await invoke<CloudSession>('cloud_password_register',{baseUrl,email,password,code});cloudStatus.value='账户创建并登录成功'}catch(error){cloudStatus.value=`注册失败：${String(error)}`}}
async function cloudCodeLogin(baseUrl:string,email:string,code:string){cloudStatus.value='正在验证…';try{cloudSession.value=await invoke<CloudSession>('cloud_code_login',{baseUrl,email,code});cloudStatus.value='验证码登录成功'}catch(error){cloudStatus.value=`登录失败：${String(error)}`}}
async function logoutCloud(){try{await invoke('cloud_logout');cloudSession.value=null;cloudStatus.value='已退出云账户'}catch(error){cloudStatus.value=`退出失败：${String(error)}`}}
async function createCloudTransfer(password:string,ttlHours:number,oneTime:boolean){if(password.length<8){cloudStatus.value='迁移密码至少需要 8 个字符';return}cloudStatus.value='正在本机加密并创建链接…';try{const result=await invoke<CloudTransfer>('cloud_create_transfer',{password,uiState:collectUiState(),ttlHours,oneTime});cloudTransferLink.value=result.link;cloudStatus.value=`链接已创建，将于 ${new Date(result.expires_at).toLocaleString('zh-CN')} 过期`}catch(error){cloudStatus.value=`创建失败：${String(error)}`}}
async function importCloudTransfer(link:string,password:string){if(password.length<8){cloudStatus.value='请输入创建链接时使用的迁移密码';return}cloudStatus.value='正在下载密文并在本机解密…';try{const state=await invoke<Record<string,string>>('cloud_import_transfer',{link,password});for(const [key,value] of Object.entries(state))localStorage.setItem(key,value);cloudStatus.value='云端迁移完成，正在重新载入';setTimeout(()=>location.reload(),500)}catch(error){cloudStatus.value=`迁移失败：${String(error)}`}}
async function refreshRemoteContent(){lastContentRefresh.value=Date.now();const baseUrl=currentCloudBaseUrl();try{remoteContent.value=await invoke<RemoteContentItem[]>('cloud_public_content',{baseUrl})}catch{/* 公告服务离线不影响本地监控 */}}
/** 无需登录的匿名设备心跳：随机安装标识只用于去重，不包含邮箱、机器名或硬件信息。 */
async function sendAppPresence(event:'launch'|'heartbeat'){
 lastPresencePing=Date.now();presenceEventSeq+=1
 try{
  const ack=await invoke<PresenceAckV2>('cloud_app_presence',{baseUrl:currentCloudBaseUrl(),appVersion:appVersion.value,event,sessionId:presenceSessionId,eventSeq:presenceEventSeq,sentAt:new Date().toISOString()})
  presenceHealth.value={lastSuccess:ack.server_time,lastError:'',httpStatus:'200',consecutiveFailures:0}
  localStorage.setItem('token-manager-presence-health',JSON.stringify(presenceHealth.value))
  if(ack.latest_release)window.dispatchEvent(new CustomEvent('token-manager-release-available',{detail:ack.latest_release}))
 }catch(error){
  presenceHealth.value={...presenceHealth.value,lastError:String(error),httpStatus:String(error).match(/\b\d{3}\b/)?.[0]||'',consecutiveFailures:presenceHealth.value.consecutiveFailures+1}
  localStorage.setItem('token-manager-presence-health',JSON.stringify(presenceHealth.value))
 }
}
function dismissRemoteItem(id:string){const next=new Set(dismissedRemoteContent.value);next.add(id);dismissedRemoteContent.value=next;sessionStorage.setItem('token-manager-dismissed-content',JSON.stringify([...next]))}
function openRemoteItem(item:RemoteContentItem){if(item.action_url)void openUrl(item.action_url)}
function collectUiState(){const state:Record<string,string>={};for(let index=0;index<localStorage.length;index++){const key=localStorage.key(index);if(key?.startsWith('token-manager-'))state[key]=localStorage.getItem(key)||''}return state}
async function exportBackup(){backupStatus.value='';if(backupPassword.value.length<8){backupStatus.value='迁移密码至少需要 8 个字符';return}const path=await save({defaultPath:`Token-Manager-${new Date().toISOString().slice(0,10)}.tmbak`,filters:[{name:'Token Manager 加密备份',extensions:['tmbak']}]});if(!path)return;try{await invoke('export_encrypted_backup',{path,password:backupPassword.value,uiState:collectUiState()});backupStatus.value='加密备份已导出'}catch(error){backupStatus.value=`导出失败：${String(error)}`}}
async function importBackup(){backupStatus.value='';if(backupPassword.value.length<8){backupStatus.value='请输入导出时使用的迁移密码';return}const path=await open({multiple:false,filters:[{name:'Token Manager 加密备份',extensions:['tmbak']}]});if(typeof path!=='string')return;try{const state=await invoke<Record<string,string>>('import_encrypted_backup',{path,password:backupPassword.value});for(const [key,value] of Object.entries(state))localStorage.setItem(key,value);backupStatus.value='迁移完成，正在重新载入';setTimeout(()=>location.reload(),500)}catch(error){backupStatus.value=`导入失败：${String(error)}`}}
</script>
<template>
 <!-- 旧版模块式悬浮窗模板保留在版本历史中；v0.6.5 起改用下方按仪表盘分类的结构。
 <main v-if="false" class="floating-shell" :class="[{collapsed:floatingCollapsed},`mode-${globalMode}`]" :style="appThemeStyle">
  <header @mousedown="startFloatingDrag"><div><b>Token Manager <em>v{{appVersion}}</em></b><select v-if="!floatingCollapsed" aria-label="切换悬浮窗模型" :value="selectedDashboard" @change="changeFloatingDashboard"><option value="__all__">全部模型 · 全局统计</option><option value="__codex__">Codex · 专属监控</option><option v-for="item in modelDashboards" :value="item.key">{{item.label}}</option></select><small v-else>迷你模式</small></div><nav aria-label="悬浮窗控制"><button v-if="!floatingCollapsed" @click="floatingCustomizing=!floatingCustomizing">{{floatingCustomizing?'完成':'自定义'}}</button><button v-if="!floatingCollapsed" :class="{proxyLive:proxyEndpoints.length}" :title="proxyEndpoints.length?'API 数据通道实时运行':'建立 API 数据通道'" :aria-label="proxyEndpoints.length?'API 数据通道实时运行':'建立 API 数据通道'" @click="startAllProxies()"><ProxyGatewayIcon class="compact" /></button><button v-if="!floatingCollapsed" :title="floatingConfig.layout==='grid'?'切换为单列':'切换为卡片网格'" :aria-label="floatingConfig.layout==='grid'?'切换为单列布局':'切换为卡片网格布局'" @click="setFloatingLayout(floatingConfig.layout==='grid'?'list':'grid')">{{floatingConfig.layout==='grid'?'☷':'▦'}}</button><button :title="floatingCollapsed?'展开':'折叠'" :aria-label="floatingCollapsed?'展开悬浮窗':'折叠悬浮窗'" @click="toggleFloatingCollapsed">{{floatingCollapsed?'＋':'−'}}</button><button title="关闭悬浮窗" aria-label="关闭悬浮窗" @click="closeFloating">×</button></nav></header>
   <div v-if="floatingCollapsed" class="mini-core-row"><span v-for="id in miniFloatingModules" :class="`mini-${id}`"><i v-if="id==='globalRemaining'||id==='codexQuota'" class="mini-ring-progress" :style="{background:`conic-gradient(${balanceLevel==='critical'?'#FF3B30':balanceLevel==='warning'?'#FF9500':'#34C759'} ${globalRemaining}%,#F2F2F7 0)`}"><em>{{globalRemaining}}%</em></i><div v-else-if="id==='todayTokens'" class="mini-collapsed-bars"><i v-for="bar in floatingBars" :style="{height:bar.height+'%'}" :title="`${bar.label} · ${bar.tokens.toLocaleString()} Token`"></i></div><template v-else><small>{{floatingModuleTitle(id)}}</small><b>{{floatingModuleValue(id)}}</b></template></span></div>
  <TransitionGroup v-else name="floating-reflow" tag="div" class="floating-modules" :class="floatingConfig.layout+'-layout'">
   <section v-for="id in effectiveFloatingModules" :key="id" class="floating-module" :class="`module-${id}`" @dragover.prevent @drop="dropFloatingModule(id)">
     <div v-if="floatingCustomizing" class="floating-card-actions"><button class="floating-drag-handle" draggable="true" title="按住拖动调整位置" :aria-label="`拖动 ${floatingModuleTitle(id)} 调整位置`" @dragstart="startFloatingModuleDrag(id)" @dragend="draggedFloatingModule=null"><span aria-hidden="true">⠿</span> 拖动</button><button class="floating-remove" title="从悬浮窗移除" @click="toggleFloatingModuleValue(id,false)">删除</button></div>
    <template v-if="id==='globalRemaining'"><div class="floating-module-head"><span>{{selectedDashboard==='__codex__'?'Codex 预算剩余':isApiDashboard?'账户余额状态':'全局总剩余额度'}}</span><strong>{{isApiDashboard?(dashboardBalance?'¥'+dashboardBalance.total_balance.toFixed(2):'—'):globalRemaining+'%'}}</strong></div><div v-if="!isApiDashboard" class="track"><span :style="{width:(100-globalRemaining)+'%'}"></span></div><small v-if="isApiDashboard&&dashboardBalance">充值余额 ¥{{dashboardBalance.topped_up_balance.toFixed(2)}} · 赠送 ¥{{dashboardBalance.granted_balance.toFixed(2)}}</small><small>{{isApiDashboard?(dashboardBalance?'官方更新 '+new Date(dashboardBalance.synced_at).toLocaleTimeString('zh-CN'):balanceStatusText):'以可验证的 Codex 个人预算为准'}}</small></template>
    <template v-else-if="id==='todayTokens'"><div class="floating-module-head"><span>{{selectedDashboard==='__all__'?'今日总消耗 Token':selectedDashboard==='__codex__'?'Codex 今日 Token':'当前模型今日 Token'}}</span><strong><AnimatedNumber :value="floatingTodayTokens" /></strong></div><div class="mini-bars floating-interactive-chart" role="img" aria-label="最近五小时 Token 柱状图"><div v-for="bar in floatingBars" class="mini-bar floating-chart-point" tabindex="0" :aria-label="`${bar.label}，${bar.tokens.toLocaleString()} Token`"><i :style="{height:bar.height+'%'}"></i><small>{{bar.label.slice(0,2)}}</small><em class="floating-chart-tooltip"><b>{{bar.label}}</b><span>{{bar.tokens.toLocaleString()}} Token</span></em></div></div></template>
    <template v-else-if="id==='estimatedDuration'"><div class="floating-module-head"><span>剩余开发预估</span><strong>{{codexInsights.debugHours.toFixed(1)}} 小时</strong></div><small>完整生成 {{codexInsights.generationHours.toFixed(1)}} 小时 · 局部调试 {{codexInsights.debugHours.toFixed(1)}} 小时</small></template>
    <template v-else-if="id==='taskCapacity'"><div class="floating-module-head"><span>可执行任务数量</span><strong>{{codexInsights.bugFixes}} 次</strong></div><small>脚本 {{codexInsights.scripts}} · 重构 {{codexInsights.refactors}} · Bug {{codexInsights.bugFixes}}</small></template>
    <template v-else-if="id==='weekCost'"><div class="floating-module-head"><span>{{selectedDashboard==='__all__'?'本周总消耗金额':'当前模型本周金额'}}</span><strong><AnimatedNumber :value="floatingWeekCost" :decimals="2" prefix="¥" /></strong></div><small>最近 7 天已计价 API 调用</small></template>
    <template v-else-if="id==='codexQuota'"><div class="floating-module-head"><span>Codex 5 小时余额</span><strong>{{remaining5h}}%</strong></div><div class="track"><span :style="{width:used5hPercent+'%'}"></span></div><div class="floating-module-head sub"><span>7 天滚动额度</span><strong>{{remaining7d}}%</strong></div><small>下一笔额度释放倒计时 {{codex7dResetText}}</small></template>
    <template v-else-if="id==='modelBalances'"><div class="floating-module-head"><span>各模型分项余额</span><strong>{{modelBalanceRows.length}}</strong></div><div class="model-balance-list"><span v-if="!modelBalanceRows.length">尚未添加模型仪表盘</span><span v-for="row in modelBalanceRows"><b>{{row.label}}</b><small>{{row.value}} · 官方余额视接口权限</small></span></div></template>
    <template v-else-if="id==='warning'"><div class="balance-alert" :class="isDeepSeekDashboard?(dashboardBalance?.is_available?'normal':'critical'):balanceLevel">{{floatingModelNotice}}</div></template>
    <template v-else-if="id==='cacheChart'"><div class="floating-module-head"><span>缓存命中率</span><strong>{{dashboardCacheRate}}%</strong></div><div class="floating-ring floating-interactive-chart" tabindex="0" :aria-label="`缓存命中率 ${dashboardCacheRate}%，命中 ${dashboardCached.toLocaleString()} Token`" :style="{background:`conic-gradient(#111 ${dashboardCacheRate}%,#e5e5ea 0)`}"><i></i><b>{{dashboardCacheRate}}%</b><em class="floating-chart-tooltip"><b>缓存命中</b><span>{{dashboardCached.toLocaleString()}} / {{dashboardInput.toLocaleString()}} Token</span></em></div></template>
    <template v-else-if="id==='costChart'"><div class="floating-module-head"><span>7 天消费金额</span><strong>¥{{floatingWeekCost.toFixed(2)}}</strong></div><div class="floating-chart-bars floating-interactive-chart" role="img" aria-label="7 天消费金额柱状图"><span v-for="bar in modelChartSeries" class="floating-chart-point" tabindex="0" :aria-label="`${bar.day}，人民币 ${bar.cost.toFixed(4)} 元`"><i :style="{height:bar.costHeight+'%'}"></i><em class="floating-chart-tooltip"><b>{{bar.day}}</b><span>¥{{bar.cost.toFixed(4)}}</span></em></span></div></template>
    <template v-else-if="id==='requestChart'"><div class="floating-module-head"><span>7 天 API 请求</span><strong>{{floatingTodayCalls}}</strong></div><div class="floating-chart-bars floating-interactive-chart" role="img" aria-label="7 天 API 请求柱状图"><span v-for="bar in modelChartSeries" class="floating-chart-point" tabindex="0" :aria-label="`${bar.day}，${bar.calls} 次请求`"><i :style="{height:bar.callsHeight+'%'}"></i><em class="floating-chart-tooltip"><b>{{bar.day}}</b><span>{{bar.calls}} 次</span></em></span></div></template>
    <template v-else-if="id==='tokenTrendChart'"><div class="floating-module-head"><span>7 天 Token 趋势</span><strong>{{floatingTokenTotal7d.toLocaleString()}}</strong></div><div class="floating-line-wrap floating-interactive-chart" role="img" aria-label="7 天 Token 趋势折线图"><svg class="floating-line-chart" viewBox="0 0 160 56" preserveAspectRatio="none"><polyline :points="floatingTokenPoints" fill="none" stroke="currentColor" stroke-width="2.5" vector-effect="non-scaling-stroke"/></svg><span v-for="point in floatingTokenPlot" class="floating-line-point floating-chart-point" tabindex="0" :style="{left:point.x+'%',top:point.y+'%'}" :aria-label="`${point.day}，${point.value.toLocaleString()} Token`"><i></i><em class="floating-chart-tooltip"><b>{{point.day}}</b><span>{{point.value.toLocaleString()}} Token</span></em></span></div></template>
    <template v-else><div class="floating-module-head"><span>{{selectedDashboard==='__all__'?'今日调用总次数':selectedDashboard==='__codex__'?'Codex 今日 turn':'当前模型今日调用'}}</span><strong><AnimatedNumber :value="floatingTodayCalls" /></strong></div><small v-if="selectedDashboard==='__all__'">API 调用 {{todayCalls-todayCodexTurns}} 次 · Codex turn {{todayCodexTurns}} 次</small><small v-else-if="selectedDashboard==='__codex__'">本地日志 turn 数</small><small v-else>{{selectedDashboardDef?.provider}} · {{selectedDashboardDef?.model}}</small></template>
   </section>
  </TransitionGroup>
  <div v-if="!floatingCollapsed&&floatingCustomizing" class="floating-card-library"><span>添加模块</span><button v-for="id in disabledFloatingModules" :key="id" @click="toggleFloatingModuleValue(id,true)">＋ {{floatingModuleTitle(id)}}</button><small v-if="!disabledFloatingModules.length">所有模块都已添加</small></div>
  <small class="floating-sync" role="status" aria-live="polite"><span class="status-inline" :class="{'is-live':proxyEndpoints.length}"><ProxyGatewayIcon class="compact" /><b>代理</b><em>{{proxyEndpoints.length?'实时':'待启动'}}</em></span><span class="status-inline cc-switch-inline" :class="{'is-live':ccSwitch.local_routing}"><img class="cc-switch-mark compact" :src="ccSwitchOfficialLogo" alt="" aria-hidden="true"><b>CC Switch</b><em>{{ccSwitch.local_routing?'路由就绪':ccSwitch.running?'进程运行':'待机'}}</em></span><time>{{nextRefreshText}}</time></small>
  <i v-if="!floatingCollapsed" class="floating-resize-handle" title="拖动调整悬浮窗大小" role="separator" tabindex="0" aria-label="调整悬浮窗大小，可使用方向键" @keydown="resizeFloatingByKeyboard" @pointerdown="startFloatingResize" @pointermove="moveFloatingResize" @pointerup="endFloatingResize" @pointercancel="endFloatingResize"></i>
 </main> -->
 <main v-if="floatingMode" class="floating-v3-host" :class="[`theme-${appTheme.id}`,`glass-quality-${glassQuality}`,{'theme-dark':appThemeDark}]" :style="appThemeStyle">
  <InteractionEffects />
  <FloatingWindowV3
   :mode="floatingConfig.mode"
   :rows="floatingVisibleViews"
   :selected-key="selectedDashboard"
    :expanded-keys="floatingConfig.expandedKeys"
    :pinned-keys="floatingConfig.pinnedKeys"
    :allow-multiple-expanded="floatingConfig.allowMultipleExpanded"
   :metric="floatingChartMetric"
   :available-metrics="availableFloatingMetrics"
   :syncing="isSyncing"
   :next-refresh="nextRefreshText"
   :refreshed-text="refreshedText"
   :proxy-label="proxyConnectionLabel"
   :proxy-active="proxyRecentlyActive||proxyEndpoints.length>0"
   :cc-label="ccSwitch.local_routing?'已接管':ccSwitch.running?'运行中':'待机'"
   :cc-active="ccSwitch.local_routing"
   :always-on-top="floatingConfig.alwaysOnTop"
   :interaction="floatingConfig.interaction"
   :render-status="floatingRenderStatus"
   :logo-url="tokenManagerLogo"
   :codex-logo-url="codexOfficialLogo"
   @drag-start="startFloatingDrag"
   @resize-start="startFloatingResizeDirection"
   @select="selectDashboard"
   @toggle-expand="toggleFloatingDashboard"
   @set-metric="setFloatingChartMetric"
   @set-mode="setFloatingMode"
   @refresh="syncFloatingData"
   @close="closeFloating"
   @toggle-always-on-top="toggleFloatingAlwaysOnTop"
    @set-interaction="setFloatingInteraction"
    @reorder="reorderFloatingRows"
    @toggle-pin="toggleFloatingPin"
   />
 </main>
 <main v-if="!floatingMode" class="shell" :class="[`mode-${globalMode}`,`theme-${appTheme.id}`,`glass-quality-${glassQuality}`,{'theme-dark':appThemeDark}]" :style="appThemeStyle">
  <UpdateNotice />
 <InteractionEffects />
 <OnboardingFlow :open="onboardingOpen" :detected="{codex:Boolean(codexTokens||codexQuota),claude:Boolean(claudeUsage.length),opencode:syncHealthItems.some(item=>item.source_id==='opencode-local'&&item.status==='healthy')}" @finish="finishOnboarding('finish')" @skip="finishOnboarding('skip')" @open-accounts="page='accounts';onboardingOpen=false" @select-mode="setGlobalMode" @detect="refreshSyncHealth" />
 <SyncHealthPanel :open="syncHealthOpen" :items="syncHealthItems" :syncing="isSyncing||opencodeLocalSyncing" @close="syncHealthOpen=false" @refresh="sourceId=>sourceId?syncReliabilitySource(sourceId):syncData()" />
 <LiquidGlassEnvironment v-if="appTheme.material==='liquid'" :quality="glassQuality" :distortion="glassDistortion" :background-url="effectiveLiquidBackgroundImage" :video-path="liquidBackgroundVideoPath" />
  <EdgeScrollRail />
  <div class="app-titlebar" data-liquid-surface="titlebar" data-tauri-drag-region @dblclick="toggleMainWindowMaximize">
   <div class="app-titlebar-brand" data-tauri-drag-region><img :src="tokenManagerLogo" alt=""><span data-tauri-drag-region>Token Manager</span></div>
   <div class="app-window-controls" aria-label="窗口控制">
    <button class="app-window-control" type="button" title="最小化" aria-label="最小化窗口" @click="minimizeMainWindow"><Minus aria-hidden="true" /></button>
    <button class="app-window-control" type="button" title="最大化或还原" aria-label="最大化或还原窗口" @click="toggleMainWindowMaximize"><Maximize2 aria-hidden="true" /></button>
    <button class="app-window-control app-window-close" type="button" title="关闭到托盘" aria-label="关闭到系统托盘" @click="closeMainWindow"><X aria-hidden="true" /></button>
   </div>
  </div>
  <aside data-liquid-surface="primary"><div class="brand"><img class="brand-logo" :src="tokenManagerLogo" alt="Token Manager"><span>Token Manager</span><small>v{{appVersion}}</small></div><p class="tagline">你的本地 AI 控制中心</p>
   <nav aria-label="主导航"><button v-for="item in navItems" :key="item.id" :class="{active:page===item.id}" :aria-current="page===item.id?'page':undefined" @click="page=item.id"><component :is="item.icon" class="nav-icon" aria-hidden="true" :size="18" :stroke-width="1.8" /><span>{{item.label}}</span></button></nav>
   <div class="sidebar-material-footer" data-liquid-surface>
    <div class="sidebar-material-status"><i aria-hidden="true"></i><span><b>{{appTheme.name}}</b><small>{{appTheme.material==='liquid'?'实时折射材质':'本地轻量模式'}}</small></span></div>
    <div class="privacy">⌁ 所有数据仅保存在本机<br>密钥受 Windows 加密保护</div>
   </div>
  </aside>
   <section class="content">
      <header><div class="header-copy"><h1>{{ {command:'AI 控制中心',dashboard:'模型用量',prompts:'Prompt 中心',arena:'Arena 排行榜',accounts:'账户与模型',reports:'报告中心',floating:'悬浮窗',settings:'设置'}[page] }}</h1><p role="status" aria-live="polite">电脑时间 {{currentTimeText}} · 上次刷新 {{refreshedText}} · {{nextRefreshText}}</p></div><div class="header-actions"><SuccessBurst :trigger="successBurstKey" /><ThemeSwitcher /><button class="health-launch" :class="{warning:syncHealthItems.some(item=>item.status==='error')}" type="button" title="查看所有数据源的同步状态" aria-label="打开监控健康中心" @click="syncHealthOpen=true"><span><i aria-hidden="true"></i><b>监控健康</b></span><small>{{syncHealthItems.filter(item=>item.status==='healthy').length}}/{{syncHealthItems.length||0}} 正常</small></button><button class="monitor-launch" :class="{running:proxyEndpoints.length,receiving:proxyRecentlyActive}" :disabled="autoConnectBusy" @click="monitorPrimaryAction"><ProxyGatewayIcon /><span class="monitor-launch-copy"><b>{{proxyRecentlyActive?'正在监控':autoConnectBusy?'正在自动接入':clientIntegrationConnected?'已自动接入':proxyEndpoints.length?'自动接入':'开启并自动接入'}}</b><small>{{proxyRecentlyActive?`${lastProxyTraffic?.provider} · ${lastProxyTraffic?.model}`:clientIntegrationConnected?'重启调用工具后自动统计':proxyEndpoints.length?'无需手动填写 Base URL':'本机转发 · 自动配置'}}</small></span></button><button class="sync" :class="{loading:isSyncing}" :disabled="isSyncing" @click="syncData(true)"><span class="sync-icon" aria-hidden="true">↻</span>{{isSyncing?'正在同步…':'同步数据'}}</button></div></header>
    <section v-if="page==='dashboard'||page==='accounts'" class="monitor-status-bar" :class="{'is-live':proxyRecentlyActive||ccSwitch.local_routing,'is-proxy-live':proxyRecentlyActive,'is-proxy-ready':proxyEndpoints.length&&!proxyRecentlyActive,'is-cc-live':ccSwitch.local_routing}" role="status" aria-live="polite">
     <div class="monitor-status-brand"><span class="monitor-status-icon"><ProxyGatewayIcon /></span><SupplementalHelp help-id="monitor-route-detail" kind="status" :detail="allProxyStatus||proxyConnectionDetail"><span class="monitor-status-copy"><small>实时监控链路</small><b>{{proxyRecentlyActive?'API 用量正在实时接收':clientIntegrationConnected?'调用工具已自动接入':proxyEndpoints.length?'代理已启动，可自动接入':ccSwitch.local_routing?'CC Switch 路由已接管':'等待建立数据通道'}}</b></span></SupplementalHelp></div>
    <div class="monitor-status-signals">
     <span class="monitor-signal" :class="{active:proxyRecentlyActive,ready:proxyEndpoints.length&&!proxyRecentlyActive}"><i></i><span><small>API 代理</small><b>{{proxyConnectionLabel}}</b></span></span>
     <span class="monitor-signal" :class="{active:ccSwitch.local_routing}"><img :src="ccSwitchOfficialLogo" alt="CC Switch"><span><small>CC Switch</small><b>{{ccSwitch.local_routing?'路由已接管':ccSwitch.running?'进程运行':ccSwitch.installed?'待启动':'未安装'}}</b></span></span>
    </div>
    <button class="monitor-status-action" :disabled="isSyncing||autoConnectBusy" @click="monitorPrimaryAction">{{autoConnectBusy?'接入中…':!proxyEndpoints.length?'开启并自动接入':!proxyRecentlyActive?'自动接入':'立即刷新'}}</button>
    <div v-if="unattributedSpend" class="proxy-unattributed-alert">{{unattributedSpend}}</div>
    <div v-if="ccSwitch.conflicting_variables.length" class="cc-conflict-alert"><span><b>检测到旧版全局代理冲突</b><small>{{ccSwitch.conflicting_variables.join('、')}} · CC Switch 共存模式不会覆盖全局环境</small></span><button v-if="ccSwitch.safe_repair_available" type="button" @click="repairCcSwitch">一键安全修复</button><em v-else>请先查看冲突项，软件不会自动删除</em></div>
    <details v-if="proxyEndpoints.length" class="monitor-endpoints"><summary>Agent 接入账户与高级手动地址</summary><div><article v-for="endpoint in proxyEndpoints" :key="endpoint.local_url"><span><b>{{endpoint.name}}</b><small>OpenAI 兼容 · {{endpoint.account_id===autoConnectedAccountId?'当前 Agent 已接入':'可选择接入'}}</small><code>{{endpoint.local_url}}</code></span><button type="button" :disabled="autoConnectBusy||!selectedDashboardDef?.agentId" @click="autoConnectEndpoint(endpoint,selectedDashboardDef?.agentId)">接入当前 Agent</button><button type="button" @click="copyProxyUrl(endpoint.local_url)">复制</button><template v-if="endpoint.anthropic_url"><span><small>Claude Code / Anthropic 兼容</small><code>{{endpoint.anthropic_url}}</code></span><button type="button" @click="copyProxyUrl(endpoint.anthropic_url)">复制</button></template></article></div></details>
   </section>
   <section v-if="activeAnnouncement" class="remote-content remote-announcement" aria-label="软件公告">
    <div class="remote-content-badge">公告</div><div class="remote-content-copy"><b>{{activeAnnouncement.title}}</b><p>{{activeAnnouncement.body}}</p></div><button v-if="activeAnnouncement.action_url" class="remote-content-action" @click="openRemoteItem(activeAnnouncement)">{{activeAnnouncement.action_label||'查看详情'}}</button><button class="remote-content-close" aria-label="关闭公告" title="关闭公告" @click="dismissRemoteItem(activeAnnouncement.id)">×</button>
   </section>
   <section v-if="activeAd" class="remote-content remote-ad" aria-label="广告">
    <div class="remote-content-badge">广告</div><div class="remote-content-copy"><b>{{activeAd.title}}</b><p>{{activeAd.body}}</p></div><button v-if="activeAd.action_url" class="remote-content-action" @click="openRemoteItem(activeAd)">{{activeAd.action_label||'了解详情'}}</button><button class="remote-content-close" aria-label="关闭广告" title="关闭广告" @click="dismissRemoteItem(activeAd.id)">×</button>
   </section>
   <div :key="page" class="page-panel">
    <CommandCenter v-if="page==='command'" :today-tokens="todayTotalTokens" :today-calls="todayCalls" :today-cost="combinedUsage.filter(item=>localDayKey(new Date(item.at))===localDayKey(new Date(now))).reduce((sum,item)=>sum+item.cost,0)" :active-models="activeModelCount" :account-count="savedAccounts.length" :proxy-count="proxyEndpoints.length" :cc-connected="ccSwitch.local_routing" :codex-remaining="remaining7d" :week-cost="weekCost" :monthly-budget="monthlyBudget" :activities="commandActivities" @open-dashboard="page='dashboard'" @open-prompts="page='prompts'" @open-arena="page='arena'" @start-proxy="startAllProxies" />
    <PromptCenter v-if="page==='prompts'" />
<ArenaCenter v-if="page==='arena'" ref="arenaCenter" />
    <ReportCenter v-if="page==='reports'" :usage="usage" @export-all="exportBill()" @export-model="exportBill" />
    <SettingsCenter v-if="page==='settings'||page==='floating'" :initial-section="page==='floating'?'floating':'general'" :single-section="page==='floating'"
     :floating-enabled="floatingEnabled" :floating-items="floatingModuleCatalog" :floating-order="floatingConfig.order" :floating-selected="floatingConfig.enabled" :floating-layout="floatingConfig.layout" :floating-mode="floatingConfig.mode" :floating-interaction="floatingConfig.interaction" :floating-always-on-top="floatingConfig.alwaysOnTop" :floating-preview-rows="floatingDashboardViews" :floating-selected-key="selectedDashboard" :floating-visible-keys="floatingConfig.visibleKeys" :floating-pinned-keys="floatingConfig.pinnedKeys" :floating-allow-multiple-expanded="floatingConfig.allowMultipleExpanded" :floating-sizes="floatingConfig.sizes" :floating-metric="floatingChartMetric" :floating-render-status="floatingRenderStatus" :token-manager-logo="tokenManagerLogo" :codex-logo="codexOfficialLogo" :mini-mode="floatingConfig.mode==='capsule'" :mini-modules="floatingConfig.mini"
     :budget5h="budget5h" :budget7d="budget7d" :budget-status="budgetSaved" :proxy-upstream="proxyUpstream" :proxy-port="proxyPort" :proxy-status="proxyStatus" :backup-password="backupPassword" :backup-status="backupStatus" :account-count="savedAccounts.length" :dashboard-count="modelDashboards.length" :proxy-endpoint-count="proxyEndpoints.length" :cc-switch-installed="ccSwitch.installed" :cc-switch-running="ccSwitch.running" :cc-switch-routing="ccSwitch.local_routing" :cc-switch-detail="ccSwitch.detail" :cloud-session="cloudSession" :cloud-status="cloudStatus" :cloud-transfer-link="cloudTransferLink"
     @toggle-floating="toggleFloating" @toggle-floating-item="toggleFloatingModuleValue" @reorder-floating-item="(source,target)=>moveFloatingModuleTo(source as FloatingModuleId,target as FloatingModuleId)" @set-floating-layout="setFloatingLayout" @set-floating-mode="setFloatingMode" @set-floating-visible="setFloatingVisible" @reorder-floating-row="reorderFloatingRows" @toggle-floating-pin="toggleFloatingPin" @set-floating-multiple-expanded="checked=>{floatingConfig.allowMultipleExpanded=checked;persistFloatingConfig()}" @set-floating-size="setFloatingSize" @set-floating-interaction="setFloatingInteraction" @toggle-floating-always-on-top="toggleFloatingAlwaysOnTop" @set-mini-mode="setMiniModeValue" @set-mini-module="updateMiniModuleValue" @save-budget="saveBudgetFromSettings" @start-proxy="startProxyFromSettings" @export-backup="exportBackupFromSettings" @import-backup="importBackupFromSettings" @cloud-request-code="requestCloudCode" @cloud-password-login="cloudPasswordLogin" @cloud-password-register="cloudPasswordRegister" @cloud-code-login="cloudCodeLogin" @cloud-logout="logoutCloud" @cloud-create-transfer="createCloudTransfer" @cloud-import-transfer="importCloudTransfer" @open-accounts="page='accounts'" @start-all-proxies="startAllProxies" @reopen-onboarding="onboardingOpen=true" />
    <template v-if="page==='dashboard'">
    <section class="dashboard-switcher" aria-label="模型仪表盘切换" @wheel="scrollDashboardSwitcher"><button :class="{active:selectedDashboard==='__all__'}" :aria-pressed="selectedDashboard==='__all__'" @click="selectDashboard('__all__')">全部模型</button><button class="codex-dashboard-tab" :class="{active:selectedDashboard==='__codex__'}" :aria-pressed="selectedDashboard==='__codex__'" @click="selectDashboard('__codex__')">Codex 专属</button><button class="claude-dashboard-tab" :class="{active:selectedDashboard==='__claude__'}" :aria-pressed="selectedDashboard==='__claude__'" @click="selectDashboard('__claude__')">Claude Code 专属</button><div v-for="item in modelDashboards" :key="item.key" class="dashboard-tab" :class="{active:selectedDashboard===item.key}"><button class="dashboard-tab-label" :aria-pressed="selectedDashboard===item.key" @click="selectDashboard(item.key)">{{item.label}}</button><button class="dashboard-remove" type="button" :title="`删除 ${item.label} 仪表盘`" :aria-label="`删除 ${item.label} 仪表盘`" @click.stop="removeDashboard(item.key)"><span aria-hidden="true">×</span></button></div><button class="add-dashboard" @click="openDashboardPicker">＋ 添加仪表盘</button></section>
 <section class="dashboard-context-heading" data-liquid-ignore><div><img v-if="selectedDashboard==='__codex__'" class="codex-official-mark" :src="codexOfficialLogo" alt="Codex 官方标志" /><ProviderMark v-else-if="isClaudeDashboard" name="Anthropic" /><ProviderMark v-else-if="isApiDashboard" :name="selectedDashboardDef?.provider||''" /><div><span class="eyebrow">{{selectedDashboard==='__all__'?'全平台分析':selectedDashboard==='__codex__'?'Codex 本地分析':selectedDashboard==='__claude__'?'Claude Code 本地分析':`${selectedDashboardDef?.provider} · 独立模型`}}</span><SupplementalHelp help-id="dashboard-heading-detail" detail="图表优先展示；下方再呈现 Token、请求、缓存、余额和来源说明。"><h2>{{dashboardTitle}}</h2></SupplementalHelp></div></div><span>{{selectedDashboard==='__codex__'?codexSeries.length:dashboardUsage.length}} 条真实记录</span></section>
     <DataSourceBadge class="dashboard-source-badge" :kind="dashboardSourceMeta.kind" :accuracy="dashboardSourceMeta.accuracy" :collected-at="dashboardSourceMeta.collectedAt" />
     <section v-if="!(selectedDashboard==='__codex__'?codexSeries.length:selectedDashboard==='__all__'?combinedUsage.length:dashboardUsage.length)" class="dashboard-zero-state"><b>还没有真实用量数据</b><p>添加账户、开启本地代理，或连接 Codex、Claude Code、OpenCode 本地数据后，这里才会生成真实图表。</p><button @click="page='accounts'">连接数据源</button></section>
     <AnalyticsCharts v-else :usage="selectedDashboard==='__all__'?combinedUsage:dashboardUsage" :codex-series="selectedDashboard==='__all__'||selectedDashboard==='__codex__'?codexSeries:[]" :context="selectedDashboard==='__all__'?'all':selectedDashboard==='__codex__'?'codex':'model'" :now="now" :monthly-budget="monthlyBudget" :used5h="100-remaining5h" :budget5h="100" :used7d="100-remaining7d" :budget7d="100" />
     <div v-if="selectedDashboard==='__all__'" class="dashboard-heading"><div><span class="eyebrow">模型仪表盘</span><h2>{{dashboardTitle}}</h2></div><span>{{dashboardUsage.length}} 条真实调用</span></div>
    <section v-if="selectedDashboard==='__all__'" class="all-dashboard">
     <div class="all-summary"><div><span>今日全平台总 Token</span><strong><AnimatedNumber :value="todayTotalTokens" /></strong><p>Codex 本地日志与所有 API 代理调用的统一总览</p></div><dl><div><dt>API 请求</dt><dd><AnimatedNumber :value="dashboardToday.length" /></dd></div><div><dt>活跃模型</dt><dd><AnimatedNumber :value="activeModelCount" /></dd></div><div><dt>今日成本</dt><dd><AnimatedNumber :value="dashboardCost" :decimals="2" prefix="¥" /></dd></div><div><dt>已连接账户</dt><dd><AnimatedNumber :value="savedAccounts.length" /></dd></div></dl></div>
      <div class="all-overview-grid">
       <section class="today-models overview-block">
        <div class="section-title"><div><h2>今日使用模型</h2><span>彩色品牌标志 · 按 Token 从高到低</span></div><strong>{{todayUsedProviders.length}} 个活跃</strong></div>
        <div v-if="todayUsedProviders.length" class="today-logo-list">
         <div v-for="item in todayUsedProviders" :key="item.provider" class="today-model-item" :title="`${item.provider} · ${item.tokens.toLocaleString()} Token · ${item.calls} 次`">
          <img v-if="item.provider==='Codex'" class="today-codex-logo" :src="codexOfficialLogo" alt="Codex"><ProviderMark v-else :name="item.provider" />
          <span><b>{{item.provider}}</b><small>{{item.tokens.toLocaleString()}} Token</small></span><em>{{item.calls}} 次</em>
         </div>
        </div>
        <div v-else class="today-model-empty"><img :src="tokenManagerLogo" alt=""><span><b>今天尚未记录调用</b><small>启动代理或使用 Codex / Claude Code 后会自动出现。</small></span></div>
       </section>
       <section class="usage-composition overview-block">
        <div class="section-title"><div><h2>模型用量构成</h2><span>近 30 天 Token 占比</span></div><span>从高到低</span></div>
       <div v-if="providerBreakdown.length" class="provider-ranking">
        <div v-for="row in providerBreakdown" :key="row.provider" class="provider-ranking-row">
         <div class="provider-ranking-identity"><ProviderMark :name="row.provider" /><span><b :title="row.provider">{{row.provider}}</b><small>{{row.calls}} 次 · ¥{{row.cost.toFixed(2)}}</small></span></div>
         <div class="provider-ranking-meter" :aria-label="`${row.provider} 占比 ${row.percent.toFixed(1)}%`"><i><em :style="{width:row.percent+'%',background:providerColor(row.provider)}"></em></i><small>{{row.percent.toFixed(1)}}%</small></div>
          <strong :title="`${row.tokens.toLocaleString()} Token`"><span>{{row.tokens.toLocaleString()}}</span><small>Token</small></strong>
        </div>
       </div>
       <div v-else class="inline-empty">开启 API 实时监控后，这里会显示各平台占比。</div>
      </section>
       <aside class="monitor-aside overview-block">
        <div class="monitor-section-heading"><div><h2>数据通道</h2><small>状态只在发生变化时更新</small></div><span>{{proxyEndpoints.length||ccSwitch.local_routing?'已就绪':'待连接'}}</span></div>
        <div class="connection-state" :class="{'is-live':proxyRecentlyActive,'is-ready':proxyEndpoints.length&&!proxyRecentlyActive}"><div class="status-title"><ProxyGatewayIcon /><span>API 代理</span><em>{{proxyConnectionLabel}}</em></div><b>{{proxyRecentlyActive?'调用已经进入 Token Manager':clientIntegrationConnected?'调用工具连接配置已完成':proxyEndpoints.length?`${proxyEndpoints.length} 条通道可自动接入`:'当前未建立数据通道'}}</b><small>{{proxyConnectionDetail}}</small></div>
       <div class="cc-switch-state" :class="{'is-live':ccSwitch.local_routing,'is-running':ccSwitch.running}"><div class="status-title"><img class="cc-switch-mark" :src="ccSwitchOfficialLogo" alt="CC Switch 官方标志"><span>CC Switch</span><em>{{ccSwitch.local_routing?'路由就绪':ccSwitch.running?'进程运行':ccSwitch.installed?'待启动':'未安装'}}</em></div><b>{{ccSwitch.local_routing?'本地路由已建立':ccSwitch.running?'等待接管本地路由':ccSwitch.installed?'当前未运行':'未检测到客户端'}}</b><small>{{ccSwitch.detail}}</small></div>
       <button v-if="!proxyEndpoints.length" @click="startAllProxies()">立即开启实时监控</button>
      </aside>
     </div>
    </section>
     <section v-if="selectedDashboard==='__all__'" class="card all-token-history legacy-chart-section">
     <div class="token-chart-header"><div><h2>Token 消耗记录</h2><p>输入与输出 Token 合计；缓存命中属于输入的一部分，不重复计算。</p></div><div class="token-range" aria-label="Token 图表时间范围"><button :class="{active:tokenChartRange===7}" :aria-pressed="tokenChartRange===7" @click="setTokenChartRange(7)">7 天</button><button :class="{active:tokenChartRange===30}" :aria-pressed="tokenChartRange===30" @click="setTokenChartRange(30)">30 天</button></div></div>
     <div class="token-chart-summary"><span><small>周期累计</small><b>{{allTokenChart.total.toLocaleString()}}</b></span><span><small>日均 Token</small><b>{{tokenChartDailyAverage.toLocaleString()}}</b></span><span><small>峰值日期</small><b>{{allTokenChart.peak?.day||'—'}}</b></span><span><small>活跃 AI</small><b>{{allTokenChart.activeProviders}}</b></span></div>
     <div v-if="allTokenChart.total" class="stacked-token-chart" :class="{'is-30-days':tokenChartRange===30}">
      <div v-for="bar in allTokenChart.rows" :key="bar.key" class="token-day" tabindex="0" :aria-label="`${bar.day}，总计 ${bar.total.toLocaleString()} Token`">
       <div class="token-tooltip"><b>{{bar.day}} · {{bar.total.toLocaleString()}} Token</b><span v-for="segment in bar.segments" :key="segment.provider"><i :style="{background:segment.color}"></i>{{segment.provider}}<strong>{{segment.tokens.toLocaleString()}}</strong></span><small v-if="!bar.segments.length">当日没有调用</small></div>
       <div class="token-stack"><i v-for="segment in bar.segments" :key="segment.provider" :style="{height:segment.height+'%',background:segment.color}" :title="`${segment.provider} ${segment.tokens.toLocaleString()} Token`"></i></div>
       <small>{{bar.label||' '}}</small>
      </div>
     </div>
      <div v-else class="token-chart-empty"><b>还没有可绘制的真实 Token 记录</b><span>开启 API 实时监控或使用 Codex 后，数据会自动写入并在这里按天累计。</span><button v-if="!proxyEndpoints.length" @click="startAllProxies()">一键开启 API 实时监控</button></div>
     <div v-if="allTokenChart.legend.length" class="token-legend" aria-label="AI 颜色图例"><span v-for="item in allTokenChart.legend" :key="item.provider"><i :style="{background:item.color}"></i><ProviderMark :name="item.provider==='Codex'?'OpenAI':item.provider"/><b>{{item.provider}}</b><small>{{item.tokens.toLocaleString()}}</small></span></div>
    </section>
    <section v-else-if="selectedDashboard==='__codex__'" class="codex-dashboard">
<div class="codex-title"><div><span>Codex · 本地深度监控</span><SupplementalHelp help-id="codex-title-detail" kind="source" detail="优先使用 Codex 本地 rate_limits 事件；缺失的窗口才使用个人 Token 预算估算，全程不读取登录凭据。"><h2>直接读取客户端报告的额度状态</h2></SupplementalHelp></div><button class="outline" @click="syncData()">刷新 Codex 数据</button></div>
     <div class="codex-kpis"><article class="card"><span>累计 Token</span><strong><AnimatedNumber v-if="codexTokens!==null" :value="codexTokens" /><template v-else>—</template></strong><small>本地状态库 threads.tokens_used</small></article><article class="card"><span>当前活跃会话</span><strong><AnimatedNumber v-if="codexActiveTokens!==null" :value="codexActiveTokens" /><template v-else>—</template></strong><small>最近更新线程 Token</small></article><article class="card"><span>今日消耗</span><strong><AnimatedNumber :value="codexTodayTokens" /></strong><small>{{todayCodexTurns}} 个本地 turn</small></article><article class="card"><span>额度状态时间</span><strong class="time-value">{{codexQuota?.observed_at?new Date(codexQuota.observed_at*1000).toLocaleTimeString('zh-CN'):'—'}}</strong><small>{{codexQuotaObservedText}}</small></article></div>
     <div class="codex-quota-card card"><div><SupplementalHelp help-id="codex-5h-source" kind="source" :detail="codex5hIsReported?`客户端 rate_limits · 重置倒计时 ${codex5hResetText}`:`个人预算估算 · 本地已用 ${used5h.toLocaleString()} / ${budget5h.toLocaleString()} Token`"><span class="quota-source" :class="{reported:codex5hIsReported}">{{codex5hIsReported?'客户端报告':'个人预算估算'}}</span></SupplementalHelp><h2>5 小时滚动额度剩余</h2><strong><AnimatedNumber :value="remaining5h" suffix="%" /></strong><div class="track"><span :style="{width:used5hPercent+'%'}"></span></div></div><div><SupplementalHelp help-id="codex-7d-source" kind="source" :detail="codex7dIsReported?`客户端 rate_limits · 全额重置 ${codex7dResetText}`:`个人预算估算 · 下一笔额度释放 ${codex7dResetText}`"><span class="quota-source" :class="{reported:codex7dIsReported}">{{codex7dIsReported?'客户端报告':'个人预算估算'}}</span></SupplementalHelp><h2>7 天滚动额度剩余</h2><strong><AnimatedNumber :value="remaining7d" suffix="%" /></strong><div class="track"><span :style="{width:used7dPercent+'%'}"></span></div></div></div>
    </section>
    <section v-else-if="selectedDashboard==='__claude__'" class="provider-overview card claude-overview"><div class="provider-overview-title"><ProviderMark name="Anthropic" /><div><span class="eyebrow">CLAUDE CODE · 本地专属仪表盘</span><h2>Claude Code 用量</h2><p>自动解析 ~/.claude/projects 中的 usage 元数据；不读取对话正文，每 30 秒更新。</p></div></div><div class="provider-overview-grid"><span><small>今日总 Token</small><b><AnimatedNumber :value="dashboardInput+dashboardOutput" /></b></span><span><small>缓存命中</small><b><AnimatedNumber :value="dashboardCached" /></b></span><span><small>今日请求</small><b><AnimatedNumber :value="dashboardTodayCalls" /></b></span><span><small>最近调用</small><b>{{dashboardLastUsage?new Date(dashboardLastUsage.at).toLocaleTimeString('zh-CN',{hour:'2-digit',minute:'2-digit'}):'暂无'}}</b></span></div></section>
    <section v-else-if="isApiDashboard" class="provider-overview card"><div class="provider-overview-title"><ProviderMark :name="selectedDashboardDef?.provider||''" /><div><span class="eyebrow">{{selectedDashboardDef?.provider}} · {{isLocalAgentDashboard?'本地 Agent':'独立仪表盘'}}</span><h2>{{selectedDashboardDef?.model}}</h2><p>{{isLocalAgentDashboard?(selectedAgentSource?.path||'自动发现路径'):(dashboardAccount?.name||'已配置账户')}} · 仅显示当前数据源{{selectedDashboardDef?.kind==='local_agent_model'?'和模型':''}}，不混入其他 Agent。</p></div></div><div class="provider-overview-grid"><span><small>今日总 Token</small><b><AnimatedNumber :value="dashboardInput+dashboardOutput" /></b></span><span><small>缓存命中率</small><b><AnimatedNumber :value="dashboardCacheRate" suffix="%" /></b></span><span><small>今日请求</small><b><AnimatedNumber :value="dashboardTodayCalls" /></b></span><span><small>最近调用</small><b>{{dashboardLastUsage?new Date(dashboardLastUsage.at).toLocaleTimeString('zh-CN',{hour:'2-digit',minute:'2-digit'}):'暂无'}}</b></span></div></section>
    <CodexRetentionPanel v-if="selectedDashboard==='__codex__'" :points="codexSeries" :seven-day-used="used7d" :seven-day-budget="budget7d" :remaining5h="remaining5h" :remaining7d="remaining7d" :reset-text="codex7dResetText" />
    <section v-if="selectedDashboard==='__all__'" class="all-cost-analytics"><div class="card cost-share"><div class="section-title"><div><h2>本月模型花费占比</h2><span>仅统计已有价格来源的调用</span></div><strong>¥{{monthlyCostTotal.toFixed(2)}}</strong></div><div class="cost-share-body"><div class="cost-ring" :style="{background:monthlyCostGradient}"><span>已计价<br><b>{{monthlyProviderCosts.length}}</b> 个平台</span></div><div><div v-for="item in monthlyProviderCosts.slice(0,6)"><ProviderMark :name="item.provider"/><span><b>{{item.provider}}</b><small>¥{{item.cost.toFixed(2)}}</small></span><strong>{{monthlyCostTotal?Math.round(item.cost/monthlyCostTotal*100):0}}%</strong></div><p v-if="!monthlyProviderCosts.length">暂无已计价调用，图表不会填充模拟数据。</p></div></div></div><div class="card cost-trend"><div class="section-title"><div><h2>30 天消费趋势</h2><span>鼠标悬停模型明细可查看具体金额</span></div><small>{{priceCatalog.version}}</small></div><svg viewBox="0 0 600 150" preserveAspectRatio="none" aria-label="30天消费金额折线图"><polyline :points="monthlyCostTrend" fill="none" stroke="#111" stroke-width="3" vector-effect="non-scaling-stroke"/></svg><div><span>30 天前</span><span>今天</span></div></div></section>
    <AgentDrilldown v-if="isLocalAgentDashboard" :usage="dashboardUsage" :source-available="!syncHealthItems.some(item=>item.source_id===selectedDashboardDef?.sourceId&&item.status==='error')" />
    <section v-if="isDeepSeekDashboard" class="deepseek-monitor">
     <div class="deepseek-metrics">
      <article class="card balance-primary"><span>官方可用余额</span><strong><AnimatedNumber v-if="dashboardBalance" :value="dashboardBalance.total_balance" :decimals="2" prefix="¥" /><template v-else>—</template></strong><small>{{dashboardBalance?.is_available?'账户可正常调用':balanceError||'等待官方余额同步'}}</small></article>
      <article class="card"><span>充值余额</span><strong><AnimatedNumber v-if="dashboardBalance" :value="dashboardBalance.topped_up_balance" :decimals="2" prefix="¥" /><template v-else>—</template></strong><small>DeepSeek 官方返回</small></article>
      <article class="card"><span>赠送余额</span><strong><AnimatedNumber v-if="dashboardBalance" :value="dashboardBalance.granted_balance" :decimals="2" prefix="¥" /><template v-else>—</template></strong><small>未过期赠金</small></article>
      <article class="card"><span>本次监控变化</span><strong :class="{negative:balanceChange<0}"><AnimatedNumber :value="balanceChange" :decimals="2" prefix="¥" /></strong><small>最近 30 天本机采样</small></article>
     </div>
     <section class="card balance-chart"><div class="row"><div><span class="eyebrow">DEEPSEEK · 官方余额</span><h2>余额变化曲线</h2></div><small>{{dashboardBalance?'更新于 '+new Date(dashboardBalance.synced_at).toLocaleString('zh-CN'):'每 30 秒自动同步'}}</small></div><div v-if="deepseekBalanceHistory.length" class="balance-plot"><svg viewBox="0 0 600 160" preserveAspectRatio="none" aria-label="DeepSeek 余额变化曲线"><polyline :points="deepseekBalancePoints" fill="none" stroke="#111" stroke-width="4" vector-effect="non-scaling-stroke"/><circle v-for="(point,index) in deepseekBalanceHistory" :cx="deepseekBalanceHistory.length===1?300:index/(deepseekBalanceHistory.length-1)*600" :cy="140-(point.total_balance-Math.min(...deepseekBalanceHistory.map(x=>x.total_balance)))/Math.max(.01,Math.max(...deepseekBalanceHistory.map(x=>x.total_balance))-Math.min(...deepseekBalanceHistory.map(x=>x.total_balance)))*115" r="6"><title>{{new Date(point.at).toLocaleString('zh-CN')}} · ¥{{point.total_balance.toFixed(2)}}</title></circle></svg><div><span>起始 ¥{{deepseekBalanceHistory[0].total_balance.toFixed(2)}}</span><span>当前 ¥{{deepseekBalanceHistory[deepseekBalanceHistory.length-1].total_balance.toFixed(2)}}</span></div></div><div v-else class="balance-chart-empty">首次同步后开始记录真实余额变化，不使用模拟数据。</div></section>
     <p class="deepseek-source">余额来自 DeepSeek 官方 <code>/user/balance</code>；V4 Pro 本地代理按缓存命中、缓存未命中和输出 Token 分项计价。</p>
    </section>
    <section v-if="isOpenCodeGoDashboard" class="opencode-go-monitor">
     <div class="opencode-go-heading card">
      <div class="provider-overview-title"><ProviderMark name="OpenCode Go" /><div><span class="eyebrow">OPENCODE GO · 套餐额度</span><h2>本机观测额度</h2><p>API Key {{opencodeGoDiscoveredModels.length?'已验证':'等待验证'}} · {{opencodeGoDiscoveredModels.length||providerModels['OpenCode Go'].length}} 个模型可选</p></div></div>
      <button class="outline" @click="openUrl('https://opencode.ai/workspace')">查看官方精确用量 ↗</button>
     </div>
     <div class="opencode-go-quota-grid">
      <article v-for="window in opencodeGoQuotaWindows" :key="window.label" class="card opencode-go-quota-card">
       <div><span>{{window.label}}</span><b>剩余约 {{window.remainingPercent.toFixed(1)}}%</b></div>
       <strong>${{window.observedUsd.toFixed(3)}} <small>/ ${{window.limit}}</small></strong>
       <div class="opencode-go-quota-track" role="progressbar" :aria-label="`${window.label}本机观测额度占用`" :aria-valuenow="Math.round(window.usedPercent)" aria-valuemin="0" aria-valuemax="100"><i :style="{width:`${window.usedPercent}%`}"></i></div>
       <small>本机代理已观测 {{window.usedPercent.toFixed(1)}}%</small>
      </article>
     </div>
     <p class="opencode-go-source">套餐上限来自 OpenCode Go 官方公开规则：$12 / 5 小时、$30 / 周、$60 / 月。官方未公开仅凭 API Key 查询剩余额度的接口，因此这里不会把本机观测值伪装成官方余额。</p>
    </section>
<section v-if="isApiDashboard&&!dashboardUsage.length&&!dashboardBalance" class="card dashboard-empty" role="status"><span aria-hidden="true">◎</span><div><h2>尚未收到 {{selectedDashboardDef?.model}} 的调用数据</h2><p>{{isLocalAgentDashboard?'请先在对应 Agent 中产生一次真实调用，再检查数据路径或执行立即同步；此处不会填充模拟图表。':'点击“开启实时代理”建立本机端口；自动配置只作用于用户明确选择且结构已验证的 Agent。'}}</p><div class="empty-actions"><button v-if="!isLocalAgentDashboard" @click="startAllProxies()">开启实时代理</button><button class="outline" :disabled="isSyncing" @click="syncReliabilitySource(selectedDashboardDef?.sourceId||undefined)">{{isSyncing?'正在同步…':'立即同步'}}</button></div></div></section>
    <div v-if="isApiDashboard" class="model-kpis"><article class="card token-primary"><span>今日 Token</span><strong><AnimatedNumber :value="dashboardInput+dashboardOutput" /></strong><small>输入 {{dashboardInput.toLocaleString()}} · 输出 {{dashboardOutput.toLocaleString()}}</small></article><article class="card"><span>API 请求</span><strong><AnimatedNumber :value="dashboardTodayCalls" /></strong><small>成功 {{dashboardTodayCalls-dashboardFailedCalls}} · 失败 {{dashboardFailedCalls}}</small></article><article class="card"><span>缓存命中</span><strong><AnimatedNumber :value="dashboardCached" /></strong><small>命中率 {{dashboardCacheRate}}%</small></article><article class="card"><span>今日成本</span><strong><AnimatedNumber :value="dashboardCost" :decimals="4" prefix="¥" /></strong><small>{{dashboardCost?'按已验证价格':'尚未配置模型价格'}}</small></article><article class="card balance-compact"><span>账户余额</span><strong><AnimatedNumber v-if="dashboardBalance" :value="dashboardBalance.total_balance" :decimals="2" prefix="¥" /><template v-else>—</template></strong><small v-if="dashboardBalance">充值 ¥{{dashboardBalance.topped_up_balance.toFixed(2)}} · 赠送 ¥{{dashboardBalance.granted_balance.toFixed(2)}}</small><small v-else>{{balanceStatusText}}</small></article></div>
    <div v-if="isApiDashboard" class="model-chart-grid"><section v-for="chart in [{key:'costHeight',title:'消费金额',unit:'元'},{key:'tokenHeight',title:'Token 累计',unit:'Token'},{key:'callsHeight',title:'API 请求',unit:'次'},{key:'cacheHeight',title:'缓存命中',unit:'Token'}]" class="card metric-chart"><div class="row"><h2>{{chart.title}}</h2><small>近 7 天 · 悬停查看</small></div><div class="metric-bars"><div v-for="bar in modelChartSeries" class="metric-bar" tabindex="0"><div class="bar-tooltip"><b>{{bar.day}}</b><span v-if="chart.key==='costHeight'">¥{{bar.cost.toFixed(4)}}</span><span v-else-if="chart.key==='tokenHeight'">{{bar.cumulative.toLocaleString()}} Token</span><span v-else-if="chart.key==='callsHeight'">{{bar.calls}} 次</span><span v-else>{{bar.cached.toLocaleString()}} Token</span></div><i :style="{height:(bar as any)[chart.key]+'%'}"></i><small>{{bar.label}}</small></div></div></section></div>
    <section v-if="isApiDashboard" class="card advice model-advice"><h2>{{selectedDashboardDef?.model}} 数据说明</h2><p>{{isLocalAgentDashboard?'Token、请求次数和缓存来自本地文件或只读数据库；只有源数据真实提供额度时才显示额度。':'Token、请求次数和缓存命中来自本地代理响应；余额按平台官方接口能力单独标注。'}}</p><p v-for="text in dashboardAdvice">{{text}}</p></section>
    <div v-if="selectedDashboard==='__codex__'" class="grid"><section class="card chart"><div class="row"><h2>Codex · 近 7 天 turn 用量</h2><small class="chart-hint">悬停查看每日具体数据</small></div><div class="usage-bars"><div v-for="bar in chartBars" :key="bar.day" class="usage-bar" tabindex="0"><div class="bar-tooltip"><b>{{bar.day}}</b><span>Codex {{bar.codex.toLocaleString()}}</span><span>总量 {{bar.total.toLocaleString()}}</span></div><i :style="{height:bar.height+'%'}"></i><small>{{bar.label}}</small></div></div></section><section class="card advice"><h2>Codex 数据说明</h2><p v-for="text in dashboardAdvice">{{text}}</p></section></div>
    <section v-if="selectedDashboard==='__codex__'" class="card recent"><div class="row"><h2>最近 Codex turns</h2><small>本地日志，不含会话正文</small></div><table><thead><tr><th>时间</th><th>模型</th><th>Token</th><th>来源</th></tr></thead><tbody><tr v-for="point in [...codexSeries].reverse().slice(0,12)"><td>{{new Date(point.at*1000).toLocaleString('zh-CN')}}</td><td>{{point.model}}</td><td>{{point.tokens.toLocaleString()}}</td><td>Codex 本地日志</td></tr><tr v-if="!codexSeries.length"><td colspan="4" class="table-empty">尚未解析到 Codex turn 记录</td></tr></tbody></table></section><section v-else class="card recent"><div class="row"><h2>最近 API 请求</h2><button class="text" @click="exportBill">导出账单</button></div><table><thead><tr><th>平台 / 模型</th><th>状态</th><th>输入 / 输出 / 缓存</th><th>成本</th></tr></thead><tbody><tr v-for="u in dashboardUsage"><td><b>{{u.provider}}</b><small>{{u.model}}</small></td><td>{{u.task==='失败'?'失败':'成功'}}</td><td>{{u.input.toLocaleString()}} / {{u.output.toLocaleString()}} / {{u.cached.toLocaleString()}}</td><td>{{u.cost?'¥'+u.cost.toFixed(4):'未计价'}}</td></tr><tr v-if="!dashboardUsage.length"><td colspan="4" class="table-empty">尚无真实代理请求记录</td></tr></tbody></table></section>
   </template>
    <template v-else-if="page==='accounts'">
     <div class="account-intro"><div><h2>接入你的模型账户</h2><p class="intro">密钥永久保存为 Windows DPAPI 密文；应用重启后自动恢复账户和模型选项。</p></div><button v-if="savedAccounts.length" class="danger-outline" @click="clearSavedAccounts">清空全部密钥</button></div>
     <section class="local-agent-center card" aria-labelledby="local-agent-title">
      <div class="local-agent-heading"><div><span class="eyebrow">无需 API Key</span><h2 id="local-agent-title">本地 Agent 自动监控</h2><SupplementalHelp summary="仅读取本地用量元数据" detail="读取 Token、模型、请求、缓存、时间和会话标识；不读取提示词、回复正文、代码、Cookie 或认证内容。自动路径会在每台电脑按当前 Windows 用户重新解析。" /></div><button class="outline" :disabled="modelsLoading" @click="refreshSyncHealth">{{modelsLoading?'检测中…':`重新检测 ${localAgents.length || 27} 款 Agent`}}</button></div>
      <div class="local-agent-grid">
       <article v-for="agent in localAgents" :key="agent.id" class="local-agent-card" :class="{detected:agent.detected,enabled:agentSourceFor(agent)?.enabled}">
        <div class="local-agent-identity"><ProviderMark :name="agent.provider" /><span><b>{{agent.name}}</b><small>{{agent.collector_kind.toUpperCase()}} · {{agent.support_state==='detected_only'?'仅检测安装':agent.support_state==='experimental'?'实验适配':agent.schema_version?'结构已识别':'等待识别'}}</small></span><i :title="agent.detected?'已发现本机数据':'未自动发现'">{{agent.detected?'已发现':'未发现'}}</i></div>
        <SupplementalHelp :summary="agent.detail" :detail="`${agent.detail}；自动路径不会绑定当前电脑用户名。`" />
        <div class="agent-capabilities"><span v-for="capability in agent.capabilities" :key="capability">{{capability}}</span></div>
        <code :title="agent.path_candidates[0]?.template">{{agentSourceFor(agent)?.path_mode==='manual'?(agentSourceFor(agent)?.path||'手动路径'):agent.path_candidates[0]?.template||'等待选择路径'}}</code>
        <details v-if="agentSourceFor(agent)?.path||agent.path_candidates[0]?.resolved_path" class="agent-actual-path"><summary>实际位置</summary><code>{{agentSourceFor(agent)?.path||agent.path_candidates[0]?.resolved_path}}</code></details>
        <small class="agent-action-status">{{agentActionStatus[agent.id]||syncHealthItems.find(item=>item.source_id===agentSourceFor(agent)?.id)?.detail||'每 30 秒安全补扫'}}</small>
        <div class="agent-actions" :class="{triple:Boolean(agent.proxy_protocol)}"><button class="outline" @click="chooseAgentSource(agent)">选择路径</button><button :disabled="agent.support_state==='detected_only'||(!agent.detected&&!agentSourceFor(agent)?.detected)" @click="toggleAgentSource(agent)">{{agent.support_state==='detected_only'?'暂无用量字段':agentSourceFor(agent)?.enabled?'暂停监控':'启用监控'}}</button><button v-if="agent.proxy_protocol" class="outline" title="仅配置当前 Agent 的安全 Base URL 链路，不进行 TLS 劫持" @click="startAllProxies(agent.id)">接入代理</button></div>
       </article>
      </div>
     </section>
     <div class="providers"><article class="card provider" v-for="p in accounts">
      <div class="provider-body"><div><div class="provider-head"><ProviderMark :name="p.name" /><h2>{{p.name==='OpenCode Go'?'OpenCode Go / Zen':p.name}}</h2></div><p>{{p.description}}</p><p v-if="p.name==='OpenCode Go'" class="opencode-go-account-note">Go 订阅与 Zen 网关分开配置；本地 Agent 统一显示为 OpenCode CLI</p><span class="badge" :class="p.source">{{p.source}}</span></div>
       <div v-if="savedAccounts.some(a=>a.provider===p.name)" class="saved-accounts"><div v-for="account in savedAccounts.filter(a=>a.provider===p.name)"><span><b>{{account.name}}</b><small>{{account.base_url}}</small></span><button class="icon-action" @click="openConfig(p,account)">编辑</button><button class="icon-action danger" @click="deleteSavedAccount(account.id)">删除</button></div></div>
       <div v-if="p.name==='OpenCode Go'" class="opencode-local-source"><div><b>本地用量记录</b><small>{{opencodeLocalStatus}}</small><code v-if="opencodeLocalPath">{{opencodeLocalPath}}</code></div><label class="opencode-source-mode">监测方式<select v-model="opencodeLocalMode" :disabled="opencodeLocalSyncing" @change="changeOpenCodeLocalMode"><option value="auto">自动（优先 SQLite）</option><option value="sqlite">仅 SQLite</option><option value="json">仅 JSON / JSONL</option></select></label><button class="outline" :disabled="opencodeLocalSyncing" @click="chooseOpenCodeLocalSource">{{opencodeLocalSyncing?'正在读取…':'选择目录并同步'}}</button></div>
      </div>
      <div class="account-actions"><button class="outline" @click="openProviderPortal(p)">官网 ↗</button><button @click="openConfig(p)">＋ {{p.name==='OpenCode Go'?'添加 Go / Zen API Key':'添加账户'}}</button></div>
     </article></div>
    </template>
   </div>
  </section>
  <Transition name="modal-fade"><div v-if="dashboardModal" class="modal-backdrop" @click.self="dashboardModal=false"><section class="config-modal compact dashboard-picker"><button class="close" @click="dashboardModal=false">×</button><span class="eyebrow">添加仪表盘</span><h2>选择 API 模型或本地 Agent</h2><div class="dashboard-picker-tabs" role="tablist"><button :class="{active:dashboardPickerTab==='api'}" role="tab" :aria-selected="dashboardPickerTab==='api'" @click="dashboardPickerTab='api';newDashboardKey=''">API 账户</button><button :class="{active:dashboardPickerTab==='agent'}" role="tab" :aria-selected="dashboardPickerTab==='agent'" @click="dashboardPickerTab='agent';newDashboardKey=''">本地 Agent</button></div><p v-if="modelsLoading">正在读取账户模型并检测本地 Agent…</p><template v-else-if="dashboardPickerTab==='api'"><p>{{availableModels.length?'已优先读取账户实际模型；不支持模型发现的平台显示内置目录。':'请先在“账户与模型”中添加一个 API 账户。'}}</p><select v-model="newDashboardKey" :disabled="!availableModels.length"><option value="">选择账户与模型</option><option v-for="item in availableModels" :key="item.key" :value="item.key">{{item.label}}</option></select></template><template v-else><p>{{availableAgentDashboards.length?'可添加整个 Agent 总览，也可添加已发现的单个模型。':'尚未检测到本地 Agent，请先到“账户与模型”选择数据路径。'}}</p><select v-model="newDashboardKey" :disabled="!availableAgentDashboards.length"><option value="">选择本地 Agent 或模型</option><option v-for="item in availableAgentDashboards" :key="item.key" :value="item.key">{{item.label}}</option></select></template><button :disabled="!newDashboardKey" @click="addDashboard">添加仪表盘</button></section></div></Transition>
  <Transition name="modal-fade"><div v-if="configuring" class="modal-backdrop" @click.self="configuring=null"><section class="config-modal"><button class="close" @click="configuring=null">×</button><div class="provider-head"><ProviderMark :name="configuring.name" /><div><span class="eyebrow">安全配置</span><h2>{{configuring.name==='OpenCode Go'?'OpenCode Go / Zen':configuring.name}}</h2></div></div><p>API Key 将使用当前 Windows 用户的 DPAPI 加密保存。</p><div v-if="configuring.name==='OpenCode Go'" class="opencode-go-config-note"><b>选择服务类型</b><div class="opencode-variant"><button :class="{active:configOpenCodeVariant==='go'}" @click="setOpenCodeVariant('go')">Go 订阅</button><button :class="{active:configOpenCodeVariant==='zen'}" @click="setOpenCodeVariant('zen')">Zen 网关</button></div><span>{{configOpenCodeVariant==='go'?'Go 使用 /zen/go/v1，并显示 Go 套餐语义。':'Zen 使用 /zen/v1，只展示代理实测用量，不套用 Go 额度。'}}</span></div><label>账户名称<input v-model.trim="configName"></label><label>Base URL<input v-model.trim="configUrl" placeholder="https://api.example.com"></label><label>API Key<input v-model="configKey" type="password" :placeholder="editingAccountId?'留空则继续使用已加密保存的密钥':'粘贴官网创建的密钥'"></label><button @click="saveConfig">{{configuring.name==='OpenCode Go'?`保存并验证 ${configOpenCodeVariant==='go'?'Go':'Zen'} API Key`:'加密保存账户'}}</button><small>{{configStatus}}</small></section></div></Transition>
 </main>
</template>

<style scoped>
.local-agent-center{display:grid;gap:18px;margin-bottom:18px}.local-agent-heading{display:flex;align-items:flex-start;justify-content:space-between;gap:20px}.local-agent-heading h2{margin:4px 0 6px}.local-agent-heading p{max-width:760px;margin:0;color:var(--tm-muted);font-size:12px;line-height:1.6}.local-agent-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:12px}.local-agent-card{display:grid;align-content:start;gap:11px;min-width:0;padding:16px;border:1px solid var(--tm-line);border-radius:16px;background:color-mix(in srgb,var(--tm-bg) 84%,transparent)}.local-agent-card.enabled{border-color:color-mix(in srgb,var(--tm-ink) 28%,var(--tm-line))}.local-agent-identity{display:grid;grid-template-columns:auto minmax(0,1fr) auto;align-items:center;gap:10px}.local-agent-identity>span{display:grid;gap:3px;min-width:0}.local-agent-identity b{font-size:12px}.local-agent-identity small{overflow:hidden;color:var(--tm-muted);font-size:9px;text-overflow:ellipsis;white-space:nowrap}.local-agent-identity>i{padding:5px 8px;border-radius:999px;background:var(--tm-surface);color:var(--tm-muted);font-size:8px;font-style:normal}.local-agent-card.detected .local-agent-identity>i{background:color-mix(in srgb,#34c759 12%,var(--tm-bg));color:color-mix(in srgb,#34c759 72%,var(--tm-ink))}.local-agent-card>p{min-height:34px;margin:0;color:var(--tm-muted);font-size:10px;line-height:1.55}.agent-capabilities{display:flex;flex-wrap:wrap;gap:5px}.agent-capabilities span{padding:5px 7px;border:1px solid var(--tm-line);border-radius:999px;color:var(--tm-muted);font-size:8px}.local-agent-card>code{overflow:hidden;padding:9px 10px;border-radius:10px;background:var(--tm-surface);color:var(--tm-ink);font:9px/1.4 ui-monospace,SFMono-Regular,Consolas,monospace;text-overflow:ellipsis;white-space:nowrap}.agent-action-status{min-height:28px;color:var(--tm-muted);font-size:9px;line-height:1.5}.agent-actions{display:grid;grid-template-columns:1fr 1fr;gap:7px;margin-top:auto}.agent-actions button{min-width:0}.dashboard-picker-tabs{display:grid;grid-template-columns:1fr 1fr;gap:5px;padding:5px;border-radius:13px;background:var(--tm-surface)}.dashboard-picker-tabs button{border:0!important;background:transparent!important;color:var(--tm-muted)!important}.dashboard-picker-tabs button.active{background:var(--tm-bg)!important;color:var(--tm-ink)!important;box-shadow:0 1px 4px rgba(0,0,0,.07)}
.agent-actions.triple{grid-template-columns:repeat(3,minmax(0,1fr))}.agent-actions button{padding-inline:8px}
.agent-actual-path{min-width:0}.agent-actual-path summary{cursor:pointer;color:var(--tm-muted);font-size:9px}.agent-actual-path code{display:block;max-width:100%;margin-top:7px;overflow:hidden;padding:8px 9px;border-radius:9px;background:var(--tm-surface);font:8px/1.45 ui-monospace,SFMono-Regular,Consolas,monospace;text-overflow:ellipsis;white-space:nowrap}.cc-conflict-alert{display:flex;grid-column:1/-1;align-items:center;gap:12px;padding:11px 13px;border:1px solid color-mix(in srgb,#ff9500 34%,var(--tm-line));border-radius:13px;background:color-mix(in srgb,#ff9500 7%,var(--tm-bg));color:var(--tm-ink)}.cc-conflict-alert span{display:grid;flex:1;gap:3px}.cc-conflict-alert small,.cc-conflict-alert em{color:var(--tm-muted);font-size:9px;font-style:normal}.cc-conflict-alert button{padding:8px 11px;border:0;border-radius:9px;background:var(--tm-ink);color:var(--tm-on-ink);font:inherit;font-size:9px}.opencode-variant{display:grid;grid-template-columns:1fr 1fr;gap:5px;padding:4px;border-radius:11px;background:var(--tm-bg)}.opencode-variant button{padding:8px;border:0;border-radius:8px;background:transparent;color:var(--tm-muted)}.opencode-variant button.active{background:var(--tm-ink);color:var(--tm-on-ink)}
@media(max-width:1180px){.local-agent-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:760px){.local-agent-grid{grid-template-columns:1fr}.local-agent-heading{flex-direction:column}.local-agent-heading>button{width:100%}}
.legacy-chart-section,.all-cost-analytics,.report,.settings,.beginner-guide,.backup-settings{display:none}
.opencode-go-monitor{display:grid;gap:14px;margin-bottom:18px}.opencode-go-heading{display:flex;align-items:center;justify-content:space-between;gap:18px}.opencode-go-heading h2{margin:3px 0}.opencode-go-heading p{margin:0;color:var(--tm-muted)}.opencode-go-quota-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:12px}.opencode-go-quota-card{display:grid;gap:12px;min-width:0}.opencode-go-quota-card>div:first-child{display:flex;align-items:center;justify-content:space-between;gap:12px;color:var(--tm-muted)}.opencode-go-quota-card>div:first-child b{color:var(--tm-ink);font-size:12px}.opencode-go-quota-card>strong{font-size:26px;letter-spacing:-.04em}.opencode-go-quota-card>strong small{color:var(--tm-muted);font-size:13px;font-weight:500}.opencode-go-quota-track{height:9px;overflow:hidden;border-radius:999px;background:var(--tm-surface)}.opencode-go-quota-track i{display:block;height:100%;min-width:2px;border-radius:inherit;background:var(--tm-ink);transition:width .3s var(--apple-ease-out,ease-out)}.opencode-go-quota-card>small,.opencode-go-source{color:var(--tm-muted)}.opencode-go-source{margin:0 4px;font-size:12px;line-height:1.65}.opencode-go-account-note{margin-top:8px!important;color:var(--tm-ink)!important;font-size:12px;font-weight:650}.opencode-go-config-note{display:grid;gap:4px;margin:4px 0 2px;padding:12px 14px;border:1px solid var(--tm-line);border-radius:12px;background:var(--tm-surface);font-size:12px}.opencode-go-config-note span{color:var(--tm-muted);line-height:1.55}.opencode-go-config-note code{font-family:inherit;color:var(--tm-ink)}
.opencode-local-source{display:flex;grid-column:1/-1;align-items:center;justify-content:space-between;gap:14px;margin-top:13px;padding:13px 14px;border:1px solid var(--tm-line);border-radius:14px;background:var(--tm-surface)}.opencode-local-source>div{display:grid;min-width:0;gap:4px}.opencode-local-source b{font-size:12px}.opencode-local-source small{color:var(--tm-muted);font-size:10px;line-height:1.5}.opencode-local-source code{max-width:620px;overflow:hidden;color:var(--tm-ink);font:9px/1.4 ui-monospace,SFMono-Regular,Consolas,monospace;text-overflow:ellipsis;white-space:nowrap}.opencode-local-source button{flex:0 0 auto}
@media(max-width:900px){.opencode-go-quota-grid{grid-template-columns:1fr}.opencode-go-heading,.opencode-local-source{align-items:flex-start;flex-direction:column}.opencode-go-heading button,.opencode-local-source button{width:100%}}
.floating-card-actions{display:flex;justify-content:flex-end;gap:4px;margin:-6px -5px 8px}.floating-card-actions button{padding:5px 7px!important;border:1px solid var(--tm-line)!important;border-radius:8px!important;background:var(--tm-surface)!important;color:var(--tm-ink)!important;font-size:8px!important}.floating-card-actions .floating-drag-handle{cursor:grab}.floating-card-actions .floating-drag-handle:active{cursor:grabbing;transform:scale(.96)}.floating-card-actions .floating-remove:hover{border-color:#ff3b30!important;background:#ff3b30!important;color:#fff!important}.floating-card-library{display:flex;flex-wrap:wrap;align-items:center;gap:5px;margin:8px 0 18px;padding:10px;border-radius:12px;background:var(--tm-surface)}.floating-card-library>span{width:100%;font-size:9px;font-weight:700}.floating-card-library button{padding:6px 8px!important;border:1px solid var(--tm-line)!important;background:var(--tm-bg)!important;color:var(--tm-ink)!important;font-size:8px!important}.floating-card-library small{color:var(--tm-muted);font-size:8px}.floating-line-chart{position:absolute;inset:0;width:100%;height:100%;margin:0;color:var(--tm-ink);border-bottom:1px solid var(--tm-line)}
.floating-interactive-chart{position:relative;outline:none}.floating-module{transition:transform .24s cubic-bezier(.22,1,.36,1),border-color .16s ease,box-shadow .24s ease}.floating-module:hover{transform:translateY(-2px);border-color:color-mix(in srgb,var(--tm-ink) 18%,var(--tm-line))!important;box-shadow:0 5px 12px rgba(0,0,0,.07)}.floating-chart-point{position:relative;outline:none;transition:opacity .16s ease,transform .24s cubic-bezier(.22,1,.36,1)}.floating-chart-tooltip{position:absolute;z-index:12;left:50%;bottom:calc(100% + 7px);display:grid;gap:2px;min-width:max-content;padding:7px 9px;transform:translate(-50%,5px) scale(.96);border-radius:9px;background:var(--tm-ink);color:var(--tm-on-ink);opacity:0;pointer-events:none;font-style:normal;font-size:8px;line-height:1.35;box-shadow:0 7px 18px rgba(0,0,0,.16);transition:opacity .16s ease,transform .22s cubic-bezier(.22,1,.36,1)}.floating-chart-tooltip b{font-size:8px}.floating-chart-tooltip span{color:color-mix(in srgb,var(--tm-on-ink) 74%,transparent)}.floating-chart-point:hover .floating-chart-tooltip,.floating-chart-point:focus-visible .floating-chart-tooltip,.floating-interactive-chart:hover>.floating-chart-tooltip,.floating-interactive-chart:focus-visible>.floating-chart-tooltip{opacity:1;transform:translate(-50%,0) scale(1)}.mini-bars:hover .floating-chart-point:not(:hover):not(:focus-visible),.floating-chart-bars:hover .floating-chart-point:not(:hover):not(:focus-visible){opacity:.34}.mini-bar:hover,.mini-bar:focus-visible{transform:translateY(-2px)}.mini-bar>i{transform-origin:bottom;transition:transform .22s cubic-bezier(.22,1,.36,1)}.mini-bar:hover>i,.mini-bar:focus-visible>i{transform:scaleX(1.16) scaleY(1.04)}.floating-ring{transition:transform .24s cubic-bezier(.22,1,.36,1)}.floating-ring:hover,.floating-ring:focus-visible{transform:scale(1.035)}.floating-ring>.floating-chart-tooltip{bottom:calc(100% + 4px)}.floating-line-wrap{position:relative;height:78px;margin-top:8px}.floating-line-point{position:absolute;z-index:3;width:18px;height:18px;transform:translate(-50%,-50%)}.floating-line-point>i{position:absolute;inset:5px;border:2px solid var(--tm-ink);border-radius:50%;background:var(--tm-bg);transition:transform .2s cubic-bezier(.22,1,.36,1),background .16s ease}.floating-line-point:hover>i,.floating-line-point:focus-visible>i{transform:scale(1.65);background:var(--tm-ink)}.floating-line-point .floating-chart-tooltip{bottom:16px}
.shell>aside,.card,.all-dashboard,.all-overview-grid>section,.monitor-aside,.provider-overview,.codex-dashboard .card,.floating-module{background:var(--tm-bg)!important;color:var(--tm-ink)!important;border-color:var(--tm-line)!important}.content>header,.dashboard-switcher,.dashboard-context-heading{color:var(--tm-ink)}.shell button:not(.monitor-launch):not(.sync):not(.dashboard-remove):not(.dashboard-tab-label):not(.add-dashboard),.floating-shell button:not(.proxyLive){background:var(--tm-ink);color:var(--tm-on-ink);border-color:var(--tm-line)}.shell .outline,.shell .text,.shell .icon-action{background:var(--tm-bg)!important;color:var(--tm-ink)!important}.privacy,.proxy-launch-status,.inline-empty,.connection-state,.cc-switch-state,.floating-sync{background:var(--tm-surface)!important;color:var(--tm-muted)!important}.track,.mini-bars,.usage-bars{border-color:var(--tm-line)}.track span,.metric-bars i,.usage-bars i,.mini-bars i,.floating-chart-bars i{background:var(--tm-ink)!important}.monitor-launch:not(.running),.connection-state strong:not(.online),.cc-switch-state:not(.connected) b{color:#ff3b30!important}.monitor-launch:not(.running){border-color:#ff3b30!important;background:color-mix(in srgb,#ff3b30 9%,var(--tm-bg))!important}.monitor-launch.running,.connection-state strong.online,.cc-switch-state.connected b,.floating-shell .proxyLive{color:#34c759!important}.monitor-launch.running{border-color:#34c759!important;background:color-mix(in srgb,#34c759 10%,var(--tm-bg))!important}.cc-switch-state.connected i{background:#34c759!important}.cc-switch-state:not(.connected) i{background:#ff3b30!important}
.shell,.floating-shell{color:var(--tm-ink);background:var(--tm-surface)}
.codex-official-mark{width:44px;height:44px;flex:0 0 44px;object-fit:contain}
.dashboard-context-heading{display:flex;align-items:center;justify-content:space-between;gap:24px;margin:18px 0 14px}.dashboard-context-heading>div{display:flex;align-items:center;gap:12px}.dashboard-context-heading h2{margin:4px 0;font-size:23px}.dashboard-context-heading p,.dashboard-context-heading>span{margin:0;color:#6e6e73;font-size:10px}.dashboard-heading{display:none}.floating-ring{display:grid;place-items:center;width:84px;height:84px;margin:12px auto 2px;border-radius:50%}.floating-ring>i{grid-area:1/1;width:60px;height:60px;border-radius:50%;background:var(--tm-bg)}.floating-ring>b{grid-area:1/1;z-index:1;font-size:13px}.floating-chart-bars{display:flex;align-items:flex-end;gap:6px;height:72px;margin-top:11px;padding:5px;border-bottom:1px solid #d1d1d6}.floating-chart-bars>.floating-chart-point{display:flex;flex:1;align-items:flex-end;justify-content:center;height:100%;min-width:0}.floating-chart-bars>.floating-chart-point>i{display:block;width:min(14px,70%);min-height:3px;border-radius:6px 6px 2px 2px;background:#111;transform-origin:bottom;transition:transform .22s cubic-bezier(.22,1,.36,1)}.floating-chart-bars>.floating-chart-point:hover>i,.floating-chart-bars>.floating-chart-point:focus-visible>i{transform:scaleX(1.18) scaleY(1.05)}.mode-simple .model-chart-grid,.mode-simple .deepseek-monitor .balance-chart,.mode-simple .recent,.mode-simple .model-advice{display:none}
.mini-core-row .mini-globalRemaining,.mini-core-row .mini-codexQuota,.mini-core-row .mini-todayTokens{display:grid;place-items:center}.mini-ring-progress{display:grid;place-items:center;width:54px;height:54px;border-radius:50%;font-style:normal}.mini-ring-progress::before{content:"";grid-area:1/1;width:39px;height:39px;border-radius:50%;background:#fff}.mini-ring-progress em{z-index:1;grid-area:1/1;color:#1d1d1f;font-size:10px;font-style:normal;font-weight:700}.mini-collapsed-bars{display:flex;align-items:flex-end;gap:4px;width:82px;height:48px;padding:4px 3px;border-bottom:1px solid #d8d8dc}.mini-collapsed-bars i{flex:1;min-height:3px;border-radius:4px 4px 1px 1px;background:#007aff}
.brand{font-size:19px;white-space:nowrap}.brand span{font-size:25px}
.sync { min-width: 122px; }
.sync:disabled { cursor: wait; opacity: .82; }
.sync-icon { display: inline-block; margin-right: 7px; font-size: 16px; line-height: 10px; vertical-align: -1px; }
.sync.loading .sync-icon { animation: spin .75s linear infinite; }
.observed { width: 100%; opacity: .65; }
.empty { width: 0; }
.budget-fields { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin: 16px 0; }
.budget-fields label { display: block; border: 0; padding: 0; color: #86868B; font-size: 12px; }
.budget-fields input { display: block; width: 100%; margin-top: 7px; padding: 10px; border: 1px solid #e5e5ea; border-radius: 10px; font: inherit; color: #1D1D1F; }
.save-note { margin-left: 12px; color: #1a8e4a; font-size: 12px; }
.proxy-fields { display: grid; grid-template-columns: 2fr 1fr; gap: 12px; margin: 16px 0; }
.proxy-fields label { display: block; border: 0; padding: 0; color: #86868B; font-size: 12px; }
.proxy-fields input { display: block; width: 100%; margin-top: 7px; padding: 10px; border: 1px solid #e5e5ea; border-radius: 10px; font: inherit; color: #1D1D1F; }
.usage-bars{height:178px;display:flex;align-items:flex-end;gap:14px;padding:18px 10px 22px;border-bottom:1px solid #e5e5e5}.usage-bar{height:100%;flex:1;display:flex;align-items:flex-end;justify-content:center;position:relative}.usage-bar>i{width:min(32px,72%);min-height:3px;background:#111;border-radius:7px 7px 3px 3px}.usage-bar>small{position:absolute;bottom:-20px;color:#737373;font-size:10px}.bar-tooltip{display:none;position:absolute;z-index:5;bottom:calc(100% + 8px);left:50%;transform:translateX(-50%);min-width:155px;padding:10px 12px;background:#111;color:#fff;border-radius:10px;font-size:10px;line-height:1.55;box-shadow:0 8px 24px rgba(0,0,0,.18)}.bar-tooltip b,.bar-tooltip span{display:block}.usage-bar:hover .bar-tooltip{display:block}
.chart-hint { color: #737373; font-size: 11px; }
.account-intro { display: flex; align-items: center; justify-content: space-between; margin-bottom: 18px; }
.account-intro h2 { margin-bottom: 5px; }
.account-intro > span { border: 1px solid #dedede; border-radius: 99px; padding: 6px 10px; color: #525252; font-size: 12px; }
.dashboard-switcher{display:flex;gap:8px;align-items:center;overflow:auto;margin-bottom:18px}.dashboard-switcher button{background:#fff;color:#525252;border:1px solid #dedede;padding:8px 12px;white-space:nowrap}.dashboard-switcher button.active{background:#111;color:#fff;border-color:#111}.dashboard-switcher i{font-style:normal;margin-left:6px;opacity:.65}.dashboard-switcher .add-dashboard{border-style:dashed;color:#111}.dashboard-heading{display:flex;align-items:end;justify-content:space-between;margin-bottom:14px}.dashboard-heading h2{font-size:23px;margin:4px 0 0}.dashboard-heading>span{color:#737373;font-size:12px}.compact{width:380px}.compact select,.compact input{display:block;width:100%;padding:11px;border:1px solid #ddd;border-radius:10px;font:inherit;margin:12px 0}.compact button:not(.close){width:100%}
.account-actions{display:flex;gap:8px}.outline{background:#fff;color:#111;border:1px solid #d4d4d4}.modal-backdrop{position:fixed;inset:0;background:rgba(0,0,0,.35);display:grid;place-items:center;z-index:20}.config-modal{width:420px;background:#fff;border-radius:18px;padding:26px;position:relative;box-shadow:0 20px 60px rgba(0,0,0,.2)}.config-modal p,.config-modal small{display:block;color:#737373;font-size:12px;line-height:1.6}.config-modal label{display:block;margin:14px 0;color:#525252;font-size:12px}.config-modal input{width:100%;margin-top:6px;padding:11px;border:1px solid #ddd;border-radius:10px;font:inherit}.close{position:absolute;right:15px;top:13px;background:transparent;color:#111;font-size:25px;padding:2px 8px}
.provider-body{min-width:0;flex:1}.saved-accounts{margin-top:12px;display:grid;gap:6px}.saved-accounts>div{display:flex;align-items:center;gap:6px;background:#f5f5f5;padding:8px;border-radius:10px}.saved-accounts span{min-width:0;flex:1}.saved-accounts b,.saved-accounts small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.saved-accounts b{font-size:12px}.saved-accounts small{font-size:10px;color:#737373}.icon-action{padding:5px 7px;background:#fff;color:#111;border:1px solid #ddd;font-size:10px}.danger,.danger-outline{color:#c62828}.danger-outline{background:#fff;border:1px solid #e3b4b4}.floating-shell{width:100vw;height:100vh;padding:16px;background:rgba(255,255,255,.98);border:1px solid #ddd;border-radius:20px;overflow:hidden;color:#111}.floating-shell header{margin:0 0 12px;display:flex;align-items:center;justify-content:space-between}.floating-shell header div{display:grid}.floating-shell header small{color:#737373;font-size:10px}.floating-shell nav{display:flex;gap:4px}.floating-shell nav button{padding:2px 8px;background:#f2f2f2;color:#111}.floating-shell .floating-value{display:flex;justify-content:space-between;align-items:end}.floating-value span{font-size:11px;color:#737373}.floating-value strong{font-size:25px}.floating-stats{display:flex;justify-content:space-between;font-size:10px;color:#737373;margin-top:10px}.floating-stats b{color:#111}.floating-sync{display:block;margin-top:9px;color:#737373;font-size:9px}.floating-shell.collapsed{height:68px;padding:13px 16px}.floating-shell.collapsed header{margin:0;height:40px}
.floating-quotas{display:flex;justify-content:space-between;font-size:11px;color:#737373}.floating-quotas b{font-size:17px;color:#111;margin-left:4px}.mini-bars{height:59px;display:flex;align-items:flex-end;gap:10px;padding:4px 8px 15px;margin-top:2px;border-bottom:1px solid #eee}.mini-bar{height:100%;flex:1;display:flex;align-items:flex-end;justify-content:center;position:relative}.mini-bar i{display:block;width:18px;min-height:2px;background:#111;border-radius:5px 5px 2px 2px}.mini-bar small{position:absolute;bottom:-13px;color:#86868b;font-size:8px}.balance-alert{margin-top:7px;padding:5px 8px;border-radius:8px;background:#f2f2f7;color:#525252;font-size:9px}.balance-alert.warning{background:#fff4df;color:#8a5600}.balance-alert.critical{background:#fff0ef;color:#c6251d}.floating-sync{margin-top:6px}
.balance-alert.warning,.balance-alert.critical{background:#fff0ef;color:#ff3b30;font-weight:700}.floating-module{container-type:inline-size}.floating-module-head strong{max-width:62%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}@container(max-width:180px){.floating-module{padding:9px}.floating-module-head strong{font-size:15px}.floating-module small{font-size:8px}}
.floating-shell header{cursor:move}.floating-shell header nav,.floating-shell header button,.floating-shell header select{cursor:pointer}.floating-shell header select{width:185px;margin-top:3px;padding:2px 22px 2px 6px;border:0;border-radius:7px;background:#f2f2f7;color:#525252;font:inherit;font-size:9px;outline:none}.floating-shell header b{line-height:1.1}
.floating-shell header em{font-style:normal;font-size:8px;font-weight:500;color:#86868b}.brand small{font-size:9px;color:#86868b;font-weight:500;vertical-align:middle}
.provider-overview{display:flex;align-items:center;justify-content:space-between;gap:28px;margin-bottom:18px}.provider-overview-title{display:flex;align-items:flex-start;gap:13px;min-width:260px}.provider-overview-title h2{font-size:21px;margin:5px 0 7px}.provider-overview-title p{margin:0;color:#737373;font-size:12px;line-height:1.55}.provider-overview-grid{display:grid;grid-template-columns:repeat(4,minmax(92px,1fr));gap:10px;flex:1}.provider-overview-grid span{padding:13px;background:#f5f5f5;border-radius:12px}.provider-overview-grid small,.provider-overview-grid b{display:block}.provider-overview-grid small{color:#737373;font-size:10px;margin-bottom:7px}.provider-overview-grid b{font-size:16px;white-space:nowrap}
.overview-metrics{display:grid;grid-template-columns:repeat(4,1fr);gap:14px;margin-bottom:18px}.overview-metrics span,.overview-metrics small{display:block;color:#737373;font-size:11px}.overview-metrics strong{display:block;margin:9px 0;font-size:23px;letter-spacing:-.6px}.overview-metrics .card{padding:18px 20px}
.settings{max-width:1100px}.floating-configurator{display:grid;grid-template-columns:minmax(0,1fr) 310px;gap:22px;margin-top:24px;padding-top:22px;border-top:1px solid #e5e5e5}.setting-section-title{display:flex;align-items:flex-start;justify-content:space-between;gap:16px}.setting-section-title h3,.mini-settings h3{margin:0 0 5px;font-size:15px}.setting-section-title p,.mini-settings p{margin:0}.setting-section-title>span{padding:5px 8px;border-radius:99px;background:#f2f2f7;color:#525252;font-size:10px;white-space:nowrap}.module-checklist{display:grid;gap:7px;margin-top:14px}.settings .module-checklist label{display:flex;align-items:center;gap:10px;padding:10px 12px;border:1px solid #e5e5ea;border-radius:12px;background:#fff;cursor:grab}.module-checklist label:active{cursor:grabbing}.module-checklist i{font-style:normal;color:#b0b0b5;letter-spacing:-2px}.module-checklist input{margin:0;accent-color:#111}.module-checklist span{min-width:0}.module-checklist b,.module-checklist small{display:block}.module-checklist b{font-size:12px}.module-checklist small{margin-top:3px;color:#86868b;font-size:10px}.mini-settings{display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-top:18px;padding:15px;background:#f7f7f8;border-radius:14px}.mini-settings h3,.mini-settings>p{grid-column:1/-1}.settings .mini-settings label{padding:0;border:0;color:#737373;font-size:10px}.mini-settings select{display:block;width:100%;margin-top:6px;padding:9px;border:1px solid #dedee3;border-radius:10px;background:#fff;font:inherit;color:#111}.settings .floating-preview{width:auto;padding:0;border:0;background:transparent;display:block}.floating-preview>span{font-size:10px;font-weight:700;letter-spacing:1px}.preview-window{margin-top:9px;padding:15px;background:#fff;border:1px solid #dedee3;border-radius:20px}.preview-window header{margin:0 0 10px}.preview-window header b{font-size:14px}.preview-window header small{color:#86868b;font-size:9px}.preview-module{display:flex;align-items:center;justify-content:space-between;padding:8px 0;border-top:1px solid #eee}.preview-module span{color:#737373;font-size:10px}.preview-module b{font-size:12px}.floating-preview>p{font-size:10px;line-height:1.5}
.floating-modules{display:grid;gap:7px;max-height:calc(100vh - 64px);overflow:auto;padding-right:2px}.floating-module{padding:9px 10px;border-radius:12px;background:#f7f7f8}.floating-module-head{display:flex;align-items:center;justify-content:space-between;gap:10px}.floating-module-head span{color:#737373;font-size:10px}.floating-module-head strong{font-size:17px}.floating-module-head.sub{margin-top:8px}.floating-module>small{display:block;margin-top:5px;color:#86868b;font-size:9px}.floating-module .track{height:6px;margin:6px 0}.floating-module .mini-bars{height:52px;margin-top:5px;background:#fff;border-radius:9px;border-bottom:0}.model-balance-list{display:grid;gap:5px;margin-top:7px}.model-balance-list>span{display:block;padding-top:5px;border-top:1px solid #e5e5e5;font-size:9px;color:#86868b}.model-balance-list b,.model-balance-list small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.model-balance-list b{color:#525252}.mini-core-row{display:grid;grid-template-columns:1fr 1fr;gap:7px}.mini-core-row>span{padding:8px 10px;background:#f2f2f7;border-radius:10px}.mini-core-row small,.mini-core-row b{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.mini-core-row small{color:#86868b;font-size:8px}.mini-core-row b{margin-top:3px;font-size:13px}.floating-shell.collapsed{height:116px;padding:12px 16px}.floating-shell.collapsed header{margin:0 0 5px;height:40px}
@media(max-width:1150px){.floating-configurator{grid-template-columns:1fr}.settings .floating-preview{display:none}}
.settings .mini-settings .mini-mode-toggle{grid-column:1/-1;display:flex;align-items:center;gap:7px;padding:8px 10px;border:0;border-radius:10px;background:#fff;color:#111;font-size:11px}.mini-mode-toggle input{margin:0}
.floating-layout-setting{display:flex;align-items:center;gap:7px;margin-top:13px;padding:7px;background:#f2f2f7;border-radius:12px}.floating-layout-setting>span{margin:0 auto 0 4px;color:#737373;font-size:10px}.floating-layout-setting button{padding:7px 10px;background:#fff;color:#525252;border:1px solid #dedee3;font-size:10px}.floating-layout-setting button.active{background:#111;color:#fff;border-color:#111}.preview-window.grid{display:grid;grid-template-columns:1fr 1fr;gap:0 12px}.preview-window.grid header,.preview-window.grid>small{grid-column:1/-1}
.floating-shell{position:relative;display:flex;flex-direction:column;width:100%;height:100%;padding:12px;background:#fff;border:1px solid #d8d8dc;border-radius:20px;overflow:hidden;clip-path:inset(0 round 20px);box-shadow:none;color:#111}.floating-shell>header{flex:0 0 auto;margin:0 0 9px;padding:2px 2px 7px;border-bottom:1px solid #ededf0}.floating-shell>header div{min-width:0;flex:1}.floating-shell>header b{display:block;width:max-content;max-width:100%}.floating-shell>header select{width:min(185px,100%)}.floating-shell>header nav{flex:0 0 auto}.floating-modules{flex:1 1 auto;min-height:0;max-height:none;overflow-x:hidden;overflow-y:auto;display:grid;align-content:start;gap:8px;padding:1px 5px 8px 1px;scrollbar-gutter:stable}.floating-modules.grid-layout{grid-template-columns:repeat(auto-fit,minmax(min(100%,145px),1fr));grid-auto-flow:dense}.floating-modules.list-layout{grid-template-columns:1fr}.floating-module{min-width:0;height:max-content;padding:11px;border:1px solid #e4e4e7;border-radius:14px;background:#f7f7f8;transition:opacity .18s ease,transform .3s cubic-bezier(.22,1,.36,1)}.grid-layout .module-codexQuota,.grid-layout .module-modelBalances{grid-column:1/-1}.floating-reflow-move{transition:transform .34s cubic-bezier(.22,1,.36,1)}.floating-reflow-enter-active{transition:opacity .22s ease,transform .34s cubic-bezier(.22,1,.36,1)}.floating-reflow-leave-active{position:absolute;transition:opacity .15s ease,transform .15s ease}.floating-reflow-enter-from,.floating-reflow-leave-to{opacity:0;transform:scale(.96)}.floating-sync{flex:0 0 auto;display:block;margin:0;padding:7px 18px 1px 2px;border-top:1px solid #ededf0;background:#fff;color:#737373;font-size:9px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.floating-resize-handle{position:absolute;z-index:5;right:4px;bottom:4px;width:18px;height:18px;cursor:nwse-resize}.floating-resize-handle::after{content:"";position:absolute;right:2px;bottom:2px;width:8px;height:8px;border-right:2px solid #9c9ca1;border-bottom:2px solid #9c9ca1;border-radius:0 0 3px}.floating-shell.collapsed{width:100%;height:100%;padding:10px 12px}.floating-shell.collapsed>header{height:39px;margin:0 0 6px}.floating-shell.collapsed .floating-sync{margin-top:auto}.floating-modules::-webkit-scrollbar{width:7px}.floating-modules::-webkit-scrollbar-track{background:transparent}.floating-modules::-webkit-scrollbar-thumb{background:#c7c7cc;border:2px solid #fff;border-radius:99px}
.floating-resize-handle{touch-action:none;user-select:none}
.shell>aside nav button{background:transparent!important;color:var(--tm-muted)!important}.shell>aside nav button i{color:currentColor}.shell>aside nav button:hover{background:var(--tm-surface)!important;color:var(--tm-ink)!important}.shell>aside nav button.active{background:var(--tm-ink)!important;color:var(--tm-on-ink)!important;font-weight:600}.shell>aside nav button:focus-visible,.dashboard-switcher button:focus-visible{outline:2px solid var(--tm-ink);outline-offset:2px}
.dashboard-switcher{scrollbar-width:none;overscroll-behavior-inline:contain}.dashboard-switcher::-webkit-scrollbar{display:none}.dashboard-switcher>button{background:var(--tm-bg)!important;color:var(--tm-muted)!important;border-color:var(--tm-line)!important}.dashboard-switcher>button:hover{background:var(--tm-surface)!important;color:var(--tm-ink)!important}.dashboard-switcher>button.active{background:var(--tm-ink)!important;color:var(--tm-on-ink)!important;border-color:var(--tm-ink)!important}.dashboard-switcher>button.add-dashboard{border-style:dashed!important;color:var(--tm-ink)!important}
.dashboard-switcher .dashboard-tab{display:flex;align-items:center;border:1px solid var(--tm-line);border-radius:12px;background:var(--tm-bg);white-space:nowrap;transition:border-color .18s ease,background-color .18s ease}.dashboard-switcher .dashboard-tab:hover{background:var(--tm-surface);border-color:color-mix(in srgb,var(--tm-ink) 22%,var(--tm-line))}.dashboard-switcher .dashboard-tab>button{border:0;border-radius:0;background:transparent!important}.dashboard-switcher .dashboard-tab-label{padding:8px 10px 8px 12px;color:var(--tm-muted)!important}.dashboard-switcher .dashboard-tab.active{background:var(--tm-ink);border-color:var(--tm-ink)}.dashboard-switcher .dashboard-tab.active .dashboard-tab-label{color:var(--tm-on-ink)!important}.dashboard-switcher .dashboard-remove{display:grid;place-items:center;width:26px;height:26px;margin-right:3px;padding:0!important;border-radius:8px!important;color:var(--tm-muted)!important;opacity:0;background:transparent!important;font-size:15px;line-height:1;transition:opacity .15s ease,background-color .15s ease,color .15s ease}.dashboard-switcher .dashboard-tab:hover .dashboard-remove,.dashboard-switcher .dashboard-remove:focus-visible{opacity:.72}.dashboard-switcher .dashboard-tab.active .dashboard-remove{color:var(--tm-on-ink)!important}.dashboard-switcher .dashboard-remove:hover,.dashboard-switcher .dashboard-remove:focus-visible{opacity:1!important;background:#ff3b30!important;color:#fff!important;outline:none}.dashboard-empty{display:flex;align-items:flex-start;gap:16px;margin-bottom:18px;padding:28px}.dashboard-empty>span{display:grid;place-items:center;width:42px;height:42px;flex:0 0 auto;border-radius:12px;background:#f2f2f7;font-size:20px}.dashboard-empty h2{margin:1px 0 8px;font-size:18px}.dashboard-empty p{max-width:680px;margin:0;color:#737373;font-size:12px;line-height:1.7}.empty-actions{display:flex;gap:9px;margin-top:15px}.usage-bar:focus .bar-tooltip{display:block}.usage-bar:focus{outline:2px solid rgba(0,122,255,.45);outline-offset:4px;border-radius:6px}@media(hover:none){.dashboard-switcher .dashboard-remove{opacity:.58}}@media(max-width:1050px){.provider-overview{align-items:flex-start;flex-direction:column}.provider-overview-grid{width:100%}.overview-metrics{grid-template-columns:repeat(2,1fr)}.dashboard-switcher{max-width:100%}.settings{width:100%}.floating-configurator{grid-template-columns:minmax(0,1fr)}}@media(prefers-reduced-motion:reduce){.dashboard-switcher .dashboard-tab,.dashboard-switcher .dashboard-remove,.floating-module,.floating-reflow-move,.floating-reflow-enter-active,.floating-reflow-leave-active{transition:none}.floating-card-actions .floating-drag-handle:active{transform:none}}
.provider-awaiting{min-width:190px;padding:14px 17px;border-radius:14px;background:#f7f7f8}.provider-awaiting small,.provider-awaiting b,.provider-awaiting span{display:block}.provider-awaiting small{color:#86868b;font-size:10px}.provider-awaiting b{margin:7px 0;font-size:17px}.provider-awaiting span{width:max-content;padding:4px 7px;border-radius:99px;background:#e8f6eb;color:#2e7d45;font-size:9px}
.deepseek-monitor{display:grid;gap:18px;margin-bottom:18px}.deepseek-metrics{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:14px}.deepseek-metrics .card{padding:18px 20px}.deepseek-metrics span,.deepseek-metrics small{display:block;color:#737373;font-size:11px}.deepseek-metrics strong{display:block;margin:9px 0;font-size:23px;letter-spacing:-.5px}.deepseek-metrics .balance-primary{background:#111;color:#fff}.deepseek-metrics .balance-primary span,.deepseek-metrics .balance-primary small{color:#bdbdc2}.deepseek-metrics strong.negative{color:#ff3b30}.balance-chart{padding:22px}.balance-chart h2{margin:5px 0 0}.balance-plot{margin-top:18px}.balance-plot svg{display:block;width:100%;height:170px;overflow:visible;background:linear-gradient(to bottom,#f7f7f8,#fff);border-radius:14px}.balance-plot circle{fill:#fff;stroke:#111;stroke-width:4;vector-effect:non-scaling-stroke}.balance-plot>div{display:flex;justify-content:space-between;margin-top:8px;color:#737373;font-size:10px}.balance-chart-empty{display:grid;place-items:center;height:150px;margin-top:18px;border-radius:14px;background:#f7f7f8;color:#86868b;font-size:11px}.deepseek-source{margin:0;color:#737373;font-size:11px}.deepseek-source code{padding:2px 5px;border-radius:5px;background:#f2f2f7;color:#111}@media(max-width:1050px){.deepseek-metrics{grid-template-columns:repeat(2,minmax(0,1fr))}}
.model-kpis{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:14px;margin-bottom:18px}.model-kpis .card{padding:18px 20px}.model-kpis span,.model-kpis small{display:block;color:#737373;font-size:11px}.model-kpis strong{display:block;margin:9px 0;font-size:23px;letter-spacing:-.6px}.model-kpis small{line-height:1.45}.table-empty{text-align:center;padding:28px;color:#86868b}.recent{overflow-x:auto}@media(max-width:1050px){.model-kpis{grid-template-columns:repeat(2,minmax(0,1fr))}}
.model-kpis{grid-template-columns:1.35fr repeat(3,minmax(0,1fr));gap:10px}.model-kpis .card{padding:14px 16px}.model-kpis .token-primary{background:#111;color:#fff}.model-kpis .token-primary span,.model-kpis .token-primary small{color:#c7c7cc}.model-kpis .balance-compact{grid-column:1/-1;display:grid;grid-template-columns:120px 150px 1fr;align-items:center;gap:16px}.model-kpis .balance-compact strong{margin:0}.model-chart-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px;margin-bottom:12px}.metric-chart{padding:18px}.metric-chart h2{font-size:14px}.metric-bars{display:flex;align-items:flex-end;gap:10px;height:150px;margin-top:14px;padding-top:20px}.metric-bar{position:relative;display:flex;flex:1;flex-direction:column;align-items:center;justify-content:flex-end;height:100%;min-width:0}.metric-bar>i{display:block;width:min(24px,65%);min-height:2px;border-radius:7px 7px 3px 3px;background:#111}.metric-bar>small{margin-top:7px;color:#86868b;font-size:8px}.metric-bar:hover .bar-tooltip,.metric-bar:focus .bar-tooltip{display:block}.metric-bar:focus{outline:2px solid rgba(0,122,255,.4);outline-offset:3px;border-radius:6px}.model-advice{margin-bottom:18px}.monitor-aside{display:grid;align-content:start;gap:18px}.monitor-aside>div+div{padding-top:16px;border-top:1px solid #ededf0}.monitor-aside span,.monitor-aside small{display:block;color:#737373;font-size:10px}.today-logo-list{display:flex;flex-wrap:wrap;gap:8px;margin-top:11px}.today-logo-list>span{position:relative}.today-logo-list :deep(.provider-mark){width:38px;height:38px}.connection-state strong{margin:7px 0 4px!important}.cc-switch-state>span{display:flex;align-items:center;gap:7px;color:#111;font-weight:700}.cc-switch-state i{width:7px;height:7px;border-radius:50%;background:#c7c7cc}.cc-switch-state.connected i{background:#34c759}.cc-switch-state b{display:block;margin:6px 0 3px;font-size:12px}.cc-switch-state.connected b{color:#248a3d}
.header-actions{display:flex;align-items:center;gap:9px}.monitor-launch{display:flex;align-items:center;gap:8px;padding:11px 16px;background:#111}.monitor-launch span{color:#ff3b30;font-size:11px}.monitor-launch.running span,.floating-shell button.proxyLive{color:#34c759}.proxy-launch-status{display:flex;align-items:center;flex-wrap:wrap;gap:8px 14px;margin:-14px 0 22px;padding:12px 14px;border-radius:12px;background:#f2f2f7;color:#525252;font-size:11px}.proxy-launch-status>div{display:flex;flex-wrap:wrap;gap:6px;width:100%}.proxy-launch-status code{padding:6px 8px;border-radius:7px;background:#fff;color:#111}.all-dashboard{display:grid;gap:16px;margin-bottom:18px}.all-summary{display:flex;align-items:flex-end;justify-content:space-between;gap:32px;padding:26px 28px;border-radius:16px;background:#111;color:#fff}.all-summary>div>span,.all-summary p{color:#b7b7bd;font-size:11px}.all-summary>div>strong{display:block;margin:7px 0 4px;font-size:34px;letter-spacing:-1.2px}.all-summary p{margin:0}.all-summary dl{display:flex;gap:30px;margin:0}.all-summary dl div{min-width:72px}.all-summary dt{color:#aaaab0;font-size:10px}.all-summary dd{margin:6px 0 0;font-size:19px;font-weight:700}.all-overview-grid{display:grid;grid-template-columns:minmax(0,1.6fr) minmax(230px,.6fr);gap:16px}.all-overview-grid>section,.all-overview-grid>aside{padding:22px;border-radius:16px;background:#fff;border:1px solid #e5e5e5}.section-title{display:flex;align-items:center;justify-content:space-between}.section-title span{color:#86868b;font-size:10px}.provider-ranking{display:grid;gap:13px;margin-top:18px}.provider-ranking>div{display:grid;grid-template-columns:145px minmax(100px,1fr) 90px;align-items:center;gap:12px}.provider-ranking span b,.provider-ranking span small{display:block}.provider-ranking span b{font-size:12px}.provider-ranking span small{margin-top:3px;color:#86868b;font-size:9px}.provider-ranking i{height:7px;overflow:hidden;border-radius:99px;background:#ececef}.provider-ranking em{display:block;height:100%;border-radius:99px;background:#111}.provider-ranking>div>strong{text-align:right;font-size:11px}.inline-empty{padding:30px 0;color:#86868b;font-size:11px}.all-overview-grid>aside strong{display:block;margin:16px 0 7px;font-size:18px}.all-overview-grid>aside strong.online{color:#248a3d}.all-overview-grid>aside p{color:#737373;font-size:11px;line-height:1.6}.codex-dashboard{display:grid;gap:16px;margin-bottom:18px}.codex-title{display:flex;align-items:flex-start;justify-content:space-between;gap:20px}.codex-title span{font-size:11px;font-weight:700}.codex-title h2{margin:5px 0 7px;font-size:22px}.codex-title p{max-width:680px;margin:0;color:#737373;font-size:11px;line-height:1.6}.codex-kpis{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:14px}.codex-kpis .card{padding:18px 20px}.codex-kpis span,.codex-kpis small{display:block;color:#737373;font-size:10px}.codex-kpis strong{display:block;margin:9px 0;font-size:22px}.codex-kpis .time-value{font-size:18px}.codex-quota-card{display:grid;grid-template-columns:1fr 1fr;gap:34px}.codex-quota-card>div>strong{display:block;margin:4px 0;font-size:26px}.codex-quota-card small{color:#737373;font-size:10px}.beginner-guide{margin-top:18px;padding:26px;border-radius:16px;background:#fff;border:1px solid #e5e5e5}.guide-heading{display:flex;align-items:flex-start;justify-content:space-between}.guide-heading p{margin:6px 0 0;color:#737373;font-size:12px}.guide-heading>span{padding:5px 8px;border-radius:99px;background:#f2f2f7;font-size:10px}.beginner-guide ol{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:0;margin:22px 0 0;padding:0;list-style:none;counter-reset:guide}.beginner-guide li{counter-increment:guide;padding:18px 22px 20px 50px;position:relative;border-top:1px solid #ededf0}.beginner-guide li:nth-child(odd){border-right:1px solid #ededf0}.beginner-guide li::before{content:counter(guide);position:absolute;left:12px;top:17px;display:grid;place-items:center;width:25px;height:25px;border-radius:8px;background:#111;color:#fff;font-size:11px;font-weight:700}.beginner-guide li>b{font-size:13px}.beginner-guide li p{min-height:48px;margin:7px 0 12px;color:#626267;font-size:11px;line-height:1.55}.beginner-guide li code{display:block;padding:9px;border-radius:8px;background:#f2f2f7;color:#111;font-size:10px;overflow:hidden;text-overflow:ellipsis}.beginner-guide li>small{display:block;margin-top:7px;color:#86868b;font-size:9px}.codex-guide-note{display:flex;gap:16px;padding:14px 16px;border-radius:12px;background:#f2f2f7;font-size:11px}.codex-guide-note span{color:#626267}.floating-shell button.proxyLive{background:#111}@media(max-width:1050px){.header-actions{align-items:stretch;flex-direction:column}.monitor-launch,.sync{justify-content:center}.all-summary{align-items:flex-start;flex-direction:column}.all-summary dl{width:100%;justify-content:space-between}.all-overview-grid{grid-template-columns:1fr}.codex-kpis{grid-template-columns:repeat(2,minmax(0,1fr))}.beginner-guide ol{grid-template-columns:1fr}.beginner-guide li:nth-child(odd){border-right:0}.codex-quota-card{grid-template-columns:1fr}}
@media(max-width:1050px){.model-kpis{grid-template-columns:repeat(2,minmax(0,1fr))}.model-kpis .token-primary{grid-column:1/-1}.model-chart-grid{grid-template-columns:1fr}}@media(prefers-reduced-motion:reduce){.sync-icon,.balance-alert{animation:none!important}.metric-bar>i,.floating-chart-point,.floating-chart-point>i,.floating-chart-tooltip,.floating-ring{transition:none!important}.floating-module:hover,.mini-bar:hover,.mini-bar:focus-visible,.floating-ring:hover,.floating-ring:focus-visible{transform:none!important}}
@keyframes spin { to { transform: rotate(360deg); } }@keyframes quotaPulse{50%{opacity:.48}}
.backup-settings{display:grid;grid-template-columns:minmax(0,1fr) 230px auto;align-items:end;gap:16px;margin-top:18px;padding:22px}.backup-settings h2{margin:0 0 6px}.backup-settings p{max-width:70ch;margin:0;color:#737373;font-size:11px;line-height:1.6}.backup-settings label{color:#737373;font-size:10px}.backup-settings input{display:block;width:100%;margin-top:6px;padding:10px;border:1px solid #dedee2;border-radius:10px}.backup-actions{display:flex;gap:8px}.backup-settings>small{grid-column:1/-1;color:#737373;font-size:9px}@media(max-width:1050px){.backup-settings{grid-template-columns:1fr}.backup-settings>small{grid-column:auto}}
.all-cost-analytics{display:grid;grid-template-columns:1fr 1.2fr;gap:14px;margin-bottom:18px}.all-cost-analytics>.card{padding:20px}.all-cost-analytics .section-title>div span{display:block;margin-top:4px}.all-cost-analytics .section-title>strong{font-size:19px}.cost-share-body{display:grid;grid-template-columns:128px 1fr;align-items:center;gap:20px;margin-top:18px}.cost-ring{display:grid;place-items:center;width:128px;height:128px;border-radius:50%}.cost-ring>span{display:grid;place-items:center;width:82px;height:82px;border-radius:50%;background:#fff;color:#737373;text-align:center;font-size:9px;line-height:1.4}.cost-ring b{color:#111;font-size:17px}.cost-share-body>div:last-child{display:grid;gap:8px}.cost-share-body>div:last-child>div{display:grid;grid-template-columns:28px 1fr auto;align-items:center;gap:8px}.cost-share-body :deep(.provider-mark){width:28px;height:28px;border-radius:8px}.cost-share-body :deep(.provider-mark img){width:17px;height:17px}.cost-share-body span b,.cost-share-body span small{display:block;font-size:9px}.cost-share-body span small{margin-top:2px;color:#737373}.cost-share-body>div:last-child>div>strong{font-size:10px}.cost-share-body p{color:#86868b;font-size:10px}.cost-trend svg{display:block;width:100%;height:150px;margin-top:16px;border-radius:12px;background:#f7f7f8}.cost-trend>div:last-child{display:flex;justify-content:space-between;margin-top:7px;color:#86868b;font-size:9px}@media(max-width:1050px){.all-cost-analytics{grid-template-columns:1fr}}
.all-token-history{margin-bottom:18px;padding:22px 24px}.token-chart-header{display:flex;align-items:flex-start;justify-content:space-between;gap:22px}.token-chart-header h2{margin:0;font-size:16px}.token-chart-header p{margin:6px 0 0;color:#626267;font-size:10px;line-height:1.5}.token-range{display:flex;gap:4px;padding:4px;border-radius:12px;background:#f2f2f7}.token-range button{min-width:58px;padding:7px 10px;border:0;border-radius:9px;background:transparent;color:#626267;font-size:10px}.token-range button:hover{background:#fff;color:#111}.token-range button:focus-visible{outline:2px solid #007aff;outline-offset:2px}.token-range button.active{background:#1d1d1f;color:#fff}.token-chart-summary{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:1px;margin-top:19px;overflow:hidden;border-radius:12px;background:#e5e5ea}.token-chart-summary>span{padding:12px 14px;background:#f7f7f8}.token-chart-summary small,.token-chart-summary b{display:block}.token-chart-summary small{color:#737373;font-size:9px}.token-chart-summary b{margin-top:5px;font-size:15px}.stacked-token-chart{display:grid;grid-template-columns:repeat(7,minmax(24px,1fr));align-items:end;gap:10px;height:236px;margin-top:22px;padding:26px 4px 0;border-bottom:1px solid #d8d8dc}.stacked-token-chart.is-30-days{grid-template-columns:repeat(30,minmax(8px,1fr));gap:4px}.token-day{position:relative;display:flex;min-width:0;height:210px;flex-direction:column;align-items:center;justify-content:flex-end}.token-day:focus{outline:none}.token-stack{display:flex;width:min(34px,72%);height:180px;min-width:5px;flex-direction:column-reverse;justify-content:flex-start;overflow:hidden;border-radius:7px 7px 2px 2px;background:#f2f2f7}.is-30-days .token-stack{width:72%;border-radius:4px 4px 1px 1px}.token-stack>i{display:block;width:100%;min-height:1px;flex:0 0 auto}.token-day>small{height:18px;margin-top:7px;color:#737373;font-size:8px;white-space:nowrap}.token-tooltip{position:absolute;z-index:4;left:50%;bottom:44px;display:none;width:190px;max-height:210px;padding:11px 12px;overflow:auto;transform:translateX(-50%);border-radius:12px;background:#1d1d1f;color:#fff;box-shadow:0 4px 8px rgba(0,0,0,.16);font-size:9px}.token-day:first-child .token-tooltip{left:0;transform:none}.token-day:last-child .token-tooltip{right:0;left:auto;transform:none}.token-day:hover .token-tooltip,.token-day:focus .token-tooltip{display:grid;gap:7px}.token-tooltip>b{font-size:10px}.token-tooltip>span{display:grid;grid-template-columns:7px 1fr auto;align-items:center;gap:6px;color:#e5e5ea}.token-tooltip>span i{width:7px;height:7px;border-radius:50%}.token-tooltip>span strong{color:#fff}.token-tooltip>small{color:#c7c7cc}.token-legend{display:flex;flex-wrap:wrap;gap:8px 18px;margin-top:19px}.token-legend>span{position:relative;display:grid;grid-template-columns:26px auto;grid-template-rows:auto auto;align-items:center;column-gap:8px;padding-left:10px}.token-legend>span>i{position:absolute;left:0;width:4px;height:26px;border-radius:99px}.token-legend :deep(.provider-mark){grid-row:1/3;width:26px;height:26px;border-radius:8px}.token-legend :deep(.provider-mark img){width:16px;height:16px}.token-legend b{font-size:9px}.token-legend small{color:#737373;font-size:8px}.token-chart-empty{display:grid;justify-items:start;gap:7px;margin-top:20px;padding:30px;border-radius:14px;background:#f7f7f8}.token-chart-empty b{font-size:13px}.token-chart-empty span{max-width:65ch;color:#737373;font-size:10px;line-height:1.6}.token-chart-empty button{margin-top:5px}@media(max-width:1050px){.token-chart-summary{grid-template-columns:repeat(2,1fr)}.stacked-token-chart{gap:6px}.stacked-token-chart.is-30-days{overflow-x:auto;grid-template-columns:repeat(30,18px);justify-content:start;padding-bottom:2px}.token-chart-header{align-items:stretch;flex-direction:column}.token-range{align-self:flex-start}}
.page-panel{animation:panel-fade-in .2s cubic-bezier(.22,1,.36,1) both}.header-actions{position:relative}.modal-fade-enter-active{transition:opacity .2s cubic-bezier(.22,1,.36,1)}.modal-fade-leave-active{transition:opacity .15s ease}.modal-fade-enter-from,.modal-fade-leave-to{opacity:0}.modal-fade-enter-active .config-modal{transition:opacity .2s cubic-bezier(.22,1,.36,1)}.modal-fade-enter-from .config-modal{opacity:0}.card,.provider-overview,.floating-module{transition:transform .2s cubic-bezier(.22,1,.36,1),box-shadow .2s cubic-bezier(.22,1,.36,1)}.card:hover,.provider-overview:hover{transform:translateY(-1px);box-shadow:0 3px 8px rgba(0,0,0,.055)}.metric-bar>i,.usage-bar>i,.token-stack>i,.floating-chart-bars i,.mini-bar i{transform-origin:bottom;animation:bar-rise .32s cubic-bezier(.22,1,.36,1) both}@keyframes panel-fade-in{from{opacity:0}to{opacity:1}}@keyframes bar-rise{from{transform:scaleY(.06);opacity:.45}to{transform:scaleY(1);opacity:1}}
:global(.motion-off) .page-panel,:global(.motion-off) .metric-bar>i,:global(.motion-off) .usage-bar>i,:global(.motion-off) .token-stack>i,:global(.motion-off) .floating-chart-bars i,:global(.motion-off) .mini-bar i{animation:none!important}:global(.motion-off) .card,:global(.motion-off) .provider-overview,:global(.motion-off) .floating-module,:global(.motion-off) .modal-fade-enter-active,:global(.motion-off) .modal-fade-leave-active{transition:none!important}:global(.motion-off) .card:hover,:global(.motion-off) .provider-overview:hover{transform:none;box-shadow:none}@media(prefers-reduced-motion:reduce){.page-panel,.metric-bar>i,.usage-bar>i,.token-stack>i,.floating-chart-bars i,.mini-bar i{animation:none!important}.card,.provider-overview,.modal-fade-enter-active,.modal-fade-leave-active{transition:none!important}.card:hover,.provider-overview:hover{transform:none;box-shadow:none}}
.remote-content{display:grid;grid-template-columns:auto minmax(0,1fr) auto auto;align-items:center;gap:13px;margin:-8px 0 18px;padding:14px 15px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-bg);color:var(--tm-ink);animation:panel-fade-in .2s ease}.remote-content+.remote-content{margin-top:-10px}.remote-content-badge{padding:5px 8px;border-radius:99px;background:var(--tm-ink);color:var(--tm-on-ink);font-size:9px;font-weight:700;letter-spacing:.06em}.remote-ad .remote-content-badge{background:var(--tm-surface);color:var(--tm-muted);border:1px solid var(--tm-line)}.remote-content-copy{min-width:0}.remote-content-copy b{display:block;font-size:12px}.remote-content-copy p{margin:4px 0 0;color:var(--tm-muted);font-size:10px;line-height:1.55;white-space:pre-wrap}.remote-content-action{padding:8px 12px!important;border-radius:10px!important;white-space:nowrap}.remote-content-close{display:grid;place-items:center;width:30px;height:30px;padding:0!important;border-radius:9px!important;background:transparent!important;color:var(--tm-muted)!important;font-size:18px}.remote-content-close:hover{background:var(--tm-surface)!important;color:var(--tm-ink)!important}@media(max-width:760px){.remote-content{grid-template-columns:auto 1fr auto}.remote-content-copy{grid-column:2/4}.remote-content-action{grid-column:2}.remote-content-close{grid-row:1;grid-column:3}}
/* 覆盖旧悬浮窗的宽度与 header 后代规则，确保新版三枚控制键和右侧环形读数始终留在窗口内。 */
.floating-dashboard-shell{box-sizing:border-box!important;width:100%!important;height:100%!important;font-family:var(--apple-font)!important}
.floating-dashboard-shell>.floating-dashboard-header{position:relative;box-sizing:border-box;width:100%;display:flex!important;min-width:0;padding-right:6.5rem!important}
.floating-dashboard-shell>.floating-dashboard-header .floating-brand{display:flex!important;flex:1 1 auto;min-width:0;width:auto}
.floating-dashboard-shell>.floating-dashboard-header .floating-brand>span{display:block!important;min-width:0}
.floating-dashboard-shell>.floating-dashboard-header nav{position:absolute;right:.125rem;top:.125rem;display:flex!important;width:auto;min-width:max-content}
.floating-dashboard-shell>.floating-dashboard-list{box-sizing:border-box;width:100%;min-width:0}
.floating-dashboard-shell .floating-dashboard-item{position:relative;box-sizing:border-box;width:100%;min-width:0;display:block!important;padding-right:2.125rem}
.floating-dashboard-shell .floating-dashboard-summary{position:relative;box-sizing:border-box;width:100%;min-width:0;grid-template-columns:2.5rem minmax(0,1fr)!important;padding-right:4.25rem!important}
.floating-dashboard-shell .mini-progress-ring{position:absolute;right:2.625rem;top:50%;transform:translateY(-50%)}
.floating-dashboard-shell .floating-dashboard-summary:hover .mini-progress-ring{transform:translateY(-50%) scale(1.035)}
.floating-dashboard-shell .floating-dashboard-expand{position:absolute;right:0;top:0;bottom:auto;box-sizing:border-box;width:2.125rem;height:4.75rem;min-width:0}
.floating-dashboard-shell .floating-dashboard-chart{margin-right:-2.125rem}
/* 常驻联合状态栏：用明暗、填充和明确文字同时表达代理与 CC Switch 状态。 */
.monitor-status-bar{display:grid;grid-template-columns:minmax(280px,1fr) auto auto;align-items:center;gap:14px;margin:-11px 0 21px;padding:14px 15px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-surface);color:var(--tm-ink);transition:background-color .22s ease,color .22s ease,border-color .22s ease}
.monitor-status-bar.is-live{border-color:var(--tm-ink);background:var(--tm-ink);color:var(--tm-on-ink)}
.monitor-status-brand{display:flex;align-items:center;gap:11px;min-width:0}.monitor-status-icon{display:grid;place-items:center;width:38px;height:38px;flex:0 0 auto;border-radius:12px;background:var(--tm-bg);color:var(--tm-ink)}.monitor-status-icon :deep(svg){width:20px;height:20px}.monitor-status-brand>span:last-child{display:grid;gap:2px;min-width:0}.monitor-status-brand small{font-size:8px;letter-spacing:.08em;opacity:.58}.monitor-status-brand b{font-size:13px}.monitor-status-brand em{max-width:650px;overflow:hidden;color:currentColor;font-size:9px;font-style:normal;line-height:1.4;opacity:.58;text-overflow:ellipsis;white-space:nowrap}
.monitor-status-signals{display:flex;gap:7px}.monitor-signal{display:grid;grid-template-columns:24px auto;align-items:center;gap:7px;min-width:112px;padding:8px 10px;border:1px solid color-mix(in srgb,currentColor 13%,transparent);border-radius:12px;background:color-mix(in srgb,var(--tm-bg) 42%,transparent);color:currentColor}.monitor-signal>i{width:8px;height:8px;margin-left:8px;border-radius:50%;background:currentColor;opacity:.3}.monitor-signal.active>i{opacity:1;box-shadow:0 0 0 5px color-mix(in srgb,currentColor 12%,transparent)}.monitor-signal>img{width:24px;height:24px;object-fit:contain;border-radius:7px;background:#fff}.monitor-signal>span{display:grid;gap:2px}.monitor-signal small{font-size:7px;opacity:.56}.monitor-signal b{font-size:9px;white-space:nowrap}.monitor-status-bar.is-live .monitor-signal{background:rgba(255,255,255,.08);border-color:rgba(255,255,255,.14)}.monitor-status-bar.is-live .monitor-signal.active{background:#fff;color:#111}
.monitor-status-action{min-width:98px;padding:10px 12px;border:0;border-radius:11px;background:var(--tm-ink);color:var(--tm-on-ink);font:inherit;font-size:9px;font-weight:700;white-space:nowrap}.monitor-status-bar.is-live .monitor-status-action{background:var(--tm-bg);color:var(--tm-ink)}.monitor-status-action:disabled{opacity:.5}.monitor-endpoints{grid-column:1/-1;padding-top:10px;border-top:1px solid color-mix(in srgb,currentColor 14%,transparent)}.monitor-endpoints summary{width:max-content;cursor:pointer;font-size:9px;opacity:.68}.monitor-endpoints>div{display:flex;flex-wrap:wrap;gap:6px;margin-top:8px}.monitor-endpoints code{padding:6px 8px;border-radius:8px;background:color-mix(in srgb,var(--tm-bg) 80%,transparent);color:var(--tm-ink);font-size:8px}
.monitor-launch span{color:inherit!important}.monitor-launch.running{background:var(--tm-ink)!important;color:var(--tm-on-ink)!important}.monitor-launch.running span,.floating-shell button.proxyLive{color:inherit!important}
@media(max-width:1180px){.monitor-status-bar{grid-template-columns:1fr auto}.monitor-status-signals{grid-column:1}.monitor-status-action{grid-column:2;grid-row:1/3}.monitor-status-brand em{max-width:460px}}
@media(max-width:760px){.monitor-status-bar{grid-template-columns:1fr}.monitor-status-signals{grid-column:auto;flex-wrap:wrap}.monitor-status-action{grid-column:auto;grid-row:auto}.monitor-status-brand em{white-space:normal}.monitor-signal{flex:1}.monitor-endpoints{grid-column:auto}}
.health-launch{display:flex;min-height:42px;flex:0 0 auto;align-items:center;justify-content:center;gap:10px;padding:8px 11px;border:1px solid var(--tm-line);border-radius:13px;background:var(--tm-bg);color:var(--tm-ink);box-shadow:none}.health-launch>span{display:flex;align-items:center;gap:7px;white-space:nowrap}.health-launch i{width:7px;height:7px;flex:0 0 auto;border-radius:50%;background:#34c759;box-shadow:0 0 0 4px color-mix(in srgb,#34c759 13%,transparent)}.health-launch b{font-size:10px}.health-launch small{color:var(--tm-muted);font-size:8px;white-space:nowrap}.health-launch:hover{background:var(--tm-surface)}.health-launch:focus-visible{outline:2px solid var(--tm-ink);outline-offset:2px}.health-launch.warning i{background:#ff9f0a;box-shadow:0 0 0 4px color-mix(in srgb,#ff9f0a 14%,transparent)}
.opencode-local-source{flex-wrap:wrap}.opencode-source-mode{display:grid;gap:4px;min-width:164px;color:var(--tm-muted);font-size:9px}.opencode-source-mode select{min-height:36px;padding:7px 30px 7px 10px;border:1px solid var(--tm-line);border-radius:10px;background:var(--tm-bg);color:var(--tm-ink);font:inherit;font-size:10px}
.dashboard-context-heading .source-badge{margin-left:auto}.dashboard-zero-state{display:grid;justify-items:center;gap:8px;padding:44px 24px;border:1px dashed var(--tm-line);border-radius:16px;background:var(--tm-card);text-align:center}.dashboard-zero-state b{font-size:17px}.dashboard-zero-state p{max-width:520px;margin:0;color:var(--tm-muted);font-size:12px;line-height:1.6}
@media(max-width:1180px){.content>header{align-items:stretch;flex-direction:column;gap:14px;margin-bottom:22px}.header-copy{min-width:0}.header-actions{display:flex!important;flex-direction:column!important;align-items:stretch!important;gap:8px;width:100%;max-width:100%}.header-actions>.theme-switcher,.health-launch,.monitor-launch,.sync{width:100%;min-width:0}.header-actions :deep(.theme-trigger){width:100%;justify-content:center}.monitor-launch-copy{min-width:0}.monitor-launch-copy b,.monitor-launch-copy small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}}
@media(max-width:1050px){.health-launch small{display:none}.health-launch{padding-inline:14px}.monitor-launch{order:initial;flex-basis:auto}.sync{margin-left:0}}
</style>
