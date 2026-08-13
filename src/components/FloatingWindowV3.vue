<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { ChevronDown, ChevronRight, GripVertical, Maximize2, Minimize2, Minus, Pin, PinOff, Plus, RefreshCw, X } from '@lucide/vue'
import ProviderMark from './ProviderMark.vue'
import AnimatedNumber from './AnimatedNumber.vue'
import SmoothProgressRing from './SmoothProgressRing.vue'
import FloatingUsageChart from './FloatingUsageChart.vue'
import type { FloatingInteractionMode, FloatingMode } from '../features/floatingPreferences'

export type FloatingMetric = 'tokens' | 'calls' | 'cached' | 'cost'

export interface FloatingDailyPoint {
  key: string
  label: string
  tokens: number
  calls: number
  cached: number
  cost: number
}

export interface FloatingDashboardView {
  key: string
  title: string
  subtitle: string
  provider: string
  kind: 'all' | 'codex' | 'claude' | 'model' | 'agent'
  todayTokens: number
  calls: number
  weekCost: number
  weekInput: number
  weekOutput: number
  weekCached: number
  cacheRate: number
  ringPercent: number
  ringLabel: string
  ringCaption: string
  dataSource: string
  balanceText: string
  lastUpdated: string
  hasData: boolean
  daily: FloatingDailyPoint[]
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
  pinnedKeys?: string[]
  allowMultipleExpanded?: boolean
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
  reorder: [source: string, target: string]
  togglePin: [key: string]
}>()

const selected = computed(() => props.rows.find(row => row.key === props.selectedKey) || props.rows[0])
const capsulePreviewOpen = ref(false)
const previewSuppressed = ref(false)
const capsuleEnterTimer = ref<number | null>(null)
const capsuleLeaveTimer = ref<number | null>(null)
const dragStartPoint = ref<{ x: number; y: number } | null>(null)
const draggedRowKey = ref('')
const collapsed = computed(() => props.mode === 'capsule')
const full = computed(() => props.mode === 'full')
const activeMetric = computed<FloatingMetric>(() => props.availableMetrics.includes(props.metric) ? props.metric : (props.availableMetrics[0] || 'tokens'))
const metricLabel: Record<FloatingMetric, string> = { tokens: 'Token', calls: '请求', cached: '缓存', cost: '消费' }

function logoName(row: FloatingDashboardView) { return row.kind === 'claude' ? 'Anthropic' : row.provider }
function safePercent(value: number) { return Math.max(0, Math.min(100, Number.isFinite(value) ? value : 0)) }
function dailyTotal(row: FloatingDashboardView, metric: FloatingMetric = 'tokens') {
  return row.daily.reduce((sum, point) => sum + Math.max(0, Number(point[metric]) || 0), 0)
}
function miniTrend(row: FloatingDashboardView) {
  const max = Math.max(1, ...row.daily.map(point => point.tokens))
  return row.daily.map(point => ({ ...point, height: point.tokens > 0 ? Math.max(10, point.tokens / max * 100) : 0 }))
}
function onResize(direction: string, event: MouseEvent) {
  if (!props.preview && event.button === 0) emit('resizeStart', direction, event)
}
function clearPreviewTimers() {
  if (capsuleEnterTimer.value !== null) window.clearTimeout(capsuleEnterTimer.value)
  if (capsuleLeaveTimer.value !== null) window.clearTimeout(capsuleLeaveTimer.value)
  capsuleEnterTimer.value = capsuleLeaveTimer.value = null
}
function openCapsulePreview() {
  if (previewSuppressed.value) return
  if (capsuleLeaveTimer.value !== null) window.clearTimeout(capsuleLeaveTimer.value)
  capsuleEnterTimer.value = window.setTimeout(() => { if (!previewSuppressed.value) capsulePreviewOpen.value = true }, 240)
}
function closeCapsulePreview() {
  if (capsuleEnterTimer.value !== null) window.clearTimeout(capsuleEnterTimer.value)
  capsuleLeaveTimer.value = window.setTimeout(() => { capsulePreviewOpen.value = false }, 180)
}
function startHeaderDrag(event: MouseEvent) {
  if (props.preview || event.button !== 0) return
  dragStartPoint.value = { x: event.screenX, y: event.screenY }
  let started = false
  const move = (next: MouseEvent) => {
    if (!dragStartPoint.value || started) return
    const dx = next.screenX - dragStartPoint.value.x
    const dy = next.screenY - dragStartPoint.value.y
    if (Math.hypot(dx, dy) < 4) return
    started = true
    previewSuppressed.value = true
    capsulePreviewOpen.value = false
    emit('dragStart', next)
  }
  const up = () => {
    window.removeEventListener('mousemove', move)
    window.removeEventListener('mouseup', up)
  }
  window.addEventListener('mousemove', move)
  window.addEventListener('mouseup', up, { once: true })
}
function resetDragSuppression() {
  dragStartPoint.value = null
  previewSuppressed.value = false
}
function onRowDrop(target: string) {
  if (draggedRowKey.value && draggedRowKey.value !== target) emit('reorder', draggedRowKey.value, target)
  draggedRowKey.value = ''
}
onBeforeUnmount(clearPreviewTimers)
</script>

<template>
  <section class="floating-shell" :class="[`mode-${mode}`, { collapsed, 'is-preview': preview }]" :aria-label="collapsed ? 'Token Manager 胶囊悬浮窗' : 'Token Manager 悬浮窗'">
    <template v-if="selected">
      <header class="floating-header" :class="{ collapsed }" data-floating-control-zone title="拖动超过 4px 后移动悬浮窗" @mousedown="startHeaderDrag" @mouseenter="resetDragSuppression">
        <div class="floating-brand">
          <img :src="logoUrl" alt="Token Manager">
          <span><b>Token Manager</b><small v-if="!collapsed">{{ syncing ? '正在同步数据' : `30 秒自动刷新 · ${refreshedText}` }}</small></span>
        </div>
        <nav v-if="!preview" aria-label="悬浮窗控制" @mousedown.stop>
          <button v-if="!collapsed" :disabled="syncing" title="立即刷新" aria-label="立即刷新" @click.stop="emit('refresh')"><RefreshCw :size="14" :class="{ spinning: syncing }" /></button>
          <button v-if="mode === 'compact'" title="切换为大版" aria-label="切换为大版悬浮窗" @click.stop="emit('setMode', 'full')"><Maximize2 :size="14" /></button>
          <button v-else-if="mode === 'full'" title="切换为紧凑版" aria-label="切换为紧凑版悬浮窗" @click.stop="emit('setMode', 'compact')"><Minimize2 :size="14" /></button>
          <button v-if="collapsed" title="展开悬浮窗" aria-label="展开悬浮窗" @click.stop="emit('setMode', 'compact')"><Plus :size="15" /></button>
          <button v-else title="折叠为胶囊" aria-label="折叠为胶囊" @click.stop="emit('setMode', 'capsule')"><Minus :size="15" /></button>
          <button title="关闭悬浮窗" aria-label="关闭悬浮窗" @click.stop="emit('close')"><X :size="15" /></button>
        </nav>
      </header>

      <section v-if="collapsed" class="floating-summary capsule-summary">
        <span class="floating-logo">
          <img v-if="selected.kind === 'all'" :src="logoUrl" alt="">
          <img v-else-if="selected.kind === 'codex'" :src="codexLogoUrl" alt="">
          <ProviderMark v-else :name="logoName(selected)" />
        </span>
        <span class="floating-copy" tabindex="0" @mouseenter="openCapsulePreview" @mouseleave="closeCapsulePreview" @focusin="openCapsulePreview" @focusout="closeCapsulePreview">
          <small>{{ selected.title }}</small>
          <b><AnimatedNumber :value="selected.todayTokens" format="compact" suffix=" Token" /></b>
          <span>{{ selected.dataSource }}</span>
        </span>
        <SmoothProgressRing :value="safePercent(selected.ringPercent)" :label="selected.ringLabel" :caption="selected.ringCaption" :source="selected.dataSource" :updated-at="selected.lastUpdated" :warning="selected.kind === 'codex' && safePercent(selected.ringPercent) <= 20" tooltip-placement="left" :size="48" />
      </section>

      <aside v-if="collapsed" class="capsule-preview" :class="{ open:capsulePreviewOpen }" aria-label="当前监控对象用量预览" @mouseenter="openCapsulePreview" @mouseleave="closeCapsulePreview">
        <span><small>今日 Token</small><b><AnimatedNumber :value="selected.todayTokens" format="compact" /></b></span>
        <span><small>今日请求</small><b><AnimatedNumber :value="selected.calls" suffix=" 次" /></b></span>
        <span><small>本周消费</small><b><AnimatedNumber :value="selected.weekCost" :decimals="2" format="currency" /></b></span>
        <span><small>更新时间</small><b>{{ selected.lastUpdated }}</b></span>
      </aside>

      <main v-else class="floating-content">
        <section v-if="mode === 'compact'" class="compact-model-grid" aria-label="并列模型用量">
          <button v-for="row in rows" :key="row.key" class="compact-model-card" :class="{ active: row.key === selected.key, pinned: pinnedKeys?.includes(row.key) }" :aria-pressed="row.key === selected.key" @click="emit('select',row.key)">
            <span class="compact-model-card__logo"><img v-if="row.kind === 'all'" :src="logoUrl" alt=""><img v-else-if="row.kind === 'codex'" :src="codexLogoUrl" alt=""><ProviderMark v-else :name="logoName(row)" /></span>
            <span class="compact-model-card__copy"><small>{{ row.title }}</small><b><AnimatedNumber :value="row.todayTokens" format="compact" suffix=" Token" /></b><span>{{ row.lastUpdated }}</span></span>
            <SmoothProgressRing :value="safePercent(row.ringPercent)" :label="row.ringLabel" :caption="row.ringCaption" :source="row.dataSource" :updated-at="row.lastUpdated" :warning="row.kind === 'codex' && safePercent(row.ringPercent) <= 20" tooltip-placement="left" :size="46" />
            <span class="compact-model-card__preview">
              <span><small>请求</small><b><AnimatedNumber :value="row.calls" suffix=" 次" /></b></span>
              <span><small>费用</small><b><AnimatedNumber :value="row.weekCost" :decimals="2" format="currency" /></b></span>
              <span><small>缓存</small><b><AnimatedNumber :value="row.weekCached" format="compact" /></b></span>
              <span class="compact-mini-bars"><i v-for="point in miniTrend(row)" :key="point.key" :style="{height:`${point.height}%`}"></i></span>
              <em>{{ row.dataSource }}</em>
            </span>
          </button>
        </section>

        <template v-else>
        <section class="floating-summary">
          <span class="floating-logo">
            <img v-if="selected.kind === 'all'" :src="logoUrl" alt="">
            <img v-else-if="selected.kind === 'codex'" :src="codexLogoUrl" alt="">
            <ProviderMark v-else :name="logoName(selected)" />
          </span>
          <span class="floating-copy">
            <small>{{ selected.title }}</small>
            <b><AnimatedNumber :value="selected.todayTokens" suffix=" Token" /></b>
            <span>{{ selected.subtitle }} · {{ selected.dataSource }}</span>
          </span>
          <SmoothProgressRing :value="safePercent(selected.ringPercent)" :label="selected.ringLabel" :caption="selected.ringCaption" :source="selected.dataSource" :updated-at="selected.lastUpdated" :warning="selected.kind === 'codex' && safePercent(selected.ringPercent) <= 20" />
        </section>

        <label class="floating-selector">
          <span>监控对象</span>
          <select :value="selected.key" @change="emit('select', ($event.target as HTMLSelectElement).value)"><option v-for="row in rows" :key="row.key" :value="row.key">{{ row.title }}</option></select>
          <ChevronDown :size="14" aria-hidden="true" />
        </label>

        <div class="floating-stats">
          <span><small>今日请求</small><b><AnimatedNumber :value="selected.calls" suffix=" 次" /></b></span>
          <span><small>本周消费</small><b><AnimatedNumber :value="selected.weekCost" :decimals="2" format="currency" /></b></span>
          <span><small>{{ selected.kind === 'codex' ? '本周可验证 Token' : '输入 / 输出' }}</small><b v-if="selected.kind === 'codex'"><AnimatedNumber :value="dailyTotal(selected)" format="compact" /></b><b v-else><AnimatedNumber :value="selected.weekInput" format="compact" /> / <AnimatedNumber :value="selected.weekOutput" format="compact" /></b></span>
          <span><small>{{ selected.kind === 'codex' ? '7 天额度' : '缓存 Token' }}</small><b v-if="selected.kind === 'codex'">{{ selected.ringLabel }}</b><b v-else><AnimatedNumber :value="selected.weekCached" format="compact" /></b></span>
        </div>

        <section class="floating-chart" aria-labelledby="floating-chart-title">
          <div class="floating-chart-head"><span><b id="floating-chart-title">近七天真实用量</b><small>{{ selected.balanceText }}</small></span><time>{{ selected.lastUpdated }}</time></div>
          <nav class="floating-tabs" aria-label="图表类型"><button v-for="item in availableMetrics" :key="item" :class="{ active: activeMetric === item }" :aria-pressed="activeMetric === item" @click="emit('setMetric', item)">{{ metricLabel[item] }}</button></nav>
          <FloatingUsageChart :points="selected.daily" :metric="activeMetric" :source="selected.dataSource" :updated-at="selected.lastUpdated" :has-data="selected.hasData" />
        </section>

        <section v-if="full && rows.length > 1" class="floating-agents" aria-label="Agent 用量列表">
          <div class="floating-section-head"><b>全部监控对象</b><small>{{ rows.length }} 项 · 点击逐行展开</small></div>
           <article v-for="row in rows" :key="row.key" :class="{ active: row.key === selected.key, expanded: expandedKeys.includes(row.key), pinned: pinnedKeys?.includes(row.key), dragging: draggedRowKey===row.key }" draggable="true" @dragstart="draggedRowKey=row.key" @dragend="draggedRowKey=''" @dragover.prevent @drop.prevent="onRowDrop(row.key)">
            <span class="agent-drag" aria-hidden="true"><GripVertical :size="14" /></span>
            <button class="agent-main" :aria-pressed="row.key === selected.key" @click="emit('select', row.key)">
              <span class="agent-logo"><img v-if="row.kind === 'all'" :src="logoUrl" alt=""><img v-else-if="row.kind === 'codex'" :src="codexLogoUrl" alt=""><ProviderMark v-else :name="logoName(row)" /></span>
              <span><b>{{ row.title }}</b><small>{{ row.dataSource }} · {{ row.lastUpdated }}</small></span>
              <span class="agent-numbers"><strong><AnimatedNumber :value="row.todayTokens" format="compact" /></strong><small>Token</small></span>
              <span class="agent-numbers"><strong><AnimatedNumber :value="row.weekCost" :decimals="2" format="currency" /></strong><small>本周费用</small></span>
            </button>
             <button class="agent-pin" :aria-label="`${pinnedKeys?.includes(row.key)?'取消固定':'固定'} ${row.title}`" @click="emit('togglePin',row.key)"><PinOff v-if="pinnedKeys?.includes(row.key)" :size="14"/><Pin v-else :size="14"/></button>
             <button class="agent-expand" :aria-label="`${expandedKeys.includes(row.key) ? '收起' : '展开'} ${row.title}`" @click="emit('toggleExpand', row.key)"><ChevronRight :size="15" /></button>
            <div v-if="expandedKeys.includes(row.key)" class="agent-detail">
              <span><small>输入 Token</small><b><AnimatedNumber :value="row.weekInput" format="compact" /></b></span>
              <span><small>输出 Token</small><b><AnimatedNumber :value="row.weekOutput" format="compact" /></b></span>
              <span><small>缓存 Token</small><b><AnimatedNumber :value="row.weekCached" format="compact" /></b></span>
              <div class="agent-trend" :class="{ empty: !row.hasData }" aria-label="七天 Token 迷你趋势"><i v-for="point in miniTrend(row)" :key="point.key" :style="{ height: `${point.height}%` }" :title="`${point.label} · ${point.tokens.toLocaleString()} Token`"></i><small v-if="!row.hasData">暂无真实记录</small></div>
            </div>
          </article>
        </section>
        </template>
      </main>

      <footer v-if="!collapsed" class="floating-footer" data-floating-control-zone>
        <span :class="{ active: proxyActive }" :title="`API 代理 ${proxyLabel}`"><i></i><b>代理</b><small>{{ proxyLabel }}</small></span>
        <span :class="{ active: ccActive }" :title="`CC Switch ${ccLabel}`"><i></i><b>CC</b><small>{{ ccLabel }}</small></span>
        <time :title="nextRefresh">{{ nextRefresh }}</time>
      </footer>
    </template>

    <template v-if="!preview && !collapsed"><i v-for="direction in ['North','NorthEast','East','SouthEast','South','SouthWest','West','NorthWest']" :key="direction" class="resize-handle" :class="`is-${direction.toLowerCase()}`" @mousedown="onResize(direction,$event)"></i></template>
  </section>
</template>

<style scoped>
.floating-shell{--radius:24px;position:relative;display:flex;width:calc(100% - 2px);height:calc(100% - 2px);margin:1px;flex-direction:column;padding:12px;overflow:hidden;border:0;border-radius:var(--radius);background:var(--tm-bg,#fff);color:var(--tm-ink,#1d1d1f);box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--tm-ink,#1d1d1f) 10%,transparent);clip-path:inset(0 round var(--radius));font-family:var(--apple-font,"PingFang SC","Microsoft YaHei",sans-serif);font-size:11px}.floating-shell.collapsed{--radius:28px;padding:10px}.floating-shell.is-preview{min-height:390px;pointer-events:none}.floating-shell.collapsed.is-preview{min-height:152px}.floating-header{display:flex;min-height:42px;flex:0 0 auto;align-items:center;justify-content:space-between;gap:10px;margin-bottom:9px;padding:0 2px 8px;border-bottom:1px solid color-mix(in srgb,var(--tm-ink) 9%,transparent);cursor:grab;user-select:none}.floating-header.collapsed{min-height:34px;margin-bottom:5px;padding-bottom:5px}.floating-header:active{cursor:grabbing}.floating-brand{display:flex;min-width:0;align-items:center;gap:8px}.floating-brand>img{width:29px;height:29px;border-radius:9px;object-fit:contain}.floating-brand>span{display:grid;min-width:0}.floating-brand b,.floating-brand small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.floating-brand b{font-size:13px}.floating-brand small{margin-top:1px;color:var(--tm-muted);font-size:9px}.floating-header nav{display:flex;flex:0 0 auto;gap:3px}.floating-header button{display:grid;width:29px;height:29px;place-items:center;padding:0;border:0;border-radius:10px;background:transparent;color:inherit;cursor:pointer}.floating-header button:hover{background:var(--tm-surface);transform:translateY(-1px)}.floating-header button:last-child:hover{background:#ff3b30;color:#fff}.floating-header button:focus-visible,.floating-shell select:focus-visible,.floating-tabs button:focus-visible{outline:2px solid color-mix(in srgb,var(--tm-accent,#007aff) 72%,transparent);outline-offset:1px}.floating-content{display:grid;min-height:0;flex:1 1 auto;align-content:start;gap:10px;padding:1px 5px 9px 1px;overflow-x:hidden;overflow-y:auto;scrollbar-color:color-mix(in srgb,var(--tm-ink) 20%,transparent) transparent;scrollbar-width:thin}.floating-content::-webkit-scrollbar{width:5px}.floating-content::-webkit-scrollbar-thumb{border-radius:99px;background:color-mix(in srgb,var(--tm-ink) 20%,transparent)}.floating-summary{position:relative;z-index:3;display:grid;min-height:74px;grid-template-columns:44px minmax(0,1fr) 54px;align-items:center;gap:11px;padding:11px 12px;border-radius:17px;background:color-mix(in srgb,var(--tm-surface) 76%,var(--tm-bg));box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--tm-ink) 7%,transparent);overflow:visible}.capsule-summary{min-height:70px;flex:1 1 auto;padding:8px 10px}.floating-logo{display:grid;width:42px;height:42px;place-items:center;overflow:hidden;border-radius:13px;background:var(--tm-surface)}.floating-logo>img,.floating-logo :deep(.provider-mark){width:100%;height:100%;object-fit:contain}.floating-logo>img{box-sizing:border-box;padding:4px}.floating-copy{display:grid;min-width:0;gap:2px}.floating-copy small{color:var(--tm-muted);font-size:10px}.floating-copy b{overflow:hidden;font-size:16px;font-variant-numeric:tabular-nums;text-overflow:ellipsis;white-space:nowrap}.floating-copy>span{overflow:hidden;color:var(--tm-muted);font-size:9px;text-overflow:ellipsis;white-space:nowrap}.capsule-preview{position:absolute;z-index:12;right:10px;bottom:10px;left:10px;display:grid;min-height:76px;grid-template-columns:1fr 1fr;gap:7px;padding:10px 12px;border:1px solid color-mix(in srgb,var(--tm-ink) 9%,transparent);border-radius:18px;background:color-mix(in srgb,var(--tm-bg) 96%,transparent);box-shadow:0 14px 34px color-mix(in srgb,var(--tm-ink) 15%,transparent);opacity:0;pointer-events:none;transform:translateY(4px) scale(.985);transition:opacity .2s ease,transform .2s cubic-bezier(.22,1,.36,1);backdrop-filter:blur(22px) saturate(145%)}.floating-shell.collapsed:hover .capsule-preview{opacity:1;transform:none}.capsule-preview>span{display:grid;min-width:0;gap:1px}.capsule-preview small{color:var(--tm-muted);font-size:9px}.capsule-preview b{overflow:hidden;font-size:11px;text-overflow:ellipsis;white-space:nowrap}.floating-selector{position:relative;display:grid;grid-template-columns:auto minmax(0,1fr) 16px;align-items:center;gap:10px;padding:8px 10px;border:1px solid color-mix(in srgb,var(--tm-ink) 10%,transparent);border-radius:14px;background:var(--tm-surface);color:var(--tm-muted);font-size:10px}.floating-selector select{width:100%;min-width:0;min-height:34px;padding:6px 28px 6px 10px;border:1px solid color-mix(in srgb,var(--tm-ink) 10%,transparent);border-radius:10px;appearance:none;background:var(--tm-bg);color:var(--tm-ink);font-family:inherit;font-size:11px;font-weight:600;line-height:1.2}.floating-selector svg{position:absolute;right:20px;color:var(--tm-ink);pointer-events:none}.floating-stats{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}.floating-stats>span{display:grid;min-width:0;gap:3px;padding:10px 11px;border-radius:14px;background:var(--tm-surface)}.floating-stats small{color:var(--tm-muted);font-size:9px}.floating-stats b{overflow:hidden;font-size:11px;font-variant-numeric:tabular-nums;text-overflow:ellipsis;white-space:nowrap}.mode-full .floating-stats{grid-template-columns:repeat(4,minmax(0,1fr))}.floating-chart{min-height:224px;padding:12px;border:1px solid color-mix(in srgb,var(--tm-ink) 10%,transparent);border-radius:17px;background:color-mix(in srgb,var(--tm-bg) 94%,transparent);overflow:visible}.mode-full .floating-chart{min-height:244px;padding:15px}.floating-chart-head{display:flex;align-items:flex-end;justify-content:space-between;gap:12px}.floating-chart-head>span{display:grid;min-width:0;gap:2px}.floating-chart-head b{font-size:13px}.floating-chart-head small,.floating-chart-head time{overflow:hidden;color:var(--tm-muted);font-size:9px;text-overflow:ellipsis;white-space:nowrap}.floating-tabs{display:flex;gap:4px;margin:10px 0;padding:3px;border-radius:12px;background:var(--tm-surface)}.floating-tabs button{min-height:29px;flex:1;padding:5px;border:0;border-radius:9px;background:transparent;color:var(--tm-muted);font-family:inherit;font-size:10px;font-weight:600;line-height:1;cursor:pointer}.floating-tabs button.active{background:var(--tm-bg);color:var(--tm-ink);box-shadow:0 1px 4px color-mix(in srgb,var(--tm-ink) 10%,transparent)}.floating-agents{display:grid;gap:7px}.floating-section-head{display:flex;align-items:center;justify-content:space-between;padding:5px 2px}.floating-section-head b{font-size:12px}.floating-section-head small{color:var(--tm-muted);font-size:9px}.floating-agents>article{position:relative;display:grid;grid-template-columns:minmax(0,1fr) 38px;overflow:hidden;border:1px solid color-mix(in srgb,var(--tm-ink) 9%,transparent);border-radius:16px;background:color-mix(in srgb,var(--tm-surface) 72%,var(--tm-bg))}.floating-agents>article.active{border-color:color-mix(in srgb,var(--tm-accent,#007aff) 42%,transparent);box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--tm-accent,#007aff) 9%,transparent)}.agent-main{display:grid;min-width:0;min-height:62px;grid-template-columns:36px minmax(0,1fr) minmax(58px,auto) minmax(66px,auto);align-items:center;gap:9px;padding:9px 11px;border:0;background:transparent;color:inherit;text-align:left;cursor:pointer}.agent-main>span:nth-child(2){display:grid;min-width:0;gap:2px}.agent-main b,.agent-main small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.agent-main b{font-size:11px}.agent-main small{color:var(--tm-muted);font-size:9px}.agent-logo,.agent-logo :deep(.provider-mark){display:grid;width:34px;height:34px;place-items:center;border-radius:10px}.agent-logo>img{width:30px;height:30px;border-radius:9px;object-fit:contain}.agent-numbers{display:grid;justify-items:end;gap:1px}.agent-numbers strong{font-size:11px;font-variant-numeric:tabular-nums}.agent-expand{display:grid;min-width:38px;min-height:62px;place-items:center;border:0;border-left:1px solid color-mix(in srgb,var(--tm-ink) 8%,transparent);background:transparent;color:var(--tm-muted);cursor:pointer}.floating-agents>article.expanded .agent-expand svg{transform:rotate(90deg)}.agent-detail{display:grid;grid-column:1/-1;grid-template-columns:repeat(3,minmax(0,1fr));gap:7px;padding:10px 11px 11px;border-top:1px solid color-mix(in srgb,var(--tm-ink) 8%,transparent)}.agent-detail>span{display:grid;min-width:0;gap:2px}.agent-detail small{color:var(--tm-muted);font-size:9px}.agent-detail b{overflow:hidden;font-size:10px;text-overflow:ellipsis;white-space:nowrap}.agent-trend{position:relative;display:flex;height:42px;grid-column:1/-1;align-items:flex-end;gap:5px;padding-top:5px}.agent-trend i{min-width:4px;min-height:0;flex:1;border-radius:99px;background:var(--tm-accent)}.agent-trend>small{position:absolute;inset:0;display:grid;place-items:center;color:var(--tm-muted);font-size:9px}.floating-footer{display:grid;min-height:33px;grid-template-columns:minmax(0,1fr) minmax(0,1fr) auto;flex:0 0 auto;align-items:center;gap:8px;padding:7px 3px 0;border-top:1px solid color-mix(in srgb,var(--tm-ink) 9%,transparent);color:var(--tm-muted);font-size:9px}.floating-footer span{display:flex;min-width:0;align-items:center;gap:5px}.floating-footer span>i{width:6px;height:6px;flex:0 0 auto;border-radius:50%;background:currentColor;opacity:.28}.floating-footer span.active{color:var(--tm-accent)}.floating-footer span.active>i{opacity:1;box-shadow:0 0 0 3px color-mix(in srgb,currentColor 13%,transparent)}.floating-footer small{overflow:hidden;font-size:9px;text-overflow:ellipsis;white-space:nowrap}.floating-footer time{max-width:150px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.resize-handle{position:absolute;z-index:20}.resize-handle.is-north,.resize-handle.is-south{right:12px;left:12px;height:7px;cursor:ns-resize}.resize-handle.is-north{top:0}.resize-handle.is-south{bottom:0}.resize-handle.is-east,.resize-handle.is-west{top:12px;bottom:12px;width:7px;cursor:ew-resize}.resize-handle.is-east{right:0}.resize-handle.is-west{left:0}.resize-handle.is-northeast,.resize-handle.is-northwest,.resize-handle.is-southeast,.resize-handle.is-southwest{width:14px;height:14px}.resize-handle.is-northeast{top:0;right:0;cursor:nesw-resize}.resize-handle.is-northwest{top:0;left:0;cursor:nwse-resize}.resize-handle.is-southeast{right:0;bottom:0;cursor:nwse-resize}.resize-handle.is-southwest{bottom:0;left:0;cursor:nesw-resize}@media(max-width:380px){.floating-stats{grid-template-columns:1fr}.floating-footer{grid-template-columns:1fr 1fr}.floating-footer time{grid-column:1/-1}.floating-summary{grid-template-columns:40px minmax(0,1fr) 50px}.floating-logo{width:40px;height:40px}.floating-copy b{font-size:15px}}@media(prefers-reduced-motion:reduce){.floating-header button,.capsule-preview{animation:none;transition:none}}
</style>
<style scoped>
.mode-full .floating-agents{grid-template-columns:1fr;align-items:start}.mode-full .floating-agents>.floating-section-head{grid-column:1/-1}.mode-full .floating-agents>article{min-width:0}.mode-full .agent-main{grid-template-columns:34px minmax(0,1fr) minmax(64px,auto) minmax(64px,auto);gap:7px}.mode-full .agent-detail{grid-template-columns:repeat(3,minmax(0,1fr))}.floating-agents>article{grid-template-columns:22px minmax(0,1fr) 36px 36px;transition:opacity var(--tm-motion-standard,220ms) var(--tm-ease-out),transform var(--tm-motion-standard,220ms) var(--tm-ease-out)}.floating-agents>article.dragging{opacity:.55;transform:scale(.992)}.floating-agents>article.pinned{border-color:color-mix(in srgb,var(--tm-accent) 28%,var(--tm-line))}.agent-drag{display:grid;grid-row:1;place-items:center;color:var(--tm-muted);cursor:grab}.agent-main{grid-column:2}.agent-pin,.agent-expand{display:grid;min-width:36px;min-height:62px;place-items:center;border:0;border-left:1px solid color-mix(in srgb,var(--tm-ink) 8%,transparent);background:transparent;color:var(--tm-muted);cursor:pointer}.agent-detail{grid-column:1/-1}.agent-pin:hover,.agent-expand:hover{background:var(--tm-surface);color:var(--tm-ink)}
</style>
<style scoped>
.compact-model-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px;align-content:start}.compact-model-card{position:relative;display:grid;min-width:0;min-height:82px;grid-template-columns:38px minmax(0,1fr) 46px;align-items:center;gap:9px;padding:10px;border:1px solid color-mix(in srgb,var(--tm-ink) 9%,transparent);border-radius:17px;background:color-mix(in srgb,var(--tm-surface) 72%,var(--tm-bg));color:var(--tm-ink);font-family:inherit;text-align:left;cursor:pointer;overflow:visible}.compact-model-card.active{border-color:color-mix(in srgb,var(--tm-accent) 48%,var(--tm-line));box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--tm-accent) 12%,transparent)}.compact-model-card:hover{z-index:8;transform:translateY(-1px)}.compact-model-card__logo,.compact-model-card__logo :deep(.provider-mark){display:grid;width:38px;height:38px;place-items:center;border-radius:12px}.compact-model-card__logo>img{width:34px;height:34px;border-radius:10px;object-fit:contain}.compact-model-card__copy{display:grid;min-width:0;gap:2px}.compact-model-card__copy small{overflow:hidden;color:var(--tm-muted);font-size:9px;text-overflow:ellipsis;white-space:nowrap}.compact-model-card__copy b{overflow:hidden;font-size:13px;text-overflow:ellipsis;white-space:nowrap}.compact-model-card__copy>span{color:var(--tm-muted);font-size:9px}.compact-model-card__preview{position:absolute;z-index:30;top:calc(100% + 6px);right:0;left:0;display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:6px;padding:10px;border:1px solid color-mix(in srgb,var(--tm-ink) 10%,transparent);border-radius:14px;background:color-mix(in srgb,var(--tm-bg) 97%,transparent);box-shadow:0 16px 38px color-mix(in srgb,var(--tm-ink) 15%,transparent);opacity:0;pointer-events:none;transform:translateY(-3px) scale(.985);transition:opacity .18s ease,transform .2s cubic-bezier(.22,1,.36,1);backdrop-filter:blur(22px) saturate(145%)}.compact-model-card:is(:hover,:focus-visible) .compact-model-card__preview{opacity:1;transform:none}.compact-model-card__preview>span:not(.compact-mini-bars){display:grid;gap:2px}.compact-model-card__preview small{color:var(--tm-muted);font-size:9px}.compact-model-card__preview b{font-size:10px}.compact-model-card__preview em{grid-column:1/-1;overflow:hidden;color:var(--tm-muted);font-size:9px;font-style:normal;text-overflow:ellipsis;white-space:nowrap}.compact-mini-bars{display:flex;height:28px;grid-column:1/-1;align-items:flex-end;gap:4px;padding-top:3px}.compact-mini-bars i{min-width:3px;flex:1;border-radius:999px;background:var(--tm-accent)}@media(max-width:410px){.compact-model-grid{grid-template-columns:1fr}}@media(prefers-reduced-motion:reduce){.compact-model-card,.compact-model-card__preview{transition:none}}:global(.motion-off) .compact-model-card,:global(.motion-off) .compact-model-card__preview{transition:none}
</style>
<style scoped>
/* 紧凑窗口使用横向整行卡片，避免两列布局挤压标题、数值与圆环。 */
.compact-model-grid{grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}.compact-model-card{min-height:78px;grid-template-columns:38px minmax(72px,1fr) 46px;padding:9px}.compact-model-card__copy small{font-size:9px}.compact-model-card__copy b{font-size:13px}.compact-model-card__preview{top:calc(100% + 6px);right:0;left:0;width:auto;grid-template-columns:repeat(3,minmax(0,1fr));transform:translateY(-2px) scale(.985)}.compact-model-card:is(:hover,:focus-visible) .compact-model-card__preview{transform:none}.compact-model-card.pinned::after{position:absolute;top:7px;right:7px;width:5px;height:5px;border-radius:99px;background:var(--tm-accent);content:""}
/* 胶囊的详情只在文字区出现；圆环提示固定向左展开，始终留在裁切范围内。 */
.capsule-summary{z-index:20}.floating-shell.collapsed:hover .capsule-preview{opacity:0;transform:translateY(4px) scale(.985)}.capsule-preview.open{opacity:1!important;transform:none!important}.capsule-summary :deep(.smooth-ring){z-index:40}
@media(max-width:410px){.compact-model-grid{grid-template-columns:1fr}.compact-model-card__preview{padding:8px}.compact-model-card{grid-template-columns:36px minmax(100px,1fr) 46px}}

/* 悬停详情使用高不透明主题表面，避免底层 Codex/全局数值穿透后叠字。 */
.capsule-preview,.compact-model-card__preview{
  border-color:color-mix(in srgb,var(--tm-ink,#1d1d1f) 14%,transparent);
  background-color:var(--tm-overlay-surface,#fff)!important;
  background-image:none!important;
  box-shadow:0 16px 38px color-mix(in srgb,var(--tm-ink,#1d1d1f) 22%,transparent),inset 0 0 0 1px color-mix(in srgb,var(--tm-bg,#fff) 70%,transparent);
  color:var(--tm-overlay-ink,#1d1d1f)!important;
  isolation:isolate;
  filter:none!important;
  backdrop-filter:none!important;
  -webkit-backdrop-filter:none!important;
}
</style>
