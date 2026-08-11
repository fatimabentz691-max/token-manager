<script setup lang="ts">
import { computed } from 'vue'
import { ChevronDown, Minus, Plus, RefreshCw, X } from '@lucide/vue'
import ProviderMark from './ProviderMark.vue'
import type { FloatingInteractionMode, FloatingMode } from '../features/floatingPreferences'

export type FloatingMetric = 'tokens' | 'calls' | 'cached' | 'cost'

export interface FloatingSeriesPoint {
  key: string
  label: string
  value: number
  height: number
}

export interface FloatingDashboardView {
  key: string
  title: string
  subtitle: string
  provider: string
  kind: 'all' | 'codex' | 'claude' | 'model'
  todayTokens: number
  calls: number
  weekCost: number
  cacheRate: number
  ringPercent: number
  ringLabel: string
  ringCaption: string
  dataSource: string
  balanceText: string
  series: Record<FloatingMetric, FloatingSeriesPoint[]>
}

export interface FloatingRenderStatusView {
  backend: 'dxgi-d3d11' | 'acrylic' | 'translucent'
  state: 'active' | 'degraded' | 'stopped'
  fps: number
  detail: string
}

const props = withDefaults(defineProps<{
  mode: FloatingMode
  rows: FloatingDashboardView[]
  selectedKey: string
  expandedKeys: string[]
  metric: FloatingMetric
  availableMetrics: FloatingMetric[]
  syncing: boolean
  nextRefresh: string
  refreshedText: string
  proxyLabel: string
  proxyActive: boolean
  ccLabel: string
  ccActive: boolean
  alwaysOnTop: boolean
  interaction: FloatingInteractionMode
  renderStatus: FloatingRenderStatusView
  logoUrl: string
  codexLogoUrl: string
  preview?: boolean
}>(), { preview: false })

const emit = defineEmits<{
  dragStart: [event: MouseEvent]
  resizeStart: [direction: string, event: MouseEvent]
  select: [key: string]
  toggleExpand: [key: string]
  setMetric: [metric: FloatingMetric]
  setMode: [mode: FloatingMode]
  refresh: []
  close: []
  toggleAlwaysOnTop: []
  setInteraction: [mode: FloatingInteractionMode]
}>()

const selected = computed(() => props.rows.find(row => row.key === props.selectedKey) || props.rows[0])
const collapsed = computed(() => props.mode === 'capsule')
const metricLabel: Record<FloatingMetric, string> = {
  tokens: 'Token',
  calls: '请求',
  cached: '缓存',
  cost: '消费',
}

function logoName(row: FloatingDashboardView) {
  return row.kind === 'claude' ? 'Anthropic' : row.provider
}

function metricText(metric: FloatingMetric, value: number) {
  if (metric === 'cost') return `¥${value.toFixed(4)}`
  if (metric === 'calls') return `${Math.round(value).toLocaleString()} 次`
  return `${Math.round(value).toLocaleString()} Token`
}

function onResize(direction: string, event: MouseEvent) {
  if (!props.preview && event.button === 0) emit('resizeStart', direction, event)
}
</script>

<template>
  <!-- 胶囊态恢复 v0.8.5 的经典折叠结构；展开态继续复用同一份数据与组件。 -->
  <section
    class="floating-classic"
    :class="[{ collapsed, 'is-preview': preview }]"
    :aria-label="collapsed ? 'Token Manager 经典折叠悬浮窗' : 'Token Manager 悬浮窗'"
  >
    <template v-if="selected">
      <header
        class="floating-classic__header"
        :class="{ 'is-collapsed': collapsed }"
        data-floating-control-zone
        data-tauri-drag-region
        title="按住此处拖动悬浮窗"
        @mousedown="emit('dragStart', $event)"
      >
        <div class="floating-classic__brand" data-tauri-drag-region>
          <img :src="logoUrl" alt="Token Manager">
          <span data-tauri-drag-region>
            <b data-tauri-drag-region>Token Manager</b>
            <small v-if="!collapsed" data-tauri-drag-region>{{ syncing ? '正在同步数据' : `30 秒自动刷新 · ${refreshedText}` }}</small>
          </span>
        </div>
        <nav v-if="!preview" :aria-label="collapsed ? '折叠悬浮窗控制' : '悬浮窗控制'" @mousedown.stop>
          <button v-if="!collapsed" :disabled="syncing" title="立即刷新" aria-label="立即刷新" @click.stop="emit('refresh')">
            <RefreshCw :size="14" :class="{ spinning: syncing }" />
          </button>
          <button v-if="collapsed" title="展开悬浮窗" aria-label="展开悬浮窗" @click.stop="emit('setMode', 'compact')">
            <Plus :size="15" />
          </button>
          <button v-else title="折叠悬浮窗" aria-label="折叠悬浮窗" @click.stop="emit('setMode', 'capsule')">
            <Minus :size="15" />
          </button>
          <button title="关闭悬浮窗" aria-label="关闭悬浮窗" @click.stop="emit('close')">
            <X :size="15" />
          </button>
        </nav>
      </header>

      <section v-if="collapsed" class="floating-classic__summary is-collapsed-summary">
        <span class="floating-classic__logo">
          <img v-if="selected.kind === 'all'" :src="logoUrl" alt="">
          <img v-else-if="selected.kind === 'codex'" :src="codexLogoUrl" alt="">
          <ProviderMark v-else :name="logoName(selected)" />
        </span>
        <span class="floating-classic__copy">
          <small>{{ selected.title }}</small>
          <b>{{ selected.todayTokens.toLocaleString() }} Token</b>
          <em>{{ selected.subtitle }}</em>
        </span>
        <span
          class="floating-classic__ring"
          :style="{ '--ring-value': `${selected.ringPercent * 3.6}deg` }"
          :class="{ warning: selected.kind === 'codex' && selected.ringPercent <= 20 }"
          :title="`${selected.ringCaption} ${selected.ringLabel}`"
        >
          <i></i><b>{{ selected.ringLabel }}</b>
        </span>
      </section>

      <main v-else class="floating-classic__content">
        <section class="floating-classic__summary">
          <span class="floating-classic__logo">
            <img v-if="selected.kind === 'all'" :src="logoUrl" alt="">
            <img v-else-if="selected.kind === 'codex'" :src="codexLogoUrl" alt="">
            <ProviderMark v-else :name="logoName(selected)" />
          </span>
          <span class="floating-classic__copy">
            <small>{{ selected.title }}</small>
            <b>{{ selected.todayTokens.toLocaleString() }} Token</b>
            <em>{{ selected.subtitle }}</em>
          </span>
          <span
            class="floating-classic__ring"
            :style="{ '--ring-value': `${selected.ringPercent * 3.6}deg` }"
            :class="{ warning: selected.kind === 'codex' && selected.ringPercent <= 20 }"
          >
            <i></i><b>{{ selected.ringLabel }}</b>
          </span>
        </section>

        <label class="floating-classic__selector">
          <span>监控对象</span>
          <select :value="selected.key" @change="emit('select', ($event.target as HTMLSelectElement).value)">
            <option v-for="row in rows" :key="row.key" :value="row.key">{{ row.title }}</option>
          </select>
          <ChevronDown :size="14" aria-hidden="true" />
        </label>

        <div class="floating-classic__stats">
          <span><small>今日请求</small><b>{{ selected.calls.toLocaleString() }}</b></span>
          <span><small>本周消费</small><b>¥{{ selected.weekCost.toFixed(2) }}</b></span>
          <span><small>{{ selected.kind === 'codex' ? '7 天额度' : '缓存命中' }}</small><b>{{ selected.kind === 'codex' ? selected.ringLabel : `${selected.cacheRate}%` }}</b></span>
        </div>

        <section class="floating-classic__chart">
          <div class="floating-classic__chart-head">
            <span><b>{{ metricLabel[metric] }}趋势</b><small>近 7 天真实记录</small></span>
            <strong>{{ metricText(metric, selected.series[metric].reduce((sum, item) => sum + item.value, 0)) }}</strong>
          </div>
          <nav class="floating-classic__tabs" aria-label="图表类型">
            <button
              v-for="item in availableMetrics"
              :key="item"
              :class="{ active: metric === item }"
              :aria-pressed="metric === item"
              @click="emit('setMetric', item)"
            >{{ metricLabel[item] }}</button>
          </nav>
          <div
            class="floating-classic__bars"
            :class="{ empty: !selected.series[metric].some(item => item.value > 0) }"
            role="img"
            :aria-label="`${selected.title}近七天${metricLabel[metric]}图表`"
          >
            <span v-for="point in selected.series[metric]" :key="point.key" tabindex="0">
              <i :style="{ height: `${point.height}%` }"></i><small>{{ point.label }}</small>
              <em><b>{{ point.label }}</b>{{ metricText(metric, point.value) }}</em>
            </span>
            <p v-if="!selected.series[metric].some(item => item.value > 0)">暂无{{ metricLabel[metric] }}记录<br><small>下一次刷新后自动更新</small></p>
          </div>
        </section>
      </main>

      <footer v-if="!collapsed" class="floating-classic__footer" data-floating-control-zone>
        <span :class="{ active: proxyActive }"><i></i>API 代理 {{ proxyLabel }}</span>
        <span :class="{ active: ccActive }"><i></i>CC Switch {{ ccLabel }}</span>
        <time>{{ nextRefresh }}</time>
      </footer>
    </template>

    <template v-if="!preview && !collapsed">
      <i
        v-for="direction in ['North', 'NorthEast', 'East', 'SouthEast', 'South', 'SouthWest', 'West', 'NorthWest']"
        :key="direction"
        class="floating-classic__resize"
        :class="`is-${direction.toLowerCase()}`"
        @mousedown="onResize(direction, $event)"
      ></i>
    </template>
  </section>
</template>

<style scoped>
.floating-classic {
  --radius: 24px;
  box-sizing: border-box;
  position: relative;
  display: flex;
  width: calc(100% - 2px);
  height: calc(100% - 2px);
  margin: 1px;
  flex-direction: column;
  padding: 11px;
  overflow: hidden;
  border: 0;
  border-radius: var(--radius);
  background: var(--tm-bg, #fff);
  color: var(--tm-ink, #1d1d1f);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tm-ink, #1d1d1f) 11%, transparent);
  clip-path: inset(0 round var(--radius));
  font-family: "SF Pro Display", "SF Pro Text", "PingFang SC", "Microsoft YaHei", sans-serif;
  font-kerning: normal;
}

.floating-classic.is-preview {
  min-height: 380px;
  pointer-events: none;
}

.floating-classic.collapsed {
  --radius: 28px;
  min-height: 0;
  padding: 11px;
}

.floating-classic.collapsed.is-preview {
  min-height: 152px;
}

.floating-classic__header {
  display: flex;
  min-height: 44px;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 9px;
  padding: 1px 2px 8px;
  border-bottom: 1px solid color-mix(in srgb, var(--tm-ink, #1d1d1f) 10%, transparent);
  cursor: grab;
  user-select: none;
}

.floating-classic__header.is-collapsed {
  min-height: 38px;
  margin-bottom: 6px;
  padding: 0 2px 6px;
}

.floating-classic__header:active {
  cursor: grabbing;
}

.floating-classic__brand {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 9px;
}

.floating-classic__brand > img {
  width: 30px;
  height: 30px;
  border-radius: 9px;
  object-fit: contain;
}

.floating-classic__brand > span {
  display: grid;
  min-width: 0;
}

.floating-classic__brand b {
  overflow: hidden;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.floating-classic__brand small {
  margin-top: 2px;
  color: var(--tm-muted, #86868b);
  font-size: 8px;
  white-space: nowrap;
}

.floating-classic__header nav {
  display: flex;
  flex: 0 0 auto;
  gap: 4px;
}

.floating-classic__header button {
  display: grid;
  width: 30px;
  height: 30px;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 10px;
  background: transparent;
  color: inherit;
  cursor: pointer;
  transition: background-color .2s ease, transform .2s cubic-bezier(.22, 1, .36, 1);
}

.floating-classic__header button:hover {
  background: var(--tm-surface, #f2f2f7);
  transform: translateY(-1px);
}

.floating-classic__header button:focus-visible,
.floating-classic__selector select:focus-visible,
.floating-classic__tabs button:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--tm-accent, #007aff) 72%, transparent);
  outline-offset: 2px;
}

.floating-classic__header button:last-child:hover {
  background: #ff3b30;
  color: #fff;
}

.floating-classic__content {
  display: grid;
  min-height: 0;
  flex: 1 1 auto;
  align-content: start;
  gap: 10px;
  padding: 1px 4px 8px 1px;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-color: color-mix(in srgb, var(--tm-ink, #1d1d1f) 22%, transparent) transparent;
  scrollbar-width: thin;
}

.floating-classic__content::-webkit-scrollbar {
  width: 5px;
}

.floating-classic__content::-webkit-scrollbar-thumb {
  border-radius: 99px;
  background: color-mix(in srgb, var(--tm-ink, #1d1d1f) 22%, transparent);
}

.floating-classic__summary {
  display: grid;
  min-height: 72px;
  grid-template-columns: 44px minmax(0, 1fr) 52px;
  align-items: center;
  gap: 11px;
  padding: 12px;
  border: 0;
  border-radius: 16px;
  background: color-mix(in srgb, var(--tm-surface, #f2f2f7) 74%, var(--tm-bg, #fff));
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tm-ink, #1d1d1f) 8%, transparent);
}

.floating-classic__summary.is-collapsed-summary {
  min-height: 72px;
  flex: 1 1 auto;
  padding: 9px 11px;
}

.floating-classic__logo {
  display: grid;
  width: 42px;
  height: 42px;
  place-items: center;
  overflow: hidden;
  border-radius: 12px;
  background: var(--tm-surface, #f2f2f7);
}

.floating-classic__logo > img,
.floating-classic__logo :deep(.provider-mark) {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.floating-classic__logo > img {
  box-sizing: border-box;
  padding: 4px;
}

.floating-classic__copy {
  display: grid;
  min-width: 0;
}

.floating-classic__copy small {
  color: var(--tm-muted, #86868b);
  font-size: 8px;
}

.floating-classic__copy b {
  overflow: hidden;
  margin: 2px 0;
  color: inherit;
  font-size: 15px;
  font-variant-numeric: tabular-nums;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.floating-classic__copy em {
  overflow: hidden;
  color: var(--tm-muted, #86868b);
  font-size: 7px;
  font-style: normal;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.floating-classic__ring {
  --ring-value: 0deg;
  position: relative;
  display: grid;
  width: 48px;
  height: 48px;
  place-items: center;
  border-radius: 50%;
  background: conic-gradient(var(--tm-accent, var(--chart-1, #1d1d1f)) var(--ring-value), color-mix(in srgb, var(--tm-ink, #1d1d1f) 10%, transparent) 0);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tm-ink, #1d1d1f) 5%, transparent);
}

.floating-classic__ring::before {
  position: absolute;
  inset: 5px;
  border-radius: inherit;
  background: color-mix(in srgb, var(--tm-bg, #fff) 90%, var(--tm-surface, #f2f2f7));
  content: "";
}

.floating-classic__ring > b {
  position: relative;
  font-size: 9px;
}

.floating-classic__ring.warning {
  background: conic-gradient(#ff3b30 var(--ring-value), color-mix(in srgb, #ff3b30 14%, transparent) 0);
}

.floating-classic__selector {
  position: relative;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) 16px;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, var(--tm-ink, #1d1d1f) 12%, transparent);
  border-radius: 14px;
  background: var(--tm-surface, #f2f2f7);
  color: var(--tm-muted, #86868b);
  font-size: 8px;
}

.floating-classic__selector select {
  width: 100%;
  min-width: 0;
  min-height: 32px;
  padding: 6px 28px 6px 10px;
  border: 1px solid color-mix(in srgb, var(--tm-ink, #1d1d1f) 11%, transparent);
  border-radius: 11px;
  appearance: none;
  background: var(--tm-bg, #fff);
  color: var(--tm-ink, #1d1d1f);
  font: 600 9px/1.2 inherit;
}

.floating-classic__selector svg {
  position: absolute;
  right: 22px;
  color: var(--tm-ink, #1d1d1f);
  pointer-events: none;
}

.floating-classic__stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 7px;
}

.floating-classic__stats > span {
  display: grid;
  min-width: 0;
  gap: 4px;
  padding: 10px;
  border-radius: 13px;
  background: var(--tm-surface, #f2f2f7);
}

.floating-classic__stats small {
  color: var(--tm-muted, #86868b);
  font-size: 7px;
}

.floating-classic__stats b {
  overflow: hidden;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.floating-classic__chart {
  min-height: 176px;
  padding: 12px;
  border: 1px solid color-mix(in srgb, var(--tm-ink, #1d1d1f) 12%, transparent);
  border-radius: 16px;
  background: var(--tm-bg, #fff);
}

.floating-classic__chart-head {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: 10px;
}

.floating-classic__chart-head > span {
  display: grid;
}

.floating-classic__chart-head b {
  font-size: 11px;
}

.floating-classic__chart-head small {
  margin-top: 2px;
  color: var(--tm-muted, #86868b);
  font-size: 7px;
}

.floating-classic__chart-head strong {
  overflow: hidden;
  max-width: 52%;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.floating-classic__tabs {
  display: flex;
  gap: 3px;
  margin: 10px 0;
  padding: 3px;
  border-radius: 11px;
  background: var(--tm-surface, #f2f2f7);
}

.floating-classic__tabs button {
  min-height: 28px;
  flex: 1;
  padding: 4px;
  border: 0;
  border-radius: 9px;
  background: transparent;
  color: var(--tm-muted, #86868b);
  font: 600 8px/1 inherit;
  cursor: pointer;
}

.floating-classic__tabs button.active {
  background: var(--tm-bg, #fff);
  color: var(--tm-ink, #1d1d1f);
  box-shadow: 0 1px 4px color-mix(in srgb, var(--tm-ink, #1d1d1f) 10%, transparent);
}

.floating-classic__bars {
  position: relative;
  display: grid;
  height: 92px;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  align-items: end;
  gap: 6px;
  padding: 12px 3px 0;
  border-bottom: 1px solid color-mix(in srgb, var(--tm-ink, #1d1d1f) 11%, transparent);
}

.floating-classic__bars > span {
  position: relative;
  display: flex;
  height: 72px;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  outline: 0;
}

.floating-classic__bars > span > i {
  display: block;
  width: min(18px, 70%);
  min-height: 2px;
  border-radius: 6px 6px 2px 2px;
  background: var(--tm-accent, var(--chart-1, #1d1d1f));
  animation: floating-bar-rise .32s cubic-bezier(.22, 1, .36, 1) both;
  transform-origin: bottom;
}

.floating-classic__bars > span > small {
  margin-top: 4px;
  color: var(--tm-muted, #86868b);
  font-size: 6px;
}

.floating-classic__bars > span > em {
  position: absolute;
  z-index: 8;
  bottom: calc(100% + 5px);
  left: 50%;
  display: grid;
  min-width: max-content;
  padding: 7px 8px;
  border-radius: 9px;
  background: var(--tm-ink, #1d1d1f);
  color: var(--tm-on-ink, #fff);
  font-size: 7px;
  font-style: normal;
  opacity: 0;
  pointer-events: none;
  transform: translate(-50%, 4px);
  transition: opacity .18s ease, transform .18s ease;
}

.floating-classic__bars > span > em b {
  margin-bottom: 2px;
}

.floating-classic__bars > span:hover > em,
.floating-classic__bars > span:focus-visible > em {
  opacity: 1;
  transform: translate(-50%, 0);
}

.floating-classic__bars > p {
  position: absolute;
  inset: 12px 0 0;
  display: grid;
  place-items: center;
  align-content: center;
  margin: 0;
  border-radius: 11px;
  background: var(--tm-surface, #f2f2f7);
  color: var(--tm-muted, #86868b);
  font-size: 8px;
  text-align: center;
}

.floating-classic__bars > p small {
  margin-top: 3px;
  font-size: 7px;
}

.floating-classic__footer {
  display: flex;
  min-height: 28px;
  flex: 0 0 auto;
  align-items: center;
  gap: 10px;
  padding: 5px 4px 0;
  border-top: 1px solid color-mix(in srgb, var(--tm-ink, #1d1d1f) 9%, transparent);
  color: var(--tm-muted, #86868b);
  font-size: 7px;
}

.floating-classic__footer span {
  display: flex;
  align-items: center;
  gap: 5px;
  white-space: nowrap;
}

.floating-classic__footer span > i {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  opacity: .28;
}

.floating-classic__footer span.active {
  color: var(--tm-accent, #007aff);
}

.floating-classic__footer span.active > i {
  opacity: 1;
  box-shadow: 0 0 0 3px color-mix(in srgb, currentColor 14%, transparent);
}

.floating-classic__footer time {
  margin-left: auto;
  white-space: nowrap;
}

.floating-classic__resize {
  position: absolute;
  z-index: 20;
}

.floating-classic__resize.is-north,
.floating-classic__resize.is-south {
  right: 12px;
  left: 12px;
  height: 7px;
  cursor: ns-resize;
}

.floating-classic__resize.is-north { top: 0; }
.floating-classic__resize.is-south { bottom: 0; }

.floating-classic__resize.is-east,
.floating-classic__resize.is-west {
  top: 12px;
  bottom: 12px;
  width: 7px;
  cursor: ew-resize;
}

.floating-classic__resize.is-east { right: 0; }
.floating-classic__resize.is-west { left: 0; }

.floating-classic__resize.is-northeast,
.floating-classic__resize.is-northwest,
.floating-classic__resize.is-southeast,
.floating-classic__resize.is-southwest {
  width: 14px;
  height: 14px;
}

.floating-classic__resize.is-northeast { top: 0; right: 0; cursor: nesw-resize; }
.floating-classic__resize.is-northwest { top: 0; left: 0; cursor: nwse-resize; }
.floating-classic__resize.is-southeast { right: 0; bottom: 0; cursor: nwse-resize; }
.floating-classic__resize.is-southwest { bottom: 0; left: 0; cursor: nesw-resize; }

@keyframes floating-bar-rise {
  from { transform: scaleY(.12); }
  to { transform: scaleY(1); }
}

@media (prefers-reduced-motion: reduce) {
  .floating-classic__bars > span > i,
  .floating-classic__header button {
    animation: none;
    transition: none;
  }
}
</style>
