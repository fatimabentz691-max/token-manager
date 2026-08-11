<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

const rail = ref<HTMLElement | null>(null)
const thumb = ref<HTMLElement | null>(null)
const scrollable = ref(false)
const dragging = ref(false)
const thumbHeight = ref(48)
const thumbTop = ref(0)
const scrollTop = ref(0)
const scrollMax = ref(0)

let resizeObserver: ResizeObserver | null = null
let dragStartY = 0
let dragStartScroll = 0

function readMetrics() {
  const root = document.documentElement
  const max = Math.max(0, root.scrollHeight - window.innerHeight)
  const railHeight = rail.value?.clientHeight ?? 0
  const nextThumbHeight = railHeight > 0
    ? Math.max(44, Math.min(railHeight, railHeight * (window.innerHeight / root.scrollHeight)))
    : 44
  const nextScrollTop = Math.min(max, Math.max(0, window.scrollY))
  const travel = Math.max(0, railHeight - nextThumbHeight)

  scrollMax.value = max
  scrollTop.value = nextScrollTop
  scrollable.value = max > 2 && travel > 0
  thumbHeight.value = nextThumbHeight
  thumbTop.value = max > 0 ? (nextScrollTop / max) * travel : 0
}

function scrollToRatio(clientY: number, centerThumb = true) {
  const bounds = rail.value?.getBoundingClientRect()
  if (!bounds || scrollMax.value <= 0) return
  const travel = Math.max(1, bounds.height - thumbHeight.value)
  const offset = centerThumb ? thumbHeight.value / 2 : 0
  const position = Math.min(travel, Math.max(0, clientY - bounds.top - offset))
  window.scrollTo({ top: (position / travel) * scrollMax.value, behavior: centerThumb ? 'smooth' : 'auto' })
}

function handleRailPointerDown(event: PointerEvent) {
  if (!scrollable.value || event.target === thumb.value) return
  scrollToRatio(event.clientY)
}

function beginDrag(event: PointerEvent) {
  if (!scrollable.value) return
  event.preventDefault()
  event.stopPropagation()
  dragging.value = true
  dragStartY = event.clientY
  dragStartScroll = window.scrollY
  thumb.value?.setPointerCapture(event.pointerId)
}

function moveDrag(event: PointerEvent) {
  if (!dragging.value) return
  const railHeight = rail.value?.clientHeight ?? 0
  const travel = Math.max(1, railHeight - thumbHeight.value)
  const deltaScroll = ((event.clientY - dragStartY) / travel) * scrollMax.value
  window.scrollTo({ top: dragStartScroll + deltaScroll, behavior: 'auto' })
}

function endDrag(event: PointerEvent) {
  if (!dragging.value) return
  dragging.value = false
  if (thumb.value?.hasPointerCapture(event.pointerId)) thumb.value.releasePointerCapture(event.pointerId)
}

function handleKeydown(event: KeyboardEvent) {
  if (!scrollable.value) return
  const pageStep = Math.max(160, window.innerHeight * .82)
  const targets: Record<string, number> = {
    ArrowUp: window.scrollY - 56,
    ArrowDown: window.scrollY + 56,
    PageUp: window.scrollY - pageStep,
    PageDown: window.scrollY + pageStep,
    Home: 0,
    End: scrollMax.value,
  }
  const target = targets[event.key]
  if (target === undefined) return
  event.preventDefault()
  window.scrollTo({ top: target, behavior: 'smooth' })
}

onMounted(async () => {
  await nextTick()
  readMetrics()
  window.addEventListener('scroll', readMetrics, { passive: true })
  window.addEventListener('resize', readMetrics, { passive: true })
  resizeObserver = new ResizeObserver(readMetrics)
  resizeObserver.observe(document.documentElement)
  if (document.body) resizeObserver.observe(document.body)
})

onBeforeUnmount(() => {
  window.removeEventListener('scroll', readMetrics)
  window.removeEventListener('resize', readMetrics)
  resizeObserver?.disconnect()
})
</script>

<template>
  <div
    ref="rail"
    class="edge-scroll-rail"
    :class="{ 'is-scrollable': scrollable, 'is-dragging': dragging }"
    role="scrollbar"
    aria-label="页面滚动"
    aria-orientation="vertical"
    :aria-valuemin="0"
    :aria-valuemax="Math.round(scrollMax)"
    :aria-valuenow="Math.round(scrollTop)"
    :tabindex="scrollable ? 0 : -1"
    @pointerdown="handleRailPointerDown"
    @keydown="handleKeydown"
  >
    <span class="edge-scroll-track" aria-hidden="true"></span>
    <span
      ref="thumb"
      class="edge-scroll-thumb"
      :style="{ height: `${thumbHeight}px`, transform: `translate3d(0, ${thumbTop}px, 0)` }"
      @pointerdown="beginDrag"
      @pointermove="moveDrag"
      @pointerup="endDrag"
      @pointercancel="endDrag"
    ></span>
  </div>
</template>

<style scoped>
.edge-scroll-rail {
  position: fixed;
  z-index: 90;
  top: 3rem;
  right: 0;
  bottom: .5rem;
  width: 1.125rem;
  border-radius: 999px 0 0 999px;
  pointer-events: none;
  outline: none;
  touch-action: none;
}

.edge-scroll-rail.is-scrollable {
  pointer-events: auto;
}

.edge-scroll-track,
.edge-scroll-thumb {
  position: absolute;
  right: .1875rem;
  width: .375rem;
  border-radius: 999px;
  opacity: 0;
  transition:
    opacity 180ms cubic-bezier(.2, .8, .2, 1),
    width 180ms cubic-bezier(.2, .8, .2, 1),
    background-color 180ms cubic-bezier(.2, .8, .2, 1);
}

.edge-scroll-track {
  inset-block: 0;
  background: color-mix(in srgb, var(--tm-ink) 5%, transparent);
}

.edge-scroll-thumb {
  top: 0;
  min-height: 2.75rem;
  cursor: grab;
  background: color-mix(in srgb, var(--tm-ink) 34%, transparent);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tm-surface) 36%, transparent);
  will-change: transform;
}

.edge-scroll-rail:hover .edge-scroll-track,
.edge-scroll-rail:hover .edge-scroll-thumb,
.edge-scroll-rail:focus-visible .edge-scroll-track,
.edge-scroll-rail:focus-visible .edge-scroll-thumb,
.edge-scroll-rail.is-dragging .edge-scroll-track,
.edge-scroll-rail.is-dragging .edge-scroll-thumb {
  opacity: 1;
}

.edge-scroll-rail:hover .edge-scroll-thumb,
.edge-scroll-rail:focus-visible .edge-scroll-thumb,
.edge-scroll-rail.is-dragging .edge-scroll-thumb {
  width: .4375rem;
  background: color-mix(in srgb, var(--tm-ink) 52%, transparent);
}

.edge-scroll-rail.is-dragging .edge-scroll-thumb {
  cursor: grabbing;
}

.theme-liquid-glass .edge-scroll-track {
  background: color-mix(in srgb, var(--tm-glass, var(--tm-surface)) 74%, transparent);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tm-ink) 8%, transparent);
  backdrop-filter: blur(12px) saturate(125%);
}

.theme-liquid-glass .edge-scroll-thumb {
  background: color-mix(in srgb, var(--tm-ink) 40%, transparent);
  box-shadow:
    inset 0 0 0 1px color-mix(in srgb, white 34%, transparent),
    0 2px 8px color-mix(in srgb, var(--tm-ink) 12%, transparent);
}

@media (prefers-reduced-motion: reduce) {
  .edge-scroll-track,
  .edge-scroll-thumb {
    transition: none;
  }
}
</style>
