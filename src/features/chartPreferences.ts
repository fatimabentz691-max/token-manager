import { ref } from 'vue'

export type DashboardMode = 'simple' | 'advanced'
export type ChartRange = 'today' | '7d' | '30d'
export type ChartId =
  | 'monthlyShare'
  | 'cacheDoubleRing'
  | 'codexCategory'
  | 'quota7d'
  | 'tokenTrend'
  | 'costTrend'
  | 'callTrend'
  | 'tokenCacheBars'
  | 'modelCompare'
  | 'weekCost'
  | 'budgetGauge'
  | 'quota5h'
  | 'cacheGauge'
  | 'tokenCallCombo'
  | 'topRanking'

export const chartCatalog: Array<{ id: ChartId; title: string; description: string }> = [
  { id: 'monthlyShare', title: '月度消费占比', description: '各平台人民币消费占比环形图' },
  { id: 'cacheDoubleRing', title: '缓存命中双环', description: 'Token 与请求两个维度的命中占比' },
  { id: 'codexCategory', title: 'Codex 任务分类', description: '生成、调试、问答三类消耗占比' },
  { id: 'quota7d', title: 'Codex 7 天额度', description: '滚动额度占用环形进度' },
  { id: 'tokenTrend', title: 'Token 消耗趋势', description: '今日、7 天或 30 天多模型曲线' },
  { id: 'costTrend', title: '消费金额趋势', description: '每日金额与预算预警水平线' },
  { id: 'callTrend', title: 'API 调用趋势', description: '每日 API 请求次数变化' },
  { id: 'tokenCacheBars', title: 'Token 与缓存节省', description: '每日分组柱状对比' },
  { id: 'modelCompare', title: '当日模型对比', description: '适合快速比较的横向短柱图' },
  { id: 'weekCost', title: '周开销对比', description: '最近四周人民币开销' },
  { id: 'budgetGauge', title: '月度预算占用', description: '正常、警告、危险三色进度环' },
  { id: 'quota5h', title: 'Codex 5 小时额度', description: '圆角额度进度条与精确 Token' },
  { id: 'cacheGauge', title: '缓存命中率', description: '缓存命中率环形指示器' },
  { id: 'tokenCallCombo', title: 'Token / 请求双轴', description: 'Token 与请求次数组合趋势' },
  { id: 'topRanking', title: '近 7 天消耗排行', description: '消耗最高的五个模型或会话' }
]

const allIds = chartCatalog.map(item => item.id)
export const simpleChartIds: ChartId[] = ['tokenTrend', 'modelCompare', 'budgetGauge', 'quota5h']

function loadEnabled(): ChartId[] {
  try {
    const saved = JSON.parse(localStorage.getItem('token-manager-chart-enabled') || 'null')
    return Array.isArray(saved) ? allIds.filter(id => saved.includes(id)) : [...allIds]
  } catch {
    return [...allIds]
  }
}

function loadOrder(): ChartId[] {
  try {
    const saved = JSON.parse(localStorage.getItem('token-manager-chart-order') || 'null')
    const order = Array.isArray(saved) ? allIds.filter(id => saved.includes(id)) : []
    for (const id of allIds) if (!order.includes(id)) order.push(id)
    return order
  } catch {
    return [...allIds]
  }
}

const mode = ref<DashboardMode>(localStorage.getItem('token-manager-dashboard-mode') === 'advanced' ? 'advanced' : 'simple')
const range = ref<ChartRange>((['today', '7d', '30d'].includes(localStorage.getItem('token-manager-chart-range') || '') ? localStorage.getItem('token-manager-chart-range') : '7d') as ChartRange)
const enabled = ref<ChartId[]>(loadEnabled())
const order = ref<ChartId[]>(loadOrder())

export function useChartPreferences() {
  function setMode(value: DashboardMode) {
    mode.value = value
    localStorage.setItem('token-manager-dashboard-mode', value)
  }

  function setRange(value: ChartRange) {
    range.value = value
    localStorage.setItem('token-manager-chart-range', value)
  }

  function setChartEnabled(id: ChartId, value: boolean) {
    enabled.value = value ? [...new Set([...enabled.value, id])] : enabled.value.filter(item => item !== id)
    localStorage.setItem('token-manager-chart-enabled', JSON.stringify(enabled.value))
  }

  function resetCharts() {
    enabled.value = [...allIds]
    order.value = [...allIds]
    localStorage.setItem('token-manager-chart-enabled', JSON.stringify(enabled.value))
    localStorage.setItem('token-manager-chart-order', JSON.stringify(order.value))
  }

  function moveChart(source: ChartId, target: ChartId) {
    if (source === target) return
    const next = [...order.value]
    const from = next.indexOf(source)
    const to = next.indexOf(target)
    if (from < 0 || to < 0) return
    next.splice(from, 1)
    next.splice(to, 0, source)
    order.value = next
    localStorage.setItem('token-manager-chart-order', JSON.stringify(next))
  }

  function moveChartBy(id: ChartId, delta: -1 | 1) {
    const visible = order.value.filter(item => enabled.value.includes(item))
    const index = visible.indexOf(id)
    const target = visible[index + delta]
    if (target) moveChart(id, target)
  }

  return { mode, range, enabled, order, setMode, setRange, setChartEnabled, resetCharts, moveChart, moveChartBy }
}
