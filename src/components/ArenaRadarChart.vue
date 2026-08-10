<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { init, use, type EChartsType } from 'echarts/core'
import type { EChartsOption } from 'echarts'
import { RadarChart } from 'echarts/charts'
import { RadarComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { useThemePreferences } from '../features/themePreferences'
import { useMotionPreferences } from '../features/motionPreferences'

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

interface DimensionDefinition {
  id: string
  label: string
}

const props = defineProps<{
  dimensions: ArenaDimension[]
  loading: boolean
  status: string
  model: string
}>()

use([RadarChart, RadarComponent, TooltipComponent, CanvasRenderer])

const dimensionDefinitions: DimensionDefinition[] = [
  { id: 'coding', label: '编程' },
  { id: 'math', label: '数学' },
  { id: 'instruction_following', label: '指令遵循' },
  { id: 'multi_turn', label: '多轮对话' },
  { id: 'creative_writing', label: '创意写作' },
  { id: 'longer_query', label: '长问题处理' },
]

const chartElement = ref<HTMLElement | null>(null)
const { theme } = useThemePreferences()
const { motionEnabled } = useMotionPreferences()
const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)')
let chart: EChartsType | null = null
let resizeObserver: ResizeObserver | null = null

const orderedDimensions = computed(() => {
  const values = new Map(props.dimensions.map(item => [item.id, item]))
  return dimensionDefinitions.map(definition => ({
    ...definition,
    data: values.get(definition.id) ?? null,
  }))
})

const hasCompleteRadar = computed(() => orderedDimensions.value.every(item => (
  item.data?.percentile !== null
  && item.data?.percentile !== undefined
  && Number.isFinite(item.data.percentile)
)))

const positionValues = computed(() => orderedDimensions.value.map(item => (
  Math.max(0, Math.min(100, Number(item.data?.percentile)))
)))

const statusText = computed(() => {
  if (props.loading) return '正在同步官方分类数据'
  if (props.status.trim()) return props.status.trim()
  return hasCompleteRadar.value ? '官方分类数据已就绪' : '等待完整官方分类数据'
})

const statusState = computed(() => (
  props.loading ? 'loading' : hasCompleteRadar.value ? 'ready' : 'incomplete'
))

const appleFontStack = '"SF Pro Text", "SF Pro Display", "PingFang SC", "PingFang TC", "苹方-简", -apple-system, BlinkMacSystemFont, "Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", sans-serif'

function mixHex(left: string, right: string, amount: number) {
  const parse = (value: string) => [1, 3, 5].map(index => Number.parseInt(value.slice(index, index + 2), 16))
  const from = parse(left)
  const to = parse(right)
  return `#${from.map((value, index) => (
    Math.round(value + (to[index] - value) * amount).toString(16).padStart(2, '0')
  )).join('')}`
}

function hexToRgba(value: string, alpha: number) {
  const channels = [1, 3, 5].map(index => Number.parseInt(value.slice(index, index + 2), 16))
  return `rgba(${channels[0]},${channels[1]},${channels[2]},${alpha})`
}

function escapeHtml(value: string) {
  return value.replace(/[&<>"']/g, character => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;',
  })[character] ?? character)
}

function formatNumber(value: number | null, maximumFractionDigits = 1) {
  if (value === null || !Number.isFinite(value)) return '—'
  return new Intl.NumberFormat('zh-CN', { maximumFractionDigits }).format(value)
}

function rankText(item: ArenaDimension | null) {
  if (item?.sync_state === 'error') return '同步失败'
  if (!item || item.rank === null) return '未上榜'
  return `#${formatNumber(item.rank, 0)} / ${item.total_models !== null && item.total_models > 0 ? formatNumber(item.total_models, 0) : '—'}`
}

function dimensionStatusText(item: ArenaDimension | null) {
  if (item?.sync_state === 'error') return '本项同步失败，稍后自动重试'
  if (!item || item.rank === null) return '官方分类未上榜'
  return confidenceText(item)
}

function scoreText(item: ArenaDimension | null) {
  return item?.score === null || item?.score === undefined ? '—' : formatNumber(item.score)
}

function votesText(item: ArenaDimension | null) {
  return item?.votes === null || item?.votes === undefined ? '—' : formatNumber(item.votes, 0)
}

function confidenceText(item: ArenaDimension | null) {
  if (!item || item.score_lower === null || item.score_upper === null) return '置信区间暂无'
  return `置信区间 ${formatNumber(item.score_lower)}–${formatNumber(item.score_upper)}`
}

function publishedText(item: ArenaDimension | null) {
  if (!item?.published_at) return '发布时间暂无'
  const date = new Date(item.published_at)
  if (Number.isNaN(date.getTime())) return '发布时间暂无'
  return `发布于 ${date.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' })}`
}

function chartOption(): EChartsOption {
  const value = theme.value
  const separator = mixHex(value.ink, value.background, value.dark ? .78 : .88)
  const subtle = mixHex(value.ink, value.background, value.dark ? .62 : .72)
  const tooltipBackground = value.dark
    ? mixHex(value.background, value.ink, .12)
    : mixHex(value.ink, value.background, .04)

  return {
    animation: motionEnabled.value && !reduceMotion.matches,
    animationDuration: 320,
    animationDurationUpdate: 260,
    animationEasing: 'cubicOut',
    animationEasingUpdate: 'cubicOut',
    backgroundColor: 'transparent',
    textStyle: {
      color: value.ink,
      fontFamily: appleFontStack,
    },
    tooltip: {
      trigger: 'item',
      confine: true,
      backgroundColor: tooltipBackground,
      borderColor: mixHex(value.ink, value.background, value.dark ? .72 : .24),
      borderWidth: 1,
      borderRadius: 12,
      padding: [11, 13],
      extraCssText: `box-shadow:0 12px 30px ${hexToRgba(value.glow, .18)};backdrop-filter:blur(18px);`,
      textStyle: {
        color: value.dark ? value.ink : value.background,
        fontFamily: appleFontStack,
        fontSize: 11,
      },
      formatter: () => {
        const rows = orderedDimensions.value.map((item, index) => {
          const detail = item.data
          const position = positionValues.value[index]
          return `<div style="display:flex;justify-content:space-between;gap:24px;margin-top:7px">
            <span style="opacity:.72">${escapeHtml(item.label)}</span>
            <b>${formatNumber(position)} · ${escapeHtml(rankText(detail))}</b>
          </div>`
        }).join('')
        return `<div style="min-width:210px">
          <b style="font-size:12px">${escapeHtml(props.model || '当前模型')}</b>
          <div style="margin-top:3px;opacity:.62">官方分类位置指数</div>
          ${rows}
        </div>`
      },
    },
    radar: {
      center: ['50%', '51%'],
      radius: '67%',
      startAngle: 90,
      splitNumber: 4,
      shape: 'polygon',
      indicator: dimensionDefinitions.map(item => ({ name: item.label, max: 100, min: 0 })),
      axisName: {
        color: subtle,
        fontFamily: appleFontStack,
        fontSize: 11,
        fontWeight: 600,
      },
      axisNameGap: 11,
      axisLine: {
        lineStyle: {
          color: separator,
          width: 1,
        },
      },
      splitLine: {
        lineStyle: {
          color: [separator],
          width: 1,
        },
      },
      splitArea: {
        areaStyle: {
          color: [
            hexToRgba(value.accent, value.dark ? .018 : .014),
            hexToRgba(value.accent, value.dark ? .048 : .032),
          ],
        },
      },
    },
    series: [{
      name: props.model || '当前模型',
      type: 'radar',
      symbol: 'circle',
      symbolSize: 6,
      lineStyle: {
        color: value.accent,
        width: 2,
      },
      itemStyle: {
        color: value.accent,
        borderColor: value.background,
        borderWidth: 2,
      },
      areaStyle: {
        color: hexToRgba(value.accent, value.dark ? .22 : .14),
      },
      emphasis: {
        lineStyle: { width: 3 },
        itemStyle: {
          shadowBlur: 12,
          shadowColor: hexToRgba(value.glow, .34),
        },
      },
      data: [{
        name: '官方分类位置指数',
        value: positionValues.value,
      }],
    }],
  }
}

async function renderChart() {
  await nextTick()
  if (!chartElement.value || !hasCompleteRadar.value || props.loading) return
  if (!chart) {
    chart = init(chartElement.value, undefined, {
      renderer: 'canvas',
      devicePixelRatio: Math.min(window.devicePixelRatio || 1, 2),
      useDirtyRect: true,
    })
  }
  chart.setOption(chartOption(), true)
}

function disposeChart() {
  resizeObserver?.disconnect()
  resizeObserver = null
  chart?.dispose()
  chart = null
}

function attachChartElement() {
  disposeChart()
  if (!chartElement.value) return
  resizeObserver = new ResizeObserver(() => chart?.resize())
  resizeObserver.observe(chartElement.value)
  void renderChart()
}

function handleMotionPreference() {
  void renderChart()
}

watch(chartElement, attachChartElement)
watch(
  [orderedDimensions, theme, motionEnabled, () => props.model, () => props.loading],
  () => { void renderChart() },
  { deep: true },
)

onMounted(() => {
  reduceMotion.addEventListener('change', handleMotionPreference)
  attachChartElement()
})

onBeforeUnmount(() => {
  reduceMotion.removeEventListener('change', handleMotionPreference)
  disposeChart()
})
</script>

<template>
  <section
    class="arena-radar-card"
    :class="`is-${statusState}`"
    :aria-busy="loading"
    aria-labelledby="arena-radar-title"
  >
    <header class="arena-radar-header">
      <div>
        <span class="arena-radar-eyebrow">官方六维分类位置</span>
        <h3 id="arena-radar-title">{{ model || '请选择模型' }}</h3>
        <p>这是按官方分类名次换算的“位置指数”，不是绝对能力分。</p>
      </div>
      <span class="arena-radar-status" role="status" aria-live="polite">
        <i aria-hidden="true"></i>
        {{ statusText }}
      </span>
    </header>

    <div v-if="loading" class="arena-radar-loading" aria-label="正在加载官方分类数据">
      <div class="radar-loading-shape" aria-hidden="true"><i></i><i></i><i></i></div>
      <b>正在同步官方六维分类数据</b>
      <span>完成后将按官方分类名次换算位置指数。</span>
    </div>

    <div
      v-else-if="!hasCompleteRadar"
      class="arena-radar-empty"
      role="status"
    >
      <span class="radar-empty-mark" aria-hidden="true"><i></i><i></i><i></i><i></i><i></i><i></i></span>
      <div>
        <b>官方分类数据不完整</b>
        <p>六项官方分类都有名次换算结果后才绘制闭合雷达；缺失项不会按 0 计入。</p>
      </div>
    </div>

    <div
      v-else
      ref="chartElement"
      class="arena-radar-canvas"
      role="img"
      :aria-label="`${model || '当前模型'}的官方六维分类位置指数雷达图`"
    ></div>

    <ul class="arena-dimension-list" aria-label="官方六维分类明细">
      <li
        v-for="item in orderedDimensions"
        :key="item.id"
        :class="{
          missing: item.data?.rank === null || !item.data,
          failed: item.data?.sync_state === 'error',
        }"
      >
        <div class="dimension-heading">
          <span>{{ item.label }}</span>
          <strong>{{ rankText(item.data) }}</strong>
        </div>
        <div class="dimension-metrics">
          <span>Score <b>{{ scoreText(item.data) }}</b></span>
          <span>票数 <b>{{ votesText(item.data) }}</b></span>
        </div>
        <small :title="item.data?.error || `${confidenceText(item.data)}；${publishedText(item.data)}`">
          {{ dimensionStatusText(item.data) }}
        </small>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.arena-radar-card {
  container-type: inline-size;
  display: grid;
  gap: 18px;
  min-width: 0;
  padding: 20px;
  overflow: hidden;
  border: 1px solid var(--tm-line);
  border-radius: 16px;
  background: color-mix(in srgb, var(--tm-bg) 92%, transparent);
  color: var(--tm-ink);
}

.arena-radar-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
}

.arena-radar-header > div {
  min-width: 0;
}

.arena-radar-eyebrow {
  display: block;
  margin-bottom: 5px;
  color: var(--tm-muted);
  font-size: 10px;
  font-weight: 650;
  letter-spacing: .04em;
}

.arena-radar-header h3 {
  margin: 0;
  font-size: clamp(17px, 2vw, 21px);
  font-weight: 700;
  letter-spacing: -.025em;
}

.arena-radar-header p {
  max-width: 54ch;
  margin: 6px 0 0;
  color: var(--tm-muted);
  font-size: 11px;
  line-height: 1.55;
}

.arena-radar-status {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  max-width: 260px;
  padding: 7px 10px;
  border: 1px solid var(--tm-line);
  border-radius: 999px;
  background: var(--tm-surface);
  color: var(--tm-muted);
  font-size: 9px;
  font-weight: 600;
  line-height: 1.35;
}

.arena-radar-status i {
  width: 6px;
  height: 6px;
  flex: 0 0 auto;
  border-radius: 50%;
  background: currentColor;
  opacity: .52;
}

.is-ready .arena-radar-status {
  border-color: color-mix(in srgb, var(--tm-accent) 36%, var(--tm-line));
  color: var(--tm-ink);
}

.is-ready .arena-radar-status i {
  background: var(--tm-accent);
  opacity: 1;
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--tm-accent) 12%, transparent);
}

.is-loading .arena-radar-status i {
  border: 1px solid currentColor;
  border-top-color: transparent;
  background: transparent;
  animation: radar-status-spin .8s linear infinite;
}

.arena-radar-canvas {
  width: 100%;
  height: clamp(280px, 34vw, 380px);
  min-height: 280px;
}

.arena-radar-loading,
.arena-radar-empty {
  display: grid;
  min-height: 280px;
  place-items: center;
  align-content: center;
  gap: 9px;
  padding: 32px;
  border-radius: 14px;
  background: var(--tm-surface);
  text-align: center;
}

.arena-radar-loading b,
.arena-radar-empty b {
  font-size: 13px;
}

.arena-radar-loading > span,
.arena-radar-empty p {
  max-width: 52ch;
  margin: 0;
  color: var(--tm-muted);
  font-size: 10px;
  line-height: 1.6;
}

.radar-loading-shape,
.radar-empty-mark {
  position: relative;
  width: 58px;
  height: 58px;
  margin-bottom: 5px;
  border: 1px solid var(--tm-line);
  border-radius: 18px;
  background: color-mix(in srgb, var(--tm-bg) 70%, transparent);
}

.radar-loading-shape::before,
.radar-loading-shape::after {
  content: "";
  position: absolute;
  inset: 12px;
  border: 1px solid var(--tm-line);
  border-radius: 50%;
}

.radar-loading-shape::after {
  inset: 20px;
  border-color: var(--tm-accent);
  animation: radar-loading-pulse 1.2s ease-in-out infinite;
}

.radar-empty-mark i {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 5px;
  height: 5px;
  margin: -2.5px;
  border-radius: 50%;
  background: var(--tm-muted);
  opacity: .58;
}

.radar-empty-mark i:nth-child(1) { transform: translateY(-19px); }
.radar-empty-mark i:nth-child(2) { transform: translate(16px, -9px); }
.radar-empty-mark i:nth-child(3) { transform: translate(16px, 9px); }
.radar-empty-mark i:nth-child(4) { transform: translateY(19px); }
.radar-empty-mark i:nth-child(5) { transform: translate(-16px, 9px); }
.radar-empty-mark i:nth-child(6) { transform: translate(-16px, -9px); }

.arena-dimension-list {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1px;
  margin: 0;
  padding: 1px;
  overflow: hidden;
  border-radius: 14px;
  background: var(--tm-line);
  list-style: none;
}

.arena-dimension-list li {
  display: grid;
  gap: 8px;
  min-width: 0;
  padding: 12px 13px;
  background: var(--tm-bg);
}

.arena-dimension-list li.missing {
  color: var(--tm-muted);
}

.arena-dimension-list li.failed {
  box-shadow: inset 2px 0 0 color-mix(in srgb, var(--tm-accent) 55%, transparent);
}

.arena-dimension-list li.failed small {
  color: color-mix(in srgb, var(--tm-accent) 72%, var(--tm-muted));
}

.dimension-heading,
.dimension-metrics {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 9px;
}

.dimension-heading span {
  overflow: hidden;
  font-size: 10px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dimension-heading strong {
  flex: 0 0 auto;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.dimension-metrics {
  justify-content: flex-start;
  color: var(--tm-muted);
  font-size: 8px;
}

.dimension-metrics b {
  color: var(--tm-ink);
  font-size: 9px;
  font-variant-numeric: tabular-nums;
}

.arena-dimension-list small {
  overflow: hidden;
  color: var(--tm-muted);
  font-size: 8px;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@keyframes radar-status-spin {
  to { transform: rotate(360deg); }
}

@keyframes radar-loading-pulse {
  50% {
    opacity: .36;
    transform: scale(.82);
  }
}

@container (max-width: 26rem) {
  .arena-radar-header {
    flex-direction: column;
  }

  .arena-radar-status {
    max-width: 100%;
  }

  .arena-radar-canvas,
  .arena-radar-loading,
  .arena-radar-empty {
    min-height: 260px;
    height: 260px;
  }

  .arena-dimension-list {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@container (max-width: 18rem) {
  .arena-dimension-list {
    grid-template-columns: 1fr;
  }
}

@media (prefers-reduced-motion: reduce) {
  .is-loading .arena-radar-status i,
  .radar-loading-shape::after {
    animation: none;
  }
}

:global(.motion-off) .is-loading .arena-radar-status i,
:global(.motion-off) .radar-loading-shape::after {
  animation: none;
}
</style>
