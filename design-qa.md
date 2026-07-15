# Token Manager 下载站设计 QA

- Source visual truth: `public/images/design-reference.png`
- Implementation screenshot: `qa-desktop-final.png`
- Responsive screenshot: `qa-mobile-final.png`
- Full-view comparison: `qa-comparison.png`
- Viewport: desktop 1440 × 1024; mobile 390 × 844
- State: 首页首屏、浅色主题、未滚动
- Browser: Microsoft Edge headless production preview

## Full-view comparison evidence

源稿与实现均采用白色画布、左侧超大黑色 Token Manager 标题、右侧真实产品窗口、顶部轻量导航和单一黑色下载主操作。实现保留了源稿的大面积留白、约 20px 圆角、细灰分隔线和克制的液态玻璃边缘。

实现有意将源稿的 Windows 与隐私信息合并到首屏说明，并把完整功能内容延伸到首屏以下；这不改变首屏视觉层级和下载转化路径。

## Focused region evidence

- Hero typography: `qa-desktop-final.png` 中两行英文标题保持完整，无文字与产品截图重叠。
- Product imagery: 使用真实 `dashboard.png`，没有用代码绘图、占位图或伪造数据替代。
- Mobile layout: `qa-mobile-final.png` 中标题、正文和主下载按钮位于安全宽度内；CTA 黑色区域像素范围为 x=27–330，不触碰 390px 视口边缘。
- Download artifacts: 最终构建包含安装版 5,164,689 bytes 与便携版 17,887,744 bytes。

## Required fidelity surfaces

- Fonts and typography: Apple 优先字体栈；标题重量、紧字距与源稿一致，中文使用苹方优先并提供 Windows 字体降级。
- Spacing and layout rhythm: 1380px 桌面内容框、宽松首屏留白、三段证明条与后续章节形成稳定纵向节奏。
- Colors and visual tokens: 仅使用黑、白和中性灰；玻璃效果只使用透明度、模糊和轻边框，没有彩色渐变。
- Image quality and asset fidelity: 使用正式应用图标与真实软件截图，圆角和缩放不扭曲原始界面。
- Copy and content: 清楚区分 Codex 本地估算、官方余额、本地代理和账单导入，不把估算值宣传成官方实时额度。

## Comparison history

1. Initial findings:
   - P1: 桌面超大标题越过左侧网格并与产品图重叠。
   - P1: 手机端标题和主下载按钮超出安全宽度。
2. Fixes made:
   - 调整桌面 Hero 网格比例、标题字号上限和产品图间距。
   - 为 390px 视口设置独立标题尺度、Hero 最大宽度和 CTA 安全宽度。
   - 强制根节点白色背景并保留 reduced-motion 支持。
3. Post-fix evidence:
   - `qa-desktop-final.png`: 桌面标题完整，产品图与标题之间保留明确空隙。
   - `qa-mobile-final.png`: 手机标题完整，CTA 未触边，无横向操作被遮挡。

## Findings

没有剩余 P0、P1 或 P2 问题。

## Primary interactions checked

- 页面内锚点均对应真实章节 ID。
- FAQ 使用原生 `details/summary`，支持鼠标与键盘。
- 两个下载链接指向构建产物中的真实 EXE，文件尺寸与发布产物一致。
- 生产构建完成，无 React/Vite 构建错误。

## Follow-up polish

- P3: 后续获得新版应用截图后，可替换当前仍显示早期界面版本号的截图，不影响当前下载和功能说明。

final result: passed
