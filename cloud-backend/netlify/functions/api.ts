import { getDeployStore, getStore } from "@netlify/blobs";
import type { Config, Context } from "@netlify/functions";

type User = {
  id: string;
  email: string;
  password_salt: string;
  password_hash: string;
  created_at: string;
};
type Session = {
  id: string;
  user_id: string;
  email: string;
  expires_at: string;
  created_at: string;
  last_seen: string;
};
type LoginCode = {
  email: string;
  purpose: string;
  code_hash: string;
  expires_at: string;
  attempts: number;
  created_at: string;
};
type ContentItem = {
  id: string;
  kind: "announcement" | "ad";
  title: string;
  body: string;
  action_label: string;
  action_url: string;
  starts_at: string;
  ends_at: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
};
type Transfer = {
  id: string;
  user_id: string;
  label: string;
  payload: string;
  expires_at: string;
  max_downloads: number;
  download_count: number;
  revoked_at: string | null;
  created_at: string;
};
type AppPresence = {
  id: string;
  first_seen: string;
  last_seen: string;
  last_launch: string;
  app_version: string;
  launches: number;
  heartbeats: number;
  last_session_id?: string;
  last_event_seq?: number;
  last_client_sent_at?: string;
};
type ReleaseInfo = {
  version: string;
  channel: string;
  platform: string;
  title: string;
  download_url: string;
  updater_url: string;
  file_name: string;
  size_bytes: number;
  sha256: string;
  signature: string;
  notes: string;
  highlights?: string[];
  fixes?: string[];
  published_at: string;
  enabled: boolean;
  updated_at: string;
};
type DownloadEvent = {
  id: string;
  version: string;
  file_name?: string;
  created_at: string;
  country_code?: string;
  city?: string;
  client?: string;
  source?: string;
  from_version?: string;
};
type SiteContent = {
  hero_eyebrow: string;
  hero_lead: string;
  hero_description: string;
  download_title: string;
  download_description: string;
  privacy_note: string;
  updated_at: string;
};

const encode = (value: string) => new TextEncoder().encode(value);
// 免费计划无法创建细粒度 Secret 时使用随机强密钥的不可逆哈希；密钥本体不进入源码。
const ADMIN_FALLBACK_HASH =
  "33f25e78c5159a63643d80571345d9c7928015138a79648a109058b64427c44e";
/**
 * 官网部署在 GitHub Pages，与 Netlify API 不同源。
 * 公共读取接口和管理后台都需要在浏览器中访问，因此所有 JSON 响应统一携带 CORS；
 * 管理权限仍由 x-admin-key 校验，CORS 本身不会绕过鉴权。
 */
const corsHeaders = {
  "access-control-allow-origin": "*",
  "access-control-allow-headers": "authorization, content-type, x-admin-key",
  "access-control-allow-methods": "GET, POST, DELETE, OPTIONS",
  "access-control-max-age": "86400",
};
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
      ...corsHeaders,
    },
  });
const error = (status: number, message: string) =>
  json({ error: message }, status);
const now = () => new Date().toISOString();
const random = (bytes = 24) => {
  const data = crypto.getRandomValues(new Uint8Array(bytes));
  return Buffer.from(data).toString("base64url");
};
const sha = async (value: string) =>
  Buffer.from(
    new Uint8Array(await crypto.subtle.digest("SHA-256", encode(value))),
  ).toString("hex");
const safeEqual = (a: string, b: string) => {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++)
    result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return result === 0;
};
/**
 * Netlify 的新旧函数运行时对环境变量提供了两套入口。
 * 统一读取两者，避免 CLI 已配置变量但函数只能从 process.env 获取时鉴权失效。
 */
const runtimeEnv = (name: string) => {
  try {
    const value = Netlify.env.get(name);
    if (value) return value;
  } catch {
    // 旧函数运行时可能没有 Netlify.env，继续读取标准环境。
  }
  return (
    globalThis as typeof globalThis & {
      process?: { env?: Record<string, string | undefined> };
    }
  ).process?.env?.[name] ?? "";
};
const normalizeEmail = (value: unknown) => {
  const email = String(value ?? "")
    .trim()
    .toLowerCase();
  if (
    email.length > 254 ||
    !email.includes("@") ||
    email.startsWith("@") ||
    email.endsWith("@")
  )
    throw new ApiFault(400, "邮箱格式无效");
  return email;
};
const store = (name: string) =>
  Netlify.context?.deploy?.context === "production"
    ? getStore(name, { consistency: "strong" })
    : getDeployStore(name, { consistency: "strong" });
const users = () => store("tm-users");
const userIndex = () => store("tm-user-index");
const sessions = () => store("tm-sessions");
const codes = () => store("tm-login-codes");
const logins = () => store("tm-login-events");
const contents = () => store("tm-content");
const siteContents = () => store("tm-site-content");
const transfers = () => store("tm-transfers");
const transferTokens = () => store("tm-transfer-tokens");
const limits = () => store("tm-rate-limits");
const presences = () => store("tm-app-presence");
const releases = () => store("tm-releases");
const downloads = () => store("tm-download-events");
class ApiFault extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

async function requestBody(req: Request) {
  try {
    return (await req.json()) as Record<string, unknown>;
  } catch {
    throw new ApiFault(400, "请求正文必须是 JSON");
  }
}
async function passwordDigest(password: string, salt: string) {
  const base = await crypto.subtle.importKey(
    "raw",
    encode(password),
    "PBKDF2",
    false,
    ["deriveBits"],
  );
  const bits = await crypto.subtle.deriveBits(
    {
      name: "PBKDF2",
      hash: "SHA-256",
      salt: Buffer.from(salt, "base64url"),
      iterations: 310000,
    },
    base,
    256,
  );
  return Buffer.from(new Uint8Array(bits)).toString("base64url");
}
async function rateLimit(key: string, max: number, seconds: number) {
  const id = await sha(key);
  const current = (await limits().get(id, { type: "json" })) as {
    attempts: number;
    window_start: string;
  } | null;
  const time = Date.now();
  if (
    !current ||
    time - new Date(current.window_start).getTime() > seconds * 1000
  ) {
    await limits().setJSON(id, { attempts: 1, window_start: now() });
    return;
  }
  if (current.attempts >= max)
    throw new ApiFault(429, "操作过于频繁，请稍后再试");
  await limits().setJSON(id, { ...current, attempts: current.attempts + 1 });
}
async function userByEmail(email: string) {
  const id = await userIndex().get(await sha(email), { type: "text" });
  if (!id) return null;
  return (await users().get(id, { type: "json" })) as User | null;
}
async function createSession(user: User, method: string) {
  const token = random(32);
  const time = now();
  const session: Session = {
    id: `ses_${random(10)}`,
    user_id: user.id,
    email: user.email,
    expires_at: new Date(Date.now() + 30 * 86400000).toISOString(),
    created_at: time,
    last_seen: time,
  };
  await sessions().setJSON(await sha(token), session);
  await logins().setJSON(`${Date.now()}-${random(7)}`, {
    id: `login_${random(8)}`,
    user_id: user.id,
    method,
    created_at: time,
  });
  return {
    access_token: token,
    expires_at: session.expires_at,
    user: { id: user.id, email: user.email },
  };
}
async function currentSession(req: Request) {
  const token =
    req.headers.get("authorization")?.replace(/^Bearer\s+/i, "") ?? "";
  if (!token) throw new ApiFault(401, "需要登录");
  const key = await sha(token);
  const session = (await sessions().get(key, {
    type: "json",
  })) as Session | null;
  if (!session || session.expires_at <= now())
    throw new ApiFault(401, "登录已过期");
  session.last_seen = now();
  await sessions().setJSON(key, session);
  return { key, session };
}
async function consumeCode(email: string, purpose: string, code: string) {
  const key = await sha(`${email}:${purpose}`);
  const record = (await codes().get(key, { type: "json" })) as LoginCode | null;
  if (!record || record.expires_at <= now() || record.attempts >= 5)
    throw new ApiFault(401, "验证码无效或已过期");
  const secret = Netlify.env.get("CODE_SECRET") ?? "";
  if (!secret) throw new ApiFault(503, "验证码服务尚未配置");
  if (
    !safeEqual(
      record.code_hash,
      await sha(`${secret}:${email}:${purpose}:${code}`),
    )
  ) {
    record.attempts++;
    await codes().setJSON(key, record);
    throw new ApiFault(401, "验证码错误");
  }
  await codes().delete(key);
}
async function requireAdmin(req: Request) {
  const supplied = req.headers.get("x-admin-key") ?? "";
  const expected = runtimeEnv("ADMIN_KEY");
  const configuredHash = runtimeEnv("TOKEN_MANAGER_ADMIN_KEY_SHA256")
    .trim()
    .toLowerCase();
  if (expected) {
    if (!safeEqual(supplied, expected)) throw new ApiFault(401, "管理密钥错误");
    return;
  }
  const expectedHash = /^[a-f0-9]{64}$/.test(configuredHash)
    ? configuredHash
    : ADMIN_FALLBACK_HASH;
  if (!safeEqual(await sha(supplied), expectedHash))
    throw new ApiFault(401, "管理密钥错误");
}
async function allJson<T>(name: string): Promise<T[]> {
  const target = store(name);
  const { blobs } = await target.list();
  const rows = await Promise.all(
    blobs.map(
      (item) => target.get(item.key, { type: "json" }) as Promise<T | null>,
    ),
  );
  return rows.filter((item) => item !== null) as T[];
}
function activeContent(items: ContentItem[]) {
  const time = now();
  return items
    .filter(
      (item) =>
        item.enabled &&
        item.starts_at <= time &&
        (!item.ends_at || item.ends_at > time),
    )
    .sort((a, b) =>
      a.kind === b.kind
        ? b.updated_at.localeCompare(a.updated_at)
        : a.kind === "announcement"
          ? -1
          : 1,
    );
}
function validateContent(
  body: Record<string, unknown>,
  existing?: ContentItem | null,
) {
  const kind = String(body.kind ?? "");
  if (kind !== "announcement" && kind !== "ad")
    throw new ApiFault(400, "类型只能是 announcement 或 ad");
  const title = String(body.title ?? "").trim(),
    text = String(body.body ?? "").trim();
  if (!title || [...title].length > 80)
    throw new ApiFault(400, "标题需为 1 至 80 个字符");
  if (!text || [...text].length > 500)
    throw new ApiFault(400, "内容需为 1 至 500 个字符");
  const actionUrl = String(body.action_url ?? "").trim();
  if (actionUrl && !actionUrl.startsWith("https://"))
    throw new ApiFault(400, "跳转地址必须使用 HTTPS");
  const start = body.starts_at ? new Date(String(body.starts_at)) : new Date();
  if (Number.isNaN(start.getTime())) throw new ApiFault(400, "开始时间无效");
  const end = body.ends_at ? new Date(String(body.ends_at)) : null;
  if (end && Number.isNaN(end.getTime()))
    throw new ApiFault(400, "结束时间无效");
  if (end && end <= start) throw new ApiFault(400, "结束时间必须晚于开始时间");
  const time = now();
  return {
    id: String(body.id ?? existing?.id ?? `content_${random(10)}`),
    kind,
    title,
    body: text,
    action_label: String(body.action_label ?? "")
      .trim()
      .slice(0, 30),
    action_url: actionUrl,
    starts_at: start.toISOString(),
    ends_at: end?.toISOString() ?? null,
    enabled: body.enabled !== false,
    created_at: existing?.created_at ?? time,
    updated_at: time,
  } as ContentItem;
}
const defaultSiteContent: SiteContent = {
  hero_eyebrow: "WINDOWS 10 / 11 · X64",
  hero_lead: "看清每一次 AI 消耗",
  hero_description:
    "面向开发者的本地 AI 控制中心。统一查看 Codex、Claude Code 与各平台 API 的 Token、请求、缓存、余额和人民币成本，并清楚标注每项数据究竟来自官方、本地日志、代理还是账单导入。",
  download_title: "准备好看清每一次消耗了吗？",
  download_description:
    "全平台实时统计、独立模型仪表盘、可定制悬浮窗和本地加密密钥管理。",
  privacy_note: "本地解析 · 密钥加密 · 不上传代码与日志",
  updated_at: "2026-08-10T00:00:00.000Z",
};
async function effectiveSiteContent() {
  return (
    ((await siteContents().get("current", {
      type: "json",
    })) as SiteContent | null) ?? defaultSiteContent
  );
}
function validateSiteContent(
  body: Record<string, unknown>,
  existing: SiteContent,
) {
  const field = (name: keyof Omit<SiteContent, "updated_at">, max: number) => {
    const value = String(body[name] ?? existing[name]).trim();
    if (!value || [...value].length > max)
      throw new ApiFault(400, `${name} 需为 1 至 ${max} 个字符`);
    return value;
  };
  return {
    hero_eyebrow: field("hero_eyebrow", 80),
    hero_lead: field("hero_lead", 80),
    hero_description: field("hero_description", 600),
    download_title: field("download_title", 100),
    download_description: field("download_description", 600),
    privacy_note: field("privacy_note", 120),
    updated_at: now(),
  } as SiteContent;
}
const bundledRelease: ReleaseInfo = {
  version: "0.11.11",
  channel: "stable",
  platform: "windows-x86_64",
  title: "DeepSeek Harness 实时代理版",
  download_url:
    "https://github.com/fatimabentz691-max/HUSSEL/releases/download/v0.11.11/Token.Manager_0.11.11_x64-setup.exe",
  updater_url:
    "https://github.com/fatimabentz691-max/HUSSEL/releases/download/v0.11.11/Token.Manager_0.11.11_x64-setup.exe",
  file_name: "Token.Manager_0.11.11_x64-setup.exe",
  size_bytes: 11144767,
  sha256: "1D1375A7E157FE999331DC325AE508FE9B9CE77C252FB0636D41D6944D83AC56",
  signature: "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSdit0cXZNa1lXaS9JOU1MYUYyVjRzaThPOGUvL1diano0TU5DSXJVUG01TTJCYWlTTkgrb3VYVHBFOTltcFJEdFdMOVFLQWxCVzRZZ3BEbzBDczNXVXF4dkFqeUp3TVFZPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzg2Njc4NDI0CWZpbGU6VG9rZW4gTWFuYWdlcl8wLjExLjExX3g2NC1zZXR1cC5leGUKRjFha2VIVXIraG9xbDNDNGxmVXdxRkkvcHhURmlPb1Y1YVlCZVZnWUV4a2Q0MnRHRFcxeWFkSTBVWWNUUTBYT214N1RwRkg2NnFLRXNDa1o0UzZyQXc9PQo=",
  notes:
    "新增 DeepSeek Harness 本地代理；明确区分代理监听、Agent 接入和真实请求捕获。余额变化但请求没有经过本机代理时，不再误报已完成 Token 统计。",
  highlights: [
    "自动识别 DSH_HOME 或当前用户目录下的 DeepSeek Harness",
    "通过官方 llm-deepseek.baseURL 热更新接入本机代理",
    "保留 Harness 原配置备份，不读取或改写 API Key",
  ],
  fixes: [
    "修复代理端口启动就误显示为调用工具已自动接入",
    "修复 DeepSeek 余额变化却没有捕获请求时的误导提示",
    "扩大余额与流式代理请求的关联窗口，降低延迟误报",
  ],
  published_at: "2026-08-14T03:33:52.000Z",
  enabled: true,
  updated_at: "2026-08-14T03:33:52.000Z",
};
function compareVersions(left: string, right: string) {
  const parse = (value: string) =>
    value
      .replace(/^v/i, "")
      .split(/[+-]/, 1)[0]
      .split(".")
      .map((part) => Number.parseInt(part, 10) || 0);
  const a = parse(left),
    b = parse(right);
  for (let index = 0; index < Math.max(a.length, b.length); index++) {
    const difference = (a[index] ?? 0) - (b[index] ?? 0);
    if (difference) return difference;
  }
  return 0;
}
async function effectiveRelease() {
  const stored = (await releases().get("latest", {
    type: "json",
  })) as ReleaseInfo | null;
  return stored && compareVersions(stored.version, bundledRelease.version) >= 0
    ? stored
    : bundledRelease;
}
function validateRelease(
  body: Record<string, unknown>,
  existing?: ReleaseInfo | null,
) {
  const version = String(body.version ?? "")
      .trim()
      .replace(/^v/i, ""),
    downloadUrl = String(body.download_url ?? "").trim(),
    updaterUrl = String(body.updater_url ?? existing?.updater_url ?? "").trim(),
    sha256 = String(body.sha256 ?? "")
      .trim()
      .toUpperCase(),
    signature = String(body.signature ?? existing?.signature ?? "").trim(),
    channel = String(body.channel ?? existing?.channel ?? "stable").trim(),
    platform = String(body.platform ?? existing?.platform ?? "windows-x86_64").trim();
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version))
    throw new ApiFault(400, "版本号格式应为 0.7.1");
  if (!downloadUrl.startsWith("https://"))
    throw new ApiFault(400, "安装包地址必须使用 HTTPS");
  if (updaterUrl && !updaterUrl.startsWith("https://"))
    throw new ApiFault(400, "更新包地址必须使用 HTTPS");
  if (sha256 && !/^[A-F0-9]{64}$/.test(sha256))
    throw new ApiFault(400, "SHA-256 必须是 64 位十六进制字符");
  if (!channel || channel.length > 32) throw new ApiFault(400, "更新频道无效");
  if (!platform || platform.length > 64) throw new ApiFault(400, "发布平台无效");
  const published = body.published_at
    ? new Date(String(body.published_at))
    : new Date();
  if (Number.isNaN(published.getTime()))
    throw new ApiFault(400, "发布时间无效");
  const list = (name: "highlights" | "fixes") => {
    const fallback = existing?.[name] ?? [];
    const raw = Array.isArray(body[name]) ? body[name] : fallback;
    return raw.map(item => String(item).trim()).filter(Boolean).slice(0, 8).map(item => item.slice(0, 220));
  };
  return {
    version,
    channel,
    platform,
    title: String(body.title ?? `Token Manager v${version}`)
      .trim()
      .slice(0, 100),
    download_url: downloadUrl,
    updater_url: updaterUrl,
    file_name: String(body.file_name ?? `TokenManager_${version}_x64-setup.exe`)
      .trim()
      .slice(0, 160),
    size_bytes: Math.max(0, Math.floor(Number(body.size_bytes ?? 0))),
    sha256,
    signature,
    notes: String(body.notes ?? "")
      .trim()
      .slice(0, 2000),
    highlights: list("highlights"),
    fixes: list("fixes"),
    published_at: published.toISOString(),
    enabled: body.enabled !== false,
    updated_at: now(),
  } as ReleaseInfo;
}
function normalizePath(req: Request) {
  const path = new URL(req.url).pathname;
  const marker = "/.netlify/functions/api";
  if (path.startsWith(marker))
    return `/v1${path.slice(marker.length)}`.replace(/\/$/, "");
  return path.replace(/\/$/, "");
}
function clientLabel(req: Request) {
  const ua = req.headers.get("user-agent") ?? "";
  const os = /Windows/i.test(ua)
    ? "Windows"
    : /Macintosh|Mac OS/i.test(ua)
      ? "macOS"
      : /Android/i.test(ua)
        ? "Android"
        : /iPhone|iPad/i.test(ua)
          ? "iOS"
          : "其他";
  const browser = /Edg\//i.test(ua)
    ? "Edge"
    : /Chrome\//i.test(ua)
      ? "Chrome"
      : /Firefox\//i.test(ua)
        ? "Firefox"
        : /Safari\//i.test(ua)
          ? "Safari"
          : "浏览器";
  return `${os} · ${browser}`;
}
function sourceLabel(req: Request) {
  const explicit = new URL(req.url).searchParams.get("source")?.trim().slice(0, 80);
  if (explicit) return explicit;
  const ref = req.headers.get("referer") ?? "";
  if (!ref) return "直接访问";
  try {
    return new URL(ref).hostname.slice(0, 120) || "直接访问";
  } catch {
    return "直接访问";
  }
}
function timeZoneName(value: string | null) {
  const candidate = String(value ?? "Asia/Shanghai").slice(0, 64);
  try {
    new Intl.DateTimeFormat("zh-CN", { timeZone: candidate }).format(
      new Date(),
    );
    return candidate;
  } catch {
    return "Asia/Shanghai";
  }
}
function dayKey(value: string, timezone: string) {
  return new Intl.DateTimeFormat("en-CA", {
    timeZone: timezone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date(value));
}
function dateRange(days: number, timezone: string) {
  const formatter = new Intl.DateTimeFormat("en-CA", {
    timeZone: timezone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
  const rows: string[] = [];
  for (let offset = days - 1; offset >= 0; offset--)
    rows.push(formatter.format(new Date(Date.now() - offset * 86400000)));
  return rows;
}

async function handleAppPresence(req: Request, path: string) {
  if (path !== "/v1/app/heartbeat" || req.method !== "POST") return null;
  const body = await requestBody(req),
    installId = String(body.install_id ?? "").trim(),
    version = String(body.app_version ?? "unknown")
      .trim()
      .slice(0, 32),
    event = body.event === "launch" ? "launch" : "heartbeat",
    sessionId = String(body.session_id ?? "").trim().slice(0, 96),
    eventSeq = Math.max(0, Number(body.event_seq) || 0),
    clientSentAt = String(body.sent_at ?? "").trim().slice(0, 40);
  if (!/^[a-f0-9]{64}$/.test(installId))
    throw new ApiFault(400, "匿名安装标识无效");
  const id = await sha(`install:${installId}`);
  await rateLimit(`presence:${id}`, 30, 600);
  const time = now();
  const existing = (await presences().get(id, {
    type: "json",
  })) as AppPresence | null;
  const row: AppPresence = existing
    ? {
        ...existing,
        last_seen: time,
        last_launch: event === "launch" ? time : existing.last_launch,
        app_version: version || existing.app_version,
        launches: existing.launches + (event === "launch" ? 1 : 0),
        heartbeats: existing.heartbeats + 1,
        last_session_id: sessionId || existing.last_session_id,
        last_event_seq: Math.max(existing.last_event_seq ?? 0, eventSeq),
        last_client_sent_at: clientSentAt || existing.last_client_sent_at,
      }
    : {
        id,
        first_seen: time,
        last_seen: time,
        last_launch: time,
        app_version: version || "unknown",
        launches: 1,
        heartbeats: 1,
        last_session_id: sessionId,
        last_event_seq: eventSeq,
        last_client_sent_at: clientSentAt,
      };
  await presences().setJSON(id, row);
  const release = await effectiveRelease();
  return json({
    ok: true,
    server_time: time,
    heartbeat_interval: 60,
    latest_release: release.enabled ? {
      version: release.version,
      title: release.title,
      size_bytes: release.size_bytes,
      notes: release.notes,
      highlights: release.highlights ?? [],
      fixes: release.fixes ?? [],
      published_at: release.published_at,
      download_url: release.download_url,
    } : null,
  });
}

async function handleRelease(req: Request, path: string, context: Context) {
  if (path === "/v1/release/latest" && req.method === "GET") {
    const release = await effectiveRelease();
    if (!release.enabled) throw new ApiFault(404, "当前没有可下载版本");
    return json(release);
  }
  if (path === "/v1/update/stable" && req.method === "GET") {
    const release = await effectiveRelease();
    if (!release.enabled || release.channel !== "stable")
      throw new ApiFault(404, "当前没有稳定频道更新");
    if (!release.signature || !release.updater_url)
      throw new ApiFault(503, "当前版本尚未完成更新签名，客户端不会安装");
    return json({
      version: release.version,
      notes: release.notes,
      pub_date: release.published_at,
      url: release.updater_url,
      signature: release.signature,
    });
  }
  if (path === "/v1/download/latest" && req.method === "GET") {
    const release = await effectiveRelease();
    if (!release.enabled) throw new ApiFault(404, "当前没有可下载版本");
    const downloadUrl = new URL(req.url);
    const event: DownloadEvent = {
      id: `download_${random(12)}`,
      version: release.version,
      file_name: release.file_name,
      created_at: now(),
      country_code: context.geo?.country?.code ?? "",
      city: context.geo?.city ?? "",
      client: clientLabel(req),
      source: sourceLabel(req),
      from_version: (downloadUrl.searchParams.get("from_version") ?? "").slice(0, 32),
    };
    await downloads().setJSON(`${Date.now()}-${random(8)}`, event);
    return new Response(null, {
      status: 302,
      headers: {
        location: release.download_url,
        "cache-control": "no-store, no-cache, must-revalidate, max-age=0",
        pragma: "no-cache",
        expires: "0",
        ...corsHeaders,
      },
    });
  }
  return null;
}

async function handleAuth(req: Request, path: string, context: Context) {
  if (!path.startsWith("/v1/auth/") && path !== "/v1/me") return null;
  const needsBody = [
    "/v1/auth/code/request",
    "/v1/auth/password/register",
    "/v1/auth/password/login",
    "/v1/auth/code/login",
  ].includes(path);
  const body = needsBody ? await requestBody(req) : {};
  if (path === "/v1/auth/code/request" && req.method === "POST") {
    const email = normalizeEmail(body.email),
      purpose = String(body.purpose ?? "login");
    if (!["login", "register"].includes(purpose))
      throw new ApiFault(400, "验证码用途无效");
    await rateLimit(`code:${context.ip}:${email}`, 5, 600);
    const webhook = Netlify.env.get("EMAIL_WEBHOOK_URL"),
      secret = Netlify.env.get("CODE_SECRET");
    if (!webhook || !secret)
      throw new ApiFault(
        503,
        "邮箱验证码服务尚未启用，请先使用邮箱密码注册或登录",
      );
    const code = String(Math.floor(Math.random() * 1_000_000)).padStart(6, "0");
    const record: LoginCode = {
      email,
      purpose,
      code_hash: await sha(`${secret}:${email}:${purpose}:${code}`),
      expires_at: new Date(Date.now() + 600000).toISOString(),
      attempts: 0,
      created_at: now(),
    };
    await codes().setJSON(await sha(`${email}:${purpose}`), record);
    const response = await fetch(webhook, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        ...(Netlify.env.get("EMAIL_WEBHOOK_TOKEN")
          ? {
              authorization: `Bearer ${Netlify.env.get("EMAIL_WEBHOOK_TOKEN")}`,
            }
          : {}),
      },
      body: JSON.stringify({
        to: email,
        subject: "Token Manager 登录验证码",
        text: `你的验证码是 ${code}，10 分钟内有效。`,
        code,
      }),
    });
    if (!response.ok) throw new ApiFault(502, "验证码邮件发送失败");
    return json({ ok: true, expires_in: 600 });
  }
  if (path === "/v1/auth/password/register" && req.method === "POST") {
    const email = normalizeEmail(body.email),
      password = String(body.password ?? "");
    if ([...password].length < 8)
      throw new ApiFault(400, "密码至少需要 8 个字符");
    await rateLimit(`register:${context.ip}:${email}`, 8, 3600);
    if (await userByEmail(email)) throw new ApiFault(409, "该邮箱已注册");
    if (Netlify.env.get("REQUIRE_REGISTRATION_CODE") === "true")
      await consumeCode(email, "register", String(body.code ?? ""));
    const salt = random(16);
    const user: User = {
      id: `usr_${random(12)}`,
      email,
      password_salt: salt,
      password_hash: await passwordDigest(password, salt),
      created_at: now(),
    };
    await users().setJSON(user.id, user);
    await userIndex().set(await sha(email), user.id);
    return json(await createSession(user, "password_register"));
  }
  if (path === "/v1/auth/password/login" && req.method === "POST") {
    const email = normalizeEmail(body.email),
      password = String(body.password ?? "");
    await rateLimit(`password:${context.ip}:${email}`, 20, 900);
    const user = await userByEmail(email);
    if (
      !user ||
      !safeEqual(
        await passwordDigest(password, user.password_salt),
        user.password_hash,
      )
    )
      throw new ApiFault(401, "邮箱或密码错误");
    return json(await createSession(user, "password"));
  }
  if (path === "/v1/auth/code/login" && req.method === "POST") {
    const email = normalizeEmail(body.email);
    await consumeCode(email, "login", String(body.code ?? ""));
    let user = await userByEmail(email);
    if (!user) {
      user = {
        id: `usr_${random(12)}`,
        email,
        password_salt: "",
        password_hash: "",
        created_at: now(),
      };
      await users().setJSON(user.id, user);
      await userIndex().set(await sha(email), user.id);
    }
    return json(await createSession(user, "code"));
  }
  if (path === "/v1/auth/logout" && req.method === "POST") {
    const { key } = await currentSession(req);
    await sessions().delete(key);
    return json({ ok: true });
  }
  if (path === "/v1/me" && req.method === "GET") {
    const { session } = await currentSession(req);
    return json({ id: session.user_id, email: session.email });
  }
  return null;
}

async function handleTransfers(req: Request, path: string) {
  if (path === "/v1/transfers") {
    const { session } = await currentSession(req);
    if (req.method === "GET") {
      const rows = (await allJson<Transfer>("tm-transfers"))
        .filter((item) => item.user_id === session.user_id)
        .sort((a, b) => b.created_at.localeCompare(a.created_at))
        .slice(0, 50)
        .map((item) => ({
          id: item.id,
          label: item.label,
          expires_at: item.expires_at,
          one_time: item.max_downloads === 1,
          download_count: item.download_count,
          revoked: Boolean(item.revoked_at),
          created_at: item.created_at,
        }));
      return json({ items: rows });
    }
    if (req.method === "POST") {
      const body = await requestBody(req),
        payload = String(body.payload ?? "");
      if (Buffer.byteLength(payload) > 25 * 1024 * 1024)
        throw new ApiFault(413, "迁移包超过 25MB");
      try {
        JSON.parse(payload);
      } catch {
        throw new ApiFault(400, "迁移包必须是有效 JSON 密文信封");
      }
      const token = random(32),
        id = `tr_${random(10)}`,
        hours = Math.max(1, Math.min(168, Number(body.ttl_hours ?? 24))),
        oneTime = body.one_time !== false;
      const row: Transfer = {
        id,
        user_id: session.user_id,
        label: String(body.label ?? "Token Manager 数据迁移"),
        payload,
        expires_at: new Date(Date.now() + hours * 3600000).toISOString(),
        max_downloads: oneTime ? 1 : 20,
        download_count: 0,
        revoked_at: null,
        created_at: now(),
      };
      await transfers().setJSON(id, row);
      await transferTokens().set(await sha(token), id);
      return json({
        id,
        link: `${new URL(req.url).origin}/v1/transfer/${token}`,
        expires_at: row.expires_at,
        one_time: oneTime,
      });
    }
  }
  const own = path.match(/^\/v1\/transfers\/([^/]+)$/);
  if (own && req.method === "DELETE") {
    const { session } = await currentSession(req);
    const row = (await transfers().get(own[1], {
      type: "json",
    })) as Transfer | null;
    if (!row || row.user_id !== session.user_id)
      throw new ApiFault(404, "迁移链接不存在");
    row.revoked_at = now();
    await transfers().setJSON(row.id, row);
    return json({ ok: true });
  }
  const publicMatch = path.match(
    /^\/v1\/transfer\/([^/]+)(?:\/(preview|consume))?$/,
  );
  if (publicMatch) {
    const id = await transferTokens().get(await sha(publicMatch[1]), {
      type: "text",
    });
    const row = id
      ? ((await transfers().get(id, { type: "json" })) as Transfer | null)
      : null;
    if (
      !row ||
      row.revoked_at ||
      row.expires_at <= now() ||
      row.download_count >= row.max_downloads
    )
      throw new ApiFault(410, "迁移链接已过期、撤销或使用完毕");
    if (!publicMatch[2] && req.method === "GET")
      return json({
        id: row.id,
        label: row.label,
        expires_at: row.expires_at,
        one_time: row.max_downloads === 1,
        remaining: row.max_downloads - row.download_count,
      });
    if (publicMatch[2] === "preview" && req.method === "POST")
      return json({
        id: row.id,
        label: row.label,
        expires_at: row.expires_at,
        payload: row.payload,
      });
    if (publicMatch[2] === "consume" && req.method === "POST") {
      row.download_count++;
      await transfers().setJSON(row.id, row);
      return json({
        id: row.id,
        label: row.label,
        expires_at: row.expires_at,
        payload: row.payload,
      });
    }
  }
  return null;
}

async function handleAdmin(req: Request, path: string) {
  if (!path.startsWith("/v1/admin/")) return null;
  await requireAdmin(req);
  if (path === "/v1/admin/metrics" && req.method === "GET") {
    const time = Date.now(),
      onlineSince = new Date(time - 150000).toISOString(),
      day = new Date(time - 86400000).toISOString(),
      week = new Date(time - 7 * 86400000).toISOString();
    const [
      presenceRows,
      userRows,
      contentRows,
      transferRows,
      downloadRows,
      currentRelease,
    ] = await Promise.all([
      allJson<AppPresence>("tm-app-presence"),
      allJson<User>("tm-users"),
      allJson<ContentItem>("tm-content"),
      allJson<Transfer>("tm-transfers"),
      allJson<DownloadEvent>("tm-download-events"),
      effectiveRelease(),
    ]);
    const activeItems = activeContent(contentRows),
      versions = new Map<string, number>(),
      downloadVersions = new Map<string, number>();
    for (const item of presenceRows)
      versions.set(item.app_version, (versions.get(item.app_version) ?? 0) + 1);
    for (const item of downloadRows)
      downloadVersions.set(
        item.version,
        (downloadVersions.get(item.version) ?? 0) + 1,
      );
    return json({
      total_downloads: downloadRows.length,
      downloads_24h: downloadRows.filter((item) => item.created_at >= day)
        .length,
      downloads_7d: downloadRows.filter((item) => item.created_at >= week)
        .length,
      total_installations: presenceRows.length,
      new_installations_24h: presenceRows.filter(
        (item) => item.first_seen >= day,
      ).length,
      total_registered_users: userRows.length,
      new_registered_users_7d: userRows.filter(
        (item) => item.created_at >= week,
      ).length,
      online_installations: presenceRows.filter(
        (item) => item.last_seen >= onlineSince,
      ).length,
      active_installations_24h: presenceRows.filter(
        (item) => item.last_seen >= day,
      ).length,
      active_installations_7d: presenceRows.filter(
        (item) => item.last_seen >= week,
      ).length,
      active_transfer_links: transferRows.filter(
        (item) =>
          !item.revoked_at &&
          item.expires_at > now() &&
          item.download_count < item.max_downloads,
      ).length,
      active_announcements: activeItems.filter(
        (item) => item.kind === "announcement",
      ).length,
      active_ads: activeItems.filter((item) => item.kind === "ad").length,
      current_release: currentRelease.version,
      version_distribution: [...versions]
        .sort((a, b) => b[1] - a[1])
        .map(([version, count]) => ({ version, count })),
      download_distribution: [...downloadVersions]
        .sort((a, b) => b[1] - a[1])
        .map(([version, count]) => ({ version, count })),
      generated_at: now(),
    });
  }
  if (path === "/v1/admin/presence" && req.method === "GET") {
    const url = new URL(req.url);
    const limit = Math.max(10, Math.min(100, Number.parseInt(url.searchParams.get("limit") ?? "30", 10) || 30));
    const page = Math.max(1, Number.parseInt(url.searchParams.get("page") ?? "1", 10) || 1);
    const rows = (await allJson<AppPresence>("tm-app-presence")).sort((a, b) => b.last_seen.localeCompare(a.last_seen));
    const start = (page - 1) * limit;
    const onlineSince = new Date(Date.now() - 150000).toISOString();
    return json({
      items: rows.slice(start, start + limit).map(item => ({
        anonymous_id_tail: item.id.slice(-8),
        app_version: item.app_version,
        first_seen: item.first_seen,
        last_seen: item.last_seen,
        last_launch: item.last_launch,
        launches: item.launches,
        heartbeats: item.heartbeats,
        status: item.last_seen >= onlineSince ? "online" : "offline",
        last_event_seq: item.last_event_seq ?? 0,
      })),
      page,
      limit,
      total: rows.length,
      pages: Math.max(1, Math.ceil(rows.length / limit)),
      online_window_seconds: 150,
    });
  }
  if (path === "/v1/admin/analytics" && req.method === "GET") {
    const url = new URL(req.url),
      days = Math.max(
        7,
        Math.min(
          180,
          Number.parseInt(url.searchParams.get("days") ?? "30", 10) || 30,
        ),
      ),
      timezone = timeZoneName(url.searchParams.get("timezone"));
    const [downloadRows, presenceRows, userRows] = await Promise.all([
      allJson<DownloadEvent>("tm-download-events"),
      allJson<AppPresence>("tm-app-presence"),
      allJson<User>("tm-users"),
    ]);
    const dates = dateRange(days, timezone),
      downloadsByDay = new Map<string, number>(),
      installsByDay = new Map<string, number>(),
      usersByDay = new Map<string, number>(),
      hours = Array.from({ length: 24 }, (_, hour) => ({ hour, count: 0 }));
    for (const item of downloadRows) {
      const key = dayKey(item.created_at, timezone);
      downloadsByDay.set(key, (downloadsByDay.get(key) ?? 0) + 1);
      const hour = Number(
        new Intl.DateTimeFormat("en-GB", {
          timeZone: timezone,
          hour: "2-digit",
          hourCycle: "h23",
        }).format(new Date(item.created_at)),
      );
      if (Number.isFinite(hour) && hours[hour]) hours[hour].count++;
    }
    for (const item of presenceRows) {
      const key = dayKey(item.first_seen, timezone);
      installsByDay.set(key, (installsByDay.get(key) ?? 0) + 1);
    }
    for (const item of userRows) {
      const key = dayKey(item.created_at, timezone);
      usersByDay.set(key, (usersByDay.get(key) ?? 0) + 1);
    }
    let cumulativeInstallations = presenceRows.filter(
        (item) => dayKey(item.first_seen, timezone) < dates[0],
      ).length,
      cumulativeUsers = userRows.filter(
        (item) => dayKey(item.created_at, timezone) < dates[0],
      ).length;
    const series = dates.map((date) => {
      const newInstallations = installsByDay.get(date) ?? 0,
        newUsers = usersByDay.get(date) ?? 0;
      cumulativeInstallations += newInstallations;
      cumulativeUsers += newUsers;
      return {
        date,
        downloads: downloadsByDay.get(date) ?? 0,
        new_installations: newInstallations,
        new_users: newUsers,
        cumulative_installations: cumulativeInstallations,
        cumulative_users: cumulativeUsers,
      };
    });
    return json({
      days,
      timezone,
      series,
      hourly_downloads: hours,
      generated_at: now(),
    });
  }
  if (path === "/v1/admin/downloads" && req.method === "GET") {
    const url = new URL(req.url),
      limit = Math.max(
        10,
        Math.min(
          500,
          Number.parseInt(url.searchParams.get("limit") ?? "100", 10) || 100,
        ),
      );
    const rows = (await allJson<DownloadEvent>("tm-download-events")).sort(
      (a, b) => b.created_at.localeCompare(a.created_at),
    );
    return json({
      items: rows.slice(0, limit),
      total: rows.length,
      generated_at: now(),
    });
  }
  if (path === "/v1/admin/release") {
    if (req.method === "GET")
      return json({ release: await effectiveRelease() });
    if (req.method === "POST") {
      const body = await requestBody(req),
        existing = (await releases().get("latest", {
          type: "json",
        })) as ReleaseInfo | null;
      const release = validateRelease(body, existing);
      if (compareVersions(release.version, bundledRelease.version) < 0)
        throw new ApiFault(
          409,
          `不能用 v${release.version} 覆盖最低发行版 v${bundledRelease.version}`,
        );
      await releases().setJSON("latest", release);
      return json(release);
    }
  }
  if (path === "/v1/admin/content") {
    if (req.method === "GET")
      return json({
        items: (await allJson<ContentItem>("tm-content")).sort((a, b) =>
          b.updated_at.localeCompare(a.updated_at),
        ),
      });
    if (req.method === "POST") {
      const body = await requestBody(req);
      const id = String(body.id ?? "");
      const existing = id
        ? ((await contents().get(id, { type: "json" })) as ContentItem | null)
        : null;
      const item = validateContent(body, existing);
      await contents().setJSON(item.id, item);
      return json(item);
    }
  }
  if (path === "/v1/admin/site-content") {
    if (req.method === "GET") return json(await effectiveSiteContent());
    if (req.method === "POST") {
      const body = await requestBody(req),
        item = validateSiteContent(body, await effectiveSiteContent());
      await siteContents().setJSON("current", item);
      return json(item);
    }
  }
  if (path === "/v1/admin/users" && req.method === "GET") {
    const rows = (await allJson<User>("tm-users"))
      .sort((a, b) => b.created_at.localeCompare(a.created_at))
      .map((item) => ({
        id: item.id,
        email: item.email,
        created_at: item.created_at,
      }));
    return json({ items: rows });
  }
  const contentMatch = path.match(/^\/v1\/admin\/content\/([^/]+)$/);
  if (contentMatch && req.method === "DELETE") {
    const existing = await contents().get(contentMatch[1]);
    if (!existing) throw new ApiFault(404, "内容不存在");
    await contents().delete(contentMatch[1]);
    return json({ ok: true });
  }
  const userMatch = path.match(/^\/v1\/admin\/users\/([^/]+)$/);
  if (userMatch && req.method === "DELETE") {
    const user = (await users().get(userMatch[1], {
      type: "json",
    })) as User | null;
    if (!user) throw new ApiFault(404, "用户不存在");
    await users().delete(user.id);
    await userIndex().delete(await sha(user.email));
    for (const [name, field] of [
      ["tm-sessions", "user_id"],
      ["tm-login-events", "user_id"],
      ["tm-transfers", "user_id"],
    ] as const) {
      const target = store(name);
      const { blobs } = await target.list();
      for (const blob of blobs) {
        const row = (await target.get(blob.key, { type: "json" })) as Record<
          string,
          unknown
        > | null;
        if (row?.[field] === user.id) await target.delete(blob.key);
      }
    }
    return json({ ok: true });
  }
  return null;
}

export default async (req: Request, context: Context) => {
  if (req.method === "OPTIONS") return json({ ok: true });
  try {
    const path = normalizePath(req);
    if (path === "/v1/health" && req.method === "GET")
      return json({
        ok: true,
        admin_auth_configured: Boolean(
          runtimeEnv("ADMIN_KEY") ||
            /^[a-f0-9]{64}$/.test(
              runtimeEnv("TOKEN_MANAGER_ADMIN_KEY_SHA256")
                .trim()
                .toLowerCase(),
            ) || ADMIN_FALLBACK_HASH,
        ),
        server_time: now(),
      });
    if (path === "/v1/content" && req.method === "GET")
      return json({
        items: activeContent(await allJson<ContentItem>("tm-content")),
      });
    if (path === "/v1/site-content" && req.method === "GET")
      return json(await effectiveSiteContent());
    const release = await handleRelease(req, path, context);
    if (release) return release;
    const presence = await handleAppPresence(req, path);
    if (presence) return presence;
    const auth = await handleAuth(req, path, context);
    if (auth) return auth;
    const transfer = await handleTransfers(req, path);
    if (transfer) return transfer;
    const admin = await handleAdmin(req, path);
    if (admin) return admin;
    return error(404, "接口不存在");
  } catch (cause) {
    if (cause instanceof ApiFault) return error(cause.status, cause.message);
    console.error(cause);
    return error(500, "服务器内部错误");
  }
};

export const config: Config = {
  path: "/v1/*",
  method: ["GET", "POST", "DELETE", "OPTIONS"],
};
