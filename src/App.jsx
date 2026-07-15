import { useEffect, useState } from "react";

const providers = [
  "Codex", "Claude Code", "Cursor", "OpenAI", "Anthropic", "Gemini", "DeepSeek", "豆包",
  "通义千问", "腾讯混元", "文心千帆", "智谱 GLM", "Kimi", "讯飞星火", "MiniMax",
  "阶跃星辰", "零一万物", "百川智能", "商汤日日新", "MIMO", "OpenAI 兼容服务",
];

const facts = [
  ["01", "Codex 深度监控", "自动读取本机日志，按 5 小时与 7 天滚动窗口估算剩余用量。"],
  ["02", "统一 API 统计", "输入、输出、缓存 Token、请求次数与已验证成本统一呈现。"],
  ["03", "完全本地处理", "API 密钥使用 Windows DPAPI 加密，日志与会话正文不会上传。"],
];

function DownloadButton({ portable = false, children, className = "" }) {
  const href = portable ? "/downloads/TokenManager_0.6.0_portable.exe" : "/downloads/TokenManager_0.6.0_x64-setup.exe";
  return <a className={`download-button ${className}`} href={href} download>{children}</a>;
}

export function App() {
  const [scrolled, setScrolled] = useState(false);
  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 18);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <div className="site-shell">
      <header className={`topbar ${scrolled ? "is-scrolled" : ""}`}>
        <a className="brand" href="#top" aria-label="Token Manager 首页"><img src="/images/token-manager-icon.png" alt="" /><span>Token Manager</span></a>
        <nav aria-label="主导航"><a href="#features">功能</a><a href="#providers">平台</a><a href="#privacy">隐私</a><a href="#faq">常见问题</a></nav>
        <DownloadButton className="top-download">免费下载</DownloadButton>
      </header>

      <main id="top">
        <section className="hero section-pad" aria-labelledby="hero-title">
          <div className="hero-copy">
            <p className="eyebrow">WINDOWS 10 / 11 · X64</p>
            <h1 id="hero-title"><span>Token</span><span>Manager</span></h1>
            <p className="hero-lead">看清每一次 AI 消耗</p>
            <p className="hero-description">为国内开发者打造的本地 AI 用量管家。监控 Codex 日志，统一统计模型 API，用更少的信息噪音掌握 Token、余额与成本。</p>
            <div className="hero-actions"><DownloadButton>免费下载 v0.6.0</DownloadButton><a className="text-link" href="#features">查看全部功能</a></div>
            <p className="platform-note">本地解析 · 密钥加密 · 零遥测</p>
          </div>
          <figure className="product-stage glass-panel">
            <img src="/images/dashboard.png" alt="Token Manager 模型独立仪表盘，展示 DeepSeek 账户和本地用量状态" />
            <figcaption>真实软件界面 · 数据来源始终明确标注</figcaption>
          </figure>
        </section>

        <section className="proof-strip section-pad" aria-label="核心能力概览">
          {facts.map(([number, title, copy]) => <article key={number}><span>{number}</span><div><h2>{title}</h2><p>{copy}</p></div></article>)}
        </section>

        <section id="features" className="features section-pad">
          <header className="section-heading"><p className="eyebrow">ONE LOCAL CONTROL CENTER</p><h2>从余额，到每一次调用。</h2><p>不是只看一个总数。每个平台、每个账户、每个模型都有清晰的数据边界。</p></header>
          <FeatureRow number="01 / CODEX" title="不需要 API Key，直接读取本地日志。" image="/images/dashboard.png" alt="Codex 与模型用量仪表盘界面" bullets={["生成、调试、问答三类消耗拆分", "20% / 10% 两档本地预警", "剩余开发时长与任务数量换算", "高消耗会话识别与省钱建议"]}>自动发现并增量解析 Codex 日志，统计 Token、模型、会话时间和任务类型。5 小时与 7 天滚动额度基于本地观测模拟，并明确标注为参考值。</FeatureRow>
          <FeatureRow reverse number="02 / FLOATING WINDOW" title="用量始终在眼前，不打断开发。" image="/images/floating-window.png" alt="Token Manager 模块化悬浮窗" bullets={["完整模式与仅保留两个指标的迷你模式", "柱状图、折线图与环形进度交互", "鼠标悬停显示精确数值", "托盘常驻与实时代理状态"]}>可拖动、置顶、缩放的桌面悬浮窗，把 Token、余额、预算和趋势图压缩成轻量信息层。模块可增删、排序并永久保存。</FeatureRow>
          <FeatureRow number="03 / YOUR DASHBOARD" title="每个模型，一张独立仪表盘。" image="/images/settings.png" alt="Token Manager 设置和悬浮窗自定义界面" bullets={["消费、Token、请求与缓存趋势", "模型级预算和异常消耗提醒", "按模型导出 Excel 兼容账单", "官方余额、本地代理、账单导入状态分开显示"]}>Token 放在首位，余额和成本分层呈现。所有卡片均可拖动调整，并自动补齐空位；简单模式与高级模式可以全局切换。</FeatureRow>
        </section>

        <section id="providers" className="providers section-pad">
          <div className="provider-heading"><p className="eyebrow">PROVIDER MATRIX</p><h2>国内外主流模型，统一入口。</h2><p>官方接口能力因平台与账户权限不同。Token Manager 会明确显示数据来自官方账单、本地代理、日志估算或账单导入，不伪造“实时余额”。</p></div>
          <div className="provider-list">{providers.map((provider, index) => <span key={provider}><b>{String(index + 1).padStart(2, "0")}</b>{provider}</span>)}</div>
        </section>

        <section id="privacy" className="privacy-section section-pad">
          <div><p className="eyebrow">LOCAL BY DESIGN</p><h2>你的数据，留在你的电脑。</h2></div>
          <div className="privacy-copy"><p>Token Manager 默认零遥测、零云端账号。API Key 使用当前 Windows 用户的 DPAPI 加密保存，应用重启后自动恢复。</p><dl><div><dt>密钥</dt><dd>DPAPI 本地加密</dd></div><div><dt>会话正文</dt><dd>不读取、不保存</dd></div><div><dt>代理记录</dt><dd>仅模型、Token、时间、状态与成本</dd></div><div><dt>诊断</dt><dd>仅由用户主动导出脱敏包</dd></div></dl></div>
        </section>

        <section id="download" className="download-section section-pad">
          <div className="download-main"><img src="/images/token-manager-icon.png" alt="Token Manager 图标" /><p className="eyebrow">TOKEN MANAGER v0.6.0</p><h2>准备好看清每一次消耗了吗？</h2><p>支持 Windows 10 / 11 64 位。国产 API 功能无需特殊网络；海外平台访问取决于用户自身网络和账户权限。</p><DownloadButton>下载安装版 · 4.93 MB</DownloadButton></div>
          <aside className="download-details glass-panel"><h3>下载选项</h3><a href="/downloads/TokenManager_0.6.0_x64-setup.exe" download><span>NSIS 安装版</span><small>推荐 · 自动创建快捷方式</small></a><a href="/downloads/TokenManager_0.6.0_portable.exe" download><span>便携单文件版</span><small>17.06 MB · 无需安装</small></a><div className="checksum"><span>安装版 SHA-256</span><code>1C9E2E8D…B5AAF819</code></div><div className="checksum"><span>便携版 SHA-256</span><code>B7E1A7B7…DED202E2</code></div></aside>
        </section>

        <section id="faq" className="faq section-pad"><header className="section-heading"><p className="eyebrow">FAQ</p><h2>下载之前，你可能想知道。</h2></header><div className="faq-list">
          <details><summary>Codex 的剩余额度是官方数据吗？</summary><p>不是。Codex 数据来自本机可验证日志，5 小时与 7 天额度根据用户设置的个人上限和滚动窗口模拟。页面会始终标注这是本地观测估算，精准官方额度以 ChatGPT 客户端为准。</p></details>
          <details><summary>为什么某些平台只显示 Token，没有余额？</summary><p>并非所有厂商都提供稳定的官方余额接口。Token Manager 只在平台确实提供且账户有权限时展示官方余额；否则使用本地代理精确计量 Token 和请求次数。</p></details>
          <details><summary>密钥关闭软件后还会保存吗？</summary><p>会。密钥使用 Windows DPAPI 绑定当前用户加密保存，可单独删除或批量清空，不会以明文写入本地数据库。</p></details>
          <details><summary>软件会读取或上传我的代码吗？</summary><p>不会。日志解析只提取用量相关字段，本地代理不保存请求与响应正文，软件默认没有遥测和云端同步。</p></details>
          <details><summary>怎样让 API 调用显示在仪表盘？</summary><p>添加账户后启用“一键 API 实时监控”，再把调用工具的 Base URL 指向软件提供的 127.0.0.1 本机代理地址。之后的请求会自动记录到对应模型仪表盘。</p></details>
        </div></section>
      </main>

      <footer className="section-pad"><a className="brand" href="#top"><img src="/images/token-manager-icon.png" alt="" /><span>Token Manager</span></a><p>本地 AI 用量监控，为 Windows 开发者而生。</p><span>© 2026 Token Manager</span></footer>
    </div>
  );
}

function FeatureRow({ reverse = false, number, title, image, alt, bullets, children }) {
  return <article className={`feature-row ${reverse ? "reverse" : ""}`}><div className="feature-copy"><span className="feature-number">{number}</span><h3>{title}</h3><p>{children}</p><ul>{bullets.map(item => <li key={item}>{item}</li>)}</ul></div><div className={`feature-visual glass-panel ${reverse ? "floating-visual" : ""}`}><img src={image} alt={alt} /></div></article>;
}
