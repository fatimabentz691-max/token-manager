export type FloatingModuleId =
  | 'globalRemaining'
  | 'todayTokens'
  | 'estimatedDuration'
  | 'taskCapacity'
  | 'weekCost'
  | 'codexQuota'
  | 'modelBalances'
  | 'warning'
  | 'todayCalls'
  | 'cacheChart'
  | 'costChart'
  | 'requestChart'
  | 'tokenTrendChart'

export type FloatingMode = 'capsule' | 'compact' | 'full'
export type FloatingLayout = 'grid' | 'list'
// 为兼容旧版配置保留联合类型；v0.10.4 起运行时始终迁移并锁定为可交互状态。
export type FloatingInteractionMode = 'smart' | 'interactive' | 'passthrough'

export interface FloatingWindowSize {
  width: number
  height: number
}

export interface FloatingWindowPosition {
  x: number
  y: number
}

/**
 * 悬浮窗第三版配置。所有字段都保存在 Token Manager 主程序的本地配置中，
 * 不建立第二份数据库，也不开放网络接口。
 */
export interface FloatingConfigV3 {
  version: 3
  mode: FloatingMode
  layout: FloatingLayout
  enabled: FloatingModuleId[]
  order: FloatingModuleId[]
  mini: [FloatingModuleId, FloatingModuleId]
  selectedKey: string
  /** 紧凑和完整模式中并列显示的仪表盘；为空时自动回退到当前仪表盘。 */
  visibleKeys: string[]
  pinnedKeys: string[]
  expandedKeys: string[]
  allowMultipleExpanded: boolean
  interaction: FloatingInteractionMode
  alwaysOnTop: boolean
  snapToEdges: boolean
  sizes: Record<FloatingMode, FloatingWindowSize>
  position: FloatingWindowPosition | null
}

export const floatingConfigStorageKey = 'token-manager-floating-config'

export const defaultFloatingSizes: Record<FloatingMode, FloatingWindowSize> = {
  // 胶囊保留 v0.8.5 的稳定尺寸；紧凑/完整模式为 Agent 列表和七天图表预留滚动空间。
  capsule: { width: 360, height: 152 },
  compact: { width: 440, height: 540 },
  full: { width: 560, height: 760 },
}

export function createDefaultFloatingConfig(ids: FloatingModuleId[]): FloatingConfigV3 {
  return {
    version: 3,
    mode: 'compact',
    layout: 'grid',
    enabled: [...ids],
    order: [...ids],
    mini: ['globalRemaining', 'todayTokens'],
    selectedKey: '__all__',
    visibleKeys: ['__all__', '__codex__', '__claude__'],
    pinnedKeys: ['__all__'],
    expandedKeys: [],
    allowMultipleExpanded: false,
    interaction: 'interactive',
    alwaysOnTop: true,
    snapToEdges: true,
    sizes: structuredClone(defaultFloatingSizes),
    position: null,
  }
}

function safeSize(value: unknown, fallback: FloatingWindowSize, minimum: FloatingWindowSize): FloatingWindowSize {
  const candidate = value as Partial<FloatingWindowSize> | undefined
  return {
    width: Math.max(minimum.width, Math.min(960, Number(candidate?.width) || fallback.width)),
    height: Math.max(minimum.height, Math.min(1000, Number(candidate?.height) || fallback.height)),
  }
}

/** 自动迁移 v1/v2 的 dashboard、capsule 和 miniMode 字段。 */
export function loadFloatingConfigV3(ids: FloatingModuleId[]): FloatingConfigV3 {
  const fallback = createDefaultFloatingConfig(ids)
  try {
    const raw = JSON.parse(localStorage.getItem(floatingConfigStorageKey) || 'null')
    if (!raw) return fallback
    const order = ids.filter(id => raw.order?.includes(id))
    for (const id of ids) if (!order.includes(id)) order.push(id)
    const enabled = ids.filter(id => raw.enabled?.includes(id))
    const mini = ids.filter(id => raw.mini?.includes(id)).slice(0, 2)
    const legacyMode: FloatingMode = raw.mode === 'capsule' || raw.miniMode
      ? 'capsule'
      : raw.mode === 'full'
        ? 'full'
        : 'compact'
    const mode: FloatingMode = raw.mode === 'capsule' || raw.mode === 'compact' || raw.mode === 'full'
      ? raw.mode
      : legacyMode
    return {
      version: 3,
      mode,
      layout: raw.layout === 'list' ? 'list' : 'grid',
      enabled: raw.enabled ? enabled : [...fallback.enabled],
      order,
      mini: [mini[0] || 'globalRemaining', mini[1] || 'todayTokens'],
      selectedKey: typeof raw.selectedKey === 'string' ? raw.selectedKey : '__all__',
      visibleKeys: Array.isArray(raw.visibleKeys)
        ? raw.visibleKeys.filter((value: unknown): value is string => typeof value === 'string').slice(0, 12)
        : ['__all__', '__codex__', '__claude__'],
      pinnedKeys: Array.isArray(raw.pinnedKeys)
        ? raw.pinnedKeys.filter((value: unknown): value is string => typeof value === 'string').slice(0, 12)
        : ['__all__'],
      expandedKeys: Array.isArray(raw.expandedKeys)
        ? raw.expandedKeys.filter((value: unknown): value is string => typeof value === 'string')
        : typeof raw.expandedDashboard === 'string' && raw.expandedDashboard
          ? [raw.expandedDashboard]
          : [],
      allowMultipleExpanded: raw.allowMultipleExpanded === true,
      // 旧版的“智能穿透/手动穿透”可能导致窗口无法拖动或再次展开。
      // 读取配置时直接迁移为永久交互，保证升级后无需用户手动修复。
      interaction: 'interactive',
      alwaysOnTop: raw.alwaysOnTop !== false,
      snapToEdges: raw.snapToEdges !== false,
      sizes: {
        // 旧版曾保存过 360×104 的长胶囊；这里统一迁移回稳定折叠尺寸。
        capsule: { ...defaultFloatingSizes.capsule },
        compact: safeSize(raw.sizes?.compact, defaultFloatingSizes.compact, { width: 400, height: 300 }),
        full: safeSize(raw.sizes?.full, defaultFloatingSizes.full, { width: 500, height: 520 }),
      },
      position: Number.isFinite(raw.position?.x) && Number.isFinite(raw.position?.y)
        ? { x: Number(raw.position.x), y: Number(raw.position.y) }
        : null,
    }
  } catch {
    return fallback
  }
}

export function saveFloatingConfigV3(config: FloatingConfigV3) {
  localStorage.setItem(floatingConfigStorageKey, JSON.stringify({ ...config, version: 3 }))
}
