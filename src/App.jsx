import { useEffect, useState } from "react";

const providers = [
  "Codex", "Claude Code", "Cursor", "DeepSeek Harness", "OpenAI", "Anthropic", "Gemini", "DeepSeek", "豆包",
  "通义千问", "腾讯混元", "文心千帆", "智谱 GLM", "Kimi", "讯飞星火", "MiniMax",
  "阶跃星辰", "零一万物", "百川智能", "商汤日日新", "MIMO", "OpenAI 兼容服务",
];

const facts = [
  ["01", "AI 控制中心", "用真实链路状态生成健康度；无有效数据时明确显示待检测，不制造虚假分数。"],
  ["02", "全平台即时统计", "代理请求完成即刷新，输入、输出、缓存、请求、余额与成本按来源独立落库。"],
  ["03", "液态玻璃工作台", "透明白与深色玻璃、图片或视频壁纸、扭曲强度、图表配色和动效均可独立设置。"],
];

const publicUrl = (path) => `${import.meta.env.BASE_URL}${path.replace(/^\//, "")}`;
const cloudBaseUrl = "https://token-manager-cloud.netlify.app";
const fallbackSiteContent = {
  hero_eyebrow: "WINDOWS 10 / 11 · X64",
  hero_lead: "看清每一次 AI 消耗",
  hero_description: "面向开发者的本地 AI 控制中心。统一查看 Codex、Claude Code 与各平台 API 的 Token、请求、缓存、余额和人民币成本，并清楚标注每项数据究竟来自官方、本地日志、代理还是账单导入。",
  download_title: "准备好看清每一次消耗了吗？",
  download_description: "全平台实时统计、独立模型仪表盘、可定制悬浮窗和本地加密密钥管理。",
  privacy_note: "本地解析 · 密钥加密 · 不上传代码与日志",
};
const fallbackRelease = {
  version: "0.11.13",
  title: "更新公告与 Harness 仪表盘修复版",
  download_href: `${cloudBaseUrl}/v1/download/latest?source=github-pages`,
  portable_href: "https://github.com/fatimabentz691-max/HUSSEL/releases/download/v0.11.13/TokenManager_0.11.13_x64-portable.exe",
  file_name: "Token.Manager_0.11.13_x64-setup.exe",
  size_bytes: 11144332,
  sha256: "05105FDFEAADCB9C77C682CE93A6A3C078D63A51461C91EE3297A3D42A01E2B1",
  published_at: "2026-08-14T05:40:00.000Z",
  notes: "更新公告改为与软件公告同位置的内容区内嵌横幅，修复液态玻璃主题下被左侧菜单栏遮挡的问题；同时包含 DeepSeek Harness 仪表盘修复（本地总览合并本机代理拦截用量）。",
};

const versionNumber = (value) => String(value || "").split(".").reduce((total, part) => total * 1000 + (Number.parseInt(part, 10) || 0), 0);

function DownloadButton({ children, href, className = "" }) {
  return <a className={`download-button ${className}`} href={href}>{children}</a>;
}

export function App() {
  const [scrolled, setScrolled] = useState(false);
  const [release, setRelease] = useState(fallbackRelease);
  const [siteContent, setSiteContent] = useState(fallbackSiteContent);
  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 18);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    const controller = new AbortController();
    fetch(`${cloudBaseUrl}/v1/release/latest`, { signal: controller.signal })
      .then(response => response.ok ? response.json() : Promise.reject())
      .then(data => {
        if (versionNumber(data.version) >= versionNumber(fallbackRelease.version)) {
          setRelease({ ...fallbackRelease, ...data });
        }
      })
      .catch(() => {});
    fetch(`${cloudBaseUrl}/v1/site-content`, { signal: controller.signal })
      .then(response => response.ok ? response.json() : Promise.reject())
      .then(data => setSiteContent({ ...fallbackSiteContent, ...data }))
      .catch(() => {});
    return () => {
      controller.abort();
      window.removeEventListener("scroll", onScroll);
    };
  }, []);
  const releaseSize = release.size_bytes ? `${(release.size_bytes / 1048576).toFixed(2)} MB` : "Windows 10 / 11 64 位";
  const releaseDate = new Date(release.published_at).toLocaleDateString("zh-CN");
  const trackedDownloadHref = `${cloudBaseUrl}/v1/download/latest?source=github-pages`;

  return (
    <div className="site-shell">
      <header className={`topbar ${scrolled ? "is-scrolled" : ""}`}>
        <a className="brand" href="#top" aria-label="Token Manager 首页"><img src={publicUrl("images/token-manager-icon.png")} alt="" /><span>Token Manager</span></a>
        <nav aria-label="主导航"><a href="#features">功能</a><a href="#providers">平台</a><a href="#privacy">隐私</a><a href="#faq">常见问题</a></nav>
        <DownloadButton href={trackedDownloadHref} className="top-download">免费下载</DownloadButton>
      </header>

      <main id="top">
        <section className="hero section-pad" aria-labelledby="hero-title">
          <div className="hero-copy">
            <p className="eyebrow">{siteContent.hero_eyebrow}</p>
            <h1 id="hero-title"><span>Token</span><span>Manager</span></h1>
            <p className="hero-lead">{siteContent.hero_lead}</p>
            <p className="hero-description">{siteContent.hero_description}</p>
            <div className="hero-actions"><DownloadButton href={trackedDownloadHref}>免费下载 v{release.version} · {release.title}</DownloadButton><a className="text-link" href={release.portable_href}>便携版</a><a className="text-link" href="#features">查看全部功能</a></div>
            <p className="platform-note">{siteContent.privacy_note}</p>
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
          <header className="section-heading"><p className="eyebrow">ONE LOCAL CONTROL CENTER</p><h2>从链路健康，到每一次调用。</h2><p>首页先告诉你数据是否真的接入，再把每个平台、账户和模型拆成独立仪表盘。</p></header>
          <FeatureRow number="01 / COMMAND CENTER" title="先确认监控链路，再看每一项数据。" image={publicUrl("images/dashboard-liquid-glass.png")} alt="Token Manager AI 控制中心与健康度仪表" bullets={["AI 健康度只在检测到有效数据后评分", "代理、CC Switch 与账户状态并列展示", "今日 Token、调用次数和人民币消费", "无数据时使用清晰的待检测空状态"]}>AI 控制中心把实时链路、活动记录、预算预测和智能建议集中到首屏。健康度由真实本地状态计算，不再在空数据时显示误导性的固定分数。</FeatureRow>
          <FeatureRow reverse number="02 / CODEX & CLAUDE CODE" title="无需复用登录凭据，读取客户端本地状态。" image={publicUrl("images/dashboard.png")} alt="Codex 与 Claude Code 专属用量仪表盘" bullets={["Codex 客户端 rate_limits 百分比与倒计时", "5 小时与 7 天窗口、本地日志和任务分类", "Claude Code 独立仪表盘与会话统计", "20% / 10% 两档本地预警"]}>自动发现并增量解析 Codex 与 Claude Code 的本地用量事件。只显示客户端实际写入的信息；没有官方窗口时会明确标注本地观测或个人预算估算。</FeatureRow>
          <FeatureRow number="03 / LIVE MONITORING" title="代理启动不等于接入成功，状态会说清楚。" image={publicUrl("images/floating-window.png")} alt="Token Manager 分类悬浮窗" bullets={["OpenAI 与 Anthropic 兼容入口", "OpenCode API 代理与本地 JSON 双数据源", "请求结束后立即刷新，30 秒完整同步", "流式 usage、缓存命中与请求次数落库"]}>选择账户后可一键启动本机代理并接入常见调用工具；OpenCode 还可直接读取本地 JSON/JSONL 用量记录。界面会区分“已启动”“等待调用”“正在接收”，让不同模型的 Token 与余额变化有据可查。</FeatureRow>
          <FeatureRow reverse number="04 / LIQUID GLASS" title="真正可调的液态玻璃，不只是模糊背景。" image={publicUrl("images/settings-liquid-glass.png")} alt="Token Manager 液态玻璃主题和扭曲控制" bullets={["透明白与深色玻璃全局同步", "图片与最大 1GB 视频壁纸", "扭曲、边缘弯折、放大与色散可调", "十套图表配色、主题微光和动效开关"]}>WebGL 折射、CSS 安全降级与统一圆角材质覆盖主界面、侧边栏和悬浮窗。设置会实时预览并持久保存，重启后继续使用。</FeatureRow>
          <FeatureRow number="05 / DASHBOARDS" title="卡片随你排，简单模式和高级模式随时切换。" image={publicUrl("images/dashboard.png")} alt="Token Manager 自定义模型仪表盘" bullets={["每个账户与模型拥有独立仪表盘", "卡片拖拽排序并自动补齐空位", "Token、请求、缓存、消费与余额图表", "今日、7 天、30 天切换与悬停详情"]}>图表优先、余额下置，所有卡片可在限定网格内拖拽。主界面和悬浮窗共享自定义逻辑，并为无数据状态提供清晰占位。</FeatureRow>
          <FeatureRow reverse number="06 / LOCAL AGENTS" title="DeepSeek Harness 也能一键进入本地监控链路。" image={publicUrl("images/floating-window.png")} alt="Token Manager 本地 Agent 与悬浮窗监控" bullets={["自动识别 DSH_HOME 与当前用户目录", "官方 llm-deepseek.baseURL 热更新接入", "监听、接入、捕获三阶段状态", "不读取 API Key、提示词或代码正文"]}>v0.11.11 新增 DeepSeek Harness 专属接入。只有真实请求经过 Token Manager 本机代理后才统计 Token；余额变化不能反推 Token 时会明确说明，不生成虚假数据。</FeatureRow>
          <FeatureRow number="07 / PROMPT CENTER" title="Prompt 不再散落，保存、检索、复用都在本机。" image={publicUrl("images/prompt-center.png")} alt="Token Manager Prompt 中心" bullets={["分类、收藏、最近与常用视图", "名称、标签、内容和模型联合检索", "发送前本地质量检查与优化建议", "模板、历史和统计不上传云端"]}>Prompt Center 把高频开发提示词变成可以持续积累的本地资产。支持编辑、收藏、复制、分享，以及针对结构、约束与输出格式的质量检查。</FeatureRow>
          <FeatureRow reverse number="08 / ARENA & REPORTS" title="能力排名、六维雷达和账单报告放在一起。" image={publicUrl("images/arena.png")} alt="Token Manager Arena 排行榜与六维能力雷达" bullets={["今日前十模型与官方来源日期", "六维能力雷达和名次变化", "按模型分类累计消费", "周报、月报与独立 Excel 账单"]}>Arena 在联网时检查公开榜单，离线时回退到最近缓存并标注日期。报告中心按单个模型汇总 Token、请求、缓存和金额，可分别导出 Excel。</FeatureRow>
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
          <div className="download-main"><img src={publicUrl("images/token-manager-icon.png")} alt="Token Manager 图标" /><p className="eyebrow">TOKEN MANAGER v{release.version} · WINDOWS</p><h2>{siteContent.download_title}</h2><p>{release.notes || siteContent.download_description}</p><div className="hero-actions"><DownloadButton href={trackedDownloadHref}>下载 {release.version} 安装版 · {releaseSize}</DownloadButton><a className="text-link" href={release.portable_href}>下载便携版</a></div></div>
          <aside className="download-details glass-panel"><h3>后台实时发行</h3><a href={trackedDownloadHref}><span>Token Manager v{release.version} · {release.title}</span><small>推荐 · Windows 10 / 11 64 位 · 下载次数会匿名计入维护后台</small></a><div className="checksum"><span>安装包 SHA-256</span><code>{release.sha256 ? `${release.sha256.slice(0,8)}…${release.sha256.slice(-8)}` : "发布后由后台展示"}</code></div><div className="checksum"><span>发布时间</span><code>{releaseDate}</code></div></aside>
        </section>

        <section id="faq" className="faq section-pad"><header className="section-heading"><p className="eyebrow">FAQ</p><h2>下载之前，你可能想知道。</h2></header><div className="faq-list">
          <details><summary>Codex 的剩余额度从哪里来？</summary><p>Token Manager 优先读取 Codex 客户端已经写入本地会话事件的 rate_limits 百分比和重置时间。这是客户端报告的账户额度状态，不是 Token Manager 自行猜测；若客户端没有下发某个窗口，界面会明确切换为个人预算估算。</p></details>
          <details><summary>AI 健康度是官方评分吗？</summary><p>不是。它是 Token Manager 根据本机代理、CC Switch、账户和调用事件生成的数据链路健康度。没有检测到有效数据时会显示“待检测”，不会用默认分数冒充实时结果。</p></details>
          <details><summary>怎样判断代理和 CC Switch 是否真正启用？</summary><p>主界面顶部的联合状态栏会同时显示 API 代理通道数、CC Switch 进程与路由接管状态。软件会进一步区分代理已启动、等待调用端接入和正在接收真实请求。</p></details>
          <details><summary>为什么某些平台只显示 Token，没有余额？</summary><p>并非所有厂商都提供稳定的官方余额接口。Token Manager 只在平台确实提供且账户有权限时展示官方余额；否则使用本地代理精确计量 Token 和请求次数。</p></details>
          <details><summary>密钥关闭软件后还会保存吗？</summary><p>会。密钥使用 Windows DPAPI 绑定当前用户加密保存，可单独删除或批量清空，不会以明文写入本地数据库。</p></details>
          <details><summary>软件会读取或上传我的代码吗？</summary><p>不会。日志解析只提取用量相关字段，本地代理不保存请求与响应正文。软件仅发送随机安装标识、版本与心跳时间用于聚合设备统计，不包含邮箱、机器名、硬件指纹、API Key、代码或日志。</p></details>
          <details><summary>怎样让 API 调用及时显示在仪表盘？</summary><p>添加账户后启用实时监控，再把调用工具的 Base URL 指向软件显示的 127.0.0.1 本机地址。状态从“等待调用端接入”变为“正在接收”后，请求结束会立即刷新；软件同时每 30 秒完成一次日志、余额与账户数据同步。</p></details>
        </div></section>
      </main>

      <footer className="section-pad"><a className="brand" href="#top"><img src={publicUrl("images/token-manager-icon.png")} alt="" /><span>Token Manager</span></a><p>本地 AI 用量监控，为 Windows 开发者而生。</p><span>© 2026 Token Manager</span></footer>
    </div>
  );
}

function FeatureRow({ reverse = false, number, title, image, alt, bullets, children }) {
  return <article className={`feature-row ${reverse ? "reverse" : ""}`}><div className="feature-copy"><span className="feature-number">{number}</span><h3>{title}</h3><p>{children}</p><ul>{bullets.map(item => <li key={item}>{item}</li>)}</ul></div><div className={`feature-visual glass-panel ${reverse ? "floating-visual" : ""}`}><img src={image} alt={alt} /></div></article>;
}
