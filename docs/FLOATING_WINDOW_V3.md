# Token Manager 内置悬浮窗 V3

悬浮窗是 Token Manager 主程序的第二个 Tauri 窗口，窗口标签为 `floating`。它与主界面运行在同一 EXE 和 Rust 进程中，共用 SQLite、账户密钥、用量事件、托盘与主题设置，不提供独立启动器、端口或安装包。

## 三种形态

- 胶囊：392 × 100，原生与 CSS 圆角均强制等于窗口高度的一半，形成完整半圆端帽；显示当前模型、核心 Token/额度、环形指标和连接状态。
- 紧凑：430 × 390，34px 外窗圆角，显示模型选择、三个 KPI 与七天图表。
- 完整：548 × 680，24px 外窗圆角，以卡片流显示全部模型，并允许独立展开图表。

尺寸、位置、形态、展开卡片、置顶、贴边和鼠标交互由 `FloatingConfigV3` 保存到本地。旧版 `dashboard`、`capsule` 和 `miniMode` 配置在加载时自动迁移。

## 数据规则

- 代理请求完成后由 Rust 发出 `usage-updated`，主窗口与悬浮窗立即重新读取同一份 SQLite 用量数据。
- 普通日志、账户余额和连接状态每 30 秒同步，倒计时每秒更新。
- Codex 只有在客户端日志包含 `rate_limits` 时才显示客户端报告额度，否则显示“个人预算估算”。
- API 模型优先显示官方余额；官方余额不可用时显示“今日 Token 占比”，不会伪装成余额。

## 原生窗口与材质

- Windows 11 请求 DWM 原生圆角，同时关闭系统自动深色边线，并始终使用 `SetWindowRgn` 做精确裁切兜底。
- 八方向缩放使用 Tauri 原生 `startResizeDragging`，DPI 或尺寸变化后重新计算裁切。
- 智能穿透由主进程内线程检测鼠标位置：顶部控制区和边缘保持交互，内容区穿透，按住 Alt 临时恢复交互。
- 胶囊参考 iOS 灵动岛的信息密度和控制布局，颜色使用当前 Token Manager 主题变量，不固定为纯黑。
- 所有悬浮形态使用主题纯色材质，不启用液态玻璃、桌面采样或背景折射；因此无需额外 GPU 渲染线程，也不会因捕获失败出现黑屏或白屏。

## 内部命令

- `configure_floating_renderer`
- `update_floating_surfaces`
- `set_floating_interaction_mode`
- `set_floating_window_mode`
- `get_floating_render_status`

这些命令只通过 Tauri IPC 提供给同一应用内的窗口，不监听网络端口。

## 验收

运行 `node scripts/capture-floating-v3.mjs` 会在 `screenshots/floating-v3/` 生成三种实际尺寸截图，并自动检查可见性、外窗圆角与横纵向溢出。
