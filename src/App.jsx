import { useEffect, useState } from "react";

const providers = [
  "Codex", "Claude Code", "Cursor", "OpenAI", "Anthropic", "Gemini", "DeepSeek", "豆包",
  "通义千问", "腾讯混元", "文心千帆", "智谱 GLM", "Kimi", "讯飞星火", "MiniMax",
  "阶跃星辰", "零一万物", "百川智能", "商汤日日新", "MIMO", "OpenAI 兼容服务",
];

const facts = [
  ["01", "Codex 深度监控", "读取客户端本地额度状态，显示真实百分比与重置倒计时。"],
  ["02", "全平台即时统计", "代理请求完成即刷新，5 秒本地兜底轮询，输入、输出、缓存、请求与成本全部落库。"],
  ["03", "下载与在线可见", "匿名心跳与统一下载入口帮助维护者掌握活跃设备和版本分布，不收集机器身份。"],
];

const publicUrl = (path) => `${import.meta.env.BASE_URL}${path.replace(/^\//, "")}`;
const cloudBaseUrl = "https://token-manager-cloud.netlify.app";
const fallbackRelease = {
  version: "0.7.4",
  title: "弹簧动效与仪表盘修复版",
  download_href: publicUrl("downloads/TokenManager_0.7.4_x64-setup.exe"),
  file_name: "TokenManager_0.7.4_x64-setup.exe",
  size_bytes: 8038371,
  sha256: "B21A48558CFC889B7F55DEFE52623E050F07069346995F674760122A19411D65",
  published_at: "2026-07-27T03:35:29.000Z",
  notes: "修复 DeepSeek V4 PRO 卡片圆角与微光冲突，新增可独立开关的完整弹簧动效。",
};

const versionNumber = (value) => String(value || "").split(".").reduce((total, part) => total * 1000 + (Number.parseInt(part, 10) || 0), 0);

function DownloadButton({ children, href, className = "" }) {
  return <a className={`download-button ${className}`} href={href}>{children}</a>;
}

export function App() {
  const [scrolled, setScrolled] = useState(false);
  const [release, setRelease] = useState(fallbackRelease);
  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 18);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    const controller = new AbortController();
    fetch(`${cloudBaseUrl}/v1/release/latest`, { signal: controller.signal })
      .then(response => response.ok ? response.json() : Promise.reject())
      .then(data => {
        if (versionNumber(data.version) >= versionNumber(fallbackRelease.version)) {
          setRelease({ ...fallbackRelease, ...data, download_href: `${cloudBaseUrl}/v1/download/latest` });
        }
      })
      .catch(() => {});
    return () => {
      controller.abort();
      window.removeEventListener("scroll", onScroll);
    };
  }, []);
  const releaseSize = release.size_bytes ? `${(release.size_bytes / 1048576).toFixed(2)} MB` : "Windows 10 / 11 64 位";
  const releaseDate = new Date(release.published_at).toLocaleDateString("zh-CN");

  return (
    <div className="site-shell">
      <header className={`topbar ${scrolled ? "is-scrolled" : ""}`}>
        <a className="brand" href="#top" aria-label="Token Manager 首页"><img src={publicUrl("images/token-manager-icon.png")} alt="" /><span>Token Manager</span></a>
        <nav aria-label="主导航"><a href="#features">功能</a><a href="#providers">平台</a><a href="#privacy">隐私</a><a href="#faq">常见问题</a></nav>
        <DownloadButton href={release.download_href} className="top-download">免费下载</DownloadButton>
      </header>

      <main id="top">
        <section className="hero section-pad" aria-labelledby="hero-title">
          <div className="hero-copy">
            <p className="eyebrow">WINDOWS 10 / 11 · X64</p>
            <h1 id="hero-title"><span>Token</span><span>Manager</span></h1>
            <p className="hero-lead">看清每一次 AI 消耗</p>
            <p className="hero-description">为国内开发者打造的本地 AI 用量管家。监控 Codex 日志，统一统计模型 API，用更少的信息噪音掌握 Token、余额与成本。</p>
            <div className="hero-actions"><DownloadButton href={release.download_href}>免费下载 v{release.version} · {release.title}</DownloadButton><a className="text-link" href="#features">查看全部功能</a></div>
            <p className="platform-note">本地解析 · 密钥加密 · 不上传代码与日志</p>
          </div>
          <figure className="product-stage glass-panel">
            <img src={publicUrl("images/dashboard-liquid-glass.png")} alt="Token Manager 液态玻璃 AI 控制中心" />
            <figcaption>真实软件界面 · 数据来源始终明确标注</figcaption>
          </figure>
        </section>

        <section className="proof-strip section-pad" aria-label="核心能力概览">
          {facts.map(([number, title, copy]) => <article key={number}><span>{number}</span><div><h2>{title}</h2><p>{copy}</p></div></article>)}
        </section>

        <section id="features" className="features section-pad">
          <header className="section-heading"><p className="eyebrow">ONE LOCAL CONTROL CENTER</p><h2>从余额，到每一次调用。</h2><p>不是只看一个总数。每个平台、每个账户、每个模型都有清晰的数据边界。</p></header>
          <FeatureRow number="01 / CODEX" title="不需要 API Key，直接读取客户端额度。" image={publicUrl("images/dashboard.png")} alt="Codex 与模型用量仪表盘界面" bullets={["客户端 rate_limits 百分比与倒计时", "生成、调试、问答三类消耗拆分", "20% / 10% 两档本地预警", "缺失窗口自动切换个人预算估算"]}>自动发现并增量解析 Codex 日志，同时读取客户端已经落盘的额度状态。数据源在界面中逐项标注，不读取或复用登录凭据。</FeatureRow>
          <FeatureRow reverse number="02 / LIVE MONITORING" title="开启代理，也自动接好调用工具。" image={publicUrl("images/floating-window.png")} alt="Token Manager 分类悬浮窗" bullets={["自动配置常见 Code、SDK 与 Claude Code", "连接修改前自动备份并支持一键恢复", "代理事件完成后立即刷新本地统计", "5 秒轻量兜底轮询，30 秒完整同步"]}>选择账户后点击一次，Token Manager 会启动本机代理并写入对应的用户级连接配置。重启已经运行的调用工具后，请求会自动进入统计链路，无需手工复制 Base URL。</FeatureRow>
          <FeatureRow number="03 / DEEPSEEK V4 PRO" title="V4 Pro 的余额、Token 与成本，对得上。" image={publicUrl("images/dashboard.png")} alt="DeepSeek V4 Pro 独立仪表盘" bullets={["OpenAI 与 Anthropic 两种本机入口", "V4 Pro 与 Claude Opus 别名统一归档", "流式 usage 自动提取", "余额扣减未经过代理时主动告警"]}>Token Manager 会区分“代理已启动”和“请求真正进入代理”。只有真实调用到达后状态才变为正在接收，避免余额改变却没有 Token 记录的假连接。</FeatureRow>
          <FeatureRow reverse number="04 / LIQUID GLASS" title="一键开启液态玻璃，所有窗口实时同步。" image={publicUrl("images/settings-liquid-glass.png")} alt="Token Manager 液态玻璃主题快捷设置" bullets={["主界面、仪表盘与悬浮窗统一材质", "独立快捷开关明确显示启停状态", "高质量与性能优先两档渲染", "动画、粒子与低性能设备自动降级"]}>设置页新增醒目的液态玻璃快捷控制，点击即可同步到主窗口与桌面悬浮窗。多层透射、边缘高光和空间环境光均可按设备性能调整，设置重启后不会丢失。</FeatureRow>
          <FeatureRow number="05 / PROMPT CENTER" title="Prompt 不再散落，保存、检索、复用都在本机。" image={publicUrl("images/prompt-center.png")} alt="Token Manager Prompt 中心" bullets={["分类、收藏、最近与常用视图", "名称、标签、内容和模型联合检索", "发送前本地质量检查与优化建议", "模板、历史和统计不上传云端"]}>Prompt Center 把高频开发提示词变成可以持续积累的本地资产。支持编辑、收藏、复制、分享，以及针对结构、约束与输出格式的质量检查。</FeatureRow>
          <FeatureRow reverse number="06 / ARENA" title="每天刷新 Arena 排行，用公开数据辅助选模型。" image={publicUrl("images/arena.png")} alt="Token Manager Arena 模型排行榜" bullets={["官方公开榜单快照与本地缓存", "模型分数、上下文和价格对照", "能力雷达图与自定义模型", "离线时明确显示最近缓存日期"]}>Arena 页面在联网时检查公开榜单，无法访问时回退到最近一次本地缓存，并始终显示数据日期和来源，避免把历史快照误标成实时结果。</FeatureRow>
        </section>

        <section id="providers" className="providers section-pad">
          <div className="provider-heading"><p className="eyebrow">PROVIDER MATRIX</p><h2>国内外主流模型，统一入口。</h2><p>官方接口能力因平台与账户权限不同。Token Manager 会明确显示数据来自官方账单、本地代理、日志估算或账单导入，不伪造“实时余额”。</p></div>
          <div className="provider-list">{providers.map((provider, index) => <span key={provider}><b>{String(index + 1).padStart(2, "0")}</b>{provider}</span>)}</div>
        </section>

        <section id="privacy" className="privacy-section section-pad">
          <div><p className="eyebrow">LOCAL BY DESIGN</p><h2>你的数据，留在你的电脑。</h2></div>
          <div className="privacy-copy"><p>API Key 使用当前 Windows 用户的 DPAPI 加密保存，应用重启后自动恢复。软件仅发送随机安装标识摘要、版本和心跳时间用于聚合设备统计，无需登录，也不包含邮箱、机器名或硬件指纹。</p><dl><div><dt>密钥</dt><dd>DPAPI 本地加密</dd></div><div><dt>代码与日志</dt><dd>不上传</dd></div><div><dt>代理记录</dt><dd>仅保存在本机</dd></div><div><dt>使用统计</dt><dd>匿名安装心跳</dd></div></dl></div>
        </section>

        <section id="download" className="download-section section-pad">
          <div className="download-main"><img src={publicUrl("images/token-manager-icon.png")} alt="Token Manager 图标" /><p className="eyebrow">TOKEN MANAGER v{release.version} · WINDOWS</p><h2>准备好看清每一次消耗了吗？</h2><p>{release.notes || "全平台实时统计、独立模型仪表盘、可定制悬浮窗和本地加密密钥管理。"}</p><DownloadButton href={release.download_href}>下载 {release.version} 安装版 · {releaseSize}</DownloadButton></div>
          <aside className="download-details glass-panel"><h3>后台实时发行</h3><a href={release.download_href}><span>Token Manager v{release.version} · {release.title}</span><small>推荐 · Windows 10 / 11 64 位 · 下载次数会匿名计入维护后台</small></a><div className="checksum"><span>安装包 SHA-256</span><code>{release.sha256 ? `${release.sha256.slice(0,8)}…${release.sha256.slice(-8)}` : "发布后由后台展示"}</code></div><div className="checksum"><span>发布时间</span><code>{releaseDate}</code></div></aside>
        </section>

        <section id="faq" className="faq section-pad"><header className="section-heading"><p className="eyebrow">FAQ</p><h2>下载之前，你可能想知道。</h2></header><div className="faq-list">
          <details><summary>Codex 的剩余额度从哪里来？</summary><p>Token Manager 优先读取 Codex 客户端已经写入本地会话事件的 rate_limits 百分比和重置时间。这是客户端报告的账户额度状态，不是 Token Manager 自行猜测；若客户端没有下发某个窗口，界面会明确切换为个人预算估算。</p></details>
          <details><summary>怎样判断代理和 CC Switch 是否真正启用？</summary><p>主界面顶部的联合状态栏会同时显示 API 代理通道数、CC Switch 进程与路由接管状态。链路启用后状态栏切换为高对比样式，并可展开查看每个本机接入地址。</p></details>
          <details><summary>为什么某些平台只显示 Token，没有余额？</summary><p>并非所有厂商都提供稳定的官方余额接口。Token Manager 只在平台确实提供且账户有权限时展示官方余额；否则使用本地代理精确计量 Token 和请求次数。</p></details>
          <details><summary>密钥关闭软件后还会保存吗？</summary><p>会。密钥使用 Windows DPAPI 绑定当前用户加密保存，可单独删除或批量清空，不会以明文写入本地数据库。</p></details>
          <details><summary>软件会读取或上传我的代码吗？</summary><p>不会。日志解析只提取用量相关字段，本地代理不保存请求与响应正文。软件仅发送随机安装标识、版本与心跳时间用于聚合设备统计，不包含邮箱、机器名、硬件指纹、API Key、代码或日志。</p></details>
          <details><summary>怎样让 API 调用及时显示在仪表盘？</summary><p>添加账户后启用实时监控，再把调用工具的 Base URL 指向软件显示的 127.0.0.1 本机地址。状态从“等待调用端接入”变为“正在接收”后，请求结束会立即刷新；软件同时每 5 秒读取一次本地统计作为兜底。</p></details>
        </div></section>
      </main>

      <footer className="section-pad"><a className="brand" href="#top"><img src={publicUrl("images/token-manager-icon.png")} alt="" /><span>Token Manager</span></a><p>本地 AI 用量监控，为 Windows 开发者而生。</p><span>© 2026 Token Manager</span></footer>
    </div>
  );
}

function FeatureRow({ reverse = false, number, title, image, alt, bullets, children }) {
  return <article className={`feature-row ${reverse ? "reverse" : ""}`}><div className="feature-copy"><span className="feature-number">{number}</span><h3>{title}</h3><p>{children}</p><ul>{bullets.map(item => <li key={item}>{item}</li>)}</ul></div><div className={`feature-visual glass-panel ${reverse ? "floating-visual" : ""}`}><img src={image} alt={alt} /></div></article>;
}
