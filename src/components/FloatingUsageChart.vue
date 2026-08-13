<script setup lang="ts">
import { computed, ref } from 'vue'
import AnimatedNumber from './AnimatedNumber.vue'
import type { FloatingDailyPoint, FloatingMetric } from './FloatingWindowV3.vue'

const props = defineProps<{
  points: FloatingDailyPoint[]
  metric: FloatingMetric
  source: string
  updatedAt: string
  hasData: boolean
}>()

const activeIndex = ref<number | null>(null)
const pinnedIndex = ref<number | null>(null)
const metricLabel: Record<FloatingMetric, string> = { tokens: 'Token', calls: '请求', cached: '缓存', cost: '消费' }
const values = computed(() => props.points.map(point => Math.max(0, Number(point[props.metric]) || 0)))
const maxValue = computed(() => Math.max(.0001, ...values.value))
const currentIndex = computed(() => pinnedIndex.value ?? activeIndex.value)
const currentPoint = computed(() => currentIndex.value === null ? null : props.points[currentIndex.value])
const total = computed(() => values.value.reduce((sum, value) => sum + value, 0))

const chart = computed(() => {
  const left = 12
  const right = 408
  const baseline = 112
  const maxHeight = 78
  const slot = (right - left) / Math.max(1, props.points.length)
  const width = Math.min(26, slot * .46)
  return props.points.map((point, index) => {
    const value = values.value[index]
    const height = value > 0 ? Math.max(7, value / maxValue.value * maxHeight) : 0
    const x = left + index * slot + (slot - width) / 2
    return { point, value, x, y: baseline - height, width, height, center: x + width / 2 }
  })
})

const tooltipStyle = computed(() => {
  const item = currentIndex.value === null ? null : chart.value[currentIndex.value]
  if (!item) return {}
  const percent = item.center / 420 * 100
  return { left: `${Math.max(18, Math.min(82, percent))}%` }
})

function formatCurrency(value: number) { return `¥${Math.max(0, value || 0).toFixed(4)}` }
function setActive(index: number | null) { activeIndex.value = index }
function togglePinned(index: number) { pinnedIndex.value = pinnedIndex.value === index ? null : index }
</script>

<template>
  <section class="floating-usage-chart" :class="{ 'has-active': currentIndex !== null }">
    <div class="floating-usage-chart__summary">
      <span>{{ metricLabel[metric] }} · 近 7 天</span>
      <strong>
        <AnimatedNumber v-if="metric === 'cost'" :value="total" :decimals="4" format="currency" />
        <AnimatedNumber v-else :value="total" :format="metric === 'calls' ? 'integer' : 'compact'" :suffix="metric === 'calls' ? ' 次' : ' Token'" />
      </strong>
    </div>

    <div v-if="hasData" class="floating-usage-chart__canvas" @mouseleave="setActive(null)">
      <svg viewBox="0 0 420 138" role="img" :aria-label="`近七天${metricLabel[metric]}趋势`">
        <line class="axis" x1="12" y1="112" x2="408" y2="112" />
        <g
          v-for="(item, index) in chart"
          :key="item.point.key"
          class="bar-group"
          :class="{ active: currentIndex === index, muted: currentIndex !== null && currentIndex !== index }"
          tabindex="0"
          role="button"
          :aria-label="`${item.point.label} ${metricLabel[metric]} ${item.value}`"
          @mouseenter="setActive(index)"
          @focus="setActive(index)"
          @blur="setActive(null)"
          @click="togglePinned(index)"
          @keydown.enter.prevent="togglePinned(index)"
          @keydown.space.prevent="togglePinned(index)"
        >
          <rect class="bar-hit" :x="item.x - 8" y="8" :width="item.width + 16" height="108" rx="10" />
          <rect class="bar-track" :x="item.x" y="18" :width="item.width" height="94" :rx="item.width / 2" />
          <rect class="bar-value" :x="item.x" :y="item.y" :width="item.width" :height="item.height" :rx="item.width / 2" />
          <text :x="item.center" y="130" text-anchor="middle">{{ item.point.label }}</text>
        </g>
      </svg>
      <Transition name="chart-tip">
        <div v-if="currentPoint" class="floating-usage-chart__tooltip" :style="tooltipStyle" role="tooltip">
          <strong>{{ currentPoint.label }}</strong>
          <dl>
            <div><dt>Token</dt><dd>{{ currentPoint.tokens.toLocaleString() }}</dd></div>
            <div><dt>请求</dt><dd>{{ currentPoint.calls.toLocaleString() }} 次</dd></div>
            <div><dt>缓存</dt><dd>{{ currentPoint.cached.toLocaleString() }}</dd></div>
            <div><dt>消费</dt><dd>{{ formatCurrency(currentPoint.cost) }}</dd></div>
          </dl>
          <small>{{ source }} · {{ updatedAt }}</small>
        </div>
      </Transition>
    </div>

    <div v-else class="floating-usage-chart__empty">
      <b>暂无真实{{ metricLabel[metric] }}记录</b>
      <span>连接本地 Agent 或代理后自动显示</span>
    </div>
  </section>
</template>

<style scoped>
.floating-usage-chart{display:grid;gap:8px}.floating-usage-chart__summary{display:flex;align-items:flex-end;justify-content:space-between;gap:12px}.floating-usage-chart__summary>span{color:var(--tm-muted,#6e6e73);font-size:10px}.floating-usage-chart__summary>strong{overflow:hidden;font-size:13px;font-variant-numeric:tabular-nums;text-overflow:ellipsis;white-space:nowrap}.floating-usage-chart__canvas{position:relative;height:138px}.floating-usage-chart svg{display:block;width:100%;height:100%;overflow:visible}.axis{stroke:color-mix(in srgb,var(--tm-ink,#1d1d1f) 9%,transparent);stroke-width:1}.bar-group{outline:none;cursor:pointer;transition:opacity .18s ease}.bar-hit{fill:transparent}.bar-track{fill:color-mix(in srgb,var(--tm-ink,#1d1d1f) 5%,transparent)}.bar-value{fill:var(--tm-accent,var(--chart-1,#1d1d1f));transform-box:fill-box;transform-origin:center bottom;animation:bar-rise .34s cubic-bezier(.22,1,.36,1) both;transition:filter .18s ease,opacity .18s ease,transform .18s ease}.bar-group.muted{opacity:.34}.bar-group.active .bar-value,.bar-group:focus-visible .bar-value{filter:drop-shadow(0 5px 8px color-mix(in srgb,var(--tm-accent,#007aff) 25%,transparent));transform:scaleX(1.08)}.bar-group:focus-visible .bar-hit{stroke:color-mix(in srgb,var(--tm-accent,#007aff) 55%,transparent);stroke-width:1;fill:color-mix(in srgb,var(--tm-accent,#007aff) 5%,transparent)}text{fill:var(--tm-muted,#6e6e73);font-family:var(--apple-font,"PingFang SC","Microsoft YaHei",sans-serif);font-size:9px}.floating-usage-chart__tooltip{position:absolute;z-index:20;top:3px;display:grid;width:210px;gap:7px;padding:10px 11px;border:1px solid color-mix(in srgb,var(--tm-ink,#1d1d1f) 10%,transparent);border-radius:13px;background:color-mix(in srgb,var(--tm-bg,#fff) 96%,transparent);box-shadow:0 14px 34px color-mix(in srgb,var(--tm-ink,#1d1d1f) 16%,transparent);color:var(--tm-ink,#1d1d1f);font-size:9px;pointer-events:none;transform:translateX(-50%);backdrop-filter:blur(22px) saturate(145%)}.floating-usage-chart__tooltip>strong{font-size:11px}.floating-usage-chart__tooltip dl{display:grid;grid-template-columns:1fr 1fr;gap:5px 10px;margin:0}.floating-usage-chart__tooltip dl>div{display:flex;justify-content:space-between;gap:6px}.floating-usage-chart__tooltip dt{color:var(--tm-muted,#6e6e73)}.floating-usage-chart__tooltip dd{margin:0;font-variant-numeric:tabular-nums}.floating-usage-chart__tooltip small{overflow:hidden;color:var(--tm-muted,#6e6e73);font-size:9px;text-overflow:ellipsis;white-space:nowrap}.floating-usage-chart__empty{display:grid;height:122px;place-items:center;align-content:center;gap:5px;border-radius:13px;background:var(--tm-surface,#f2f2f7);color:var(--tm-muted,#6e6e73);font-size:10px;text-align:center}.floating-usage-chart__empty b{color:var(--tm-ink,#1d1d1f);font-size:11px}.chart-tip-enter-active,.chart-tip-leave-active{transition:opacity .18s ease,transform .2s cubic-bezier(.22,1,.36,1)}.chart-tip-enter-from,.chart-tip-leave-to{opacity:0;transform:translate(-50%,3px) scale(.98)}@keyframes bar-rise{from{transform:scaleY(.08)}to{transform:scaleY(1)}}@media(prefers-reduced-motion:reduce){.bar-value,.bar-group,.chart-tip-enter-active,.chart-tip-leave-active{animation:none;transition:none}}:global(.motion-off) .bar-value,:global(.motion-off) .bar-group,:global(.motion-off) .chart-tip-enter-active,:global(.motion-off) .chart-tip-leave-active{animation:none;transition:none}
</style>
