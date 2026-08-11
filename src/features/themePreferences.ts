import { computed, ref, watch } from 'vue'
import liquidGraphiteWallpaper from '../assets/wallpapers/liquid-graphite.png?url'
import liquidPearlWallpaper from '../assets/wallpapers/liquid-pearl.png?url'
import { useChartColorPreferences } from './chartColorPreferences'

export interface AppTheme {
  id: string
  name: string
  source: string
  background: string
  ink: string
  accent: string
  glow: string
  spotlight: string
  dark: boolean
  material?: 'solid' | 'liquid'
}

export type LiquidGlassTone = 'clear' | 'dark'
export type LiquidWallpaperPreset = 'auto' | 'graphite' | 'pearl' | 'none'

const LIQUID_TRANSPARENCY_KEY = 'token-manager-liquid-transparency'
export const DEFAULT_LIQUID_TRANSPARENCY = 72

function normalizeLiquidTransparency(value: unknown) {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? Math.min(100, Math.max(0, Math.round(parsed))) : DEFAULT_LIQUID_TRANSPARENCY
}

export const liquidWallpaperPresets: ReadonlyArray<{ id: LiquidWallpaperPreset; name: string; description: string }> = [
  { id: 'auto', name: '跟随明暗', description: '深色使用石墨，纯白使用珍珠' },
  { id: 'graphite', name: '石墨流线', description: '黑白高反差，突出折射边缘' },
  { id: 'pearl', name: '珍珠流线', description: '白灰柔光，适配纯白玻璃' },
  { id: 'none', name: '无壁纸', description: '仅保留环境光和材质效果' },
]

/**
 * 默认仍为用户指定的黑白主题。其余主题沿用同一套布局，只替换背景、文字和强调色，
 * 避免换色后产生完全不同的产品视觉。
 */
export const appThemes: AppTheme[] = [
  { id: 'mono', name: '原生黑白', source: '默认', background: '#FFFFFF', ink: '#17171A', accent: '#17171A', glow: '#D7D7DD', spotlight: '#A8A8B0', dark: false },
  { id: 'liquid-glass', name: '液态玻璃', source: 'iOS 26 · visionOS', background: '#070A12', ink: '#F7F9FF', accent: '#78A8FF', glow: '#9B77FF', spotlight: '#BFD2FF', dark: true, material: 'liquid' },
  { id: 'midnight', name: '午夜液态蓝', source: 'Command Center', background: '#07111F', ink: '#F7FAFF', accent: '#6E9CFF', glow: '#795BFF', spotlight: '#82B5FF', dark: true },
  { id: 'ultraviolet', name: '深空紫', source: '低饱和', background: '#0F0B1C', ink: '#FAF8FF', accent: '#A98CFF', glow: '#6646D8', spotlight: '#C7B0FF', dark: true },
  { id: 'graphite', name: '石墨黑', source: '专注', background: '#0D0E10', ink: '#F4F4F5', accent: '#D8D8DE', glow: '#66666F', spotlight: '#E4E4EA', dark: true },
  { id: 'ocean', name: '深海青', source: '冷静', background: '#071719', ink: '#F3FEFE', accent: '#60D6D0', glow: '#208D94', spotlight: '#8CEBE6', dark: true },
  { id: 'cobalt', name: '钴蓝', source: '高对比', background: '#071329', ink: '#F5F8FF', accent: '#4F8CFF', glow: '#2654C8', spotlight: '#76A8FF', dark: true },
  { id: 'pearl', name: '珍珠蓝白', source: '浅色', background: '#F7FAFF', ink: '#18345B', accent: '#356FBA', glow: '#BDD6F4', spotlight: '#A8CBF4', dark: false },
  { id: 'mist', name: '雾紫白', source: '浅色', background: '#FBF9FF', ink: '#4B3B65', accent: '#765AA4', glow: '#DDD1F0', spotlight: '#D7C4F2', dark: false },
  { id: 'sage', name: '青瓷白', source: '浅色', background: '#F7FBF9', ink: '#245A49', accent: '#2E7B62', glow: '#C6E6D9', spotlight: '#B7E5D2', dark: false },
  { id: 'warm-stone', name: '暖石白', source: '浅色', background: '#FCFAF8', ink: '#59483B', accent: '#806554', glow: '#E8D9CE', spotlight: '#E8CDBA', dark: false },
]

const themeId = ref(localStorage.getItem('token-manager-theme') || 'mono')
const theme = computed(() => appThemes.find(item => item.id === themeId.value) || appThemes[0])
const storedLiquidAccent = localStorage.getItem('token-manager-liquid-accent') || '#78A8FF'
const liquidAccent = ref(/^#[0-9a-f]{6}$/i.test(storedLiquidAccent) ? storedLiquidAccent.toUpperCase() : '#78A8FF')
const storedLiquidTone = localStorage.getItem('token-manager-liquid-tone')
const liquidTone = ref<LiquidGlassTone>(storedLiquidTone === 'clear' ? 'clear' : 'dark')
const liquidTransparency = ref(normalizeLiquidTransparency(localStorage.getItem(LIQUID_TRANSPARENCY_KEY)))
const liquidBackgroundImage = ref(localStorage.getItem('token-manager-liquid-background-image') || '')
const storedWallpaperPreset = localStorage.getItem('token-manager-liquid-wallpaper-preset')
const liquidWallpaperPreset = ref<LiquidWallpaperPreset>(liquidWallpaperPresets.some(item => item.id === storedWallpaperPreset) ? storedWallpaperPreset as LiquidWallpaperPreset : 'auto')
const liquidBackgroundVideoPath = ref(localStorage.getItem('token-manager-liquid-background-video-path') || '')
const liquidVideoError = ref('')
const { chartColorTheme } = useChartColorPreferences()
const hasLiquidBackground = computed(() => Boolean(liquidBackgroundImage.value))
const hasLiquidVideoBackground = computed(() => Boolean(liquidBackgroundVideoPath.value))
const hasLiquidCustomBackground = computed(() => hasLiquidBackground.value || hasLiquidVideoBackground.value)
const effectiveLiquidBackgroundImage = computed(() => {
  if (liquidBackgroundImage.value) return liquidBackgroundImage.value
  if (liquidWallpaperPreset.value === 'none') return ''
  if (liquidWallpaperPreset.value === 'graphite') return liquidGraphiteWallpaper
  if (liquidWallpaperPreset.value === 'pearl') return liquidPearlWallpaper
  return liquidTone.value === 'clear' ? liquidPearlWallpaper : liquidGraphiteWallpaper
})

function notifyLiquidAppearance() {
  window.dispatchEvent(new CustomEvent('token-manager-liquid-appearance-change', { detail: Date.now() }))
}

function applyLiquidAccent(value: string, notify = true) {
  if (!/^#[0-9a-f]{6}$/i.test(value)) return
  liquidAccent.value = value.toUpperCase()
  localStorage.setItem('token-manager-liquid-accent', liquidAccent.value)
  if (notify) notifyLiquidAppearance()
}

function applyLiquidTone(value: LiquidGlassTone, notify = true) {
  liquidTone.value = value === 'clear' ? 'clear' : 'dark'
  localStorage.setItem('token-manager-liquid-tone', liquidTone.value)
  if (notify) notifyLiquidAppearance()
}

function applyLiquidTransparency(value: number, notify = true) {
  liquidTransparency.value = normalizeLiquidTransparency(value)
  localStorage.setItem(LIQUID_TRANSPARENCY_KEY, String(liquidTransparency.value))
  if (notify) notifyLiquidAppearance()
}

function applyLiquidWallpaperPreset(value: LiquidWallpaperPreset, notify = true) {
  const next = liquidWallpaperPresets.some(item => item.id === value) ? value : 'auto'
  liquidWallpaperPreset.value = next
  localStorage.setItem('token-manager-liquid-wallpaper-preset', next)
  if (notify) notifyLiquidAppearance()
}

function applyLiquidBackgroundImage(value: string, notify = true) {
  try {
    if (value) localStorage.setItem('token-manager-liquid-background-image', value)
    else localStorage.removeItem('token-manager-liquid-background-image')
    liquidBackgroundImage.value = value
    if (notify) notifyLiquidAppearance()
    return { ok: true as const }
  } catch (error) {
    return { ok: false as const, error: String(error) }
  }
}

function applyLiquidBackgroundVideoPath(value: string, notify = true) {
  const normalized = value.trim()
  if (normalized) localStorage.setItem('token-manager-liquid-background-video-path', normalized)
  else localStorage.removeItem('token-manager-liquid-background-video-path')
  liquidBackgroundVideoPath.value = normalized
  liquidVideoError.value = ''
  if (notify) notifyLiquidAppearance()
}

function syncLiquidAppearance() {
  applyLiquidAccent(localStorage.getItem('token-manager-liquid-accent') || '#78A8FF', false)
  applyLiquidTone(localStorage.getItem('token-manager-liquid-tone') === 'clear' ? 'clear' : 'dark', false)
  applyLiquidTransparency(normalizeLiquidTransparency(localStorage.getItem(LIQUID_TRANSPARENCY_KEY)), false)
  liquidBackgroundImage.value = localStorage.getItem('token-manager-liquid-background-image') || ''
  applyLiquidWallpaperPreset((localStorage.getItem('token-manager-liquid-wallpaper-preset') || 'auto') as LiquidWallpaperPreset, false)
  liquidBackgroundVideoPath.value = localStorage.getItem('token-manager-liquid-background-video-path') || ''
  liquidVideoError.value = ''
}

export function useThemePreferences() {
  function applyTheme(id: string, notify = true) {
    if (!appThemes.some(item => item.id === id)) return
    themeId.value = id
    localStorage.setItem('token-manager-theme', id)
    if (notify) window.dispatchEvent(new CustomEvent('token-manager-theme-change', { detail: id }))
  }
  function setTheme(id: string) {
    applyTheme(id, true)
  }
  /**
   * 接收来自另一个 Tauri WebView 的主题变化时静默应用，
   * 避免主窗口与悬浮窗互相重复广播同一个主题事件。
   */
  function syncTheme(id: string) {
    applyTheme(id, false)
  }
  return {
    themeId,
    theme,
    liquidAccent,
    liquidTone,
    liquidTransparency,
    liquidBackgroundImage,
    effectiveLiquidBackgroundImage,
    liquidWallpaperPreset,
    liquidWallpaperPresets,
    liquidBackgroundVideoPath,
    liquidVideoError,
    hasLiquidBackground,
    hasLiquidVideoBackground,
    hasLiquidCustomBackground,
    setTheme,
    syncTheme,
    setLiquidAccent: (value: string) => applyLiquidAccent(value, true),
    setLiquidTone: (value: LiquidGlassTone) => applyLiquidTone(value, true),
    setLiquidTransparency: (value: number) => applyLiquidTransparency(value, true),
    setLiquidBackgroundImage: (value: string) => applyLiquidBackgroundImage(value, true),
    clearLiquidBackgroundImage: () => applyLiquidBackgroundImage('', true),
    setLiquidWallpaperPreset: (value: LiquidWallpaperPreset) => applyLiquidWallpaperPreset(value, true),
    setLiquidBackgroundVideoPath: (value: string) => applyLiquidBackgroundVideoPath(value, true),
    clearLiquidBackgroundVideoPath: () => applyLiquidBackgroundVideoPath('', true),
    setLiquidVideoError: (value: string) => { liquidVideoError.value = value },
    syncLiquidAppearance,
  }
}

function mixHex(left: string, right: string, ratio: number) {
  const parse = (value: string) => [
    Number.parseInt(value.slice(1, 3), 16),
    Number.parseInt(value.slice(3, 5), 16),
    Number.parseInt(value.slice(5, 7), 16),
  ]
  const a = parse(left)
  const b = parse(right)
  const channel = (index: number) => Math.round(a[index] * (1 - ratio) + b[index] * ratio)
    .toString(16)
    .padStart(2, '0')
  return `#${channel(0)}${channel(1)}${channel(2)}`.toUpperCase()
}

export function themeStyle(value: AppTheme) {
  const liquid = value.material === 'liquid'
  const clearLiquid = liquid && liquidTone.value === 'clear'
  const effectiveDark = liquid ? !clearLiquid : value.dark
  // 纯白模式仍保留真实折射，只将材质体积色切换为高亮中性色。
  const effectiveBackground = clearLiquid ? '#FFFFFF' : value.background
  const effectiveInk = clearLiquid ? '#1D1D1F' : value.ink
  const surfaceMix = effectiveDark ? '9%' : '4%'
  // 透明白液态模式与图表共用同一主色，避免图表已换色但按钮、选中态和环境光仍残留蓝色。
  const accent = clearLiquid ? chartColorTheme.value.series[0] : liquid ? liquidAccent.value : value.accent
  const glow = clearLiquid ? chartColorTheme.value.series[2] : liquid ? mixHex(accent, '#FFFFFF', .16) : value.glow
  const spotlight = clearLiquid ? chartColorTheme.value.series[1] : liquid ? mixHex(accent, '#FFFFFF', .34) : value.spotlight
  const customBackground = effectiveLiquidBackgroundImage.value
  // “通透度”越高，材质体积色越淡。黑/白液态主题只改变体积色，透明度始终共用同一个值。
  const materialAlpha = Number((.42 - liquidTransparency.value * .0038).toFixed(3))
  const materialRgb = clearLiquid ? '255 255 255' : '5 8 15'
  const unifiedLiquidSurface = `rgb(${materialRgb} / ${materialAlpha})`
  return {
    '--tm-bg': effectiveBackground,
    '--tm-ink': effectiveInk,
    '--tm-accent': accent,
    '--tm-glow': glow,
    '--tm-spotlight': spotlight,
    '--tm-surface': liquid ? unifiedLiquidSurface : `color-mix(in srgb, ${effectiveInk} ${surfaceMix}, ${effectiveBackground})`,
    '--tm-surface-strong': liquid ? unifiedLiquidSurface : `color-mix(in srgb, ${effectiveInk} ${effectiveDark ? '15%' : '9%'}, ${effectiveBackground})`,
    '--tm-line': clearLiquid ? 'rgba(29,29,31,.12)' : `color-mix(in srgb, ${effectiveInk} ${effectiveDark ? '14%' : '16%'}, ${effectiveBackground})`,
    '--tm-muted': liquid ? (clearLiquid ? '#3A3A3C' : '#C4CCDC') : `color-mix(in srgb, ${effectiveInk} ${effectiveDark ? '62%' : '68%'}, ${effectiveBackground})`,
    '--tm-tertiary': clearLiquid ? '#48484A' : `color-mix(in srgb, ${effectiveInk} 54%, ${effectiveBackground})`,
    '--tm-on-accent': '#FFFFFF',
    '--tm-on-ink': effectiveDark ? '#07111F' : effectiveBackground,
    '--tm-glass': liquid ? unifiedLiquidSurface : effectiveDark ? 'rgba(255,255,255,.065)' : 'rgba(255,255,255,.78)',
    '--tm-glass-strong': liquid ? unifiedLiquidSurface : effectiveDark ? 'rgba(255,255,255,.105)' : 'rgba(255,255,255,.92)',
    '--tm-liquid-transparency': String(liquidTransparency.value),
    '--tm-liquid-transparency-progress': `${liquidTransparency.value}%`,
    '--tm-liquid-surface-alpha': String(materialAlpha),
    '--tm-liquid-surface-rgb': materialRgb,
    '--tm-liquid-surface': unifiedLiquidSurface,
    '--tm-shadow': liquid ? (clearLiquid ? '0 12px 30px rgba(29,29,31,.055)' : '0 28px 80px rgba(0,0,0,.36)') : effectiveDark ? '0 20px 60px rgba(0,0,0,.28)' : '0 18px 50px rgba(29,29,31,.08)',
    '--tm-glass-edge': liquid ? (clearLiquid ? 'rgba(29,29,31,.14)' : 'rgba(255,255,255,.2)') : `color-mix(in srgb, ${effectiveInk} 12%, transparent)`,
    '--tm-glass-reflection': liquid ? (clearLiquid ? 'rgba(255,255,255,.24)' : 'rgba(255,255,255,.16)') : 'rgba(255,255,255,.08)',
    '--tm-glass-blur': liquid ? '32px' : '22px',
    '--tm-ambient-a': liquid ? accent : value.accent,
    '--tm-ambient-b': liquid ? glow : value.glow,
    '--tm-liquid-background-image': customBackground ? `url("${customBackground}")` : 'none',
    colorScheme: effectiveDark ? 'dark' : 'light',
  }
}

// 将主题变量同步到根节点，让 Teleport 到 body 的编辑器、弹窗与点击粒子保持同一配色。
watch([theme, liquidAccent, liquidTone, liquidTransparency, liquidBackgroundImage, liquidWallpaperPreset, liquidBackgroundVideoPath, chartColorTheme], ([value]) => {
  const styles = themeStyle(value)
  for (const [name, styleValue] of Object.entries(styles)) {
    if (name.startsWith('--')) document.documentElement.style.setProperty(name, String(styleValue))
    else if (name === 'colorScheme') document.documentElement.style.colorScheme = String(styleValue)
  }
  const effectiveDark = value.material === 'liquid' ? liquidTone.value === 'dark' : value.dark
  document.documentElement.classList.toggle('theme-dark', effectiveDark)
  document.documentElement.classList.toggle('theme-liquid-glass', value.material === 'liquid')
  document.documentElement.classList.toggle('liquid-tone-clear', value.material === 'liquid' && liquidTone.value === 'clear')
  document.documentElement.classList.toggle('liquid-tone-dark', value.material === 'liquid' && liquidTone.value === 'dark')
  document.documentElement.dataset.theme = value.id
  document.documentElement.dataset.material = value.material || 'solid'
  document.documentElement.dataset.liquidTone = liquidTone.value
  document.documentElement.classList.toggle('liquid-custom-background', value.material === 'liquid' && hasLiquidCustomBackground.value)
}, { immediate: true })
