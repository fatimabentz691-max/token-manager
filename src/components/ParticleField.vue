<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useMotionPreferences } from '../features/motionPreferences'
import type { GlassQuality } from '../features/visualPreferences'

const props = withDefaults(defineProps<{ quality?: GlassQuality }>(), { quality: 'high' })
const canvas = ref<HTMLCanvasElement | null>(null)
const { motionEnabled, particlesEnabled } = useMotionPreferences()
let observer: ResizeObserver | undefined
let rendererObserver: MutationObserver | undefined
let particleColor = '#6E9CFF'
type Point = { x:number; y:number; r:number; alpha:number }
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
    alpha: Math.random() * .22 + .06,
  }))
}

function draw() {
  const node = canvas.value
  const ctx = node?.getContext('2d')
  if (!node || !ctx) return
  ctx.clearRect(0, 0, node.width, node.height)
  for (const point of points) {
    ctx.globalAlpha = point.alpha
    ctx.fillStyle = particleColor
    ctx.beginPath()
    ctx.arc(point.x, point.y, point.r, 0, Math.PI * 2)
    ctx.fill()
  }
  ctx.globalAlpha = 1
}

function restart() {
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
  observer = new ResizeObserver(() => { resize(); restart() })
  if (canvas.value) observer.observe(canvas.value)
  // 粒子仅绘制一次，不使用常驻动画；WebGL 接管或主题切换时再按需重绘。
  rendererObserver = new MutationObserver(restart)
  rendererObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
  restart()
})
onUnmounted(() => {
  observer?.disconnect()
  rendererObserver?.disconnect()
})
</script>

<template><canvas ref="canvas" class="particle-field" aria-hidden="true"></canvas></template>

<style scoped>
.particle-field{position:absolute;inset:0;width:100%;height:100%;pointer-events:none;opacity:.85}
</style>
