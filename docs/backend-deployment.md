# Token Manager 账户与迁移链接后端

该服务提供邮箱密码登录、邮箱验证码登录、匿名软件使用统计、软件公告、广告，以及一次性加密迁移链接。桌面端先在本机完成加密，后端只保存密文信封，看不到 API Key、日志正文或迁移密码。

## 当前公网服务（0.6.3）

- 服务地址：`https://token-manager-cloud.netlify.app`
- 管理后台：`https://token-manager-cloud.netlify.app/admin/`
- 桌面端 0.6.3 将该 HTTPS 地址设为默认账户服务，并会把旧版的 `127.0.0.1:8787` 本地地址自动迁移为公网地址。
- 邮箱密码注册、登录、退出、账户迁移、公告、广告和匿名软件使用统计已经启用。
- 邮箱验证码接口已经实现，但生产环境只有配置 `TOKEN_MANAGER_EMAIL_WEBHOOK_URL` 后才会发送邮件；未配置时会明确返回“邮件服务暂不可用”，不会伪造发送成功。

公网版本使用 Netlify Functions 与 Netlify Blobs 持久化数据。管理员密钥不要写入用户文档或前端代码，应由项目所有者单独保管，并定期轮换。

## 本地启动

```powershell
cd src-tauri
$env:TOKEN_MANAGER_CODE_SECRET="请替换为至少 32 字节随机字符串"
$env:TOKEN_MANAGER_ADMIN_KEY="后台管理密钥"
cargo run --bin token-manager-server
```

默认监听 `127.0.0.1:8787`，数据库为当前目录的 `token-manager-server.db`。健康检查地址为 `http://127.0.0.1:8787/health`。

管理后台地址为 `http://127.0.0.1:8787/admin`。本机开发环境未设置管理密钥时会使用 `token-manager-local-admin`，仅用于本机调试；生产环境不会启用该默认值。

## 生产环境变量

- `TOKEN_MANAGER_PORT`：监听端口。
- `TOKEN_MANAGER_BIND`：监听地址；默认 `127.0.0.1`。容器部署可设为 `0.0.0.0`，并由防火墙与 HTTPS 反向代理保护。
- `TOKEN_MANAGER_SERVER_DB`：SQLite 数据库绝对路径。
- `TOKEN_MANAGER_PUBLIC_BASE_URL`：生成迁移链接时使用的公网 HTTPS 地址。
- `TOKEN_MANAGER_CODE_SECRET`：验证码摘要密钥，必须长期固定并妥善保存。
- `TOKEN_MANAGER_EMAIL_WEBHOOK_URL`：邮件服务 Webhook。后端会 POST `to`、`subject`、`text`、`code`。
- `TOKEN_MANAGER_EMAIL_WEBHOOK_TOKEN`：可选的 Webhook Bearer Token。
- `TOKEN_MANAGER_ADMIN_KEY`：管理后台密钥，生产环境必须配置高强度随机值。
- `TOKEN_MANAGER_ENV=production`：生产模式。未配置邮件 Webhook 时拒绝发送验证码，不会回显验证码。

生产环境必须由 Caddy、Nginx 或云负载均衡提供 HTTPS，并只把后端端口暴露给反向代理。SQLite 数据库文件需要定期备份，管理密钥与验证码密钥不得写入仓库。

## 邮件 Webhook 示例

```json
{
  "to": "user@example.com",
  "subject": "Token Manager 登录验证码",
  "text": "你的验证码是 123456，10 分钟内有效。",
  "code": "123456"
}
```

开发环境未配置 Webhook 时，申请验证码响应中会带 `debug_code`；生产模式绝不会回显。

## 主要接口

- `POST /v1/auth/code/request`：发送登录或注册验证码。
- `POST /v1/auth/password/register`：验证码确认后设置密码。
- `POST /v1/auth/password/login`：邮箱密码登录。
- `POST /v1/auth/code/login`：邮箱验证码登录；首次登录自动创建账户。
- `GET /v1/me`、`POST /v1/auth/logout`：当前账户和退出。
- `POST /v1/transfers`：登录用户创建加密迁移链接。
- `GET /v1/transfers`、`DELETE /v1/transfers/{id}`：查看和撤销链接。
- `GET /v1/transfer/{token}`：只读取链接元数据，不消耗次数。
- `POST /v1/transfer/{token}/preview`：读取加密迁移包但不核销；客户端密码验证成功后再调用核销接口。
- `POST /v1/transfer/{token}/consume`：下载密文并消耗一次使用次数。
- `POST /v1/admin/transfers`：后台使用 `X-Admin-Key` 为指定邮箱创建链接。
- `GET /v1/content`：桌面端读取当前生效的软件公告与广告，无须登录。
- `POST /v1/app/heartbeat`：无需登录的匿名安装启动与运行心跳。
- `GET /v1/admin/metrics`：累计使用设备、当前在线、24 小时/7 天活跃设备和服务内容数量。
- `GET|POST /v1/admin/content`：查看或发布公告、广告。
- `DELETE /v1/admin/content/{id}`：删除指定内容。

迁移链接默认 24 小时过期、仅能下载一次；最大有效期 7 天，单个密文包限制 25 MB。

## 后台代用户创建链接

设置 `TOKEN_MANAGER_ADMIN_KEY` 后，可以从你的管理后台调用：

```powershell
$headers = @{ "X-Admin-Key" = $env:TOKEN_MANAGER_ADMIN_KEY }
$body = @{
  email = "user@example.com"
  label = "客服协助迁移"
  payload = Get-Content -Raw .\encrypted-envelope.json
  ttl_hours = 24
  one_time = $true
} | ConvertTo-Json
Invoke-RestMethod https://你的后端域名/v1/admin/transfers -Method Post -Headers $headers -ContentType application/json -Body $body
```

`payload` 必须来自 Token Manager 客户端生成的加密信封。管理后台不能上传或查看用户 API Key 明文。

## 软件使用统计口径

- 累计使用设备：曾向服务发送匿名心跳的随机安装实例数量，不要求注册或登录。
- 当前在线：最近 5 分钟内仍有心跳的安装实例数量。
- 24 小时/7 天使用：对应周期内启动过或保持运行的去重安装实例数量。
- 客户端首次启动时在本地生成 32 字节随机标识；服务端只保存该标识的 SHA-256 摘要、首次/最近心跳时间、应用版本和启动次数。

该口径按安装实例计数，不等同于自然人数量。同一个人在两台电脑上使用会计为两个实例。统计不会保存邮箱、IP、机器名、硬件指纹、API Key、日志或会话正文；管理后台只展示聚合数量。

## 公告与广告

在 `/admin` 中可填写标题、正文、HTTPS 跳转地址、开始时间、结束时间与启用状态。桌面端每 5 分钟自动拉取一次，公告和广告分别显示；广告带有明确的“广告”标识，用户可以关闭当前内容。生产环境建议限制同时生效的广告数量，避免影响仪表盘可读性。
