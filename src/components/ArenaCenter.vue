<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import {
  ArrowDown,
  ArrowUp,
  ExternalLink,
  Minus,
  RefreshCw,
  Search,
  SlidersHorizontal,
  Swords,
  X,
} from '@lucide/vue'
import ArenaRadarChart from './ArenaRadarChart.vue'
import ProviderMark from './ProviderMark.vue'

type BoardId = 'overall' | 'coding' | 'webdev' | 'vision' | 'search' | 'document'

interface ArenaModel {
  rank: number
  model: string
  provider: string
  license: string
  score: number | null
  score_lower: number | null
  score_upper: number | null
  votes: number | null
  category: string
  published_at: string
}

interface ArenaSnapshot {
  board: BoardId
  updated_at: string
  checked_at?: string
  official_published_at: string
  source_url: string
  source_state: 'live' | 'cache' | 'stale' | 'empty'
  fresh: boolean
  status: string
  total_models: number
  models: ArenaModel[]
}

interface ArenaDimension {
  id: string
  label: string
  sync_state?: 'live' | 'error'
  error?: string | null
  rank: number | null
  total_models: number | null
  score: number | null
  score_lower: number | null
  score_upper: number | null
  votes: number | null
  percentile: number | null
  published_at: string | null
}

interface ArenaModelProfile {
  model: string
  updated_at: string
  checked_at?: string
  source_url: string
  source_state: 'live' | 'partial' | 'cache' | 'stale' | 'empty'
  fresh: boolean
  status: string
  dimensions: ArenaDimension[]
}

interface RankHistoryEntry {
  rank: number
  score: number | null
}

const boards: { id: BoardId; label: string; description: string; source: string }[] = [
  { id: 'overall', label: '综合', description: 'Text · Overall', source: 'https://arena.ai/leaderboard/text' },
  { id: 'coding', label: '编程', description: 'Text · Coding', source: 'https://arena.ai/leaderboard/text/coding' },
  { id: 'webdev', label: 'WebDev', description: 'Code · WebDev', source: 'https://arena.ai/leaderboard/code/webdev' },
  { id: 'vision', label: '视觉', description: 'Vision · Overall', source: 'https://arena.ai/leaderboard/vision' },
  { id: 'search', label: '搜索', description: 'Search · Overall', source: 'https://arena.ai/leaderboard/search' },
  { id: 'document', label: '文档', description: 'Document · Overall', source: 'https://arena.ai/leaderboard/document' },
]

const domesticProviders = new Set([
  'DeepSeek',
  'Kimi',
  '小米 MiMo',
  '智谱 AI',
  '通义百炼',
  '豆包',
  '腾讯混元',
  '文心千帆',
  'MiniMax',
  '阶跃星辰',
  '零一万物',
  '百川智能',
  '华为',
])

function emptySnapshot(board: BoardId): ArenaSnapshot {
  const definition = boards.find(item => item.id === board) || boards[0]
  return {
    board,
    updated_at: '',
    checked_at: '',
    official_published_at: '',
    source_url: definition.source,
    source_state: 'empty',
    fresh: false,
    status: '尚未同步 Arena 官方排行榜',
    total_models: 0,
    models: [],
  }
}

const boardId = ref<BoardId>('overall')
const snapshot = ref<ArenaSnapshot>(emptySnapshot('overall'))
const selected = ref<ArenaModel | null>(null)
const loadingBoards = ref<Set<BoardId>>(new Set())
const loading = computed(() => loadingBoards.value.has(boardId.value))
const profile = ref<ArenaModelProfile | null>(null)
const profileLoading = ref(false)
const search = ref('')
const providerFilter = ref('全部实验室')
const licenseFilter = ref('全部许可证')
const displayLimit = ref(50)
const searchInput = ref<HTMLInputElement | null>(null)
const rankHistory = ref<Record<string, RankHistoryEntry>>({})
const now = ref(Date.now())
let clockTimer = 0
let autoRefreshTimer = 0
let profileRetryTimer = 0
let profileSelectionTimer = 0
let disposed = false
let profileRequestSequence = 0
let profileInFlightModel = ''
const activeBoardRequests = new Map<BoardId, number>()
const boardRequestSequences = new Map<BoardId, number>()
const sharedProfileRequests = new Map<string, Promise<ArenaModelProfile>>()
const normalRefreshMs = 30 * 60 * 1000
const retryRefreshMs = 5 * 60 * 1000

const activeBoard = computed(() => boards.find(item => item.id === boardId.value) || boards[0])
const cacheKey = (board: BoardId) => `token-manager-arena-official-v2-${board}`
const historyKey = (board: BoardId) => `token-manager-arena-history-v2-${board}`
const profileCacheKey = (model: string) => `token-manager-arena-profile-v1-${encodeURIComponent(model)}`
const boardFailureKey = (board: BoardId) => `token-manager-arena-failure-v1-${board}`
const profileFailureKey = (model: string) => `token-manager-arena-profile-failure-v1-${encodeURIComponent(model)}`

function failureCheckedAt(key: string) {
  const parsed = Number(localStorage.getItem(key) || 0)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 0
}

function failureBackoffRemaining(key: string) {
  const checkedAt = failureCheckedAt(key)
  return checkedAt ? Math.max(0, checkedAt + retryRefreshMs - Date.now()) : 0
}

function modelKey(item: ArenaModel) {
  return `${item.provider}::${item.model}`
}

function signature(value: ArenaSnapshot) {
  return JSON.stringify(value.models.map(item => [
    item.rank,
    item.model,
    item.score === null ? null : Math.round(item.score * 10),
  ]))
}

function loadRankHistory(board: BoardId) {
  try {
    const parsed = JSON.parse(localStorage.getItem(historyKey(board)) || '{}') as Record<string, RankHistoryEntry | number>
    rankHistory.value = Object.fromEntries(
      Object.entries(parsed).map(([key, value]) => [
        key,
        typeof value === 'number'
          ? { rank: value, score: null }
          : {
              rank: Number.isFinite(value.rank) ? value.rank : 0,
              score: Number.isFinite(value.score) && value.score !== 0 ? value.score : null,
            },
      ]),
    )
  } catch {
    rankHistory.value = {}
  }
}

function savePreviousSnapshot(value: ArenaSnapshot) {
  const history = Object.fromEntries(
    value.models.map(item => [modelKey(item), { rank: item.rank, score: item.score }]),
  )
  rankHistory.value = history
  localStorage.setItem(historyKey(value.board), JSON.stringify(history))
}

function snapshotCheckTime(value: ArenaSnapshot) {
  const parsed = new Date(value.checked_at || value.updated_at).getTime()
  return Number.isFinite(parsed) ? parsed : 0
}

function refreshInterval(value: ArenaSnapshot) {
  return value.source_state === 'stale' || value.source_state === 'empty'
    ? retryRefreshMs
    : normalRefreshMs
}

function isRefreshDue(value: ArenaSnapshot) {
  const checkedAt = snapshotCheckTime(value)
  return !checkedAt || Date.now() >= checkedAt + refreshInterval(value)
}

function setBoardLoading(board: BoardId, value: boolean) {
  const next = new Set(loadingBoards.value)
  if (value) next.add(board)
  else next.delete(board)
  loadingBoards.value = next
}

function loadCache(board: BoardId) {
  loadRankHistory(board)
  try {
    const value = JSON.parse(localStorage.getItem(cacheKey(board)) || 'null') as ArenaSnapshot | null
    if (value?.models?.length && value.board === board) {
      const wasStale = value.source_state === 'stale'
      snapshot.value = {
        ...value,
        fresh: false,
        source_state: wasStale ? 'stale' : 'cache',
        status: wasStale
          ? '已载入上次离线快照，将自动重试 Arena 官方连接'
          : '已载入本机快照，正在按计划检查 Arena 官方更新',
      }
      selected.value = value.models[0] || null
      return true
    }
  } catch {
    // 损坏的旧缓存直接忽略，下一步会重新读取官方数据。
  }
  snapshot.value = emptySnapshot(board)
  selected.value = null
  return false
}

function scheduleNextRefresh() {
  window.clearTimeout(autoRefreshTimer)
  if (disposed) return
  const checkedAt = snapshotCheckTime(snapshot.value)
  const dueAt = checkedAt
    ? checkedAt + refreshInterval(snapshot.value)
    : Date.now()
  const wait = Math.max(300, dueAt - Date.now())
  const scheduledBoard = boardId.value
  autoRefreshTimer = window.setTimeout(() => {
    if (!disposed && boardId.value === scheduledBoard) void refresh(false, scheduledBoard)
  }, wait)
}

async function refresh(force = false, targetBoard: BoardId = boardId.value) {
  // 同一榜单永远只允许一个请求在途；手动强制刷新只绕过缓存，不制造并发写入。
  if (activeBoardRequests.has(targetBoard)) return
  if (!force && !snapshot.value.models.length) {
    const failureKey = boardFailureKey(targetBoard)
    const remaining = failureBackoffRemaining(failureKey)
    if (remaining > 0) {
      if (boardId.value === targetBoard) {
        const checkedAt = failureCheckedAt(failureKey)
        snapshot.value = {
          ...emptySnapshot(targetBoard),
          checked_at: new Date(checkedAt).toISOString(),
          status: `Arena 官方连接暂不可用 · ${Math.ceil(remaining / 1000)} 秒后自动重试`,
        }
        scheduleNextRefresh()
      }
      return
    }
  }
  const requestId = (boardRequestSequences.get(targetBoard) || 0) + 1
  boardRequestSequences.set(targetBoard, requestId)
  activeBoardRequests.set(targetBoard, requestId)
  setBoardLoading(targetBoard, true)
  try {
    const value = await invoke<ArenaSnapshot>('fetch_arena_rankings', {
      board: targetBoard,
      force,
    })
    if (disposed || boardRequestSequences.get(targetBoard) !== requestId || value.board !== targetBoard) return
    localStorage.removeItem(boardFailureKey(targetBoard))
    if (value.models?.length) {
      const previous = targetBoard === boardId.value ? snapshot.value : null
      if (previous && previous.board === value.board && previous.models.length && signature(previous) !== signature(value)) {
        savePreviousSnapshot(previous)
      }
      localStorage.setItem(cacheKey(value.board), JSON.stringify(value))
      if (boardId.value === targetBoard) {
        snapshot.value = value
        const previousModel = selected.value?.model
        const current = selected.value
          ? value.models.find(item => modelKey(item) === modelKey(selected.value!))
          : null
        selected.value = current || value.models[0] || null
        if (selected.value && selected.value.model === previousModel) await refreshProfile(selected.value.model, force)
      }
    }
  } catch (error) {
    localStorage.setItem(boardFailureKey(targetBoard), String(Date.now()))
    if (!disposed && boardRequestSequences.get(targetBoard) === requestId && boardId.value === targetBoard) {
      snapshot.value = {
        ...snapshot.value,
        checked_at: new Date().toISOString(),
        fresh: false,
        source_state: snapshot.value.models.length ? 'stale' : 'empty',
        status: snapshot.value.models.length
          ? `联网失败，继续显示本机最后快照：${String(error)}`
          : `暂时无法读取 Arena 官方数据：${String(error)}`,
      }
    }
  } finally {
    if (activeBoardRequests.get(targetBoard) === requestId) {
      activeBoardRequests.delete(targetBoard)
      setBoardLoading(targetBoard, false)
    }
    if (!disposed && boardId.value === targetBoard) scheduleNextRefresh()
  }
}

async function selectBoard(id: BoardId) {
  if (id === boardId.value && snapshot.value.models.length) return
  boardId.value = id
  search.value = ''
  providerFilter.value = '全部实验室'
  licenseFilter.value = '全部许可证'
  displayLimit.value = 50
  const cached = loadCache(id)
  if (!cached || isRefreshDue(snapshot.value)) await refresh(false, id)
  else scheduleNextRefresh()
}

function loadProfileCache(model: string) {
  try {
    const value = JSON.parse(localStorage.getItem(profileCacheKey(model)) || 'null') as ArenaModelProfile | null
    if (value?.model === model && Array.isArray(value.dimensions)) {
      const wasStale = value.source_state === 'stale'
      profile.value = {
        ...value,
        source_url: value.source_url || 'https://arena.ai/leaderboard/text',
        source_state: wasStale ? 'stale' : 'cache',
        fresh: false,
        status: wasStale ? '已载入上次离线六维快照' : '已载入六维分类本机快照',
      }
      return true
    }
  } catch {
    // 损坏的模型详情缓存不会影响榜单，联网后会自动重建。
  }
  profile.value = null
  return false
}

async function refreshProfile(model: string, force = false) {
  window.clearTimeout(profileSelectionTimer)
  // 防止同一模型的手动与自动刷新重入，避免后完成的旧请求覆盖新缓存。
  if (profileLoading.value && profileInFlightModel === model) return
  if (!force && !profile.value?.dimensions.length) {
    const failureKey = profileFailureKey(model)
    const remaining = failureBackoffRemaining(failureKey)
    if (remaining > 0) {
      profile.value = {
        model,
        updated_at: '',
        checked_at: new Date(failureCheckedAt(failureKey)).toISOString(),
        source_url: 'https://arena.ai/leaderboard/text',
        source_state: 'empty',
        fresh: false,
        status: `官方六维分类暂不可用 · ${Math.ceil(remaining / 1000)} 秒后自动重试`,
        dimensions: [],
      }
      window.clearTimeout(profileRetryTimer)
      profileRetryTimer = window.setTimeout(() => {
        if (!disposed && selected.value?.model === model) void refreshProfile(model, false)
      }, remaining)
      return
    }
  }
  const requestId = ++profileRequestSequence
  profileInFlightModel = model
  profileLoading.value = true
  try {
    const requestKey = model.trim().toLowerCase()
    let request = force ? undefined : sharedProfileRequests.get(requestKey)
    if (!request) {
      request = invoke<ArenaModelProfile>('fetch_arena_model_profile', { model, force })
      if (!force) {
        sharedProfileRequests.set(requestKey, request)
        void request.finally(() => {
          if (sharedProfileRequests.get(requestKey) === request) sharedProfileRequests.delete(requestKey)
        }).catch(() => {})
      }
    }
    const value = await request
    if (disposed || requestId !== profileRequestSequence || selected.value?.model !== model) return
    localStorage.removeItem(profileFailureKey(model))
    profile.value = value
    localStorage.setItem(profileCacheKey(model), JSON.stringify(value))
  } catch (error) {
    if (disposed || requestId !== profileRequestSequence || selected.value?.model !== model) return
    localStorage.setItem(profileFailureKey(model), String(Date.now()))
    profile.value = profile.value
      ? {
          ...profile.value,
          source_state: 'stale',
          status: `六维分类联网失败，继续显示本机快照：${String(error)}`,
        }
      : {
          model,
          updated_at: '',
          checked_at: new Date().toISOString(),
          source_url: 'https://arena.ai/leaderboard/text',
          source_state: 'empty',
          fresh: false,
          status: `暂时无法读取官方六维分类：${String(error)}`,
          dimensions: [],
        }
  } finally {
    if (requestId === profileRequestSequence) {
      profileInFlightModel = ''
      profileLoading.value = false
      window.clearTimeout(profileRetryTimer)
      if (!disposed && (profile.value?.source_state === 'stale' || profile.value?.source_state === 'empty')) {
        const retryModel = model
        profileRetryTimer = window.setTimeout(() => {
          if (!disposed && selected.value?.model === retryModel) void refreshProfile(retryModel, false)
        }, retryRefreshMs)
      }
    }
  }
}

function searchAliases(item: ArenaModel) {
  const aliases: Record<string, string> = {
    '智谱 AI': 'glm z.ai 智谱',
    '通义百炼': 'qwen alibaba 阿里 通义',
    '豆包': 'doubao bytedance 字节 火山',
    '腾讯混元': 'hunyuan tencent 腾讯',
    '文心千帆': 'ernie baidu 百度 文心',
    '小米 MiMo': 'mimo xiaomi 小米',
    Kimi: 'moonshot 月之暗面',
    '阶跃星辰': 'stepfun step 阶跃',
    '零一万物': 'yi 01.ai lingyi 零一',
    '百川智能': 'baichuan 百川',
    华为: 'huawei 盘古 pangu 华为',
    xAI: 'grok xai',
    Meta: 'llama meta',
    'Mistral AI': 'mistral mixtral',
    NVIDIA: 'nvidia nemotron',
    Perplexity: 'sonar perplexity',
  }
  return `${item.model} ${item.provider} ${aliases[item.provider] || ''}`.toLowerCase()
}

const providers = computed(() => [
  '全部实验室',
  ...Array.from(new Set(snapshot.value.models.map(item => item.provider))).sort((a, b) => a.localeCompare(b, 'zh-CN')),
])
const licenses = computed(() => [
  '全部许可证',
  ...Array.from(new Set(snapshot.value.models.map(item => item.license))).sort((a, b) => a.localeCompare(b, 'zh-CN')),
])
const filteredModels = computed(() => {
  const keyword = search.value.trim().toLowerCase()
  return snapshot.value.models.filter(item => {
    const matchedSearch = !keyword || searchAliases(item).includes(keyword)
    const matchedProvider = providerFilter.value === '全部实验室' || item.provider === providerFilter.value
    const matchedLicense = licenseFilter.value === '全部许可证' || item.license === licenseFilter.value
    return matchedSearch && matchedProvider && matchedLicense
  })
})
const visibleModels = computed(() => filteredModels.value.slice(0, displayLimit.value))
const comparisonModels = computed(() => filteredModels.value.slice(0, 10))
const comparisonScores = computed(() => comparisonModels.value
  .map(item => item.score)
  .filter((value): value is number => value !== null && Number.isFinite(value)))
const comparisonScoreFloor = computed(() => comparisonScores.value.length ? Math.min(...comparisonScores.value) : 0)
const comparisonScoreCeiling = computed(() => comparisonScores.value.length ? Math.max(...comparisonScores.value) : 1)
const recentModels = computed(() => [...snapshot.value.models]
  .filter(item => item.published_at)
  .sort((left, right) => {
    const dateDelta = new Date(right.published_at).getTime() - new Date(left.published_at).getTime()
    return Number.isFinite(dateDelta) && dateDelta !== 0 ? dateDelta : left.rank - right.rank
  })
  .slice(0, 8))
const leader = computed(() => snapshot.value.models[0] || null)
const topDomestic = computed(() => snapshot.value.models.find(item => domesticProviders.has(item.provider)) || null)
const selectedHistory = computed(() => selected.value ? rankHistory.value[modelKey(selected.value)] : undefined)
const selectedRankDelta = computed(() => selected.value && selectedHistory.value
  ? selectedHistory.value.rank - selected.value.rank
  : null)
const selectedScoreDelta = computed(() => (
  selected.value
  && selectedHistory.value
  && selected.value.score !== null
  && selectedHistory.value.score !== null
)
  ? selected.value.score - selectedHistory.value.score
  : null)
const selectedGap = computed(() => (
  selected.value
  && leader.value
  && selected.value.score !== null
  && leader.value.score !== null
)
  ? Math.max(0, leader.value.score - selected.value.score)
  : null)
const selectedPercentile = computed(() => selected.value && snapshot.value.total_models
  ? Math.max(1, Math.ceil((selected.value.rank / snapshot.value.total_models) * 100))
  : 100)
const selectedPosition = computed(() => selected.value && snapshot.value.total_models > 1
  ? Math.max(2, 100 - ((selected.value.rank - 1) / (snapshot.value.total_models - 1)) * 100)
  : 100)
const selectedConfidenceWidth = computed(() => (
  selected.value
  && selected.value.score_lower !== null
  && selected.value.score_upper !== null
)
  ? Math.max(0, selected.value.score_upper - selected.value.score_lower)
  : null)
const sampleStatus = computed(() => {
  if (!selected.value) return '暂无数据'
  if (selected.value.votes === null) return '票数未发布'
  if (selected.value.votes >= 10_000 && selectedConfidenceWidth.value !== null && selectedConfidenceWidth.value <= 20) return '样本较充分'
  if (selected.value.votes >= 3_000) return '持续观察'
  return '样本较少'
})
const sourceLabel = computed(() => ({
  live: '已联网核对',
  cache: '本机快照',
  stale: '联网失败',
  empty: '等待首次同步',
}[snapshot.value.source_state] || '数据状态未知'))
const formattedPublishedDate = computed(() => formatDate(snapshot.value.official_published_at))
const formattedUpdatedTime = computed(() => formatDateTime(snapshot.value.updated_at))
const formattedCheckedTime = computed(() => formatDateTime(snapshot.value.checked_at || snapshot.value.updated_at))
const nextCheckText = computed(() => {
  const checkedAt = snapshotCheckTime(snapshot.value)
  if (!checkedAt) return '正在等待首次检查'
  const due = checkedAt + refreshInterval(snapshot.value)
  const seconds = Math.max(0, Math.ceil((due - now.value) / 1000))
  if (!seconds) return loading.value ? '正在检查官方更新' : '即将自动检查'
  const action = snapshot.value.source_state === 'stale' || snapshot.value.source_state === 'empty'
    ? '后重试'
    : '后检查'
  return `${Math.floor(seconds / 60)} 分 ${String(seconds % 60).padStart(2, '0')} 秒${action}`
})

function formatDate(value: string) {
  if (!value) return '尚未发布'
  const parsed = new Date(value)
  return Number.isNaN(parsed.getTime())
    ? value
    : parsed.toLocaleDateString('zh-CN', { year: 'numeric', month: 'long', day: 'numeric' })
}

function formatDateTime(value: string) {
  if (!value) return '尚未同步'
  const parsed = new Date(value)
  return Number.isNaN(parsed.getTime())
    ? value
    : parsed.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}

function formatScore(value: number | null, digits = 1) {
  return value === null || !Number.isFinite(value) ? '—' : value.toFixed(digits)
}

function formatConfidence(item: ArenaModel) {
  if (item.score_lower === null || item.score_upper === null) return '—'
  return `${item.score_lower.toFixed(1)}–${item.score_upper.toFixed(1)}`
}

function formatVotes(value: number | null) {
  return value === null || !Number.isFinite(value) ? '—' : value.toLocaleString('zh-CN')
}

function comparisonWidth(item: ArenaModel) {
  if (item.score === null || !Number.isFinite(item.score)) {
    return Math.max(14, 100 - ((item.rank - 1) / Math.max(snapshot.value.total_models - 1, 1)) * 86)
  }
  const range = Math.max(comparisonScoreCeiling.value - comparisonScoreFloor.value, 1)
  return 28 + ((item.score - comparisonScoreFloor.value) / range) * 72
}

function focusModel(item: ArenaModel) {
  search.value = item.model
  providerFilter.value = '全部实验室'
  licenseFilter.value = '全部许可证'
  selected.value = item
}

function rankDelta(item: ArenaModel) {
  const previous = rankHistory.value[modelKey(item)]
  return previous ? previous.rank - item.rank : null
}

function rankTier(item: ArenaModel) {
  if (item.rank <= 3) return '前三'
  if (item.rank <= 10) return 'TOP 10'
  if (item.rank <= 25) return 'TOP 25'
  if (item.rank <= 50) return 'TOP 50'
  const percentile = snapshot.value.total_models
    ? Math.max(1, Math.ceil((item.rank / snapshot.value.total_models) * 100))
    : 100
  return `前 ${percentile}%`
}

function trendText(item: ArenaModel) {
  const delta = rankDelta(item)
  if (delta === null) return '首次记录'
  if (delta > 0) return `上升 ${delta}`
  if (delta < 0) return `下降 ${Math.abs(delta)}`
  return '持平'
}

function select(item: ArenaModel) {
  selected.value = item
}

function clearFilters() {
  search.value = ''
  providerFilter.value = '全部实验室'
  licenseFilter.value = '全部许可证'
}

function loadMore() {
  displayLimit.value += 50
}

async function openSource() {
  await openUrl(snapshot.value.source_url || activeBoard.value.source)
}

async function openOfficialSearch(keyword = search.value) {
  const value = keyword.trim()
  const url = value
    ? `https://arena.ai/leaderboard/text?q=${encodeURIComponent(value)}`
    : 'https://arena.ai/leaderboard/text'
  await openUrl(url)
}

async function openBattle() {
  await openUrl('https://arena.ai/text')
}

function onShortcut(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
    event.preventDefault()
    searchInput.value?.focus()
  }
}

watch([search, providerFilter, licenseFilter], () => {
  displayLimit.value = 50
})

watch(filteredModels, rows => {
  if (!rows.length) return
  if (!selected.value || !rows.some(item => modelKey(item) === modelKey(selected.value!))) {
    selected.value = rows[0]
  }
})

watch(() => selected.value?.model, model => {
  window.clearTimeout(profileRetryTimer)
  window.clearTimeout(profileSelectionTimer)
  profileRequestSequence += 1
  profileInFlightModel = ''
  profileLoading.value = false
  if (!model) {
    profile.value = null
    return
  }
  loadProfileCache(model)
  profileSelectionTimer = window.setTimeout(() => {
    if (!disposed && selected.value?.model === model) void refreshProfile(model, false)
  }, 140)
})

function refreshIfDue() {
  if (!document.hidden && isRefreshDue(snapshot.value)) void refresh(false, boardId.value)
}

function onVisibilityChange() {
  if (!document.hidden) refreshIfDue()
}

function onGlobalRefresh() {
  void refresh(true, boardId.value)
}

defineExpose({ refresh })

onMounted(() => {
  disposed = false
  const cached = loadCache(boardId.value)
  if (!cached || isRefreshDue(snapshot.value)) void refresh(false, boardId.value)
  else scheduleNextRefresh()
  clockTimer = window.setInterval(() => { now.value = Date.now() }, 1000)
  window.addEventListener('keydown', onShortcut)
  window.addEventListener('focus', refreshIfDue)
  window.addEventListener('token-manager-arena-refresh', onGlobalRefresh)
  document.addEventListener('visibilitychange', onVisibilityChange)
})

onUnmounted(() => {
  disposed = true
  profileRequestSequence += 1
  window.clearInterval(clockTimer)
  window.clearTimeout(autoRefreshTimer)
  window.clearTimeout(profileRetryTimer)
  window.clearTimeout(profileSelectionTimer)
  window.removeEventListener('keydown', onShortcut)
  window.removeEventListener('focus', refreshIfDue)
  window.removeEventListener('token-manager-arena-refresh', onGlobalRefresh)
  document.removeEventListener('visibilitychange', onVisibilityChange)
})
</script>

<template>
  <section class="arena-center">
    <header class="arena-page-heading">
      <div>
        <h2>Arena 排行榜</h2>
        <p>基于 Arena 官方公开数据集，按真实名次、Score、置信区间与票数判断模型位置。</p>
      </div>
      <div class="arena-heading-actions">
        <button type="button" class="secondary-action" @click="openBattle">
          <Swords :size="16" aria-hidden="true" />
          去 Arena 对战
        </button>
        <button type="button" class="primary-action" :disabled="loading" @click="refresh(true)">
          <RefreshCw :size="16" :class="{ spinning: loading }" aria-hidden="true" />
          {{ loading ? '正在同步' : '检查更新' }}
        </button>
      </div>
    </header>

    <section class="arena-source arena-card" aria-live="polite">
      <span class="source-state" :class="snapshot.source_state"><i></i>{{ sourceLabel }}</span>
      <div>
        <b>{{ snapshot.status }}</b>
        <small>官方发布 {{ formattedPublishedDate }} · 数据更新 {{ formattedUpdatedTime }} · 本机检查 {{ formattedCheckedTime }} · {{ nextCheckText }}</small>
      </div>
      <button type="button" @click="openSource">
        官方来源
        <ExternalLink :size="14" aria-hidden="true" />
      </button>
    </section>

    <nav class="arena-board-tabs" aria-label="Arena 榜单分类">
      <button
        v-for="board in boards"
        :key="board.id"
        type="button"
        :class="{ active: boardId === board.id }"
        :aria-current="boardId === board.id ? 'page' : undefined"
        @click="selectBoard(board.id)"
      >
        <b>{{ board.label }}</b>
        <small>{{ board.description }}</small>
      </button>
    </nav>

    <section class="arena-search-panel arena-card">
      <div class="arena-search-row">
        <label class="arena-search-box">
          <Search :size="18" aria-hidden="true" />
          <input
            ref="searchInput"
            v-model="search"
            aria-label="搜索已同步模型或实验室"
            :placeholder="`搜索已同步的 ${snapshot.total_models || 0} 个模型或实验室`"
            @keydown.esc="search = ''"
            @keydown.enter.prevent="openOfficialSearch()"
          >
          <button v-if="search" type="button" aria-label="清空搜索" @click.prevent="search = ''">
            <X :size="15" aria-hidden="true" />
          </button>
          <kbd v-else>Ctrl K</kbd>
        </label>
        <button type="button" class="official-search" @click="openOfficialSearch()">
          <ExternalLink :size="16" aria-hidden="true" />
          {{ search ? '在官网搜索' : '打开官网搜索' }}
        </button>
      </div>
      <div class="arena-filter-row">
        <span><SlidersHorizontal :size="15" aria-hidden="true" />筛选</span>
        <label>
          <span class="sr-only">实验室</span>
          <select v-model="providerFilter" aria-label="按实验室筛选">
            <option v-for="provider in providers" :key="provider">{{ provider }}</option>
          </select>
        </label>
        <label>
          <span class="sr-only">许可证</span>
          <select v-model="licenseFilter" aria-label="按许可证筛选">
            <option v-for="license in licenses" :key="license">{{ license }}</option>
          </select>
        </label>
        <small>找到 <b>{{ filteredModels.length }}</b> 个结果；Enter 可直接前往 Arena 官网搜索。</small>
      </div>
    </section>

    <section class="arena-overview arena-card" aria-label="当前榜单概览">
      <div>
        <small>当前分类</small>
        <b>{{ activeBoard.label }}</b>
        <span>{{ activeBoard.description }}</span>
      </div>
      <div>
        <small>官方模型数</small>
        <b>{{ snapshot.total_models || '—' }}</b>
        <span>完整同步后可本地搜索</span>
      </div>
      <div>
        <small>榜首模型</small>
        <b>{{ leader?.model || '等待同步' }}</b>
        <span>{{ leader ? `${leader.provider} · Score ${formatScore(leader.score)}` : '暂无官方数据' }}</span>
      </div>
      <div>
        <small>国产最高</small>
        <b>{{ topDomestic?.model || '暂无' }}</b>
        <span>{{ topDomestic ? `官方第 ${topDomestic.rank} 名 · ${topDomestic.provider}` : '当前分类未找到' }}</span>
      </div>
    </section>

    <section class="arena-visual-grid" aria-label="模型排名可视化">
      <article class="arena-score-chart arena-card">
        <header>
          <span><small>官方数据图</small><b>TOP 10 Score 对比</b></span>
          <em>{{ activeBoard.label }} · 鼠标悬停查看详情</em>
        </header>
        <div v-if="comparisonModels.length" class="arena-score-bars">
          <button
            v-for="item in comparisonModels"
            :key="`score-${modelKey(item)}`"
            type="button"
            :title="`官方第 ${item.rank} 名 · Score ${formatScore(item.score)} · ${formatVotes(item.votes)} 票`"
            :class="{ active: selected && modelKey(selected) === modelKey(item) }"
            @click="select(item)"
          >
            <span class="score-bar-label"><strong>#{{ item.rank }}</strong><ProviderMark :name="item.provider" /><i>{{ item.model }}</i></span>
            <span class="score-bar-track"><i :style="{ width: `${comparisonWidth(item)}%` }"></i></span>
            <b>{{ formatScore(item.score) }}</b>
          </button>
        </div>
        <div v-else class="arena-visual-empty">同步官方榜单后显示 Score 对比图。</div>
      </article>

      <article class="arena-new-models arena-card">
        <header>
          <span><small>模型发现</small><b>近期收录模型</b></span>
          <em>仅展示官方数据集已发布条目</em>
        </header>
        <div v-if="recentModels.length" class="arena-new-model-grid">
          <button
            v-for="item in recentModels"
            :key="`recent-${modelKey(item)}`"
            type="button"
            :title="`搜索 ${item.model}`"
            @click="focusModel(item)"
          >
            <ProviderMark :name="item.provider" />
            <span><b>{{ item.model }}</b><small>{{ item.provider }} · #{{ item.rank }}</small></span>
            <time>{{ formatDate(item.published_at) }}</time>
          </button>
        </div>
        <div v-else class="arena-visual-empty">等待官方模型发布时间数据。</div>
      </article>
    </section>

    <section class="arena-layout">
      <div class="arena-table-card arena-card">
        <div class="arena-table-heading">
          <span><b>{{ activeBoard.label }}排名</b><small>名次不会被 Token Manager 重新编号</small></span>
          <em>显示 {{ Math.min(visibleModels.length, filteredModels.length) }} / {{ filteredModels.length }}</em>
        </div>
        <div class="arena-table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">名次</th>
                <th scope="col">变化</th>
                <th scope="col">模型 / 实验室</th>
                <th scope="col">Score</th>
                <th scope="col" class="col-ci">95% 置信区间</th>
                <th scope="col" class="col-votes">票数</th>
                <th scope="col">位置</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="item in visibleModels"
                :key="modelKey(item)"
                :class="{ selected: selected && modelKey(selected) === modelKey(item) }"
                :aria-selected="Boolean(selected && modelKey(selected) === modelKey(item))"
                tabindex="0"
                @click="select(item)"
                @keydown.enter.prevent="select(item)"
              >
                <td><strong>#{{ item.rank }}</strong></td>
                <td>
                  <span
                    class="rank-trend"
                    :class="{ up: (rankDelta(item) || 0) > 0, down: (rankDelta(item) || 0) < 0 }"
                    :title="trendText(item)"
                  >
                    <ArrowUp v-if="(rankDelta(item) || 0) > 0" :size="14" aria-hidden="true" />
                    <ArrowDown v-else-if="(rankDelta(item) || 0) < 0" :size="14" aria-hidden="true" />
                    <Minus v-else :size="14" aria-hidden="true" />
                    {{ rankDelta(item) === null ? '首次' : Math.abs(rankDelta(item) || 0) || '持平' }}
                  </span>
                </td>
                <td>
                  <span class="arena-model">
                    <ProviderMark :name="item.provider" />
                    <span>
                      <b>{{ item.model }}</b>
                      <small>{{ item.provider }} · {{ item.license }}</small>
                    </span>
                  </span>
                </td>
                <td><b class="score-value">{{ formatScore(item.score) }}</b></td>
                <td class="col-ci">{{ formatConfidence(item) }}</td>
                <td class="col-votes">{{ formatVotes(item.votes) }}</td>
                <td><span class="rank-tier">{{ rankTier(item) }}</span></td>
              </tr>
              <tr v-if="!visibleModels.length">
                <td colspan="7">
                  <div class="arena-empty">
                    <Search :size="24" aria-hidden="true" />
                    <b>{{ snapshot.models.length ? '本机快照中没有匹配结果' : '尚未取得官方排行榜' }}</b>
                    <span>{{ snapshot.models.length ? '可清除筛选，或在 Arena 官网搜索完整榜单。' : '请检查网络后重新同步。' }}</span>
                    <div>
                      <button v-if="snapshot.models.length" type="button" @click="clearFilters">清除筛选</button>
                      <button type="button" @click="openOfficialSearch()">前往官网</button>
                    </div>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <footer v-if="visibleModels.length < filteredModels.length">
          <button type="button" @click="loadMore">继续显示 50 个模型</button>
        </footer>
      </div>

      <aside v-if="selected" class="arena-inspector arena-card">
        <header>
          <ProviderMark :name="selected.provider" />
          <span>
            <small>{{ selected.provider }}</small>
            <h3>{{ selected.model }}</h3>
          </span>
          <button type="button" :aria-label="`在 Arena 官网搜索 ${selected.model}`" @click="openOfficialSearch(selected.model)">
            <ExternalLink :size="15" aria-hidden="true" />
          </button>
        </header>

        <div class="arena-rank-focus">
          <span><small>{{ activeBoard.label }}官方名次</small><b>#{{ selected.rank }}</b></span>
          <em>{{ rankTier(selected) }}</em>
        </div>

        <div class="arena-position-track" aria-hidden="true">
          <i :style="{ width: `${selectedPosition}%` }"></i>
        </div>
        <p class="arena-position-copy">
          位于当前 {{ snapshot.total_models }} 个模型的前 {{ selectedPercentile }}%。
          {{ selectedGap === null ? '官方分数不足，暂不计算榜首差距。' : selectedGap ? `与榜首相差 ${selectedGap.toFixed(1)} 分。` : '当前位于榜首。' }}
        </p>

        <ArenaRadarChart
          class="arena-radar-panel"
          :dimensions="profile?.dimensions || []"
          :loading="profileLoading"
          :status="profile?.status || '正在准备官方六维分类数据'"
          :model="selected.model"
        />

        <dl class="arena-official-metrics">
          <div>
            <dt>Arena Score</dt>
            <dd>{{ formatScore(selected.score) }}</dd>
          </div>
          <div>
            <dt>95% 置信区间</dt>
            <dd>{{ formatConfidence(selected) }}</dd>
          </div>
          <div>
            <dt>参与票数</dt>
            <dd>{{ formatVotes(selected.votes) }}</dd>
          </div>
          <div>
            <dt>样本状态</dt>
            <dd>{{ sampleStatus }}</dd>
          </div>
        </dl>

        <section class="arena-change-panel">
          <div>
            <small>较上次快照</small>
            <b v-if="selectedRankDelta === null">首次记录</b>
            <b v-else-if="selectedRankDelta > 0" class="positive">上升 {{ selectedRankDelta }} 名</b>
            <b v-else-if="selectedRankDelta < 0" class="negative">下降 {{ Math.abs(selectedRankDelta) }} 名</b>
            <b v-else>名次持平</b>
          </div>
          <div>
            <small>Score 变化</small>
            <b v-if="selectedScoreDelta === null">暂无对比</b>
            <b v-else>{{ selectedScoreDelta > 0 ? '+' : '' }}{{ selectedScoreDelta.toFixed(1) }}</b>
          </div>
        </section>

        <section class="arena-reading-note">
          <b>怎样看这份排名</b>
          <p>名次、Score、置信区间和票数均来自 Arena 官方数据集。上方分类可查看模型在综合、编程、WebDev、视觉、搜索和文档任务中的真实榜单位置。</p>
          <small>Token Manager 不再用综合分推算“速度、推理、视觉”等非官方能力值。</small>
        </section>
      </aside>
    </section>
  </section>
</template>

<style scoped>
.arena-center {
  container-name: arena;
  container-type: inline-size;
  color: var(--tm-ink);
}

.arena-page-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 1.5rem;
  padding: .5rem .125rem 1.25rem;
}
.arena-page-heading h2 {
  margin: 0;
  font-size: 1.5rem;
  letter-spacing: -.035em;
}
.arena-page-heading p {
  max-width: 46rem;
  margin: .45rem 0 0;
  color: var(--tm-muted);
  font-size: .8125rem;
  line-height: 1.55;
}
.arena-heading-actions {
  display: flex;
  flex: 0 0 auto;
  gap: .5rem;
}
.arena-heading-actions button,
.arena-source button,
.official-search,
.arena-empty button,
.arena-table-card footer button {
  display: inline-flex;
  min-height: 2.25rem;
  align-items: center;
  justify-content: center;
  gap: .4375rem;
  padding: 0 .8125rem;
  border: 1px solid var(--tm-line);
  border-radius: .6875rem;
  background: var(--tm-surface);
  color: var(--tm-ink);
  font: 650 .75rem/1 var(--apple-font);
}
.primary-action {
  border-color: var(--tm-accent) !important;
  background: var(--tm-accent) !important;
  color: var(--tm-on-ink) !important;
}
.arena-heading-actions button:disabled {
  cursor: wait;
  opacity: .62;
}

.arena-card {
  border: 1px solid var(--tm-line);
  border-radius: 1rem;
  background: var(--tm-glass);
  box-shadow: 0 .25rem .9rem color-mix(in srgb, var(--tm-ink) 5%, transparent);
  backdrop-filter: blur(1.125rem) saturate(145%);
}

.arena-source {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: .875rem;
  padding: .75rem .875rem;
}
.source-state {
  display: inline-flex;
  min-height: 1.75rem;
  align-items: center;
  gap: .5rem;
  padding: 0 .625rem;
  border-radius: 999px;
  background: var(--tm-surface);
  font-size: .6875rem;
  font-weight: 700;
  white-space: nowrap;
}
.source-state i {
  width: .4375rem;
  height: .4375rem;
  border-radius: 50%;
  background: var(--tm-muted);
}
.source-state.live i {
  background: var(--tm-accent);
  box-shadow: 0 0 0 .25rem color-mix(in srgb, var(--tm-accent) 13%, transparent);
}
.source-state.stale i { background: #ff9f0a; }
.arena-source > div {
  display: grid;
  min-width: 0;
  gap: .2rem;
}
.arena-source b {
  overflow: hidden;
  font-size: .75rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.arena-source small {
  color: var(--tm-muted);
  font-size: .6875rem;
}
.arena-source button {
  min-height: 2rem;
  background: transparent;
}

.arena-board-tabs {
  display: flex;
  gap: .3125rem;
  margin: .75rem 0;
  padding: .3125rem;
  overflow-x: auto;
  border: 1px solid var(--tm-line);
  border-radius: .9375rem;
  background: var(--tm-surface);
  scrollbar-width: none;
}
.arena-board-tabs::-webkit-scrollbar { display: none; }
.arena-board-tabs button {
  display: grid;
  min-width: 6.5rem;
  min-height: 2.75rem;
  flex: 1 0 auto;
  place-content: center;
  gap: .15rem;
  padding: .25rem .625rem;
  border: 0;
  border-radius: .6875rem;
  background: transparent;
  color: var(--tm-muted);
  font: inherit;
  text-align: center;
}
.arena-board-tabs button b { font-size: .75rem; }
.arena-board-tabs button small { font-size: .625rem; }
.arena-board-tabs button.active {
  background: var(--tm-bg);
  color: var(--tm-ink);
  box-shadow: 0 .125rem .5rem color-mix(in srgb, var(--tm-ink) 11%, transparent);
}

.arena-search-panel {
  display: grid;
  gap: .625rem;
  padding: .75rem;
}
.arena-search-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: .625rem;
}
.arena-search-box {
  display: grid;
  min-height: 2.75rem;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: .5rem;
  padding: 0 .75rem;
  border: 1px solid var(--tm-line);
  border-radius: .75rem;
  background: var(--tm-surface);
  color: var(--tm-muted);
  transition: border-color .18s ease, box-shadow .18s ease, background-color .18s ease;
}
.arena-search-box:focus-within {
  border-color: color-mix(in srgb, var(--tm-accent) 58%, var(--tm-line));
  background: var(--tm-bg);
  box-shadow: 0 0 0 .25rem color-mix(in srgb, var(--tm-accent) 11%, transparent);
  color: var(--tm-accent);
}
.arena-search-box input {
  min-width: 0;
  padding: .65rem 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--tm-ink);
  font: 500 .8125rem/1.3 var(--apple-font);
}
.arena-search-box input::placeholder { color: var(--tm-muted); }
.arena-search-box button {
  display: grid;
  width: 1.75rem;
  height: 1.75rem;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: .5rem;
  background: var(--tm-surface-strong);
  color: var(--tm-muted);
}
.arena-search-box kbd {
  padding: .25rem .375rem;
  border: 1px solid var(--tm-line);
  border-radius: .375rem;
  background: var(--tm-bg);
  color: var(--tm-muted);
  font: 600 .625rem/1 var(--apple-font);
}
.official-search { min-height: 2.75rem; }
.arena-filter-row {
  display: flex;
  align-items: center;
  gap: .5rem;
}
.arena-filter-row > span {
  display: inline-flex;
  align-items: center;
  gap: .375rem;
  color: var(--tm-muted);
  font-size: .6875rem;
  font-weight: 700;
}
.arena-filter-row select {
  min-height: 2.125rem;
  max-width: 12rem;
  padding: 0 1.75rem 0 .625rem;
  border: 1px solid var(--tm-line);
  border-radius: .625rem;
  background: var(--tm-bg);
  color: var(--tm-ink);
  font: 600 .6875rem/1 var(--apple-font);
}
.arena-filter-row > small {
  margin-left: auto;
  color: var(--tm-muted);
  font-size: .6875rem;
}
.arena-filter-row > small b { color: var(--tm-ink); }

.arena-overview {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  margin: .75rem 0;
  overflow: hidden;
}
.arena-overview > div {
  display: grid;
  min-width: 0;
  gap: .25rem;
  padding: .875rem 1rem;
}
.arena-overview > div + div { border-left: 1px solid var(--tm-line); }
.arena-overview small,
.arena-overview span {
  overflow: hidden;
  color: var(--tm-muted);
  font-size: .6875rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.arena-overview b {
  overflow: hidden;
  font-size: .875rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.arena-visual-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.1fr) minmax(18rem, .9fr);
  gap: .75rem;
  margin: 0 0 .75rem;
}
.arena-score-chart,
.arena-new-models {
  min-width: 0;
  padding: .875rem;
}
.arena-score-chart > header,
.arena-new-models > header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: .75rem;
}
.arena-score-chart > header span,
.arena-new-models > header span { display: grid; gap: .15rem; }
.arena-score-chart > header small,
.arena-new-models > header small,
.arena-score-chart > header em,
.arena-new-models > header em {
  color: var(--tm-muted);
  font-size: .625rem;
  font-style: normal;
}
.arena-score-chart > header b,
.arena-new-models > header b { font-size: .8125rem; }
.arena-score-bars { display: grid; gap: .25rem; }
.arena-score-bars > button {
  display: grid;
  grid-template-columns: minmax(8.5rem, 1.1fr) minmax(6rem, 1fr) 3.2rem;
  align-items: center;
  gap: .625rem;
  min-height: 2.35rem;
  padding: .25rem .375rem;
  border: 0;
  border-radius: .75rem;
  background: transparent;
  color: var(--tm-ink);
  font: inherit;
  text-align: left;
  transition: background-color .18s ease, transform .18s ease;
}
.arena-score-bars > button:hover,
.arena-score-bars > button.active {
  background: color-mix(in srgb, var(--tm-accent) 8%, var(--tm-surface));
  transform: translateY(-1px);
}
.score-bar-label {
  display: grid;
  min-width: 0;
  grid-template-columns: 2rem 1.75rem minmax(0, 1fr);
  align-items: center;
  gap: .4rem;
}
.score-bar-label strong { color: var(--tm-muted); font-size: .6875rem; }
.score-bar-label :deep(.provider-mark) { width: 1.75rem; height: 1.75rem; border-radius: .55rem; }
.score-bar-label :deep(.provider-mark img),
.score-bar-label :deep(.provider-mark__svg) { width: 1.15rem; height: 1.15rem; }
.score-bar-label i {
  overflow: hidden;
  font-size: .6875rem;
  font-style: normal;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.score-bar-track { height: .42rem; overflow: hidden; border-radius: 999px; background: var(--tm-line); }
.score-bar-track i { display: block; height: 100%; border-radius: inherit; background: var(--tm-accent); transition: width .3s cubic-bezier(.22,1,.36,1); }
.arena-score-bars > button > b { font-size: .6875rem; font-variant-numeric: tabular-nums; text-align: right; }
.arena-new-model-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .375rem; }
.arena-new-model-grid > button {
  display: grid;
  grid-template-columns: 2rem minmax(0, 1fr);
  align-items: center;
  gap: .55rem;
  min-width: 0;
  min-height: 4rem;
  padding: .55rem;
  border: 1px solid var(--tm-line);
  border-radius: .75rem;
  background: var(--tm-surface);
  color: var(--tm-ink);
  font: inherit;
  text-align: left;
}
.arena-new-model-grid > button:hover { border-color: color-mix(in srgb, var(--tm-accent) 45%, var(--tm-line)); }
.arena-new-model-grid :deep(.provider-mark) { width: 2rem; height: 2rem; border-radius: .625rem; }
.arena-new-model-grid > button > span { display: grid; min-width: 0; gap: .15rem; }
.arena-new-model-grid b,
.arena-new-model-grid small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.arena-new-model-grid b { font-size: .6875rem; }
.arena-new-model-grid small,
.arena-new-model-grid time { color: var(--tm-muted); font-size: .5625rem; }
.arena-new-model-grid time { grid-column: 2; margin-top: -.25rem; }
.arena-visual-empty { display: grid; min-height: 9rem; place-items: center; color: var(--tm-muted); font-size: .6875rem; text-align: center; }

.arena-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(17.5rem, 20rem);
  align-items: start;
  gap: .75rem;
}

@container arena (max-width: 62rem) {
  .arena-visual-grid { grid-template-columns: 1fr; }
}

@container arena (max-width: 40rem) {
  .arena-new-model-grid { grid-template-columns: 1fr; }
  .arena-score-bars > button { grid-template-columns: minmax(7.5rem, 1fr) minmax(4rem, .8fr) 2.8rem; }
  .arena-score-chart > header em,
  .arena-new-models > header em { display: none; }
}
.arena-table-card { overflow: hidden; }
.arena-table-heading {
  display: flex;
  min-height: 3.5rem;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0 .875rem;
  border-bottom: 1px solid var(--tm-line);
}
.arena-table-heading > span { display: grid; gap: .15rem; }
.arena-table-heading b { font-size: .8125rem; }
.arena-table-heading small,
.arena-table-heading em {
  color: var(--tm-muted);
  font-size: .6875rem;
  font-style: normal;
}
.arena-table-scroll { overflow: auto; }
.arena-table-card table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}
.arena-table-card th {
  height: 2.25rem;
  padding: 0 .625rem;
  color: var(--tm-muted);
  font-size: .625rem;
  font-weight: 650;
  text-align: left;
}
.arena-table-card th:nth-child(1) { width: 3.25rem; }
.arena-table-card th:nth-child(2) { width: 4.5rem; }
.arena-table-card th:nth-child(3) { width: auto; }
.arena-table-card th:nth-child(4) { width: 4.5rem; }
.arena-table-card th:nth-child(5) { width: 7rem; }
.arena-table-card th:nth-child(6) { width: 5.5rem; }
.arena-table-card th:nth-child(7) { width: 5.25rem; }
.arena-table-card tbody tr {
  border-top: 1px solid var(--tm-line);
  cursor: pointer;
  outline: 0;
  transition: background-color .18s ease;
}
.arena-table-card tbody tr:hover,
.arena-table-card tbody tr:focus-visible,
.arena-table-card tbody tr.selected {
  background: color-mix(in srgb, var(--tm-accent) 8%, var(--tm-surface));
}
.arena-table-card tbody tr.selected {
  box-shadow: inset .1875rem 0 0 var(--tm-accent);
}
.arena-table-card td {
  height: 3.75rem;
  padding: .375rem .625rem;
  overflow: hidden;
  color: var(--tm-muted);
  font-size: .6875rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.arena-table-card td:first-child strong {
  color: var(--tm-ink);
  font-size: .8125rem;
}
.arena-model {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: .625rem;
}
.arena-model :deep(.provider-mark) {
  width: 2rem;
  height: 2rem;
  flex: 0 0 auto;
  border-radius: .625rem;
}
.arena-model > span {
  display: grid;
  min-width: 0;
  gap: .18rem;
}
.arena-model b,
.arena-model small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.arena-model b {
  color: var(--tm-ink);
  font-size: .75rem;
}
.arena-model small { font-size: .625rem; }
.score-value {
  color: var(--tm-ink);
  font-size: .8125rem;
}
.rank-trend {
  display: inline-flex;
  align-items: center;
  gap: .2rem;
  color: var(--tm-muted);
  font-size: .625rem;
}
.rank-trend.up { color: #248a3d; }
.rank-trend.down { color: #d70015; }
.rank-tier {
  display: inline-flex;
  min-height: 1.5rem;
  align-items: center;
  padding: 0 .4375rem;
  border-radius: 999px;
  background: var(--tm-surface-strong);
  color: var(--tm-ink);
  font-size: .625rem;
  font-weight: 700;
}
.arena-table-card footer {
  display: grid;
  place-items: center;
  padding: .75rem;
  border-top: 1px solid var(--tm-line);
}

.arena-empty {
  display: grid;
  min-height: 16rem;
  place-items: center;
  align-content: center;
  gap: .5rem;
  color: var(--tm-muted);
  text-align: center;
  white-space: normal;
}
.arena-empty b {
  color: var(--tm-ink);
  font-size: .8125rem;
}
.arena-empty span {
  max-width: 26rem;
  font-size: .6875rem;
  line-height: 1.55;
}
.arena-empty > div { display: flex; gap: .5rem; }

.arena-inspector {
  position: sticky;
  top: .75rem;
  padding: 1rem;
}
.arena-inspector > header {
  display: grid;
  grid-template-columns: 2.5rem minmax(0, 1fr) auto;
  align-items: center;
  gap: .625rem;
}
.arena-inspector > header :deep(.provider-mark) {
  width: 2.5rem;
  height: 2.5rem;
  border-radius: .75rem;
}
.arena-inspector > header > span {
  display: grid;
  min-width: 0;
  gap: .18rem;
}
.arena-inspector > header small {
  color: var(--tm-muted);
  font-size: .6875rem;
}
.arena-inspector h3 {
  overflow: hidden;
  margin: 0;
  font-size: .875rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.arena-inspector > header button {
  display: grid;
  width: 2rem;
  height: 2rem;
  place-items: center;
  padding: 0;
  border: 1px solid var(--tm-line);
  border-radius: .625rem;
  background: var(--tm-surface);
  color: var(--tm-ink);
}
.arena-rank-focus {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 1rem;
  margin-top: 1.25rem;
}
.arena-rank-focus span { display: grid; gap: .15rem; }
.arena-rank-focus small { color: var(--tm-muted); font-size: .6875rem; }
.arena-rank-focus b {
  font-size: 2.25rem;
  letter-spacing: -.055em;
  line-height: 1;
}
.arena-rank-focus em {
  padding: .3rem .5rem;
  border-radius: 999px;
  background: var(--tm-ink);
  color: var(--tm-on-ink);
  font-size: .625rem;
  font-style: normal;
  font-weight: 750;
}
.arena-position-track {
  height: .4375rem;
  margin-top: .875rem;
  overflow: hidden;
  border-radius: 999px;
  background: var(--tm-surface-strong);
}
.arena-position-track i {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--tm-accent);
  transform-origin: left;
  animation: arena-position-in .28s cubic-bezier(.2,.8,.2,1) both;
}
.arena-position-copy {
  margin: .55rem 0 0;
  color: var(--tm-muted);
  font-size: .6875rem;
  line-height: 1.55;
}
.arena-radar-panel { margin-top: .875rem; }
.arena-official-metrics {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: .5rem;
  margin: 1rem 0 0;
}
.arena-official-metrics > div {
  display: grid;
  gap: .25rem;
  padding: .75rem;
  border-radius: .75rem;
  background: var(--tm-surface);
}
.arena-official-metrics dt {
  color: var(--tm-muted);
  font-size: .625rem;
}
.arena-official-metrics dd {
  margin: 0;
  font-size: .75rem;
  font-weight: 720;
}
.arena-change-panel {
  display: grid;
  grid-template-columns: 1fr 1fr;
  margin-top: .75rem;
  overflow: hidden;
  border: 1px solid var(--tm-line);
  border-radius: .75rem;
}
.arena-change-panel > div {
  display: grid;
  gap: .25rem;
  padding: .75rem;
}
.arena-change-panel > div + div { border-left: 1px solid var(--tm-line); }
.arena-change-panel small { color: var(--tm-muted); font-size: .625rem; }
.arena-change-panel b { font-size: .75rem; }
.positive { color: #248a3d; }
.negative { color: #d70015; }
.arena-reading-note {
  margin-top: .75rem;
  padding: .875rem;
  border-radius: .75rem;
  background: var(--tm-surface);
}
.arena-reading-note b { font-size: .75rem; }
.arena-reading-note p,
.arena-reading-note small {
  display: block;
  margin: .35rem 0 0;
  color: var(--tm-muted);
  font-size: .6875rem;
  line-height: 1.55;
}

.spinning { animation: arena-spin .8s linear infinite; }
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
@keyframes arena-spin { to { transform: rotate(360deg); } }
@keyframes arena-position-in { from { transform: scaleX(.05); opacity: .4; } to { transform: scaleX(1); opacity: 1; } }
:global(.motion-off) .arena-center * {
  animation: none !important;
  transition: none !important;
}
@media (prefers-reduced-motion: reduce) {
  .arena-center * { animation: none !important; transition: none !important; }
}

@container arena (max-width: 64rem) {
  .arena-layout { grid-template-columns: 1fr; }
  .arena-inspector {
    position: static;
    display: grid;
    grid-template-columns: minmax(12rem, .8fr) minmax(18rem, 1.2fr);
    gap: .75rem 1rem;
  }
  .arena-inspector > header,
  .arena-rank-focus,
  .arena-position-track,
  .arena-position-copy { grid-column: 1; }
  .arena-radar-panel {
    grid-column: 2;
    grid-row: 1 / span 4;
    margin-top: 0;
  }
  .arena-official-metrics,
  .arena-change-panel,
  .arena-reading-note { grid-column: 1 / -1; }
  .arena-official-metrics { grid-row: 5; margin-top: 0; }
  .arena-change-panel { grid-row: 6; margin-top: 0; }
  .arena-reading-note { grid-row: 7; margin-top: 0; }
}

@container arena (max-width: 48rem) {
  .arena-page-heading { align-items: flex-start; flex-direction: column; }
  .arena-heading-actions { width: 100%; }
  .arena-heading-actions button { flex: 1; }
  .arena-source { grid-template-columns: 1fr auto; }
  .source-state { justify-self: start; }
  .arena-source > div { grid-column: 1 / -1; grid-row: 2; }
  .arena-search-row { grid-template-columns: 1fr; }
  .official-search { width: 100%; }
  .arena-filter-row { align-items: stretch; flex-wrap: wrap; }
  .arena-filter-row label { flex: 1; }
  .arena-filter-row select { width: 100%; max-width: none; }
  .arena-filter-row > small { width: 100%; margin-left: 0; }
  .arena-overview { grid-template-columns: 1fr 1fr; }
  .arena-overview > div:nth-child(3) { border-left: 0; border-top: 1px solid var(--tm-line); }
  .arena-overview > div:nth-child(4) { border-top: 1px solid var(--tm-line); }
  .arena-table-card .col-ci,
  .arena-table-card .col-votes { display: none; }
  .arena-table-card th:nth-child(2) { width: 4rem; }
  .arena-table-card th:nth-child(4) { width: 4rem; }
  .arena-table-card th:nth-child(7) { width: 4.75rem; }
  .arena-inspector { display: block; }
  .arena-radar-panel { margin-top: .875rem; }
  .arena-official-metrics { margin-top: 1rem; }
  .arena-change-panel,
  .arena-reading-note { margin-top: .75rem; }
}
</style>
