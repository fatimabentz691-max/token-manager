import { ref, watch } from 'vue'

export type GlassQuality = 'high' | 'performance'

export interface GlassDistortionSettings {
  /** 整体折射位移倍率；保留最低强度，避免液态主题退化为普通毛玻璃。 */
  intensity: number
  /** 圆角边缘的厚度与弯折倍率。 */
  edgeBend: number
  /** 玻璃内部透镜的放大倍率。 */
  magnification: number
  /** RGB 物理色散倍率。 */
  dispersion: number
}

export const defaultGlassDistortion: GlassDistortionSettings = {
  intensity: 1,
  edgeBend: 1,
  magnification: 1,
  dispersion: 1,
}

const distortionStorageKey = 'token-manager-glass-distortion'

function clamp(value: unknown, minimum: number, maximum: number, fallback: number) {
  const number = Number(value)
  return Number.isFinite(number) ? Math.min(maximum, Math.max(minimum, number)) : fallback
}

function normalizeGlassDistortion(value: Partial<GlassDistortionSettings> | null | undefined): GlassDistortionSettings {
  return {
    intensity: clamp(value?.intensity, .35, 1.65, defaultGlassDistortion.intensity),
    edgeBend: clamp(value?.edgeBend, .45, 1.8, defaultGlassDistortion.edgeBend),
    magnification: clamp(value?.magnification, 0, 1.6, defaultGlassDistortion.magnification),
    dispersion: clamp(value?.dispersion, 0, 1.6, defaultGlassDistortion.dispersion),
  }
}

function loadGlassDistortion() {
  try {
    return normalizeGlassDistortion(JSON.parse(localStorage.getItem(distortionStorageKey) || 'null'))
  } catch {
    return { ...defaultGlassDistortion }
  }
}

const storedQuality = localStorage.getItem('token-manager-glass-quality')
const glassQuality = ref<GlassQuality>(storedQuality === 'performance' ? 'performance' : 'high')
const glassDistortion = ref<GlassDistortionSettings>(loadGlassDistortion())

watch(glassQuality, value => {
  localStorage.setItem('token-manager-glass-quality', value)
  document.documentElement.dataset.glassQuality = value
}, { immediate: true })

function applyGlassQuality(value: GlassQuality, notify: boolean) {
  glassQuality.value = value
  if (notify) window.dispatchEvent(new CustomEvent('token-manager-visual-change', { detail: value }))
}

function applyGlassDistortion(value: Partial<GlassDistortionSettings>, notify: boolean) {
  const next = normalizeGlassDistortion({ ...glassDistortion.value, ...value })
  glassDistortion.value = next
  localStorage.setItem(distortionStorageKey, JSON.stringify(next))
  if (notify) window.dispatchEvent(new CustomEvent('token-manager-glass-distortion-change', { detail: next }))
}

export function useVisualPreferences() {
  return {
    glassQuality,
    glassDistortion,
    setGlassQuality: (value: GlassQuality) => applyGlassQuality(value, true),
    syncGlassQuality: (value: GlassQuality) => applyGlassQuality(value, false),
    setGlassDistortion: (value: Partial<GlassDistortionSettings>) => applyGlassDistortion(value, true),
    syncGlassDistortion: (value: Partial<GlassDistortionSettings>) => applyGlassDistortion(value, false),
    resetGlassDistortion: () => applyGlassDistortion(defaultGlassDistortion, true),
  }
}
