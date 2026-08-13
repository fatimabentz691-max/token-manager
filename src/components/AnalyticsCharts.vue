<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { init, use, type EChartsType } from 'echarts/core'
import type { EChartsOption } from 'echarts'
import { BarChart, LineChart, PieChart } from 'echarts/charts'
import { DataZoomComponent, GridComponent, LegendComponent, MarkLineComponent, TitleComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import type { Usage } from '../types'
import { chartCatalog, useChartPreferences, type ChartId, type ChartRange, type DashboardMode } from '../features/chartPreferences'
import { useChartColorPreferences } from '../features/chartColorPreferences'
import { useThemePreferences } from '../features/themePreferences'
import { useMotionPreferences } from '../features/motionPreferences'

interface CodexPoint { at: number; tokens: number; model: string; session_id?: string; category?: string }

const props = defineProps<{
  usage: Usage[]
  codexSeries: CodexPoint[]
  now: number
  monthlyBudget: number
  used5h: number
  budget5h: number
  used7d: number
  budget7d: number
  context?: 'all' | 'codex' | 'model'
}>()

use([BarChart, LineChart, PieChart, DataZoomComponent, GridComponent, LegendComponent, MarkLineComponent, TitleComponent, TooltipComponent, CanvasRenderer])

const { mode, range, enabled, order, setMode, setRange, setChartEnabled, moveChart } = useChartPreferences()
const customizing = ref(false)
const draggedChart = ref<ChartId | null>(null)
const chartElements = new Map<ChartId, HTMLElement>()
const instances = new Map<ChartId, EChartsType>()
const { theme: rawTheme, liquidTone } = useThemePreferences()
const { chartColorTheme } = useChartColorPreferences()
const { motionEnabled } = useMotionPreferences()
const appleFontStack = '"SF Pro Text", "SF Pro Display", "PingFang SC", "PingFang TC", "苹方-简", -apple-system, BlinkMacSystemFont, "Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", sans-serif'
function mixHex(from:string,to:string,amount:number){const parse=(value:string)=>[1,3,5].map(index=>parseInt(value.slice(index,index+2),16));const a=parse(from),b=parse(to);return`#${a.map((value,index)=>Math.round(value+(b[index]-value)*amount).toString(16).padStart(2,'0')).join('')}`}
// 图表必须读取“实际呈现”的主题，而不是液态主题的原始深色定义。
// 纯白模式统一使用白底、深色文字、浅灰网格，并跟随用户选择的图表主色。
const theme = computed(() => {
  const value = rawTheme.value
  if (value.material === 'liquid' && liquidTone.value === 'clear') {
    return { ...value, background: '#FFFFFF', ink: '#1D1D1F', accent: chartColorTheme.value.series[0], glow: chartColorTheme.value.series[2], dark: false }
  }
  return value
})
const chartAppearance = theme
const chartPalette = computed(() => [...chartColorTheme.value.series])

const contextCharts: Record<'all' | 'codex' | 'model', ChartId[]> = {
  all: chartCatalog.map(item => item.id),
  codex: ['tokenTrend', 'codexCategory', 'quota7d', 'quota5h', 'tokenCallCombo', 'topRanking'],
  model: ['tokenTrend', 'costTrend', 'callTrend', 'tokenCacheBars', 'modelCompare', 'weekCost', 'cacheDoubleRing', 'cacheGauge', 'tokenCallCombo', 'topRanking']
}
const simpleByContext: Record<'all' | 'codex' | 'model', ChartId[]> = {
  all: ['tokenTrend', 'modelCompare', 'budgetGauge', 'quota5h'],
  codex: ['tokenTrend', 'codexCategory', 'quota7d', 'quota5h'],
  model: ['tokenTrend', 'costTrend', 'callTrend', 'cacheGauge']
}

const allowedCharts = computed(() => {
  const context = props.context || 'all'
  return mode.value === 'simple' ? simpleByContext[context] : contextCharts[context]
})
const visibleCharts = computed(() => order.value.filter(id => allowedCharts.value.includes(id) && enabled.value.includes(id)))
const availableCharts = computed(() => allowedCharts.value.filter(id => !enabled.value.includes(id)))
const wideChartIds = new Set<ChartId>(['tokenTrend', 'costTrend', 'tokenCallCombo'])

/**
 * 两列网格中，全宽图表会切断当前行。每个全宽图表前后的普通卡片若为奇数，
 * 让该分段的最后一张卡横跨整行，保证自定义增删和拖动后不会遗留半行空洞。
 */
const fillRowCharts = computed(() => {
  const fills = new Set<ChartId>()
  let segment: ChartId[] = []
  const closeSegment = () => {
    if (segment.length % 2 === 1) fills.add(segment[segment.length - 1])
    segment = []
  }
  for (const id of visibleCharts.value) {
    if (wideChartIds.has(id)) closeSegment()
    else segment.push(id)
  }
  closeSegment()
  return fills
})

function startChartDrag(id: ChartId, event: DragEvent) {
  if (!customizing.value) return
  draggedChart.value = id
  if (event.dataTransfer) { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData('text/plain', id) }
}
function dropChart(target: ChartId) {
  if (draggedChart.value) moveChart(draggedChart.value, target)
  draggedChart.value = null
}

function localDayKey(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

function compact(value: number) {
  return new Intl.NumberFormat('zh-CN', { notation: value >= 10_000 ? 'compact' : 'standard', maximumFractionDigits: 1 }).format(value)
}

const buckets = computed(() => {
  const count = range.value === 'today' ? 24 : range.value === '7d' ? 7 : 30
  const rows = Array.from({ length: count }, (_, index) => {
    const date = new Date(props.now)
    if (range.value === 'today') date.setHours(index, 0, 0, 0)
    else { date.setHours(0, 0, 0, 0); date.setDate(date.getDate() - (count - 1 - index)) }
    const key = range.value === 'today' ? `${localDayKey(date)}-${String(index).padStart(2, '0')}` : localDayKey(date)
    return { key, label: range.value === 'today' ? `${String(index).padStart(2, '0')}:00` : `${date.getMonth() + 1}/${date.getDate()}`, tokens: 0, cost: 0, calls: 0, cached: 0, providers: new Map<string, number>() }
  })
  const index = new Map(rows.map(row => [row.key, row]))
  for (const item of props.usage) {
    const date = new Date(item.at)
    const key = range.value === 'today' ? `${localDayKey(date)}-${String(date.getHours()).padStart(2, '0')}` : localDayKey(date)
    const row = index.get(key)
    if (!row) continue
    const tokens = item.input + item.output
    row.tokens += tokens; row.cost += item.cost; row.calls += item.request_count ?? 1; row.cached += item.cached
    row.providers.set(item.provider, (row.providers.get(item.provider) || 0) + tokens)
  }
  for (const item of props.codexSeries) {
    const date = new Date(item.at * 1000)
    const key = range.value === 'today' ? `${localDayKey(date)}-${String(date.getHours()).padStart(2, '0')}` : localDayKey(date)
    const row = index.get(key)
    if (!row) continue
    row.tokens += item.tokens; row.calls += 1
    row.providers.set('Codex', (row.providers.get('Codex') || 0) + item.tokens)
  }
  return rows
})

const providerNames = computed(() => {
  const totals = new Map<string, number>()
  for (const row of buckets.value) for (const [provider, tokens] of row.providers) totals.set(provider, (totals.get(provider) || 0) + tokens)
  return [...totals.entries()].sort((a, b) => b[1] - a[1]).map(([name]) => name)
})

const monthUsage = computed(() => props.usage.filter(item => new Date(item.at).getTime() >= props.now - 30 * 86_400_000))
const monthCostByProvider = computed(() => {
  const totals = new Map<string, number>()
  for (const item of monthUsage.value) totals.set(item.provider, (totals.get(item.provider) || 0) + item.cost)
  return [...totals.entries()].filter(([, value]) => value > 0).sort((a, b) => b[1] - a[1])
})
const monthCost = computed(() => monthUsage.value.reduce((sum, item) => sum + item.cost, 0))
const totalInput = computed(() => monthUsage.value.reduce((sum, item) => sum + item.input, 0))
const totalCached = computed(() => monthUsage.value.reduce((sum, item) => sum + item.cached, 0))
const cacheRate = computed(() => totalInput.value ? Math.min(100, totalCached.value / totalInput.value * 100) : 0)
const cachedCalls = computed(() => monthUsage.value.filter(item => item.cached > 0).reduce((sum,item)=>sum+(item.request_count??1),0))

const codexCategories = computed(() => {
  const values = new Map([['全新生成', 0], ['修改调试', 0], ['问答解释', 0]])
  for (const point of props.codexSeries) {
    const raw = point.category || '修改调试'
    const key = raw.includes('生成') ? '全新生成' : raw.includes('问答') || raw.includes('解释') ? '问答解释' : '修改调试'
    values.set(key, (values.get(key) || 0) + point.tokens)
  }
  return [...values.entries()]
})

const todayModels = computed(() => {
  const today = localDayKey(new Date(props.now))
  const totals = new Map<string, number>()
  for (const item of props.usage) if (localDayKey(new Date(item.at)) === today) totals.set(item.model, (totals.get(item.model) || 0) + item.input + item.output)
  for (const item of props.codexSeries) if (localDayKey(new Date(item.at * 1000)) === today) totals.set(item.model || 'Codex', (totals.get(item.model || 'Codex') || 0) + item.tokens)
  return [...totals.entries()].sort((a, b) => b[1] - a[1]).slice(0, 8)
})

const weekCosts = computed(() => Array.from({ length: 4 }, (_, index) => {
  const end = props.now - (3 - index) * 7 * 86_400_000
  const start = end - 7 * 86_400_000
  return { label: index === 3 ? '本周' : `${3 - index}周前`, value: props.usage.filter(item => { const at = new Date(item.at).getTime(); return at >= start && at < end }).reduce((sum, item) => sum + item.cost, 0) }
}))

const topRanking = computed(() => {
  const since = props.now - 7 * 86_400_000
  const totals = new Map<string, number>()
  for (const item of props.usage) if (new Date(item.at).getTime() >= since) totals.set(`${item.provider} / ${item.model}`, (totals.get(`${item.provider} / ${item.model}`) || 0) + item.input + item.output)
  for (const item of props.codexSeries) if (item.at * 1000 >= since) totals.set(`Codex / ${item.model}`, (totals.get(`Codex / ${item.model}`) || 0) + item.tokens)
  return [...totals.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5)
})

const used5hPercent = computed(() => Math.min(100, props.budget5h ? props.used5h / props.budget5h * 100 : 0))
const used7dPercent = computed(() => Math.min(100, props.budget7d ? props.used7d / props.budget7d * 100 : 0))
const budgetPercent = computed(() => Math.min(100, props.monthlyBudget ? monthCost.value / props.monthlyBudget * 100 : 0))

function chartColor(index: number) { return chartPalette.value[index % chartPalette.value.length] }
function setChartRef(id: ChartId, element: unknown) { if (element instanceof HTMLElement) chartElements.set(id, element); else chartElements.delete(id) }
function hasData(id: ChartId) {
  if (id === 'monthlyShare') return monthCostByProvider.value.length > 0
  if (id === 'cacheDoubleRing' || id === 'cacheGauge') return totalInput.value > 0
  if (id === 'codexCategory') return codexCategories.value.some(([, value]) => value > 0)
  if (id === 'quota7d' || id === 'quota5h') return props.budget7d > 0
  if (id === 'weekCost' || id === 'costTrend' || id === 'budgetGauge') return monthCost.value > 0
  if (id === 'modelCompare') return todayModels.value.length > 0
  if (id === 'topRanking') return topRanking.value.length > 0
  return buckets.value.some(row => row.tokens || row.calls || row.cached || row.cost)
}

function emptyOption(message: string): EChartsOption {
  const value=chartAppearance.value
  return { animation: false, title: { text: message, left: 'center', top: '42%', textStyle: { color: value.dark ? mixHex(value.ink,value.background,.42) : mixHex(value.ink,value.background,.52), fontSize: 12, fontWeight: 400 } } }
}

function commonAxis() {
  const value=chartAppearance.value
  const axis=mixHex(value.ink,value.background,value.dark ? .72 : .86)
  const label=mixHex(value.ink,value.background,value.dark ? .42 : .48)
  const split=mixHex(value.ink,value.background,value.dark ? .86 : .92)
  return {
    axisLine: { lineStyle: { color: axis } }, axisTick: { show: false },
    axisLabel: { color: label, fontSize: 10 }, splitLine: { lineStyle: { color: split } }
  }
}

function tooltip(unit = '') {
  const value=chartAppearance.value
  return { trigger: 'axis' as const, axisPointer:{type:'line' as const,lineStyle:{color:mixHex(value.ink,value.background,.68),width:1}}, backgroundColor: value.dark?mixHex(value.background,value.ink,.12):'#FFFFFF', borderColor:mixHex(value.ink,value.background,value.dark ? .78 : .86), borderWidth:1, borderRadius:12, padding: [10, 12], transitionDuration:.18, extraCssText:'box-shadow:0 10px 28px rgba(29,29,31,.12);', textStyle: { color: value.ink, fontSize: 11, fontWeight:500 }, valueFormatter: (next: unknown) => `${Number(next).toLocaleString('zh-CN')}${unit}` }
}

function zoom(): EChartsOption['dataZoom'] {
  const value=chartAppearance.value
  return range.value === '30d' ? [{ type: 'inside' }, { type: 'slider', height: 14, bottom: 2, borderColor: mixHex(value.ink,value.background,.86), fillerColor: mixHex(value.accent,value.background,.78), backgroundColor:mixHex(value.ink,value.background,.95), dataBackground:{lineStyle:{color:mixHex(value.ink,value.background,.72)},areaStyle:{color:mixHex(value.ink,value.background,.9)}}, selectedDataBackground:{lineStyle:{color:value.accent},areaStyle:{color:mixHex(value.accent,value.background,.72)}} }] : [{ type: 'inside' }]
}

function optionFor(id: ChartId): EChartsOption {
  if (!hasData(id)) return emptyOption(id.includes('quota') ? '请先设置 Codex 个人额度' : '暂无真实数据，开始调用后自动显示')
  const labels = buckets.value.map(row => row.label)
  const grid = { left: 52, right: 20, top: 35, bottom: range.value === '30d' ? 42 : 25 }
  if (id === 'monthlyShare') return { tooltip: { trigger: 'item', formatter: '{b}<br/>¥{c} · {d}%' }, legend: { bottom: 0, textStyle: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 10 } }, color: chartPalette.value, series: [{ type: 'pie', radius: ['50%', '72%'], center: ['50%', '43%'], label: { show: false }, itemStyle: { borderColor: theme.value.background, borderWidth: 3, borderRadius: 6 }, data: monthCostByProvider.value.map(([name, value]) => ({ name, value: Number(value.toFixed(4)) })) }] }
  if (id === 'cacheDoubleRing') return { tooltip: { trigger: 'item', formatter: '{a}<br/>{b}：{c} · {d}%' }, color: [chartColor(0), mixHex(theme.value.ink,theme.value.background,.84), chartColor(2), mixHex(theme.value.ink,theme.value.background,.92)], series: [{ name: 'Token', type: 'pie', radius: ['54%', '72%'], label: { show: false }, data: [{ name: '命中 Token', value: totalCached.value }, { name: '未命中 Token', value: Math.max(0, totalInput.value - totalCached.value) }] }, { name: '请求', type: 'pie', radius: ['30%', '44%'], label: { show: false }, data: [{ name: '命中请求', value: cachedCalls.value }, { name: '未命中请求', value: Math.max(0, monthUsage.value.length - cachedCalls.value) }] }] }
  if (id === 'codexCategory') return { tooltip: { trigger: 'item', formatter: '{b}<br/>{c} Token · {d}%' }, color: [chartColor(0), chartColor(1), chartColor(2)], series: [{ type: 'pie', radius: ['26%', '72%'], label: { color: mixHex(theme.value.ink,theme.value.background,.38), fontSize: 10, formatter: '{b}\n{d}%' }, itemStyle: { borderColor: theme.value.background, borderWidth: 3, borderRadius: 8 }, data: codexCategories.value.map(([name, value]) => ({ name, value })) }] }
  if (id === 'quota7d') return ringOption(used7dPercent.value, '7 天已用', chartColor(0))
  if (id === 'budgetGauge') return ringOption(budgetPercent.value, '预算占用', budgetPercent.value >= 90 ? chartColor(1) : chartColor(0))
  if (id === 'cacheGauge') return ringOption(cacheRate.value, '缓存命中率', chartColor(0))
  if (id === 'tokenTrend') return { tooltip: tooltip(' Token'), legend: { top: 0, textStyle: { fontSize: 10 } }, grid, dataZoom: zoom(), xAxis: { type: 'category', data: labels, ...commonAxis() }, yAxis: { type: 'value', axisLabel: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 10, formatter: compact }, splitLine: { lineStyle: { color: mixHex(theme.value.ink,theme.value.background,.9) } } }, color: chartPalette.value, series: providerNames.value.map((name, index) => ({ name, type: 'line', smooth: .28, symbol: 'circle', symbolSize: 5, lineStyle: { width: 2 }, itemStyle: { color: chartColor(index) }, data: buckets.value.map(row => row.providers.get(name) || 0) })) }
  if (id === 'costTrend') return { tooltip: tooltip(' 元'), grid, dataZoom: zoom(), xAxis: { type: 'category', data: labels, ...commonAxis() }, yAxis: { type: 'value', ...commonAxis(), axisLabel: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 10, formatter: (value: number) => `¥${value}` } }, series: [{ name: '消费金额', type: 'line', smooth: .3, symbolSize: 6, lineStyle: { color: chartColor(0), width: 2 }, itemStyle: { color: chartColor(0) }, data: buckets.value.map(row => Number(row.cost.toFixed(4))), markLine: { symbol: 'none', label: { formatter: '预算警戒 ¥{c}', color: chartColor(1), fontSize: 10 }, lineStyle: { color: chartColor(1), type: 'dashed' }, data: [{ yAxis: Number((props.monthlyBudget / Math.max(1, range.value === 'today' ? 30 * 24 : range.value === '7d' ? 30 : 30)).toFixed(2)) }] } }] }
  if (id === 'callTrend') return lineOption(labels, buckets.value.map(row => row.calls), 'API 请求', chartColor(0), ' 次')
  if (id === 'tokenCacheBars') return { tooltip: tooltip(' Token'), legend: { top: 0, textStyle: { fontSize: 10 } }, grid, dataZoom: zoom(), xAxis: { type: 'category', data: labels, ...commonAxis() }, yAxis: { type: 'value', ...commonAxis(), axisLabel: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 10, formatter: compact } }, color: [chartColor(0), chartColor(2)], series: [{ name: '总 Token', type: 'bar', barMaxWidth: 22, itemStyle: { borderRadius: [8, 8, 3, 3] }, data: buckets.value.map(row => row.tokens) }, { name: '缓存节省', type: 'bar', barMaxWidth: 22, itemStyle: { borderRadius: [8, 8, 3, 3] }, data: buckets.value.map(row => row.cached) }] }
  if (id === 'modelCompare') return horizontalOption(todayModels.value.map(([name]) => name), todayModels.value.map(([, value]) => value), chartColor(0))
  if (id === 'weekCost') return { tooltip: tooltip(' 元'), grid: { left: 45, right: 16, top: 20, bottom: 24 }, xAxis: { type: 'category', data: weekCosts.value.map(item => item.label), ...commonAxis() }, yAxis: { type: 'value', ...commonAxis(), axisLabel: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 10, formatter: (value: number) => `¥${value}` } }, series: [{ type: 'bar', barMaxWidth: 34, itemStyle: { color: chartColor(0), borderRadius: [9, 9, 3, 3] }, data: weekCosts.value.map(item => Number(item.value.toFixed(4))) }] }
  if (id === 'tokenCallCombo') return { tooltip: tooltip(), legend: { top: 0, textStyle: { fontSize: 10 } }, grid, dataZoom: zoom(), xAxis: { type: 'category', data: labels, ...commonAxis() }, yAxis: [{ type: 'value', name: 'Token', nameTextStyle: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 9 }, axisLabel: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 10, formatter: compact }, splitLine: { lineStyle: { color: mixHex(theme.value.ink,theme.value.background,.9) } } }, { type: 'value', name: '请求', nameTextStyle: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 9 }, axisLabel: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 10 }, splitLine: { show: false } }], series: [{ name: 'Token', type: 'bar', barMaxWidth: 18, itemStyle: { color: chartColor(2), borderRadius: [7, 7, 2, 2] }, data: buckets.value.map(row => row.tokens) }, { name: '请求次数', type: 'line', yAxisIndex: 1, smooth: .3, symbolSize: 6, lineStyle: { color: chartColor(0), width: 2 }, itemStyle: { color: chartColor(0) }, data: buckets.value.map(row => row.calls) }] }
  if (id === 'topRanking') return horizontalOption(topRanking.value.map(([name]) => name), topRanking.value.map(([, value]) => value), chartColor(0))
  return emptyOption('暂无数据')
}

function ringOption(percent: number, name: string, color: string): EChartsOption {
  return { title: { text: `${percent.toFixed(0)}%`, subtext: name, left: 'center', top: '36%', textStyle: { color: theme.value.ink, fontSize: 25, fontWeight: 700 }, subtextStyle: { color: mixHex(theme.value.ink,theme.value.background,.48), fontSize: 10 } }, tooltip: { trigger: 'item', formatter: `${name}<br/>{c}%` }, series: [{ type: 'pie', radius: ['58%', '76%'], silent: false, label: { show: false }, data: [{ name: '已使用', value: Number(percent.toFixed(2)), itemStyle: { color, borderRadius: 9 } }, { name: '剩余', value: Number((100 - percent).toFixed(2)), itemStyle: { color: mixHex(theme.value.ink,theme.value.background,theme.value.dark ? .78 : .9) } }] }] }
}

function lineOption(labels: string[], values: number[], name: string, color: string, unit: string): EChartsOption {
  return { tooltip: tooltip(unit), grid: { left: 45, right: 18, top: 24, bottom: range.value === '30d' ? 42 : 24 }, dataZoom: zoom(), xAxis: { type: 'category', data: labels, ...commonAxis() }, yAxis: { type: 'value', ...commonAxis() }, series: [{ name, type: 'line', smooth: .3, symbolSize: 6, lineStyle: { color, width: 2 }, itemStyle: { color }, data: values }] }
}

function horizontalOption(names: string[], values: number[], color: string): EChartsOption {
  return { tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' }, backgroundColor: '#1D1D1F', borderWidth: 0, textStyle: { color: '#fff', fontSize: 11 }, valueFormatter: (value: unknown) => `${Number(value).toLocaleString()} Token` }, grid: { left: 120, right: 18, top: 12, bottom: 15 }, xAxis: { type: 'value', axisLabel: { color: '#68686D', fontSize: 9, formatter: compact }, splitLine: { lineStyle: { color: '#F2F2F7' } } }, yAxis: { type: 'category', inverse: true, data: names, axisLine: { show: false }, axisTick: { show: false }, axisLabel: { color: '#525257', fontSize: 10, width: 105, overflow: 'truncate' } }, series: [{ type: 'bar', barMaxWidth: 14, itemStyle: { color, borderRadius: [0, 7, 7, 0] }, data: values }] }
}

function applyChartTheme(value:unknown):unknown {
  if(Array.isArray(value))return value.map(applyChartTheme)
  if(value&&typeof value==='object'){for(const [key,next] of Object.entries(value as Record<string,unknown>))(value as Record<string,unknown>)[key]=applyChartTheme(next);return value}
  if(typeof value!=='string')return value
  const normalized=value.toUpperCase()
  if(['#111111','#1D1D1F','#343436'].includes(normalized))return theme.value.ink
  if(['#525255','#525257','#68686D','#6E6E73','#8E8E93','#AEAEB2'].includes(normalized))return mixHex(theme.value.ink,theme.value.background,.48)
  if(['#FFFFFF','#FFF'].includes(normalized))return theme.value.background
  if(['#F2F2F7','#F7F7F8','#E5E5EA','#D1D1D6','#D1E7FF'].includes(normalized))return mixHex(theme.value.ink,theme.value.background,.86)
  return value
}

async function renderAll() {
  await nextTick()
  for (const [id, instance] of instances) if (!visibleCharts.value.includes(id)) { instance.dispose(); instances.delete(id) }
  for (const id of visibleCharts.value) {
    if (id === 'quota5h') continue
    const element = chartElements.get(id)
    if (!element) continue
    const instance = instances.get(id) || init(element, undefined, { renderer: 'canvas', devicePixelRatio: Math.min(window.devicePixelRatio || 1, 2), useDirtyRect:true })
    instances.set(id, instance)
    const themed=applyChartTheme({ animation:motionEnabled.value, animationThreshold:2400, animationDuration:360, animationDurationUpdate:300, animationEasing:'cubicOut', animationEasingUpdate:'cubicOut', backgroundColor:'transparent', textStyle: { fontFamily: appleFontStack, color: theme.value.ink }, ...optionFor(id) }) as EChartsOption
    const chartTooltip=(themed as Record<string,unknown>).tooltip
    if(chartTooltip&&typeof chartTooltip==='object'&&!Array.isArray(chartTooltip))Object.assign(chartTooltip,{
      backgroundColor:theme.value.dark?mixHex(theme.value.background,theme.value.ink,.12):'#FFFFFF',
      borderColor:mixHex(theme.value.ink,theme.value.background,theme.value.dark ? .78 : .86),
      borderWidth:1,
      borderRadius:12,
      transitionDuration:.18,
      extraCssText:`box-shadow:0 10px 28px color-mix(in srgb, ${theme.value.ink} 12%, transparent);`,
      textStyle:{fontFamily:appleFontStack,color:theme.value.ink,fontSize:11,fontWeight:500}
    })
    instance.setOption(themed, true)
  }
}

function resizeCharts() { for (const instance of instances.values()) instance.resize() }
watch([visibleCharts, range, theme, chartColorTheme, motionEnabled, () => props.usage, () => props.codexSeries, () => props.used5h, () => props.used7d, () => props.monthlyBudget], renderAll, { deep: true })
onMounted(() => { void renderAll(); window.addEventListener('resize', resizeCharts) })
onBeforeUnmount(() => { window.removeEventListener('resize', resizeCharts); for (const instance of instances.values()) instance.dispose() })
</script>

<template>
  <section class="analytics-suite" aria-labelledby="analytics-title">
    <header class="analytics-toolbar">
      <div><h2 id="analytics-title">可视化分析</h2><p class="tm-supplemental-description">{{ mode === 'simple' ? '只显示日常最需要的核心图表' : '展示完整的用量、成本、缓存和额度分析' }}</p></div>
      <div class="toolbar-actions">
        <button class="customize-trigger" :class="{active:customizing}" :aria-pressed="customizing" @click="customizing=!customizing">{{customizing?'完成自定义':'自定义卡片'}}</button>
        <div class="segmented" role="group" aria-label="显示模式"><button :class="{active:mode==='simple'}" @click="setMode('simple' as DashboardMode)">简单</button><button :class="{active:mode==='advanced'}" @click="setMode('advanced' as DashboardMode)">高级</button></div>
        <div class="segmented" role="group" aria-label="时间范围"><button v-for="item in [{value:'today',label:'今日'},{value:'7d',label:'7 天'},{value:'30d',label:'30 天'}]" :key="item.value" :class="{active:range===item.value}" @click="setRange(item.value as ChartRange)">{{ item.label }}</button></div>
      </div>
    </header>
    <TransitionGroup v-if="visibleCharts.length" name="card-reflow" tag="div" class="chart-grid" :class="`mode-${mode}`">
      <article v-for="id in visibleCharts" :key="id" class="visual-card" :class="[`chart-${id}`, {wide:wideChartIds.has(id),fill:fillRowCharts.has(id),empty:!hasData(id),dragging:draggedChart===id,customizing}]" @dragover.prevent @drop="dropChart(id)">
        <div class="visual-card-heading"><div><h3>{{ chartCatalog.find(item=>item.id===id)?.title }}</h3><p class="tm-supplemental-description">{{ chartCatalog.find(item=>item.id===id)?.description }}</p></div><div v-if="customizing" class="chart-card-actions" role="toolbar" :aria-label="`${chartCatalog.find(item=>item.id===id)?.title} 卡片操作`"><button class="drag-chart" draggable="true" :aria-label="`拖动 ${chartCatalog.find(item=>item.id===id)?.title} 调整位置`" title="按住拖动调整位置" @dragstart="startChartDrag(id,$event)" @dragend="draggedChart=null"><span aria-hidden="true">⠿</span> 拖动</button><button class="remove-chart" :aria-label="`删除 ${chartCatalog.find(item=>item.id===id)?.title}`" title="从仪表盘移除" @click.stop="setChartEnabled(id,false)">删除</button></div><span v-else>{{ range==='today'?'今日':range==='7d'?'近 7 天':'近 30 天' }}</span></div>
        <template v-if="id==='quota5h'">
          <div class="quota-progress"><div><strong>{{ (100-used5hPercent).toFixed(0) }}%</strong><span>剩余</span></div><div class="quota-track"><i :class="{warning:used5hPercent>=80,critical:used5hPercent>=90}" :style="{width:used5hPercent+'%'}"></i></div><small>已用 {{ used5h.toLocaleString() }} / {{ budget5h.toLocaleString() }} Token</small></div>
        </template>
        <div v-else :ref="element=>setChartRef(id,element)" class="echart" role="img" :aria-label="chartCatalog.find(item=>item.id===id)?.title"></div>
      </article>
    </TransitionGroup>
    <section v-if="customizing" class="chart-library"><div><h3>添加图表卡片</h3><p>只显示当前仪表盘支持、且尚未添加的图表。</p></div><div v-if="availableCharts.length"><button v-for="id in availableCharts" :key="id" @click="setChartEnabled(id,true)"><span><b>{{chartCatalog.find(item=>item.id===id)?.title}}</b><small>{{chartCatalog.find(item=>item.id===id)?.description}}</small></span><strong>＋ 添加</strong></button></div><small v-else>当前仪表盘支持的图表已全部添加。</small></section>
    <div v-else class="charts-empty"><b>当前没有启用图表</b><span>请前往设置页勾选需要展示的图表。</span></div>
  </section>
</template>

<style scoped>
.analytics-suite{margin-bottom:18px}.analytics-toolbar{display:flex;align-items:flex-start;justify-content:space-between;gap:24px;margin-bottom:14px}.analytics-toolbar h2{margin:0;color:var(--tm-ink,#1d1d1f);font-size:18px}.analytics-toolbar p{margin:6px 0 0;color:var(--tm-muted,#68686d);font-size:11px}.toolbar-actions{display:flex;align-items:center;gap:9px}.customize-trigger{padding:8px 12px;border:1px solid var(--tm-line);background:var(--tm-bg);color:var(--tm-ink);font-size:10px}.customize-trigger.active{background:var(--tm-ink);color:var(--tm-on-ink)}.segmented{display:flex;gap:3px;padding:4px;border-radius:12px;background:var(--tm-surface,#f2f2f7)}.segmented button{min-width:54px;padding:7px 10px;border:0;border-radius:9px;background:transparent;color:var(--tm-muted,#525257);font-size:10px}.segmented button:hover{background:var(--tm-bg,#fff)}.segmented button.active{background:var(--tm-ink,#111);color:var(--tm-on-ink,#fff)}.chart-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));grid-auto-flow:dense;align-items:stretch;gap:14px}.visual-card{min-width:0;height:100%;padding:19px 20px 16px;border:1px solid var(--tm-line,#ededf0);border-radius:16px;background:var(--tm-bg,#fff)}.visual-card.wide,.visual-card.fill{grid-column:1/-1}.visual-card-heading{display:flex;align-items:flex-start;justify-content:space-between;gap:15px}.visual-card-heading h3{margin:0;color:var(--tm-ink,#1d1d1f);font-size:14px}.visual-card-heading p{margin:5px 0 0;color:var(--tm-muted,#68686d);font-size:9px}.visual-card-heading>span{flex:0 0 auto;padding:4px 7px;border-radius:8px;background:var(--tm-surface,#f2f2f7);color:var(--tm-muted,#68686d);font-size:9px}.remove-chart{padding:6px 9px;border:1px solid var(--tm-line);background:var(--tm-bg);color:var(--tm-ink);font-size:9px}.echart{width:100%;height:260px;margin-top:10px}.chart-monthlyShare .echart,.chart-cacheDoubleRing .echart,.chart-codexCategory .echart,.chart-quota7d .echart,.chart-budgetGauge .echart,.chart-cacheGauge .echart{height:230px}.mode-simple .visual-card:not(.wide){min-height:310px}.quota-progress{display:grid;align-content:center;min-height:230px;padding:22px 10px}.quota-progress>div:first-child{display:flex;align-items:end;gap:8px}.quota-progress strong{font-size:34px;letter-spacing:-1px}.quota-progress span,.quota-progress small{color:var(--tm-muted,#68686d);font-size:10px}.quota-track{height:12px;margin:20px 0 10px;overflow:hidden;border-radius:99px;background:var(--tm-surface,#f2f2f7)}.quota-track i{display:block;height:100%;border-radius:99px;background:var(--tm-muted,#8e8e93)}.quota-track i.warning,.quota-track i.critical{background:var(--tm-ink,#111)}.chart-library{display:grid;gap:13px;margin-top:14px;padding:18px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-surface)}.chart-library h3{margin:0;font-size:13px}.chart-library p,.chart-library>small{margin:4px 0 0;color:var(--tm-muted);font-size:9px}.chart-library>div:last-of-type{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));grid-auto-flow:dense;gap:7px}.chart-library button{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:12px;border:1px solid var(--tm-line);background:var(--tm-bg);color:var(--tm-ink);text-align:left}.chart-library button:last-child:nth-child(odd){grid-column:1/-1}.chart-library button span{display:grid;gap:3px}.chart-library button small{color:var(--tm-muted);font-size:8px}.chart-library button strong{font-size:9px}.charts-empty{display:grid;gap:6px;padding:34px;border-radius:16px;background:var(--tm-surface,#f7f7f8);color:var(--tm-muted,#68686d)}.charts-empty b{color:var(--tm-ink,#1d1d1f);font-size:13px}.charts-empty span{font-size:11px}@media(max-width:1050px){.analytics-toolbar{flex-direction:column}.toolbar-actions{flex-wrap:wrap}.chart-grid,.chart-library>div:last-of-type{grid-template-columns:1fr}.visual-card.wide,.visual-card.fill,.chart-library button:last-child:nth-child(odd){grid-column:auto}}@media(prefers-reduced-motion:reduce){.segmented button{transition:none}}
</style>
<style scoped>
.analytics-toolbar {
  animation: analytics-toolbar-enter .28s cubic-bezier(.2,.8,.2,1) both;
}
.segmented,
.customize-trigger {
  border: 1px solid var(--tm-line);
  background: var(--tm-glass);
  backdrop-filter: blur(18px) saturate(145%);
}
.segmented button:hover {
  background: var(--tm-surface-strong);
  color: var(--tm-ink);
}
.segmented button.active,
.customize-trigger.active {
  border-color: color-mix(in srgb,var(--tm-accent) 48%,var(--tm-line));
  background: color-mix(in srgb,var(--tm-accent) 18%,var(--tm-bg));
  color: var(--tm-ink);
  box-shadow: 0 8px 24px color-mix(in srgb,var(--tm-glow) 12%,transparent), inset 0 1px 0 color-mix(in srgb,var(--tm-ink) 9%,transparent);
}
.visual-card {
  position: relative;
  overflow: hidden;
  background: var(--tm-glass);
  box-shadow: inset 0 1px 0 color-mix(in srgb,var(--tm-ink) 7%,transparent), 0 10px 28px color-mix(in srgb,var(--tm-glow) 6%,transparent);
  backdrop-filter: blur(24px) saturate(145%);
  animation: analytics-card-enter .32s cubic-bezier(.2,.8,.2,1) both;
}
.visual-card::before {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(135deg,color-mix(in srgb,var(--tm-glow) 8%,transparent),transparent 38%,color-mix(in srgb,var(--tm-accent) 5%,transparent));
  opacity: .72;
  pointer-events: none;
}
.visual-card > * { position: relative; z-index: 1; }
.visual-card:nth-child(2) { animation-delay: .035s; }
.visual-card:nth-child(3) { animation-delay: .07s; }
.visual-card:nth-child(4) { animation-delay: .105s; }
.visual-card:nth-child(5) { animation-delay: .14s; }
.visual-card:nth-child(n+6) { animation-delay: .17s; }
.visual-card:hover {
  transform: translateY(-2px) scale(1.001);
  border-color: color-mix(in srgb,var(--tm-accent) 42%,var(--tm-line));
  background: var(--tm-glass-strong);
  box-shadow: 0 18px 42px color-mix(in srgb,var(--tm-glow) 14%,transparent), inset 0 1px 0 color-mix(in srgb,var(--tm-ink) 9%,transparent);
}
.quota-track i {
  background: var(--tm-accent);
  transform-origin: left;
  animation: analytics-progress-enter .34s cubic-bezier(.2,.8,.2,1) both;
}
@keyframes analytics-toolbar-enter {
  from { opacity: 0; transform: translateY(6px); filter: blur(3px); }
  to { opacity: 1; transform: none; filter: none; }
}
@keyframes analytics-card-enter {
  from { opacity: 0; transform: translateY(9px) scale(.992); filter: blur(4px); }
  to { opacity: 1; transform: none; filter: none; }
}
@keyframes analytics-progress-enter {
  from { transform: scaleX(.04); opacity: .35; }
  to { transform: scaleX(1); opacity: 1; }
}
:global(.motion-off) .analytics-suite *,
:global(.motion-off) .visual-card::before {
  animation: none !important;
  transition: none !important;
}
@media (prefers-reduced-motion: reduce) {
  .analytics-suite *,
  .visual-card::before { animation: none !important; transition: none !important; }
}
</style>
<style scoped>
.visual-card{transition:opacity .18s var(--ease-out-quint,cubic-bezier(.22,1,.36,1)),transform .28s var(--ease-out-quint,cubic-bezier(.22,1,.36,1)),border-color .18s ease,box-shadow .2s cubic-bezier(.22,1,.36,1)}.visual-card:hover{transform:translateY(-1px);box-shadow:0 3px 8px rgba(0,0,0,.05)}.visual-card.customizing{border-color:color-mix(in srgb,var(--tm-ink) 18%,var(--tm-line))}.visual-card.dragging{opacity:.48;transform:scale(.985)}.chart-card-actions{display:flex;align-items:center;gap:5px;flex:0 0 auto}.drag-chart,.remove-chart{padding:6px 8px;border:1px solid var(--tm-line);border-radius:8px;background:var(--tm-bg);color:var(--tm-ink);font-size:9px}.drag-chart{cursor:grab}.drag-chart:active{cursor:grabbing;transform:scale(.96)}.drag-chart:hover{background:var(--tm-surface)}.remove-chart:hover{border-color:#ff3b30;background:#ff3b30;color:#fff}.quota-track i{transition:width .34s cubic-bezier(.22,1,.36,1)}.card-reflow-move{transition:transform .34s cubic-bezier(.22,1,.36,1)}.card-reflow-enter-active{transition:opacity .22s ease,transform .34s cubic-bezier(.22,1,.36,1)}.card-reflow-leave-active{position:absolute;transition:opacity .15s ease,transform .15s ease}.card-reflow-enter-from,.card-reflow-leave-to{opacity:0;transform:scale(.96)}:global(.motion-off) .visual-card,:global(.motion-off) .quota-track i,:global(.motion-off) .card-reflow-move,:global(.motion-off) .card-reflow-enter-active,:global(.motion-off) .card-reflow-leave-active{transition:none!important}:global(.motion-off) .visual-card:hover{transform:none;box-shadow:none}@media(prefers-reduced-motion:reduce){.visual-card,.quota-track i,.card-reflow-move,.card-reflow-enter-active,.card-reflow-leave-active{transition:none}.visual-card:hover,.drag-chart:active{transform:none;box-shadow:none}}
</style>
