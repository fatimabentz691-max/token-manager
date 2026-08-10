import { computed, ref } from 'vue'

export interface ChartColorTheme {
  id: string
  name: string
  description: string
  series: readonly string[]
}

/**
 * 图表配色与应用主题分离：主题负责界面材质，配色负责数据辨识。
 * 每套方案都按“主序列、次序列、辅助序列、低权重序列”的顺序设计，
 * 圆环、折线、柱状图和悬浮窗会共享同一组颜色。
 */
export const chartColorThemes: readonly ChartColorTheme[] = [
  { id: 'graphite', name: '石墨黑白', description: '默认 · 克制单色', series: ['#17171A', '#3A3A3E', '#5C5C62', '#818188', '#A6A6AD', '#C4C4CA'] },
  { id: 'arctic', name: '冰川蓝', description: '清晰 · 冷静专业', series: ['#0066CC', '#0A84FF', '#5AC8FA', '#64D2FF', '#5E5CE6', '#8E8E93'] },
  { id: 'mineral', name: '矿石青', description: '沉稳 · 长时阅读', series: ['#006D75', '#0A8F98', '#2CB6AE', '#66C7C2', '#245D66', '#8AA6A6'] },
  { id: 'jade', name: '翡翠绿', description: '低饱和 · 状态清晰', series: ['#167A5A', '#2E9B72', '#34C759', '#67C99A', '#147D67', '#98B8AA'] },
  { id: 'amber', name: '琥珀金', description: '温暖 · 金额醒目', series: ['#A85F00', '#D67D00', '#FF9F0A', '#EAB45B', '#7C5527', '#C2A377'] },
  { id: 'vermilion', name: '朱砂红', description: '锐利 · 高对比', series: ['#B93830', '#D94F46', '#FF6B5E', '#CF7A6D', '#834740', '#B79A95'] },
  { id: 'amethyst', name: '紫晶', description: '优雅 · 层次柔和', series: ['#6845AD', '#8060C1', '#9A7CE0', '#B899E6', '#5E5CE6', '#9689AC'] },
  { id: 'indigo', name: '靛蓝', description: '深邃 · 技术感', series: ['#3049A8', '#4962C7', '#667EEA', '#8096E6', '#293B82', '#91A0C7'] },
  { id: 'pearl', name: '珍珠灰蓝', description: '柔和 · 纯白玻璃', series: ['#39566D', '#587588', '#7895A7', '#9DB3C0', '#536871', '#B3C0C7'] },
  { id: 'signal', name: '系统信号', description: '多模型 · 快速区分', series: ['#007AFF', '#34C759', '#FF9F0A', '#AF52DE', '#FF375F', '#5AC8FA'] },
] as const

const storageKey = 'token-manager-chart-color-theme'
const storedId = localStorage.getItem(storageKey) || 'graphite'
const chartColorThemeId = ref(chartColorThemes.some(item => item.id === storedId) ? storedId : 'graphite')
const chartColorTheme = computed(() => chartColorThemes.find(item => item.id === chartColorThemeId.value) || chartColorThemes[0])

function applyChartColorTheme(id: string, notify: boolean) {
  if (!chartColorThemes.some(item => item.id === id)) return
  chartColorThemeId.value = id
  localStorage.setItem(storageKey, id)
  if (notify) window.dispatchEvent(new CustomEvent('token-manager-chart-color-change', { detail: id }))
}

export function chartColorStyle(value = chartColorTheme.value) {
  const colors = value.series
  return {
    '--tm-chart-1': colors[0],
    '--tm-chart-2': colors[1],
    '--tm-chart-3': colors[2],
    '--tm-chart-4': colors[3],
    '--tm-chart-5': colors[4],
    '--tm-chart-6': colors[5],
    '--tm-chart-ring': colors[0],
    '--tm-chart-ring-secondary': colors[2],
  }
}

export function useChartColorPreferences() {
  return {
    chartColorThemeId,
    chartColorTheme,
    chartColorThemes,
    setChartColorTheme: (id: string) => applyChartColorTheme(id, true),
    syncChartColorTheme: (id: string) => applyChartColorTheme(id, false),
  }
}
