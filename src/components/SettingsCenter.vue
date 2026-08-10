<script setup lang="ts">
import { invoke, isTauri } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref } from 'vue'
import ChartPreferences from './ChartPreferences.vue'
import { useChartPreferences, type DashboardMode } from '../features/chartPreferences'
import { chartColorThemes, useChartColorPreferences } from '../features/chartColorPreferences'
import { appThemes, useThemePreferences } from '../features/themePreferences'
import { useMotionPreferences } from '../features/motionPreferences'
import { useVisualPreferences } from '../features/visualPreferences'
import { currentCloudBaseUrl } from '../features/cloudConfig'
import FloatingWindowV3, { type FloatingDashboardView, type FloatingMetric, type FloatingRenderStatusView } from './FloatingWindowV3.vue'
import type { FloatingInteractionMode, FloatingMode } from '../features/floatingPreferences'

interface FloatingItem { id: string; title: string; description: string }
interface CloudSession { email: string; base_url: string; expires_at: string }
interface PreparedLiquidVideo { path: string; file_name: string; size_bytes: number; extension: string }
const props = defineProps<{
  floatingEnabled: boolean
  floatingItems: FloatingItem[]
  floatingOrder: string[]
  floatingSelected: string[]
  floatingLayout: 'grid' | 'list'
  floatingMode: FloatingMode
  floatingInteraction: FloatingInteractionMode
  floatingAlwaysOnTop: boolean
  floatingPreviewRows: FloatingDashboardView[]
  floatingSelectedKey: string
  floatingMetric: FloatingMetric
  floatingRenderStatus: FloatingRenderStatusView
  tokenManagerLogo: string
  codexLogo: string
  miniMode: boolean
  miniModules: [string, string]
  budget5h: number
  budget7d: number
  budgetStatus: string
  proxyUpstream: string
  proxyPort: number
  proxyStatus: string
  backupPassword: string
  backupStatus: string
  accountCount: number
  dashboardCount: number
  proxyEndpointCount: number
  ccSwitchInstalled: boolean
  ccSwitchRunning: boolean
  ccSwitchRouting: boolean
  ccSwitchDetail: string
  cloudSession: CloudSession | null
  cloudStatus: string
  cloudTransferLink: string
}>()
const emit = defineEmits<{
  toggleFloating: []
  toggleFloatingItem: [id: string, checked: boolean]
  reorderFloatingItem: [source: string, target: string]
  setFloatingLayout: [layout: 'grid' | 'list']
  setFloatingMode: [mode: FloatingMode]
  setFloatingInteraction: [mode: FloatingInteractionMode]
  toggleFloatingAlwaysOnTop: []
  setMiniMode: [checked: boolean]
  setMiniModule: [index: 0 | 1, id: string]
  saveBudget: [five: number, seven: number]
  startProxy: [upstream: string, port: number]
  exportBackup: [password: string]
  importBackup: [password: string]
  openAccounts: []
  startAllProxies: []
  cloudRequestCode: [baseUrl: string, email: string, purpose: 'login' | 'register']
  cloudPasswordLogin: [baseUrl: string, email: string, password: string]
  cloudPasswordRegister: [baseUrl: string, email: string, password: string, code: string]
  cloudCodeLogin: [baseUrl: string, email: string, code: string]
  cloudLogout: []
  cloudCreateTransfer: [password: string, ttlHours: number, oneTime: boolean]
  cloudImportTransfer: [link: string, password: string]
}>()

type SectionId = 'general' | 'charts' | 'floating' | 'cloud' | 'codex' | 'proxy' | 'data' | 'guide'
const section = ref<SectionId>('general')
const { mode, setMode } = useChartPreferences()
const {
  themeId,
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
  setTheme,
  setLiquidAccent,
  setLiquidTone,
  setLiquidTransparency,
  setLiquidBackgroundImage,
  clearLiquidBackgroundImage,
  setLiquidWallpaperPreset,
  setLiquidBackgroundVideoPath,
  clearLiquidBackgroundVideoPath,
} = useThemePreferences()
const { chartColorThemeId, setChartColorTheme } = useChartColorPreferences()
const { motionEnabled, particlesEnabled, springMotionEnabled, spotlightEnabled, setMotionEnabled, setParticlesEnabled, setSpringMotionEnabled, setSpotlightEnabled } = useMotionPreferences()
const { glassQuality, glassDistortion, setGlassQuality, setGlassDistortion, resetGlassDistortion } = useVisualPreferences()
const currentTheme = computed(() => appThemes.find(item => item.id === themeId.value) || appThemes[0])
const liquidGlassEnabled = computed(() => currentTheme.value.material === 'liquid')
const budgetFive = ref(props.budget5h)
const budgetSeven = ref(props.budget7d)
const upstream = ref(props.proxyUpstream)
const port = ref(props.proxyPort)
const password = ref(props.backupPassword)
const cloudMode = ref<'password' | 'code'>('password')
const cloudBaseUrl = ref(currentCloudBaseUrl())
const cloudEmail = ref('')
const cloudPassword = ref('')
const cloudCode = ref('')
const cloudTransferPassword = ref('')
const cloudImportLink = ref('')
const cloudTtlHours = ref(24)
const cloudOneTime = ref(true)
const draggedFloatingItem = ref<string | null>(null)
const floatingChartIds = new Set(['cacheChart', 'costChart', 'requestChart', 'tokenTrendChart'])
const floatingChartItems = computed(() => props.floatingOrder.filter(id => floatingChartIds.has(id)))
const floatingSummaryItems = computed(() => props.floatingOrder.filter(id => !floatingChartIds.has(id)))
const selectedFloatingCount = computed(() => props.floatingSelected.length)
const liquidBackgroundInput = ref<HTMLInputElement | null>(null)
const liquidAppearanceStatus = ref('')
const liquidColorPresets = [
  { name: '空间蓝', value: '#78A8FF' },
  { name: '星云紫', value: '#A98CFF' },
  { name: '深海青', value: '#60D6D0' },
  { name: '钴蓝', value: '#4F8CFF' },
  { name: '薄荷绿', value: '#49C7A5' },
  { name: '日落橙', value: '#FF9F5A' },
]
const distortionControls = [
  { key: 'intensity', label: '整体扭曲', detail: '同时控制边缘、内部与指针透镜的位移幅度', min: .35, max: 1.65, step: .05 },
  { key: 'edgeBend', label: '边缘弯折', detail: '增强圆角四周的厚度与背景弯曲', min: .45, max: 1.8, step: .05 },
  { key: 'magnification', label: '内部放大', detail: '调整玻璃中心对背景内容的透镜放大', min: 0, max: 1.6, step: .05 },
  { key: 'dispersion', label: '边缘色散', detail: '调整边缘轻微 RGB 分离，不产生彩色发光', min: 0, max: 1.6, step: .05 },
] as const
const liquidBackgroundPreviewStyle = computed(() => ({
  '--liquid-preview-accent': liquidAccent.value,
  backgroundImage: effectiveLiquidBackgroundImage.value
    ? `${liquidTone.value === 'clear' ? 'linear-gradient(145deg, rgba(255,255,255,.08), rgba(255,255,255,.24))' : 'linear-gradient(145deg, rgba(5,8,18,.14), rgba(5,8,18,.46))'}, url("${effectiveLiquidBackgroundImage.value}")`
    : liquidTone.value === 'clear'
      ? `radial-gradient(circle at 26% 18%, ${liquidAccent.value}24, transparent 42%), linear-gradient(145deg, #F8FAFD, #E8EDF5)`
      : `radial-gradient(circle at 26% 18%, ${liquidAccent.value}66, transparent 42%), linear-gradient(145deg, #080C16, #111827)`,
}))
const liquidVideoFileName = computed(() => liquidBackgroundVideoPath.value.split(/[\\/]/).pop() || '本地视频')
const liquidWallpaperName = computed(() => liquidWallpaperPresets.find(item => item.id === liquidWallpaperPreset.value)?.name || '跟随明暗')

function floatingItem(id: string) { return props.floatingItems.find(item => item.id === id) }

const allSections: Array<{ id: SectionId; title: string; detail: string; advanced?: boolean }> = [
  { id: 'general', title: '通用与显示模式', detail: '全局简单 / 高级模式' },
  { id: 'charts', title: '仪表盘图表', detail: '选择需要的分析组件' },
  { id: 'floating', title: '悬浮窗', detail: '模块、排序与迷你模式' },
  { id: 'cloud', title: '云账户与迁移', detail: '邮箱登录和一次性链接' },
  { id: 'codex', title: 'Codex 额度', detail: '客户端额度与估算回退', advanced: true },
  { id: 'proxy', title: 'API 实时监控', detail: '本地代理与端口', advanced: true },
  { id: 'data', title: '数据与迁移', detail: '加密备份和恢复', advanced: true },
  { id: 'guide', title: '新手配置教程', detail: '从账户接入到仪表盘' }
]
const sections = computed(() => mode.value === 'simple' ? allSections.filter(item => !item.advanced) : allSections)

function setGlobalMode(value: DashboardMode) {
  setMode(value)
  if (value === 'simple' && allSections.find(item => item.id === section.value)?.advanced) section.value = 'general'
}

function toggleLiquidGlass() {
  setTheme(liquidGlassEnabled.value ? 'mono' : 'liquid-glass')
}

function selectLiquidAccent(value: string) {
  setLiquidAccent(value)
  if (!liquidGlassEnabled.value) setTheme('liquid-glass')
  liquidAppearanceStatus.value = `液态玻璃主题色已更新为 ${value.toUpperCase()}`
}

function selectLiquidTone(value: 'clear' | 'dark') {
  setLiquidTone(value)
  if (!liquidGlassEnabled.value) setTheme('liquid-glass')
  liquidAppearanceStatus.value = value === 'clear'
    ? '已切换为纯白折射玻璃，主界面 WebGL 与悬浮窗 Acrylic 已同步开启'
    : '已切换为深色黑玻璃，悬浮窗同步使用实时桌面背板'
}

function updateDistortion(key: (typeof distortionControls)[number]['key'], event: Event) {
  setGlassDistortion({ [key]: Number((event.target as HTMLInputElement).value) })
}

function updateLiquidTransparency(event: Event) {
  const value = Number((event.target as HTMLInputElement).value)
  setLiquidTransparency(value)
  if (!liquidGlassEnabled.value) setTheme('liquid-glass')
  liquidAppearanceStatus.value = `全局液态玻璃通透度已调整为 ${Math.round(value)}%`
}

function selectLiquidWallpaper(value: 'auto' | 'graphite' | 'pearl' | 'none') {
  setLiquidWallpaperPreset(value)
  if (!liquidGlassEnabled.value) setTheme('liquid-glass')
  liquidAppearanceStatus.value = `内置壁纸已切换为 ${liquidWallpaperPresets.find(item => item.id === value)?.name || value}`
}

function openLiquidBackgroundPicker() {
  liquidBackgroundInput.value?.click()
}

function loadImage(file: File) {
  return new Promise<HTMLImageElement>((resolve, reject) => {
    const url = URL.createObjectURL(file)
    const image = new Image()
    image.decoding = 'async'
    image.onload = () => {
      URL.revokeObjectURL(url)
      resolve(image)
    }
    image.onerror = () => {
      URL.revokeObjectURL(url)
      reject(new Error('图片无法解码，请改用 PNG、JPEG 或 WebP 文件'))
    }
    image.src = url
  })
}

async function optimizeLiquidBackground(file: File) {
  const image = await loadImage(file)
  const attempts = [
    { max: 2560, quality: .86 },
    { max: 2048, quality: .78 },
    { max: 1600, quality: .70 },
  ]
  let result = ''
  for (const attempt of attempts) {
    const scale = Math.min(1, attempt.max / Math.max(image.naturalWidth, image.naturalHeight))
    const canvas = document.createElement('canvas')
    canvas.width = Math.max(1, Math.round(image.naturalWidth * scale))
    canvas.height = Math.max(1, Math.round(image.naturalHeight * scale))
    const context = canvas.getContext('2d', { alpha: false })
    if (!context) throw new Error('当前显卡环境无法处理背景图片')
    context.imageSmoothingEnabled = true
    context.imageSmoothingQuality = 'high'
    context.drawImage(image, 0, 0, canvas.width, canvas.height)
    result = canvas.toDataURL('image/webp', attempt.quality)
    // 控制在约 1.8 MB 内，保证 localStorage、加密备份和双窗口同步稳定。
    if (result.length <= 2_400_000) break
  }
  if (!result || result.length > 3_200_000) throw new Error('图片压缩后仍过大，请选择尺寸更小的图片')
  return result
}

async function handleLiquidBackgroundFile(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  if (!['image/png', 'image/jpeg', 'image/webp'].includes(file.type)) {
    liquidAppearanceStatus.value = '仅支持 PNG、JPEG 与 WebP 图片'
    return
  }
  if (file.size > 20 * 1024 * 1024) {
    liquidAppearanceStatus.value = '原图不能超过 20 MB'
    return
  }
  liquidAppearanceStatus.value = '正在本地优化背景图片…'
  try {
    const optimized = await optimizeLiquidBackground(file)
    const result = setLiquidBackgroundImage(optimized)
    if (!result.ok) throw new Error('本地存储空间不足，请先移除旧背景或选择更小图片')
    clearLiquidBackgroundVideoPath()
    if (!liquidGlassEnabled.value) setTheme('liquid-glass')
    liquidAppearanceStatus.value = '背景已保存，并实时同步到主界面与悬浮窗'
  } catch (error) {
    liquidAppearanceStatus.value = `背景设置失败：${String(error)}`
  }
}

async function openLiquidVideoPicker() {
  if (!isTauri()) {
    liquidAppearanceStatus.value = '浏览器预览无法读取本地视频，请在 Token Manager 桌面版中选择。'
    return
  }
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      title: '选择液态玻璃视频壁纸',
      filters: [{ name: '视频壁纸', extensions: ['mp4', 'webm', 'mov', 'm4v'] }],
    })
    if (!selected || Array.isArray(selected)) return
    liquidAppearanceStatus.value = '正在校验本地视频…'
    const prepared = await invoke<PreparedLiquidVideo>('prepare_liquid_background_video', { path: selected })
    setLiquidBackgroundVideoPath(prepared.path)
    if (!liquidGlassEnabled.value) setTheme('liquid-glass')
    liquidAppearanceStatus.value = `${prepared.file_name} 已启用（${(prepared.size_bytes / 1024 / 1024).toFixed(1)} MB），主界面与悬浮窗将实时同步`
  } catch (error) {
    liquidAppearanceStatus.value = `视频设置失败：${String(error)}`
  }
}

function removeLiquidBackground() {
  clearLiquidBackgroundImage()
  liquidAppearanceStatus.value = hasLiquidVideoBackground.value ? '已移除备用图片，视频壁纸保持启用' : '已恢复液态玻璃默认空间背景'
}

function removeLiquidVideoBackground() {
  clearLiquidBackgroundVideoPath()
  liquidAppearanceStatus.value = hasLiquidBackground.value ? '已清除视频壁纸，并恢复已保存的图片背景' : '已清除视频壁纸，并恢复默认空间背景'
}

function resetLiquidAppearance() {
  setLiquidAccent('#78A8FF')
  setLiquidTone('dark')
  setLiquidTransparency(72)
  clearLiquidBackgroundImage()
  setLiquidWallpaperPreset('auto')
  clearLiquidBackgroundVideoPath()
  resetGlassDistortion()
  if (!liquidGlassEnabled.value) setTheme('liquid-glass')
  liquidAppearanceStatus.value = '液态玻璃外观已恢复默认'
}

function dropFloatingItem(target: string) {
  if (draggedFloatingItem.value && draggedFloatingItem.value !== target) emit('reorderFloatingItem', draggedFloatingItem.value, target)
  draggedFloatingItem.value = null
}
function rememberCloudBase() { localStorage.setItem('token-manager-cloud-base-url', cloudBaseUrl.value.trim()) }
function copyCloudLink() { if (props.cloudTransferLink) void window.navigator.clipboard.writeText(props.cloudTransferLink) }
</script>

<template>
  <section class="settings-center">
    <aside class="settings-nav">
      <div><span>设置中心</span><h2>分类设置</h2></div>
      <button v-for="item in sections" :key="item.id" :class="{ active: section === item.id }" @click="section = item.id">
        <span><b>{{ item.title }}</b><small>{{ item.detail }}</small></span><i>›</i>
      </button>
    </aside>
    <article class="settings-detail">
      <template v-if="section === 'general'">
        <header><span>通用</span><h2>全局界面复杂度</h2><p>该选择同时作用于主仪表盘、设置页和悬浮窗。简单模式只保留日常核心操作，高级模式开放全部分析与代理设置。</p></header>
        <div class="global-mode">
          <button :class="{ active: mode === 'simple' }" @click="setGlobalMode('simple')"><b>简单模式</b><span>四项核心图表、精简悬浮窗和基础设置</span></button>
          <button :class="{ active: mode === 'advanced' }" @click="setGlobalMode('advanced')"><b>高级模式</b><span>全部图表、代理调试、预算与迁移工具</span></button>
        </div>
        <button
          class="liquid-glass-quick-control"
          :class="{ active: liquidGlassEnabled }"
          type="button"
          :aria-pressed="liquidGlassEnabled"
          @click="toggleLiquidGlass"
        >
          <span class="liquid-glass-orb" aria-hidden="true"><i></i></span>
          <span class="liquid-glass-quick-copy">
            <small>主题快捷控制 · 推荐</small>
            <b>液态玻璃主题</b>
            <em>{{ liquidGlassEnabled ? '已开启，主界面与悬浮窗正在使用空间玻璃材质' : '点击立即开启，并自动同步到悬浮窗' }}</em>
          </span>
          <span class="liquid-glass-state">
            <strong>{{ liquidGlassEnabled ? '已开启' : '未开启' }}</strong>
            <i aria-hidden="true"><em></em></i>
          </span>
        </button>
        <div class="monochrome-note"><b>{{currentTheme.material==='liquid'?'空间光学材质已启用':'克制的主题色系统'}}</b><span>{{currentTheme.material==='liquid'?'液态玻璃使用低饱和蓝紫环境光建立景深，正文与数据仍保持高对比；代理状态继续使用清晰的形状和文字标签。':'界面使用所选背景与主色生成层级；连接状态通过填充、明暗和文字标签表达，不只依赖颜色判断。'}}</span></div>
        <section class="motion-settings" aria-labelledby="motion-title">
          <div><h3 id="motion-title">动态效果</h3><p>仅在交互、数据刷新与加载时触发，不会在后台常驻渲染。</p></div>
          <label class="switch-row"><span><b>界面动画总开关</b><small>关闭淡入、数字滚动、图表缓动与悬停微动效</small></span><input type="checkbox" role="switch" :checked="motionEnabled" @change="setMotionEnabled(($event.target as HTMLInputElement).checked)"></label>
          <label class="switch-row spring-switch" :class="{disabled:!motionEnabled}"><span><b>完整弹簧动效</b><small>卡片悬停、按钮按压、页面切换、弹窗与图表使用可中断弹簧反馈</small></span><input type="checkbox" role="switch" :checked="springMotionEnabled" :disabled="!motionEnabled" @change="setSpringMotionEnabled(($event.target as HTMLInputElement).checked)"></label>
          <label class="switch-row" :class="{disabled:!motionEnabled}"><span><b>鼠标跟随微光</b><small>仅照亮光标所在卡片，光色会随当前主题自动匹配</small></span><input type="checkbox" role="switch" :checked="spotlightEnabled" :disabled="!motionEnabled" @change="setSpotlightEnabled(($event.target as HTMLInputElement).checked)"></label>
          <label class="switch-row" :class="{disabled:!motionEnabled}"><span><b>成功提示微光</b><small>保存、刷新和导出成功时显示 0.8 秒轻量光点</small></span><input type="checkbox" role="switch" :checked="particlesEnabled" :disabled="!motionEnabled" @change="setParticlesEnabled(($event.target as HTMLInputElement).checked)"></label>
          <div v-if="liquidGlassEnabled" class="glass-quality-control">
            <div><span>液态玻璃渲染质量</span><b>{{glassQuality==='high'?'高质量':'性能模式'}}</b><small>实时同步到主界面与桌面悬浮窗。</small></div>
            <div role="group" aria-label="液态玻璃渲染质量">
              <button :class="{active:glassQuality==='high'}" @click="setGlassQuality('high')"><b>高质量</b><small>完整模糊、折射光纹与景深粒子</small></button>
              <button :class="{active:glassQuality==='performance'}" @click="setGlassQuality('performance')"><b>性能模式</b><small>减少粒子、降低模糊并暂停环境漂移</small></button>
            </div>
          </div>
          <div v-if="liquidGlassEnabled" class="glass-distortion-control" data-liquid-surface>
            <header>
              <span><small>实时光学参数</small><b>自定义扭曲效果</b><em>拖动后立即同步主界面与悬浮窗</em></span>
              <button type="button" @click="resetGlassDistortion">恢复默认</button>
            </header>
            <label v-for="item in distortionControls" :key="item.key" class="distortion-slider-row">
              <span><b>{{item.label}}</b><small>{{item.detail}}</small></span>
              <span class="distortion-slider-value">{{Math.round(glassDistortion[item.key] * 100)}}%</span>
              <input
                type="range"
                :min="item.min"
                :max="item.max"
                :step="item.step"
                :value="glassDistortion[item.key]"
                :aria-label="item.label"
                :aria-valuetext="`${Math.round(glassDistortion[item.key] * 100)}%`"
                @input="updateDistortion(item.key, $event)"
              >
            </label>
            <p>低于默认值会更克制；提高边缘弯折可更清楚地观察壁纸轮廓位移。所有参数只在本机保存。</p>
          </div>
        </section>
        <section class="theme-picker" aria-labelledby="theme-title">
          <div><h3 id="theme-title">主题与空间材质</h3><p>原生黑白继续作为默认主题；“液态玻璃”会启用多层透射、边缘高光、空间环境光与轻量粒子，并同步到悬浮窗。</p></div>
          <div class="theme-grid">
            <button v-for="item in appThemes" :key="item.id" :class="{active:themeId===item.id,liquid:item.material==='liquid'}" :aria-pressed="themeId===item.id" @click="setTheme(item.id)">
              <i :style="{background:item.background}"><em :style="{background:item.material==='liquid'?liquidAccent:item.accent}"></em></i>
              <span><b>{{item.name}}</b><small>{{item.source}}</small></span>
              <strong>{{themeId===item.id?'✓':'选择'}}</strong>
            </button>
          </div>
          <section class="liquid-appearance-panel" :class="{ active: liquidGlassEnabled }" aria-labelledby="liquid-appearance-title">
            <div class="liquid-appearance-preview" :class="{ clear: liquidTone === 'clear' }" :style="liquidBackgroundPreviewStyle">
              <span><i></i><i></i><i></i></span>
              <b>{{ hasLiquidVideoBackground ? `视频壁纸 · ${liquidVideoFileName}` : hasLiquidBackground ? '自定义图片背景' : `内置壁纸 · ${liquidWallpaperName}` }}</b>
              <small>{{ hasLiquidVideoBackground ? '视频帧会持续进入 WebGL 环境纹理，折射与色散会随画面实时变化。' : '图片会进入 WebGL 环境纹理，玻璃卡片会对背景产生真实折射。' }}</small>
            </div>
            <div class="liquid-appearance-controls">
              <header>
                <span><small>液态玻璃专属</small><h4 id="liquid-appearance-title">材质、背景与主题色</h4></span>
                <button type="button" @click="resetLiquidAppearance">恢复默认</button>
              </header>
              <div class="liquid-tone-control">
                <span><b>界面明暗</b><small>深色与纯白都启用液态折射；纯白模式使用高亮中性体积色保持可读性</small></span>
                <div role="group" aria-label="液态主题界面明暗">
                  <button type="button" aria-label="纯白折射模式" :class="{ active: liquidTone === 'clear' }" :aria-pressed="liquidTone === 'clear'" @click="selectLiquidTone('clear')"><i class="tone-clear"></i><span><b>纯白折射</b><small>真实扭曲 · 高对比文字</small></span></button>
                  <button type="button" aria-label="深色黑玻璃" :class="{ active: liquidTone === 'dark' }" :aria-pressed="liquidTone === 'dark'" @click="selectLiquidTone('dark')"><i class="tone-dark"></i><span><b>深色黑</b><small>当前深色玻璃</small></span></button>
                </div>
              </div>
              <label class="liquid-transparency-control" data-liquid-surface>
                <span>
                  <b>全局通透度</b>
                  <small>统一作用于黑色、纯白、侧边栏、全部卡片与悬浮窗；数值越高，背景越通透</small>
                </span>
                <strong>{{ liquidTransparency }}%</strong>
                <input
                  type="range"
                  min="0"
                  max="100"
                  step="1"
                  :value="liquidTransparency"
                  aria-label="液态玻璃全局通透度"
                  :aria-valuetext="`${liquidTransparency}%`"
                  @input="updateLiquidTransparency"
                >
                <em><i>0%</i><i>更厚实</i><i>更通透</i><i>100%</i></em>
              </label>
              <div class="liquid-color-row">
                <label>
                  <span>自定义主题色</span>
                  <input
                    type="color"
                    :value="liquidAccent"
                    aria-label="选择液态玻璃主题色"
                    @input="selectLiquidAccent(($event.target as HTMLInputElement).value)"
                  >
                </label>
                <div role="group" aria-label="液态玻璃主题色预设">
                  <button
                    v-for="preset in liquidColorPresets"
                    :key="preset.value"
                    type="button"
                    :class="{ active: liquidAccent === preset.value }"
                    :title="preset.name"
                    :aria-label="preset.name"
                    :aria-pressed="liquidAccent === preset.value"
                    :style="{ '--preset-color': preset.value }"
                    @click="selectLiquidAccent(preset.value)"
                  ></button>
                </div>
              </div>
              <div class="liquid-wallpaper-picker">
                <span><b>内置折射壁纸</b><small>两张原创黑白壁纸使用大尺度轮廓，让玻璃边缘的位移与扭曲更容易被看见</small></span>
                <div role="group" aria-label="选择液态玻璃内置壁纸">
                  <button
                    v-for="wallpaper in liquidWallpaperPresets"
                    :key="wallpaper.id"
                    type="button"
                    :class="[{ active: liquidWallpaperPreset === wallpaper.id }, `wallpaper-${wallpaper.id}`]"
                    :aria-pressed="liquidWallpaperPreset === wallpaper.id"
                    @click="selectLiquidWallpaper(wallpaper.id)"
                  ><i></i><span><b>{{ wallpaper.name }}</b><small>{{ wallpaper.description }}</small></span></button>
                </div>
              </div>
              <div class="liquid-background-actions">
                <button type="button" class="primary" @click="openLiquidBackgroundPicker">{{ hasLiquidBackground ? '更换背景图片' : '上传背景图片' }}</button>
                <button type="button" :disabled="!hasLiquidBackground" @click="removeLiquidBackground">移除图片</button>
                <input
                  ref="liquidBackgroundInput"
                  class="sr-only"
                  type="file"
                  accept="image/png,image/jpeg,image/webp"
                  @change="handleLiquidBackgroundFile"
                >
              </div>
              <div class="liquid-video-control">
                <span><b>动态视频壁纸</b><small>{{ hasLiquidVideoBackground ? liquidVideoFileName : 'MP4 / WebM / MOV / M4V，最大 1 GB' }}</small></span>
                <div class="liquid-background-actions">
                  <button type="button" class="primary" @click="openLiquidVideoPicker">{{ hasLiquidVideoBackground ? '更换视频' : '选择本地视频' }}</button>
                  <button type="button" :disabled="!hasLiquidVideoBackground" @click="removeLiquidVideoBackground">清除视频</button>
                </div>
              </div>
              <div v-if="liquidVideoError" class="liquid-video-error" role="alert">
                <span>{{ liquidVideoError }}</span>
                <button type="button" @click="removeLiquidVideoBackground">清除视频壁纸</button>
              </div>
              <small class="liquid-appearance-status" role="status" aria-live="polite">{{ liquidAppearanceStatus || '图片与视频都只在本机读取；视频仅保存文件路径，不写入浏览器存储。' }}</small>
            </div>
          </section>
        </section>
      </template>
      <template v-else-if="section === 'charts'">
        <section class="chart-color-settings" aria-labelledby="chart-color-title">
          <header><span>图表外观</span><h2 id="chart-color-title">数据与圆环配色</h2><p>选择一套配色后，主仪表盘、模型仪表盘、圆环进度与悬浮窗会立即同步，不需要重启。</p></header>
          <div class="chart-color-grid" role="radiogroup" aria-label="图表配色方案">
            <button
              v-for="palette in chartColorThemes"
              :key="palette.id"
              type="button"
              role="radio"
              :aria-checked="chartColorThemeId === palette.id"
              :class="{ active: chartColorThemeId === palette.id }"
              @click="setChartColorTheme(palette.id)"
            >
              <span class="palette-ring" :style="{ background: `conic-gradient(${palette.series[0]} 0 36%, ${palette.series[1]} 36% 58%, ${palette.series[2]} 58% 76%, ${palette.series[3]} 76% 88%, ${palette.series[4]} 88% 100%)` }"><i></i></span>
              <span class="palette-copy"><b>{{ palette.name }}</b><small>{{ palette.description }}</small><i><em v-for="color in palette.series" :key="color" :style="{ background: color }"></em></i></span>
              <strong>{{ chartColorThemeId === palette.id ? '已使用' : '选择' }}</strong>
            </button>
          </div>
        </section>
        <ChartPreferences :show-mode="false" />
      </template>
      <template v-else-if="section === 'floating'">
        <header><span>悬浮窗</span><h2>内置桌面监控</h2><p>悬浮窗属于 Token Manager 主程序，共用账户、数据库、主题和实时用量事件。关闭窗口只会隐藏，不会退出主程序。</p></header>
        <section class="floating-control-card" :class="{active:floatingEnabled}">
          <div class="floating-control-symbol" aria-hidden="true"><i></i><i></i><i></i></div>
          <div class="floating-control-copy"><small>内置功能 · 同一 EXE</small><b>{{ floatingEnabled ? '正在桌面常驻' : '当前未启用' }}</b><span>{{ dashboardCount + 3 }} 个分类仪表盘 · API 完成即刷新 · 常规数据 30 秒同步</span></div>
          <span class="floating-live-pill"><i></i>{{ floatingEnabled ? '实时同步' : '已暂停' }}</span>
          <button class="floating-control-action" @click="emit('toggleFloating')">{{ floatingEnabled ? '关闭窗口' : '立即开启' }}</button>
        </section>
        <div class="floating-settings-layout">
          <div class="floating-settings-main">
            <section class="floating-settings-section">
              <div class="floating-section-heading"><div><span>窗口行为</span><h3>v0.8.5 经典双形态</h3></div><small>原版布局</small></div>
              <div class="floating-shape-picker" role="radiogroup" aria-label="选择悬浮窗形态">
                <button :class="{active:floatingMode==='capsule'}" role="radio" :aria-checked="floatingMode==='capsule'" @click="emit('setFloatingMode','capsule')"><i class="shape-capsule" aria-hidden="true"></i><span><b>v0.8.5 折叠胶囊</b><small>品牌控制栏、当前模型、Token 与环形指标</small></span><strong>{{floatingMode==='capsule'?'正在使用':'选择'}}</strong></button>
                <button :class="{active:floatingMode!=='capsule'}" role="radio" :aria-checked="floatingMode!=='capsule'" @click="emit('setFloatingMode','compact')"><i class="shape-compact" aria-hidden="true"></i><span><b>v0.8.5 经典窗口</b><small>单模型、三项摘要与七天趋势图</small></span><strong>{{floatingMode!=='capsule'?'正在使用':'选择'}}</strong></button>
              </div>
              <p class="floating-shape-help">点击后立即切换。折叠胶囊恢复为 360×152，展开窗口为 380×380；两种形态都可按住顶部品牌栏拖动。</p>
              <div class="floating-behavior-grid">
                <button :class="{active:floatingAlwaysOnTop}" @click="emit('toggleFloatingAlwaysOnTop')"><b>窗口置顶</b><small>{{floatingAlwaysOnTop?'始终位于其他窗口上方':'跟随普通窗口层级'}}</small></button>
                <button class="active interaction-locked" type="button" aria-disabled="true"><b>永久保持交互</b><small>始终可点击、拖动、缩放和再次展开，不再自动穿透桌面</small></button>
              </div>
            </section>
            <section class="floating-settings-section">
              <div class="floating-section-heading"><div><span>显示内容</span><h3>图表标签与数据模块</h3></div><small>{{ selectedFloatingCount }} 项已启用</small></div>
              <div class="floating-section-subtitle">图表模块</div>
              <div class="floating-list compact">
                <label v-for="id in floatingChartItems" :key="id" :class="{dragging:draggedFloatingItem===id}" @dragover.prevent @drop.prevent="dropFloatingItem(id)"><i class="floating-drag" draggable="true" title="按住拖动调整顺序" :aria-label="`拖动 ${floatingItem(id)?.title} 调整顺序`" @dragstart.stop="draggedFloatingItem=id" @dragend="draggedFloatingItem=null">⠿</i><span><b>{{ floatingItem(id)?.title }}</b><small>{{ floatingItem(id)?.description }}</small></span><input class="floating-check" type="checkbox" :checked="floatingSelected.includes(id)" @change="emit('toggleFloatingItem',id,($event.target as HTMLInputElement).checked)"></label>
              </div>
              <div class="floating-section-subtitle">摘要模块</div>
              <div class="floating-list compact">
                <label v-for="id in floatingSummaryItems" :key="id" :class="{dragging:draggedFloatingItem===id}" @dragover.prevent @drop.prevent="dropFloatingItem(id)"><i class="floating-drag" draggable="true" title="按住拖动调整顺序" :aria-label="`拖动 ${floatingItem(id)?.title} 调整顺序`" @dragstart.stop="draggedFloatingItem=id" @dragend="draggedFloatingItem=null">⠿</i><span><b>{{ floatingItem(id)?.title }}</b><small>{{ floatingItem(id)?.description }}</small></span><input class="floating-check" type="checkbox" :checked="floatingSelected.includes(id)" @change="emit('toggleFloatingItem',id,($event.target as HTMLInputElement).checked)"></label>
              </div>
            </section>
            <section class="mini-settings-card">
              <label class="switch-row"><span><b>v0.8.5 折叠显示</b><small>切换为 360×152 经典折叠窗，只保留当前模型、核心 Token 与环形指标</small></span><input type="checkbox" role="switch" :checked="floatingMode==='capsule'" @change="emit('setMiniMode',($event.target as HTMLInputElement).checked)"></label>
              <div class="mini-module-grid" :class="{disabled:!miniMode}"><label><span>核心模块 1</span><select :value="miniModules[0]" :disabled="!miniMode" @change="emit('setMiniModule',0,($event.target as HTMLSelectElement).value)"><option v-for="item in floatingItems" :key="item.id" :value="item.id">{{item.title}}</option></select></label><label><span>核心模块 2</span><select :value="miniModules[1]" :disabled="!miniMode" @change="emit('setMiniModule',1,($event.target as HTMLSelectElement).value)"><option v-for="item in floatingItems" :key="item.id" :value="item.id">{{item.title}}</option></select></label></div>
            </section>
            <section class="floating-settings-section">
              <div class="floating-section-heading"><div><span>主题外观</span><h3>跟随全局配色</h3></div><small>纯色材质</small></div>
              <div class="floating-diagnostic-row"><span><b>与 Token Manager 当前主题一致</b><small>悬浮窗保留主题颜色、图表配色和动画，但不启用液态玻璃、桌面采样或背景折射。</small></span><button @click="section='general'">前往主题设置</button></div>
            </section>
            <section class="floating-settings-section">
              <div class="floating-section-heading"><div><span>窗口诊断</span><h3>原生裁切与交互状态</h3></div><small>无需 GPU 渲染</small></div>
              <div class="floating-render-diagnostic"><b>Windows 原生窗口</b><span>永久交互</span><small>圆角由单一原生区域精确裁切；窗口始终可点击和拖动，不再启用自动或手动穿透。</small></div>
            </section>
          </div>
          <aside class="floating-preview-card">
            <div class="floating-preview-heading"><span>真实组件预览</span><b>{{ floatingMode==='capsule'?'v0.8.5 折叠':'v0.8.5 展开' }}</b></div>
            <div class="floating-preview-window v3"><FloatingWindowV3 :mode="floatingMode" :rows="floatingPreviewRows" :selected-key="floatingSelectedKey" :expanded-keys="floatingPreviewRows.slice(0,1).map(row=>row.key)" :metric="floatingMetric" :available-metrics="['tokens','calls','cached','cost']" :syncing="false" next-refresh="30 秒" refreshed-text="刚刚" :proxy-label="proxyEndpointCount?'运行中':'待启动'" :proxy-active="proxyEndpointCount>0" :cc-label="ccSwitchRouting?'已接管':ccSwitchRunning?'运行中':'待机'" :cc-active="ccSwitchRouting" :always-on-top="floatingAlwaysOnTop" :interaction="floatingInteraction" :render-status="floatingRenderStatus" :logo-url="tokenManagerLogo" :codex-logo-url="codexLogo" preview /></div>
            <p>这里与桌面悬浮窗复用同一个 Vue 组件，不再维护另一套容易错位的预览。</p>
            <small>{{ccSwitchDetail}}</small>
          </aside>
        </div>
      </template>
      <template v-else-if="section === 'codex'">
        <header><span>Codex</span><h2>客户端额度与估算回退</h2><p>软件会优先读取 Codex 本地 rate_limits 百分比和重置时间。只有当前客户端没有下发某个窗口时，才使用下面的个人 Token 上限估算。</p></header>
        <div class="fields"><label>5 小时上限<input v-model.number="budgetFive" type="number" min="1"></label><label>7 天上限<input v-model.number="budgetSeven" type="number" min="1"></label></div><button class="primary" @click="emit('saveBudget',budgetFive,budgetSeven)">保存预算</button><small>{{budgetStatus}}</small>
      </template>
      <template v-else-if="section === 'cloud'">
        <header><span>可选云账户</span><h2>登录与迁移链接</h2><p>登录只用于跨电脑迁移。迁移包先在本机用你的迁移密码加密，后端不会收到 API Key 明文、日志正文或迁移密码；你也可以完全不登录，继续使用本地迁移。</p></header>
        <section v-if="cloudSession" class="cloud-signed-in">
          <div><i aria-hidden="true">✓</i><span><b>{{cloudSession.email}}</b><small>已连接 {{cloudSession.base_url}} · 登录有效期至 {{new Date(cloudSession.expires_at).toLocaleDateString('zh-CN')}}</small></span></div>
          <button @click="emit('cloudLogout')">退出登录</button>
        </section>
        <section v-else class="cloud-login">
          <div class="login-mode"><button :class="{active:cloudMode==='password'}" @click="cloudMode='password'">邮箱＋密码</button><button :class="{active:cloudMode==='code'}" @click="cloudMode='code'">邮箱＋验证码</button></div>
          <label>后端地址<input v-model.trim="cloudBaseUrl" type="url" placeholder="https://api.example.com" @change="rememberCloudBase"></label>
          <label>邮箱<input v-model.trim="cloudEmail" type="email" autocomplete="email" placeholder="name@example.com"></label>
          <template v-if="cloudMode==='password'">
            <label>密码<input v-model="cloudPassword" type="password" autocomplete="current-password" minlength="8" placeholder="至少 8 个字符"></label>
            <div class="actions"><button class="primary" @click="emit('cloudPasswordLogin',cloudBaseUrl,cloudEmail,cloudPassword)">登录</button><button @click="emit('cloudPasswordRegister',cloudBaseUrl,cloudEmail,cloudPassword,'')">首次使用，注册账户</button></div>
            <small>首次使用可直接用邮箱和至少 8 位密码注册；已注册用户直接登录。</small>
          </template>
          <template v-else>
            <label>验证码<input v-model.trim="cloudCode" inputmode="numeric" autocomplete="one-time-code" maxlength="6" placeholder="6 位验证码"></label>
            <div class="actions"><button @click="emit('cloudRequestCode',cloudBaseUrl,cloudEmail,'login')">发送验证码</button><button class="primary" @click="emit('cloudCodeLogin',cloudBaseUrl,cloudEmail,cloudCode)">验证码登录</button></div>
          </template>
        </section>
        <section class="cloud-transfer" :class="{disabled:!cloudSession}">
          <div><h3>创建加密迁移链接</h3><p>默认 24 小时过期、使用一次后失效。密文仍需迁移密码才能解开。</p></div>
          <div class="cloud-transfer-grid"><label>迁移密码<input v-model="cloudTransferPassword" type="password" minlength="8" placeholder="至少 8 个字符"></label><label>有效期<select v-model.number="cloudTtlHours"><option :value="1">1 小时</option><option :value="24">24 小时</option><option :value="72">3 天</option><option :value="168">7 天</option></select></label></div>
          <label class="one-time"><input v-model="cloudOneTime" type="checkbox">仅允许下载一次</label>
          <button class="primary" :disabled="!cloudSession" @click="emit('cloudCreateTransfer',cloudTransferPassword,cloudTtlHours,cloudOneTime)">创建迁移链接</button>
          <div v-if="cloudTransferLink" class="created-link"><input :value="cloudTransferLink" readonly aria-label="已创建的迁移链接"><button @click="copyCloudLink">复制链接</button></div>
        </section>
        <section class="cloud-import"><div><h3>从迁移链接恢复</h3><p>无需登录即可导入；成功后远端仅一次链接会立即失效。</p></div><label>迁移链接<input v-model.trim="cloudImportLink" type="url" placeholder="https://…/v1/transfer/…"></label><label>迁移密码<input v-model="cloudTransferPassword" type="password" minlength="8"></label><button @click="emit('cloudImportTransfer',cloudImportLink,cloudTransferPassword)">读取并恢复</button></section>
        <small role="status" aria-live="polite">{{cloudStatus || '云功能完全可选；本地迁移仍可独立使用。'}}</small>
      </template>
      <template v-else-if="section === 'proxy'">
        <header><span>API 实时监控</span><h2>本地代理</h2><p>普通用户使用一键开启；高级用户可指定上游地址和监听端口。</p></header>
        <button class="primary" @click="emit('startAllProxies')">一键开启全部账户监控</button><div class="fields"><label>上游地址<input v-model.trim="upstream"></label><label>监听端口<input v-model.number="port" type="number" min="1024" max="65535"></label></div><button @click="emit('startProxy',upstream,port)">启动单账户代理</button><small>{{proxyStatus}}</small>
      </template>
      <template v-else-if="section === 'data'">
        <header><span>本地数据</span><h2>加密迁移</h2><p>迁移包离线生成，导入新电脑后密钥会重新使用该电脑的 Windows DPAPI 加密。</p></header>
        <label class="password">迁移密码<input v-model="password" type="password" minlength="8" placeholder="至少 8 个字符"></label><div class="actions"><button class="primary" @click="emit('exportBackup',password)">导出全部配置</button><button @click="emit('importBackup',password)">导入迁移包</button></div><small>{{backupStatus || '请妥善保管迁移密码。'}}</small>
      </template>
      <template v-else>
        <header><span>新手教程</span><h2>四步开始监控</h2><p>所有统计都在本机完成，不上传密钥或会话正文。</p></header>
        <ol><li><b>添加 API 账户</b><span>从平台官网创建密钥，再由 Token Manager 使用 DPAPI 加密保存。</span><button @click="emit('openAccounts')">前往账户与模型</button></li><li><b>开启实时监控</b><span>为账户创建 127.0.0.1 本机代理地址。</span><button @click="emit('startAllProxies')">一键开启</button></li><li><b>修改调用工具 Base URL</b><span>将 SDK 或编辑器的地址改为软件显示的本机代理地址。</span></li><li><b>查看独立仪表盘</b><span>图表在上、Token 与余额在下，不会混入其他模型数据。</span></li></ol>
        <div class="account-state">当前已保存 {{ accountCount }} 个加密账户</div>
      </template>
    </article>
  </section>
</template>

<style scoped>
.settings-center{display:grid;grid-template-columns:250px minmax(0,1fr);gap:16px;align-items:start}.settings-nav,.settings-detail{border:1px solid var(--tm-line,#ededf0);border-radius:20px;background:var(--tm-bg,#fff)}.settings-nav{display:grid;gap:5px;padding:16px;position:sticky;top:18px}.settings-nav>div{padding:9px 8px 15px}.settings-nav span,.settings-detail header>span{color:var(--tm-muted,#6e6e73);font-size:10px}.settings-nav h2{margin:5px 0 0;font-size:20px}.settings-nav>button{display:flex;align-items:center;justify-content:space-between;padding:12px;border:0;border-radius:12px;background:transparent;color:var(--tm-ink,#1d1d1f);text-align:left}.settings-nav>button:hover{background:var(--tm-surface,#f5f5f7)}.settings-nav>button.active{background:var(--tm-ink,#111);color:var(--tm-on-ink,#fff)}.settings-nav button span{display:grid;gap:4px}.settings-nav button b{font-size:12px}.settings-nav button small{font-size:9px;opacity:.66}.settings-nav button i{font-style:normal;font-size:20px}.settings-detail{min-height:620px;padding:30px}.settings-detail header{margin-bottom:24px}.settings-detail h2{margin:6px 0 7px;font-size:24px}.settings-detail header p{max-width:720px;margin:0;color:var(--tm-muted,#6e6e73);font-size:12px;line-height:1.7}.global-mode{display:grid;grid-template-columns:1fr 1fr;gap:12px}.global-mode button{display:grid;gap:7px;padding:20px;border:1px solid var(--tm-line,#e5e5ea);border-radius:16px;background:var(--tm-bg,#fff);color:var(--tm-ink,#1d1d1f);text-align:left}.global-mode button.active{border-color:var(--tm-ink,#111);background:var(--tm-ink,#111);color:var(--tm-on-ink,#fff)}.global-mode span{font-size:10px;opacity:.7}.monochrome-note,.mini-box{display:grid;gap:7px;margin-top:16px;padding:17px;border-radius:14px;background:var(--tm-surface,#f2f2f7)}.monochrome-note span{color:var(--tm-muted,#6e6e73);font-size:11px}.theme-picker{margin-top:22px}.theme-picker h3{margin:0;font-size:15px}.theme-picker p{margin:5px 0 13px;color:var(--tm-muted);font-size:10px}.theme-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px}.theme-grid button{display:grid;grid-template-columns:42px 1fr auto;align-items:center;gap:10px;padding:11px;border:1px solid var(--tm-line);border-radius:13px;background:var(--tm-bg);color:var(--tm-ink);text-align:left}.theme-grid button.active{border-color:var(--tm-ink);background:var(--tm-surface)}.theme-grid i{display:grid;place-items:center;width:40px;height:30px;border:1px solid var(--tm-line);border-radius:9px}.theme-grid em{width:18px;height:18px;border-radius:6px}.theme-grid span{display:grid;gap:3px}.theme-grid small,.theme-grid strong{font-size:9px;opacity:.65}.primary,.settings-detail>button,.actions button,.settings-detail li button{padding:10px 14px;border:1px solid var(--tm-ink,#111);border-radius:11px;background:var(--tm-bg,#fff);color:var(--tm-ink,#111)}.primary{background:var(--tm-ink,#111)!important;color:var(--tm-on-ink,#fff)!important}.inline-control{display:flex;align-items:center;gap:7px;margin:18px 0;padding:10px;border-radius:12px;background:var(--tm-surface,#f2f2f7)}.inline-control span{margin-right:auto;font-size:11px}.inline-control button{border:0;border-radius:9px;background:transparent;padding:7px 10px}.inline-control button.active{background:var(--tm-ink,#111);color:var(--tm-on-ink,#fff)}.floating-list{display:grid;gap:1px;overflow:hidden;border-radius:14px;background:var(--tm-line,#e5e5ea)}.floating-list label{display:flex;align-items:center;gap:11px;padding:13px;background:var(--tm-bg,#fff);transition:opacity .18s ease,transform .28s cubic-bezier(.22,1,.36,1)}.floating-list label.dragging{opacity:.48;transform:scale(.985)}.floating-drag{display:grid;place-items:center;width:30px;height:30px;border-radius:9px;background:var(--tm-surface,#f2f2f7);color:var(--tm-muted);cursor:grab;font-size:15px;font-style:normal}.floating-drag:active{cursor:grabbing}.floating-list input{accent-color:var(--tm-ink,#111)}.floating-list span{display:grid;gap:3px;flex:1}.floating-list b{font-size:11px}.floating-list small{color:var(--tm-muted,#6e6e73);font-size:9px}.mini-box{grid-template-columns:1fr 1fr}.mini-box label{grid-column:1/-1}.mini-box select,.fields input,.password input{padding:11px;border:0;border-radius:10px;background:var(--tm-surface,#f2f2f7);color:var(--tm-ink)}.fields{display:grid;grid-template-columns:1fr 1fr;gap:12px;margin:18px 0}.fields label,.password{display:grid;gap:7px;color:var(--tm-muted,#6e6e73);font-size:10px}.actions{display:flex;gap:8px;margin:14px 0}.settings-detail>small{display:block;margin-top:12px;color:var(--tm-muted,#6e6e73)}.settings-detail ol{display:grid;gap:10px;padding:0;counter-reset:steps}.settings-detail li{display:grid;grid-template-columns:1fr auto;gap:6px 18px;padding:17px;border-radius:14px;background:var(--tm-surface,#f5f5f7);list-style:none}.settings-detail li span{grid-column:1;color:var(--tm-muted,#6e6e73);font-size:11px}.settings-detail li button{grid-column:2;grid-row:1/3}.account-state{margin-top:15px;color:var(--tm-muted,#6e6e73);font-size:11px}@media(max-width:900px){.settings-center{grid-template-columns:1fr}.settings-nav{position:static;grid-template-columns:repeat(2,1fr)}.settings-nav>div{grid-column:1/-1}.global-mode,.fields,.theme-grid{grid-template-columns:1fr}}@media(prefers-reduced-motion:reduce){.floating-list label{transition:none}}
.liquid-glass-quick-control{border-color:color-mix(in srgb,var(--tm-accent) 42%,var(--tm-line))!important;background:linear-gradient(135deg,color-mix(in srgb,var(--tm-accent) 10%,var(--tm-bg)),color-mix(in srgb,var(--tm-glow) 7%,var(--tm-bg)))!important}.liquid-glass-quick-control::after{background:linear-gradient(112deg,rgba(255,255,255,.22),transparent 30% 72%,color-mix(in srgb,var(--tm-accent) 10%,transparent))}.liquid-glass-quick-control:hover{border-color:color-mix(in srgb,var(--tm-accent) 72%,var(--tm-line))!important;box-shadow:inset 0 1px 0 rgba(255,255,255,.3),0 14px 34px color-mix(in srgb,var(--tm-glow) 14%,transparent)}.liquid-glass-quick-control.active{border-color:var(--tm-accent)!important;background:linear-gradient(135deg,color-mix(in srgb,var(--tm-accent) 22%,var(--tm-bg)),color-mix(in srgb,var(--tm-glow) 15%,var(--tm-bg)))!important;box-shadow:inset 0 1px 0 rgba(255,255,255,.3),0 0 0 3px color-mix(in srgb,var(--tm-accent) 10%,transparent),0 16px 38px color-mix(in srgb,var(--tm-glow) 18%,transparent)}.liquid-glass-orb{background:radial-gradient(circle at 30% 22%,#fff 0 5%,rgba(255,255,255,.55) 8%,transparent 24%),linear-gradient(145deg,color-mix(in srgb,var(--tm-accent) 76%,#fff),var(--tm-accent) 56%,color-mix(in srgb,var(--tm-accent) 35%,#081120));box-shadow:inset 0 1px 5px rgba(255,255,255,.48),inset 0 -5px 10px rgba(12,20,61,.28),0 8px 20px color-mix(in srgb,var(--tm-glow) 24%,transparent)}.liquid-glass-quick-copy small{color:color-mix(in srgb,var(--tm-accent) 76%,var(--tm-ink))}.liquid-glass-quick-control.active .liquid-glass-state>i{background:var(--tm-accent)}.motion-settings>.glass-quality-control{background:linear-gradient(145deg,color-mix(in srgb,var(--tm-accent) 9%,var(--tm-bg)),color-mix(in srgb,var(--tm-glow) 6%,var(--tm-bg)))}
</style>
<style scoped>
/* 小窗模式按设置内容的真实宽度重排，避免主侧栏占位后发生挤压。 */
.settings-detail {
  container-name: settings-detail;
  container-type: inline-size;
}

.settings-detail > header {
  display: block;
}

.settings-detail > header > span {
  display: block;
}

.floating-shape-picker .shape-capsule {
  width: 58px;
  height: 32px;
  border-radius: 13px;
}

.floating-preview-window.v3 {
  max-width: 380px;
  margin-inline: auto;
}

@container settings-detail (max-width: 820px) {
  .floating-settings-layout {
    grid-template-columns: minmax(0, 1fr) !important;
  }

  .floating-preview-card {
    position: static !important;
    order: -1;
  }

  .floating-preview-window.v3 {
    width: min(100%, 380px);
  }
}

@container settings-detail (max-width: 620px) {
  .floating-control-card {
    grid-template-columns: auto minmax(0, 1fr);
    gap: 11px 13px;
    padding: 15px;
  }

  .floating-live-pill {
    grid-column: 2;
    justify-self: start;
  }

  .floating-control-action {
    width: 100%;
    grid-column: 1 / -1;
  }

  .floating-shape-picker,
  .floating-behavior-grid,
  .mini-module-grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .floating-shape-picker > button {
    grid-template-columns: 58px minmax(0, 1fr) auto;
  }

  .floating-section-heading {
    align-items: flex-start;
  }

  .floating-diagnostic-row {
    align-items: stretch;
    flex-direction: column;
  }

  .floating-diagnostic-row button {
    align-self: flex-start;
  }
}

@container settings-detail (max-width: 440px) {
  .floating-control-card {
    grid-template-columns: minmax(0, 1fr);
  }

  .floating-control-symbol,
  .floating-live-pill,
  .floating-control-action {
    grid-column: 1;
  }

  .floating-shape-picker > button {
    grid-template-columns: 48px minmax(0, 1fr);
  }

  .floating-shape-picker > button > i {
    width: 48px;
  }

  .floating-shape-picker strong {
    grid-column: 2;
    justify-self: start;
  }

  .floating-list.compact label {
    gap: 8px;
    padding-inline: 10px;
  }

  .floating-list.compact small {
    white-space: normal;
  }
}

@media (max-width: 1180px) {
  .settings-center {
    grid-template-columns: minmax(0, 1fr);
  }

  .settings-nav {
    position: static;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .settings-nav > div {
    grid-column: 1 / -1;
  }

  .settings-detail {
    min-height: 0;
    padding: 22px;
  }
}

@media (max-width: 780px) {
  .settings-nav {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .settings-detail {
    padding: 16px;
  }
}

@media (max-width: 520px) {
  .settings-nav {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
<style scoped>
/* 预览与原生窗口共用 v0.8.5 的逻辑尺寸和圆角。 */
.floating-preview-window.v3 { width: 100%; height: 380px !important; }
.floating-preview-window.v3:has(.floating-classic.collapsed) { height: 152px !important; border-radius: 28px; }
</style>
<style scoped>
.floating-section-subtitle{padding:8px 14px;color:var(--tm-muted);font-size:8px;font-weight:700;letter-spacing:.08em}.floating-behavior-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px;padding:0 12px 13px}.floating-behavior-grid button{display:grid;gap:4px;padding:11px;border:1px solid var(--tm-line);border-radius:12px;background:var(--tm-surface);color:var(--tm-ink);text-align:left}.floating-behavior-grid button.active{border-color:var(--tm-ink);background:var(--tm-ink);color:var(--tm-on-ink)}.floating-behavior-grid button.interaction-locked{cursor:default;box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--tm-on-ink) 12%,transparent)}.floating-behavior-grid b{font-size:10px}.floating-behavior-grid small{font-size:8px;line-height:1.45;opacity:.72}.floating-diagnostic-row{display:flex;align-items:center;justify-content:space-between;gap:14px;padding:0 15px 15px}.floating-diagnostic-row>span{display:grid;gap:4px}.floating-diagnostic-row b{font-size:11px}.floating-diagnostic-row small{color:var(--tm-muted);font-size:8px}.floating-diagnostic-row button{flex:0 0 auto;padding:8px 10px;border:0;border-radius:9px;background:var(--tm-ink);color:var(--tm-on-ink);font:inherit;font-size:8px}.floating-render-diagnostic{display:grid;grid-template-columns:1fr auto;gap:4px 10px;padding:0 15px 15px}.floating-render-diagnostic b{font-size:11px}.floating-render-diagnostic span{font-size:9px;font-variant-numeric:tabular-nums}.floating-render-diagnostic small{grid-column:1/-1;color:var(--tm-muted);font-size:8px;line-height:1.5}.floating-preview-window.v3{height:360px;padding:0;border:0;background:transparent}.status-active{color:#34c759!important}.status-degraded{color:#ff9500!important}.status-stopped{color:var(--tm-muted)!important}@media(max-width:760px){.floating-behavior-grid{grid-template-columns:1fr}}
</style>
<style scoped>
.theme-picker{padding-top:2px}.theme-grid{grid-template-columns:repeat(auto-fit,minmax(168px,1fr));gap:8px}.theme-grid button{grid-template-columns:34px 1fr auto;min-height:58px;padding:10px 11px;border-radius:12px}.theme-grid i{width:32px;height:32px;border-radius:9px}.theme-grid em{width:14px;height:20px;border-radius:5px}.theme-grid strong{font-weight:600}.theme-grid button.active{box-shadow:inset 0 0 0 1px var(--tm-ink)}
.motion-settings{display:grid;gap:1px;margin-top:16px;overflow:hidden;border-radius:14px;background:var(--tm-line)}.motion-settings>div,.switch-row{background:var(--tm-bg)}.motion-settings>div{padding:16px}.motion-settings h3{margin:0;font-size:14px}.motion-settings p{margin:5px 0 0;color:var(--tm-muted);font-size:10px}.switch-row{display:flex;align-items:center;justify-content:space-between;gap:18px;padding:13px 16px}.switch-row>span{display:grid;gap:4px}.switch-row b{font-size:11px}.switch-row small{color:var(--tm-muted);font-size:9px}.switch-row.disabled{opacity:.48}.switch-row input{position:relative;width:42px;height:24px;flex:0 0 auto;margin:0;appearance:none;border-radius:99px;background:var(--tm-line);cursor:pointer;transition:background-color .2s cubic-bezier(.22,1,.36,1)}.switch-row input::after{content:"";position:absolute;top:3px;left:3px;width:18px;height:18px;border-radius:50%;background:var(--tm-bg);box-shadow:0 1px 3px rgba(0,0,0,.18);transition:transform .24s cubic-bezier(.22,1,.36,1)}.switch-row input:checked{background:var(--tm-ink)}.switch-row input:checked::after{transform:translateX(18px)}.switch-row input:active::after{transform:scale(.92)}.switch-row input:checked:active::after{transform:translateX(18px) scale(.92)}
:global(.motion-off) .switch-row input,:global(.motion-off) .switch-row input::after{transition:none}
.cloud-login,.cloud-transfer,.cloud-import{display:grid;gap:12px;margin-top:14px;padding:18px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-bg)}
.cloud-login label,.cloud-transfer label,.cloud-import label{display:grid;gap:6px;color:var(--tm-muted);font-size:10px}
.cloud-login input,.cloud-transfer input,.cloud-transfer select,.cloud-import input,.created-link input,.register-row input{min-width:0;padding:11px 12px;border:0;border-radius:10px;background:var(--tm-surface);color:var(--tm-ink);outline:none}
.cloud-login input:focus,.cloud-transfer input:focus,.cloud-import input:focus{box-shadow:inset 0 0 0 1px var(--tm-ink)}
.login-mode{display:grid;grid-template-columns:1fr 1fr;gap:5px;padding:4px;border-radius:12px;background:var(--tm-surface)}
.login-mode button{padding:9px;border:0;border-radius:9px;background:transparent;color:var(--tm-muted)}.login-mode button.active{background:var(--tm-bg);color:var(--tm-ink);box-shadow:0 1px 4px rgba(0,0,0,.08)}
.register-row,.created-link{display:grid;grid-template-columns:1fr auto;gap:8px}.cloud-signed-in{display:flex;align-items:center;justify-content:space-between;gap:14px;padding:16px;border-radius:16px;background:var(--tm-surface)}
.cloud-signed-in>div{display:flex;align-items:center;gap:11px}.cloud-signed-in i{display:grid;place-items:center;width:30px;height:30px;border-radius:50%;background:#34c759;color:#fff;font-style:normal}.cloud-signed-in span{display:grid;gap:4px}.cloud-signed-in small{color:var(--tm-muted);font-size:9px}
.cloud-transfer h3,.cloud-import h3{margin:0;font-size:14px}.cloud-transfer p,.cloud-import p{margin:4px 0 0;color:var(--tm-muted);font-size:10px}.cloud-transfer-grid{display:grid;grid-template-columns:1fr 150px;gap:10px}.cloud-transfer .one-time{display:flex;align-items:center;gap:8px;color:var(--tm-ink)}.cloud-transfer.disabled{opacity:.58}.cloud-transfer button:disabled{cursor:not-allowed;opacity:.48}
.floating-control-card{display:grid;grid-template-columns:auto minmax(0,1fr) auto auto;align-items:center;gap:15px;margin-bottom:20px;padding:17px 18px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-surface);transition:background-color .2s ease,color .2s ease,border-color .2s ease}.floating-control-card.active{border-color:var(--tm-ink);background:var(--tm-ink);color:var(--tm-on-ink)}
.floating-control-symbol{display:flex;align-items:flex-end;gap:3px;width:42px;height:42px;padding:10px;border-radius:13px;background:var(--tm-bg)}.floating-control-symbol i{display:block;width:5px;border-radius:99px;background:var(--tm-ink)}.floating-control-symbol i:nth-child(1){height:12px}.floating-control-symbol i:nth-child(2){height:20px}.floating-control-symbol i:nth-child(3){height:16px}.floating-control-copy{display:grid;gap:3px;min-width:0}.floating-control-copy small{font-size:9px;opacity:.62}.floating-control-copy b{font-size:15px}.floating-control-copy span{font-size:9px;opacity:.64}.floating-live-pill{display:flex;align-items:center;gap:6px;padding:7px 9px;border:1px solid color-mix(in srgb,currentColor 18%,transparent);border-radius:99px;font-size:9px;white-space:nowrap}.floating-live-pill i{width:6px;height:6px;border-radius:50%;background:currentColor}.floating-control-card.active .floating-live-pill i{box-shadow:0 0 0 4px color-mix(in srgb,currentColor 13%,transparent)}.floating-control-action{min-width:86px;padding:10px 13px;border:0;border-radius:11px;background:var(--tm-ink);color:var(--tm-on-ink);font:inherit;font-size:10px;font-weight:700}.floating-control-card.active .floating-control-action{background:var(--tm-bg);color:var(--tm-ink)}
.floating-settings-layout{display:grid;grid-template-columns:minmax(0,1fr) 250px;gap:18px;align-items:start}.floating-settings-main{display:grid;gap:15px}.floating-settings-section,.mini-settings-card{overflow:hidden;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-bg)}.floating-section-heading{display:flex;align-items:center;justify-content:space-between;gap:16px;padding:15px 16px 13px}.floating-section-heading>div{display:grid;gap:3px}.floating-section-heading span{color:var(--tm-muted);font-size:9px}.floating-section-heading h3{margin:0;font-size:13px}.floating-section-heading>small{padding:5px 8px;border-radius:99px;background:var(--tm-surface);color:var(--tm-muted);font-size:8px;white-space:nowrap}.floating-settings-section .inline-control{margin:0 12px 10px}.floating-list.compact{border-top:1px solid var(--tm-line);border-radius:0;background:transparent}.floating-list.compact label{min-height:58px;padding:11px 13px}.floating-list.compact label+label{border-top:1px solid var(--tm-line)}.floating-list.compact .floating-check{position:relative;width:39px;height:23px;flex:0 0 auto;margin:0;appearance:none;border-radius:99px;background:var(--tm-line);cursor:pointer;transition:background-color .2s cubic-bezier(.22,1,.36,1)}.floating-list.compact .floating-check::after{content:"";position:absolute;top:3px;left:3px;width:17px;height:17px;border-radius:50%;background:var(--tm-bg);box-shadow:0 1px 3px rgba(0,0,0,.18);transition:transform .24s cubic-bezier(.22,1,.36,1)}.floating-list.compact .floating-check:checked{background:var(--tm-ink)}.floating-list.compact .floating-check:checked::after{transform:translateX(16px)}
.floating-shape-picker{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px;padding:0 12px 10px}.floating-shape-picker>button{display:grid;grid-template-columns:auto minmax(0,1fr) auto;align-items:center;gap:10px;min-height:66px;padding:10px 11px;border:1px solid var(--tm-line);border-radius:14px;background:var(--tm-surface);color:var(--tm-ink);font:inherit;text-align:left;transition:transform .2s cubic-bezier(.22,1,.36,1),border-color .2s ease,background-color .2s ease}.floating-shape-picker>button:hover{transform:translateY(-1px);background:color-mix(in srgb,var(--tm-surface) 72%,var(--tm-bg))}.floating-shape-picker>button.active{border-color:var(--tm-ink);background:var(--tm-bg);box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--tm-ink) 10%,transparent)}.floating-shape-picker>button>i{display:block;box-sizing:border-box;border:2px solid currentColor;background:color-mix(in srgb,currentColor 8%,transparent);opacity:.82}.floating-shape-picker .shape-capsule{width:66px;height:24px;border-radius:999px}.floating-shape-picker .shape-compact{width:54px;height:38px;border-radius:16px}.floating-shape-picker>button>span{display:grid;gap:3px;min-width:0}.floating-shape-picker b{font-size:10px}.floating-shape-picker small{overflow:hidden;color:var(--tm-muted);font-size:7px;line-height:1.4;text-overflow:ellipsis;white-space:nowrap}.floating-shape-picker strong{padding:4px 6px;border-radius:99px;background:var(--tm-surface);color:var(--tm-muted);font-size:7px;white-space:nowrap}.floating-shape-picker>button.active strong{background:var(--tm-ink);color:var(--tm-on-ink)}.floating-shape-help{margin:0;padding:0 14px 13px;color:var(--tm-muted);font-size:8px;line-height:1.55}.floating-shape-help::before{content:"形态切换";margin-right:7px;color:var(--tm-ink);font-weight:700}@media(max-width:900px){.floating-shape-picker{grid-template-columns:1fr}.floating-shape-picker>button>i{justify-self:center}}
.mini-settings-card>.switch-row{border-bottom:1px solid var(--tm-line)}.mini-module-grid{display:grid;grid-template-columns:1fr 1fr;gap:10px;padding:14px 16px 16px;transition:opacity .2s ease}.mini-module-grid.disabled{opacity:.45}.mini-module-grid label{display:grid;gap:6px;color:var(--tm-muted);font-size:9px}.mini-module-grid select{width:100%;min-width:0;padding:10px;border:0;border-radius:10px;background:var(--tm-surface);color:var(--tm-ink);font:inherit;font-size:10px;outline:none}
.floating-preview-card{position:sticky;top:18px;padding:14px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-surface)}.floating-preview-heading{display:flex;align-items:center;justify-content:space-between;margin-bottom:11px}.floating-preview-heading span{font-size:10px;font-weight:700}.floating-preview-heading b{color:var(--tm-muted);font-size:8px}.floating-preview-window{overflow:hidden;padding:10px;border:1px solid var(--tm-line);border-radius:18px;background:var(--tm-bg)}.preview-title{display:flex;align-items:center;gap:8px;padding:2px 2px 9px;border-bottom:1px solid var(--tm-line)}.preview-title>i{width:24px;height:24px;border-radius:8px;background:var(--tm-ink)}.preview-title>span{display:grid;gap:2px;flex:1}.preview-title b{font-size:9px}.preview-title small{color:var(--tm-muted);font-size:7px}.preview-title em{color:var(--tm-muted);font-style:normal;font-size:9px;letter-spacing:1px}.preview-dashboard-row{display:grid;grid-template-columns:25px minmax(0,1fr) auto auto;align-items:center;gap:7px;padding:9px 2px;border-bottom:1px solid var(--tm-line)}.preview-dashboard-row>i{width:25px;height:25px;border:5px solid var(--tm-line);border-top-color:var(--tm-ink);border-radius:50%}.preview-dashboard-row>span{display:grid;gap:2px;min-width:0}.preview-dashboard-row b{font-size:8px}.preview-dashboard-row small{color:var(--tm-muted);font-size:6px}.preview-dashboard-row strong{font-size:7px}.preview-dashboard-row em{color:var(--tm-muted);font-style:normal;font-size:11px}.preview-dashboard-row.muted{opacity:.58}.preview-status-row{display:flex;flex-wrap:wrap;gap:5px;padding-top:9px}.preview-status-row span{display:flex;align-items:center;gap:4px;padding:5px 6px;border-radius:99px;background:var(--tm-surface);color:var(--tm-muted);font-size:6px}.preview-status-row span.active{background:var(--tm-ink);color:var(--tm-on-ink)}.preview-status-row i{width:4px;height:4px;border-radius:50%;background:currentColor}.floating-preview-card>p{margin:10px 2px 0;color:var(--tm-muted);font-size:8px;line-height:1.55}.floating-preview-card>small{display:block;margin:7px 2px 0;color:var(--tm-muted);font-size:7px;line-height:1.45}
@media(max-width:1080px){.floating-settings-layout{grid-template-columns:1fr}.floating-preview-card{position:static}.floating-preview-window{max-width:360px}}
@media(max-width:900px){.cloud-transfer-grid{grid-template-columns:1fr}.floating-control-card{grid-template-columns:auto 1fr}.floating-live-pill,.floating-control-action{grid-column:auto}.mini-module-grid{grid-template-columns:1fr}}
.theme-grid button.liquid{position:relative;overflow:hidden;border-color:color-mix(in srgb,var(--tm-accent) 34%,var(--tm-line));background:linear-gradient(145deg,color-mix(in srgb,var(--tm-accent) 9%,var(--tm-bg)),color-mix(in srgb,var(--tm-glow) 7%,var(--tm-bg)))}.theme-grid button.liquid::after{content:"";position:absolute;inset:0;pointer-events:none;background:linear-gradient(115deg,rgba(255,255,255,.18),transparent 35% 70%,color-mix(in srgb,var(--tm-accent) 12%,transparent))}.theme-grid button.liquid>*{position:relative;z-index:1}.glass-quality-control{display:grid;grid-template-columns:minmax(170px,.8fr) minmax(0,1.4fr);gap:14px;margin-top:14px;padding:16px;border:1px solid color-mix(in srgb,var(--tm-accent) 28%,var(--tm-line));border-radius:16px;background:linear-gradient(145deg,color-mix(in srgb,var(--tm-accent) 8%,var(--tm-bg)),color-mix(in srgb,var(--tm-glow) 5%,var(--tm-bg)))}.glass-quality-control>div:first-child{display:grid;align-content:center;gap:4px}.glass-quality-control>div:first-child span,.glass-quality-control>div:first-child small{color:var(--tm-muted);font-size:9px}.glass-quality-control>div:first-child b{font-size:14px}.glass-quality-control>div:last-child{display:grid;grid-template-columns:1fr 1fr;gap:7px}.glass-quality-control button{display:grid;gap:4px;padding:12px;border:1px solid var(--tm-line);border-radius:12px;background:color-mix(in srgb,var(--tm-bg) 78%,transparent);color:var(--tm-ink);text-align:left}.glass-quality-control button.active{border-color:color-mix(in srgb,var(--tm-accent) 55%,var(--tm-line));background:color-mix(in srgb,var(--tm-accent) 14%,var(--tm-bg));box-shadow:inset 0 1px 0 rgba(255,255,255,.15),0 8px 22px color-mix(in srgb,var(--tm-glow) 10%,transparent)}.glass-quality-control button b{font-size:11px}.glass-quality-control button small{color:var(--tm-muted);font-size:8px;line-height:1.45}@media(max-width:900px){.glass-quality-control{grid-template-columns:1fr}.glass-quality-control>div:last-child{grid-template-columns:1fr}}
.liquid-glass-quick-control{position:relative;display:grid;grid-template-columns:auto minmax(0,1fr) auto;align-items:center;gap:15px;width:100%;margin-top:16px;padding:16px 17px;overflow:hidden;border:1px solid color-mix(in srgb,#78a8ff 42%,var(--tm-line));border-radius:17px;background:linear-gradient(135deg,color-mix(in srgb,#78a8ff 10%,var(--tm-bg)),color-mix(in srgb,#9b77ff 7%,var(--tm-bg)));color:var(--tm-ink);text-align:left;box-shadow:inset 0 1px 0 rgba(255,255,255,.24),0 10px 28px rgba(28,43,84,.08);transition:border-color .22s ease,box-shadow .22s ease,transform .22s ease}.liquid-glass-quick-control::after{content:"";position:absolute;inset:0;pointer-events:none;background:linear-gradient(112deg,rgba(255,255,255,.22),transparent 30% 72%,rgba(120,168,255,.1))}.liquid-glass-quick-control:hover{transform:translateY(-1px);border-color:color-mix(in srgb,#78a8ff 72%,var(--tm-line));box-shadow:inset 0 1px 0 rgba(255,255,255,.3),0 14px 34px rgba(47,73,145,.14)}.liquid-glass-quick-control.active{border-color:#78a8ff;background:linear-gradient(135deg,color-mix(in srgb,#78a8ff 22%,var(--tm-bg)),color-mix(in srgb,#9b77ff 15%,var(--tm-bg)));box-shadow:inset 0 1px 0 rgba(255,255,255,.3),0 0 0 3px rgba(120,168,255,.1),0 16px 38px rgba(80,92,190,.18)}.liquid-glass-quick-control>*{position:relative;z-index:1}.liquid-glass-orb{display:grid;place-items:center;width:46px;height:46px;border:1px solid rgba(255,255,255,.42);border-radius:15px;background:radial-gradient(circle at 30% 22%,#fff 0 5%,rgba(255,255,255,.55) 8%,transparent 24%),linear-gradient(145deg,#8eb8ff,#7465e8 56%,#202c5f);box-shadow:inset 0 1px 5px rgba(255,255,255,.48),inset 0 -5px 10px rgba(12,20,61,.28),0 8px 20px rgba(72,91,190,.24)}.liquid-glass-orb i{width:21px;height:21px;border:1px solid rgba(255,255,255,.68);border-radius:8px;background:rgba(255,255,255,.2);box-shadow:inset 0 1px 2px rgba(255,255,255,.52);backdrop-filter:blur(5px)}.liquid-glass-quick-copy{display:grid;gap:3px}.liquid-glass-quick-copy small{color:#5f7fc7;font-size:8px;font-weight:700;letter-spacing:.08em;text-transform:uppercase}.liquid-glass-quick-copy b{font-size:15px}.liquid-glass-quick-copy em{color:var(--tm-muted);font-size:9px;font-style:normal;line-height:1.45}.liquid-glass-state{display:grid;justify-items:end;gap:7px}.liquid-glass-state strong{font-size:9px}.liquid-glass-state>i{position:relative;width:46px;height:26px;border-radius:99px;background:var(--tm-line);box-shadow:inset 0 0 0 1px rgba(0,0,0,.04);transition:background-color .22s ease}.liquid-glass-state>i em{position:absolute;top:3px;left:3px;width:20px;height:20px;border-radius:50%;background:var(--tm-bg);box-shadow:0 2px 6px rgba(0,0,0,.18);transition:transform .24s cubic-bezier(.22,1,.36,1)}.liquid-glass-quick-control.active .liquid-glass-state>i{background:#78a8ff}.liquid-glass-quick-control.active .liquid-glass-state>i em{transform:translateX(20px)}.motion-settings>.glass-quality-control{margin:0;border-width:1px 0 0;border-radius:0;background:linear-gradient(145deg,color-mix(in srgb,#78a8ff 9%,var(--tm-bg)),color-mix(in srgb,#9b77ff 6%,var(--tm-bg)))}@media(max-width:680px){.liquid-glass-quick-control{grid-template-columns:auto 1fr}.liquid-glass-state{grid-column:1/-1;grid-template-columns:1fr auto;align-items:center;justify-items:start}.liquid-glass-state>i{justify-self:end}}
.liquid-appearance-panel{display:grid;grid-template-columns:minmax(180px,.72fr) minmax(0,1.28fr);gap:14px;margin-top:12px;padding:14px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-surface)}.liquid-appearance-panel.active{border-color:color-mix(in srgb,var(--tm-accent) 38%,var(--tm-line));box-shadow:0 12px 32px color-mix(in srgb,var(--tm-glow) 9%,transparent)}.liquid-appearance-preview{position:relative;display:grid;min-height:168px;align-content:end;gap:5px;padding:14px;overflow:hidden;border:1px solid color-mix(in srgb,var(--liquid-preview-accent) 35%,var(--tm-line));border-radius:14px;background-position:center;background-size:cover;color:#fff;box-shadow:inset 0 1px 0 rgba(255,255,255,.24)}.liquid-appearance-preview::after{content:"";position:absolute;inset:0;background:linear-gradient(115deg,rgba(255,255,255,.16),transparent 34% 70%,color-mix(in srgb,var(--liquid-preview-accent) 18%,transparent));pointer-events:none}.liquid-appearance-preview>*{position:relative;z-index:1}.liquid-appearance-preview>span{display:flex;gap:5px;margin-bottom:auto}.liquid-appearance-preview>span i{display:block;width:34px;height:24px;border:1px solid rgba(255,255,255,.32);border-radius:9px;background:rgba(255,255,255,.13);backdrop-filter:blur(9px)}.liquid-appearance-preview>span i:nth-child(2){width:52px}.liquid-appearance-preview b{font-size:12px}.liquid-appearance-preview small{max-width:34ch;color:rgba(255,255,255,.72);font-size:8px;line-height:1.5}.liquid-appearance-controls{display:grid;align-content:start;gap:12px}.liquid-appearance-controls>header{display:flex;align-items:center;justify-content:space-between;gap:12px;margin:0}.liquid-appearance-controls>header span{display:grid;gap:3px}.liquid-appearance-controls h4{margin:0;font-size:14px}.liquid-appearance-controls>header small{color:var(--tm-muted);font-size:8px}.liquid-appearance-controls>header button,.liquid-background-actions button{min-height:34px;padding:0 11px;border:1px solid var(--tm-line);border-radius:10px;background:var(--tm-bg);color:var(--tm-ink);font:inherit;font-size:9px}.liquid-color-row{display:grid;gap:8px}.liquid-color-row>label{display:flex;align-items:center;justify-content:space-between;gap:10px;color:var(--tm-muted);font-size:9px}.liquid-color-row input[type=color]{width:44px;height:30px;padding:3px;border:1px solid var(--tm-line);border-radius:9px;background:var(--tm-bg)}.liquid-color-row>div{display:flex;flex-wrap:wrap;gap:7px}.liquid-color-row>div button{position:relative;width:28px;height:28px;padding:0;border:2px solid var(--tm-bg);border-radius:50%;background:var(--preset-color)!important;box-shadow:0 0 0 1px var(--tm-line)}.liquid-color-row>div button.active::after{content:"";position:absolute;inset:7px;border:2px solid #fff;border-radius:50%;box-shadow:0 1px 3px rgba(0,0,0,.25)}.liquid-background-actions{display:flex;flex-wrap:wrap;gap:7px}.liquid-background-actions .primary{border-color:var(--tm-accent)!important;background:var(--tm-accent)!important;color:var(--tm-on-ink)!important}.liquid-background-actions button:disabled{cursor:not-allowed;opacity:.42}.liquid-appearance-status{min-height:1.35em;color:var(--tm-muted);font-size:8px;line-height:1.45}.sr-only{position:absolute;width:1px;height:1px;padding:0;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}@media(max-width:760px){.liquid-appearance-panel{grid-template-columns:1fr}.liquid-appearance-preview{min-height:145px}}
.liquid-video-control{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:11px;border:1px solid var(--tm-line);border-radius:12px;background:color-mix(in srgb,var(--tm-bg) 78%,transparent)}.liquid-video-control>span{display:grid;gap:3px;min-width:0}.liquid-video-control>span b{font-size:10px}.liquid-video-control>span small{overflow:hidden;color:var(--tm-muted);font-size:8px;text-overflow:ellipsis;white-space:nowrap}.liquid-video-control .liquid-background-actions{flex:0 0 auto}.liquid-video-error{display:flex;align-items:flex-start;justify-content:space-between;gap:10px;padding:10px 11px;border:1px solid color-mix(in srgb,#ff3b30 28%,var(--tm-line));border-radius:11px;background:color-mix(in srgb,#ff3b30 7%,var(--tm-bg));color:color-mix(in srgb,#ff3b30 72%,var(--tm-ink));font-size:8px;line-height:1.5}.liquid-video-error button{flex:0 0 auto;padding:6px 8px;border:0;border-radius:8px;background:var(--tm-ink);color:var(--tm-on-ink);font:inherit;font-size:8px}@media(max-width:760px){.liquid-video-control{align-items:stretch;flex-direction:column}.liquid-video-control .liquid-background-actions{flex:1}}
.liquid-tone-control{display:grid;gap:8px;padding:11px;border:1px solid var(--tm-line);border-radius:13px;background:color-mix(in srgb,var(--tm-bg) 76%,transparent)}.liquid-tone-control>span{display:grid;gap:3px}.liquid-tone-control>span b{font-size:10px}.liquid-tone-control>span small{color:var(--tm-muted);font-size:8px;line-height:1.45}.liquid-tone-control>div{display:grid;grid-template-columns:1fr 1fr;gap:7px}.liquid-tone-control button{display:grid;grid-template-columns:30px minmax(0,1fr);align-items:center;gap:9px;min-height:52px;padding:8px;border:1px solid var(--tm-line);border-radius:11px;background:color-mix(in srgb,var(--tm-bg) 78%,transparent);color:var(--tm-ink);text-align:left}.liquid-tone-control button.active{border-color:color-mix(in srgb,var(--tm-accent) 58%,var(--tm-line));box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--tm-accent) 18%,transparent),0 7px 18px color-mix(in srgb,var(--tm-glow) 9%,transparent)}.liquid-tone-control button>i{display:block;width:28px;height:32px;border:1px solid rgba(255,255,255,.72);border-radius:10px;box-shadow:inset 0 1px 0 rgba(255,255,255,.72),0 4px 12px rgba(0,0,0,.13)}.liquid-tone-control .tone-clear{background:linear-gradient(145deg,rgba(255,255,255,.94),rgba(229,235,244,.48))}.liquid-tone-control .tone-dark{background:linear-gradient(145deg,#333844,#080b12)}.liquid-tone-control button>span{display:grid;gap:3px;min-width:0}.liquid-tone-control button b{font-size:9px}.liquid-tone-control button small{color:var(--tm-muted);font-size:7px}@media(max-width:600px){.liquid-tone-control>div{grid-template-columns:1fr}}
.liquid-transparency-control{display:grid;grid-template-columns:minmax(0,1fr) auto;align-items:center;gap:10px;padding:12px;border:1px solid var(--tm-line);border-radius:13px;background:var(--tm-glass);color:var(--tm-ink)}.liquid-transparency-control>span{display:grid;gap:3px}.liquid-transparency-control b{font-size:10px}.liquid-transparency-control small{color:var(--tm-muted);font-size:8px;line-height:1.45}.liquid-transparency-control>strong{min-width:42px;font-size:11px;font-variant-numeric:tabular-nums;text-align:right}.liquid-transparency-control>input{grid-column:1/-1;width:100%;height:22px;margin:0;appearance:none;background:transparent;cursor:pointer}.liquid-transparency-control>input::-webkit-slider-runnable-track{height:5px;border-radius:999px;background:linear-gradient(90deg,var(--tm-accent) 0 var(--tm-liquid-transparency-progress),var(--tm-line) var(--tm-liquid-transparency-progress) 100%)}.liquid-transparency-control>input::-webkit-slider-thumb{width:18px;height:18px;margin-top:-6.5px;appearance:none;border:1px solid color-mix(in srgb,var(--tm-ink) 18%,transparent);border-radius:50%;background:var(--tm-bg);box-shadow:0 2px 7px rgba(0,0,0,.18)}.liquid-transparency-control>em{grid-column:1/-1;display:grid;grid-template-columns:auto 1fr 1fr auto;gap:6px;color:var(--tm-muted);font-size:7px;font-style:normal}.liquid-transparency-control>em i{font-style:normal}.liquid-transparency-control>em i:nth-child(2){text-align:left}.liquid-transparency-control>em i:nth-child(3){text-align:right}
.liquid-appearance-preview.clear{color:#17171a;text-shadow:0 1px 0 rgba(255,255,255,.55)}.liquid-appearance-preview.clear small{color:rgba(23,23,26,.66)}.liquid-appearance-preview.clear>span i{border-color:rgba(255,255,255,.78);background:rgba(255,255,255,.34);box-shadow:inset 0 1px 0 rgba(255,255,255,.9),0 5px 14px rgba(42,51,68,.12)}
.liquid-wallpaper-picker{display:grid;gap:8px}.liquid-wallpaper-picker>span{display:grid;gap:3px}.liquid-wallpaper-picker>span b{font-size:10px}.liquid-wallpaper-picker>span small{color:var(--tm-muted);font-size:8px;line-height:1.45}.liquid-wallpaper-picker>div{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}.liquid-wallpaper-picker button{display:grid;grid-template-columns:42px minmax(0,1fr);align-items:center;gap:9px;min-height:56px;padding:7px;border:1px solid var(--tm-line);border-radius:12px;background:color-mix(in srgb,var(--tm-bg) 76%,transparent);color:var(--tm-ink);text-align:left}.liquid-wallpaper-picker button.active{border-color:color-mix(in srgb,var(--tm-accent) 62%,var(--tm-line));box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--tm-accent) 15%,transparent)}.liquid-wallpaper-picker button>i{width:42px;height:40px;border-radius:10px;background-position:center;background-size:cover}.liquid-wallpaper-picker .wallpaper-auto>i{background:linear-gradient(135deg,#090b10 0 48%,#f8f9fb 52% 100%)}.liquid-wallpaper-picker .wallpaper-graphite>i{background:radial-gradient(ellipse at 28% 18%,#f7f7f9 0 9%,transparent 10%),linear-gradient(135deg,#08090c 20%,#5a5b60 48%,#ececef 51%,#15161a 76%)}.liquid-wallpaper-picker .wallpaper-pearl>i{border:1px solid #e5e5ea;background:radial-gradient(ellipse at 74% 26%,#42454b 0 8%,transparent 9%),linear-gradient(135deg,#fff 18%,#d5d8dd 47%,#4e5157 50%,#f4f5f7 76%)}.liquid-wallpaper-picker .wallpaper-none>i{border:1px dashed var(--tm-line);background:var(--tm-surface)}.liquid-wallpaper-picker button>span{display:grid;gap:3px;min-width:0}.liquid-wallpaper-picker button b{font-size:9px}.liquid-wallpaper-picker button small{color:var(--tm-muted);font-size:7px;line-height:1.35}@media(max-width:600px){.liquid-wallpaper-picker>div{grid-template-columns:1fr}}
.chart-color-settings{margin-bottom:18px}.chart-color-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:9px}.chart-color-grid>button{display:grid;grid-template-columns:48px minmax(0,1fr) auto;align-items:center;gap:11px;min-height:70px;padding:11px;border:1px solid var(--tm-line);border-radius:14px;background:var(--tm-bg);color:var(--tm-ink);text-align:left;transition:transform .22s cubic-bezier(.22,1,.36,1),border-color .22s ease,background-color .22s ease}.chart-color-grid>button:hover{transform:translateY(-1px);background:var(--tm-surface)}.chart-color-grid>button.active{border-color:var(--tm-ink);box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--tm-ink) 10%,transparent)}.palette-ring{display:grid;width:46px;height:46px;place-items:center;border-radius:50%}.palette-ring>i{width:28px;height:28px;border-radius:50%;background:var(--tm-bg);box-shadow:inset 0 0 0 1px var(--tm-line)}.palette-copy{display:grid;gap:3px;min-width:0}.palette-copy>b{font-size:11px}.palette-copy>small{color:var(--tm-muted);font-size:8px}.palette-copy>i{display:flex;gap:3px;margin-top:3px}.palette-copy>i em{width:13px;height:4px;border-radius:99px}.chart-color-grid>button>strong{color:var(--tm-muted);font-size:8px;white-space:nowrap}.chart-color-grid>button.active>strong{color:var(--tm-ink)}@media(max-width:760px){.chart-color-grid{grid-template-columns:1fr}}
.glass-distortion-control{display:grid;margin-top:0;border-top:1px solid var(--tm-line);background:color-mix(in srgb,var(--tm-bg) 48%,transparent)}.glass-distortion-control>header{display:flex;align-items:flex-start;justify-content:space-between;gap:16px;margin:0;padding:16px}.glass-distortion-control>header>span{display:grid;gap:3px}.glass-distortion-control>header small{color:var(--tm-accent);font-size:8px;font-weight:700;letter-spacing:.06em}.glass-distortion-control>header b{font-size:14px}.glass-distortion-control>header em{color:var(--tm-muted);font-size:8px;font-style:normal}.glass-distortion-control>header button{min-height:32px;padding:0 10px;border:1px solid var(--tm-line);border-radius:10px;background:var(--tm-bg);color:var(--tm-ink);font:inherit;font-size:8px}.distortion-slider-row{display:grid;grid-template-columns:minmax(150px,1fr) auto minmax(160px,1.25fr);align-items:center;gap:13px;min-height:60px;padding:10px 16px;border-top:1px solid color-mix(in srgb,var(--tm-line) 72%,transparent)}.distortion-slider-row>span:first-child{display:grid;gap:3px}.distortion-slider-row b{font-size:10px}.distortion-slider-row small{color:var(--tm-muted);font-size:8px;line-height:1.4}.distortion-slider-value{min-width:36px;color:var(--tm-ink);font-size:9px;font-variant-numeric:tabular-nums;text-align:right}.distortion-slider-row input[type="range"]{width:100%;height:22px;margin:0;appearance:none;background:transparent;cursor:pointer}.distortion-slider-row input[type="range"]::-webkit-slider-runnable-track{height:4px;border-radius:99px;background:linear-gradient(90deg,color-mix(in srgb,var(--tm-accent) 72%,var(--tm-ink)),var(--tm-line))}.distortion-slider-row input[type="range"]::-webkit-slider-thumb{width:17px;height:17px;margin-top:-6.5px;appearance:none;border:1px solid color-mix(in srgb,var(--tm-ink) 14%,transparent);border-radius:50%;background:var(--tm-bg);box-shadow:0 2px 5px rgba(0,0,0,.16)}.glass-distortion-control>p{margin:0;padding:11px 16px 14px;border-top:1px solid color-mix(in srgb,var(--tm-line) 72%,transparent);color:var(--tm-muted);font-size:8px;line-height:1.55}@media(max-width:760px){.distortion-slider-row{grid-template-columns:minmax(0,1fr) auto}.distortion-slider-row input{grid-column:1/-1}.glass-distortion-control>header{align-items:stretch;flex-direction:column}.glass-distortion-control>header button{align-self:flex-start}}
</style>
