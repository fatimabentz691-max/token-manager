<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useMotionPreferences } from '../features/motionPreferences'
import type { GlassQuality } from '../features/visualPreferences'

const props = withDefaults(defineProps<{ quality?: GlassQuality }>(), { quality: 'high' })
const canvas = ref<HTMLCanvasElement | null>(null)
const { motionEnabled, particlesEnabled } = useMotionPreferences()
let frame = 0
let observer: ResizeObserver | undefined
let rendererObserver: MutationObserver | undefined
let particleColor = '#6E9CFF'
type Point = { x:number; y:number; r:number; speed:number; alpha:number; depth:number }
let points: Point[] = []

function shouldDraw() {
  return motionEnabled.value
    && particlesEnabled.value
    && !document.documentElement.classList.contains('glass-webgl-active')
}

function resize() {
  const node = canvas.value
  if (!node) return
  const rect = node.getBoundingClientRect()
  const ratio = Math.min(2, window.devicePixelRatio || 1)
  node.width = Math.max(1, Math.round(rect.width * ratio))
  node.height = Math.max(1, Math.round(rect.height * ratio))
  const highCount = Math.min(42, Math.max(18, Math.round(rect.width / 38)))
  const count = props.quality === 'high' ? highCount : Math.min(16, Math.max(8, Math.round(rect.width / 95)))
  points = Array.from({ length: count }, () => ({
    x: Math.random() * node.width,
    y: Math.random() * node.height,
    r: (Math.random() * 1.3 + .45) * ratio,
    speed: (Math.random() * .08 + .025) * ratio,
    alpha: Math.random() * .22 + .06,
    depth: Math.random() * .7 + .3,
  }))
}

function draw() {
  const node = canvas.value
  const ctx = node?.getContext('2d')
  if (!node || !ctx) return
  ctx.clearRect(0, 0, node.width, node.height)
  for (const point of points) {
    point.y -= point.speed
    if (point.y < -4) { point.y = node.height + 4; point.x = Math.random() * node.width }
    ctx.globalAlpha = point.alpha
    ctx.fillStyle = particleColor
    ctx.shadowColor = particleColor
    ctx.shadowBlur = props.quality === 'high' ? point.depth * 7 : 0
    ctx.beginPath()
    ctx.arc(point.x, point.y, point.r, 0, Math.PI * 2)
    ctx.fill()
  }
  ctx.globalAlpha = 1
  ctx.shadowBlur = 0
  if (shouldDraw()) frame = requestAnimationFrame(draw)
}

function restart() {
  cancelAnimationFrame(frame)
  frame = 0
  const ctx = canvas.value?.getContext('2d')
  particleColor = getComputedStyle(document.documentElement).getPropertyValue('--tm-accent').trim() || '#6E9CFF'
  if (!shouldDraw()) {
    if (canvas.value && ctx) ctx.clearRect(0, 0, canvas.value.width, canvas.value.height)
    return
  }
  draw()
}

watch([motionEnabled, particlesEnabled, () => props.quality], () => {
  resize()
  restart()
})
onMounted(() => {
  resize()
  observer = new ResizeObserver(resize)
  if (canvas.value) observer.observe(canvas.value)
  // WebGL 已接管环境细节时粒子画布在视觉上不可见，立即停止它自己的 rAF；
  // 上下文丢失切回 CSS 安全模式后再自动恢复。
  rendererObserver = new MutationObserver(restart)
  rendererObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
  restart()
})
onUnmounted(() => {
  cancelAnimationFrame(frame)
  observer?.disconnect()
  rendererObserver?.disconnect()
})
</script>

<template><canvas ref="canvas" class="particle-field" aria-hidden="true"></canvas></template>

<style scoped>
.particle-field{position:absolute;inset:0;width:100%;height:100%;pointer-events:none;opacity:.85}
</style>
