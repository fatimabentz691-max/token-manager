<script setup lang="ts">
import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import ParticleField from './ParticleField.vue'
import LiquidGlassRenderer from './LiquidGlassRenderer.vue'
import type { GlassDistortionSettings, GlassQuality } from '../features/visualPreferences'
import { useThemePreferences } from '../features/themePreferences'

interface PreparedLiquidVideo {
  path: string
  file_name: string
  size_bytes: number
  extension: string
}

const props = defineProps<{ quality: GlassQuality; distortion: GlassDistortionSettings; compact?: boolean; backgroundUrl?: string; videoPath?: string }>()
const resolvedVideoUrl = ref('')
const { setLiquidVideoError } = useThemePreferences()
let videoResolveToken = 0
const fallbackDistortionScale = computed(() => Math.round(4 + props.distortion.intensity * 6 + props.distortion.edgeBend * 5))
const environmentStyle = computed(() => ({
  '--glass-fallback-scale': String(1.025 + props.distortion.magnification * .02),
}))

let pointerFrame = 0
function updatePointer(event: PointerEvent) {
  cancelAnimationFrame(pointerFrame)
  pointerFrame = requestAnimationFrame(() => {
    const x = `${Math.round((event.clientX / Math.max(1, window.innerWidth)) * 100)}%`
    const y = `${Math.round((event.clientY / Math.max(1, window.innerHeight)) * 100)}%`
    document.documentElement.style.setProperty('--glass-pointer-x', x)
    document.documentElement.style.setProperty('--glass-pointer-y', y)
  })
}

onMounted(() => window.addEventListener('pointermove', updatePointer, { passive: true }))
onUnmounted(() => {
  videoResolveToken += 1
  cancelAnimationFrame(pointerFrame)
  window.removeEventListener('pointermove', updatePointer)
})

watch(() => props.videoPath, async (path) => {
  const token = ++videoResolveToken
  resolvedVideoUrl.value = ''
  setLiquidVideoError('')
  if (!path) return
  if (!isTauri()) {
    setLiquidVideoError('浏览器预览无法读取电脑上的本地视频；请在 Token Manager 桌面版中预览。')
    return
  }
  try {
    // 每个 WebView 启动时都重新登记精确文件路径，重启软件后也能安全恢复视频壁纸。
    const prepared = await invoke<PreparedLiquidVideo>('prepare_liquid_background_video', { path })
    if (token !== videoResolveToken) return
    resolvedVideoUrl.value = convertFileSrc(prepared.path)
  } catch (error) {
    if (token !== videoResolveToken) return
    setLiquidVideoError(`视频壁纸无法读取：${String(error)}`)
  }
}, { immediate: true })

function handleVideoError(message: string) {
  setLiquidVideoError(message)
}
</script>

<template>
  <div class="liquid-glass-environment" :class="[{ compact, 'has-video-background': Boolean(resolvedVideoUrl) }, `quality-${quality}`]" :style="environmentStyle" aria-hidden="true">
    <LiquidGlassRenderer
      :quality="quality"
      :distortion="distortion"
      :compact="compact"
      :background-url="backgroundUrl"
      :video-url="resolvedVideoUrl"
      @video-error="handleVideoError"
    />
    <div class="glass-user-background"></div>
    <div class="glass-atmosphere glass-atmosphere-a"></div>
    <div class="glass-atmosphere glass-atmosphere-b"></div>
    <div class="glass-atmosphere glass-atmosphere-c"></div>
    <div class="glass-caustics"></div>
    <ParticleField :quality="quality" />
    <svg width="0" height="0" focusable="false">
      <filter id="token-manager-liquid-refraction">
        <feTurbulence type="fractalNoise" baseFrequency=".007 .012" numOctaves="2" seed="8" result="noise" />
        <feDisplacementMap in="SourceGraphic" in2="noise" :scale="fallbackDistortionScale" xChannelSelector="R" yChannelSelector="B" />
      </filter>
    </svg>
  </div>
</template>
