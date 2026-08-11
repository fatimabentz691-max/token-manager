# Token Manager 数据来源说明

Token Manager 将“模型调用接口”和“账户账单接口”分开处理。能够使用模型 API，并不代表该密钥同时拥有余额或组织账单权限。

每条用量都会保存 `source_kind` 与 `accuracy`。来源分为官方接口、本地代理、本地日志、本地 JSON、本地 SQLite、账单导入和个人估算；精确度分为官方、实测、导入和估算。低优先级重复记录只标记为遮蔽，不会删除原始证据。

## 所有已配置平台的统一监控

所有平台均可通过本地代理记录以下数据：

- API 请求次数、成功与失败状态；
- 输入 Token、输出 Token、缓存命中 Token；
- 请求时间、平台和模型；
- 7 天趋势、最近请求和账单导出。

代理不保存请求或响应正文。只有厂商响应实际返回 usage 字段时，Token 才会记为非零；没有 usage 的失败响应仍会计入请求次数。

当前已验证的响应格式包括 OpenAI/OpenAI 兼容、Anthropic、Google Gemini 和 DeepSeek。其他国产 OpenAI 兼容接口复用统一解析器。

DeepSeek‑V4‑Pro 会统一归档为 `deepseek-v4-pro`。通过 Anthropic 兼容入口调用时，Claude Opus 别名也会归入 V4 Pro；流式 OpenAI 请求会自动要求上游在结束块返回总 usage。代理开始收到请求时立即更新连接状态，精确 Token 与成本在响应完成后落库。

## 余额与官方账单

| 平台类型 | 当前余额来源 | 说明 |
| --- | --- | --- |
| Codex | 客户端本地 `rate_limits` 事件 | 读取客户端已经落盘的 `used_percent`、`window_minutes` 和 `resets_at`；不读取 `auth.json`，当前版本未下发的窗口使用个人预算估算并明确标注 |
| Claude Code | 本地 `~/.claude/projects/**/*.jsonl` | 仅提取 assistant message 的模型、时间和 usage 数值，不读取或保存提示词与回复正文 |
| DeepSeek | 官方 `/user/balance` | 使用普通 DeepSeek API Key，可同步人民币总余额、充值余额和赠送余额 |
| OpenCode Go | 官方 `/models` + 本机代理 + 本地 SQLite/JSON | 自动模式优先只读官方 SQLite 会话聚合列，不兼容时明确提示并回退 JSON/JSONL；只提取模型、时间、Token、缓存、成本和请求标识。套餐额度按本机已观测事件估算，官方精确用量仍以 OpenCode 控制台为准 |
| OpenAI、Anthropic | 暂不使用普通模型 Key 查询余额 | 组织 Usage/Costs 接口通常要求管理员级密钥，与普通模型 Key 权限不同 |
| Google Gemini | 本地代理 | Cloud Billing 需要独立 Google Cloud IAM 和账单账号，不等同于 Gemini API Key |
| 国内云平台 | 本地代理或账单导入 | 多数账单接口要求云账号 AK/SK、签名、地域和账单权限，不能把模型 API Key 当成账单凭据 |
| 第三方 OpenAI 兼容服务 | 本地代理 | 是否存在余额接口由中转服务自行决定 |

界面必须明确显示数据来源；没有已验证官方接口时，不显示伪造的实时余额。

## Arena 排行榜数据

Arena 页面使用 `lmarena-ai/leaderboard-dataset` 的官方结构化快照，不根据 Token Manager 的本地用量重新排序，也不从综合分虚构“速度、推理、视觉”等能力分。应用保留官方 `rank`、`rating`、置信区间、票数、实验室、许可证和发布日期。

应用每 30 分钟检查一次官方数据。联网成功、缓存命中和离线回退会使用不同状态标识；“官方发布日期”与“本机最近检查时间”也会分开显示。Arena 官方未发布新快照时，Token Manager 不会自行改变官方名次。

模型详情中的六维图分别读取官方 `coding`、`math`、`instruction_following`、`multi_turn`、`creative_writing` 与 `longer_query` 分类。图形值是由该分类真实名次换算的“分类位置指数”，不是绝对能力分；名次、Score、置信区间与票数仍会同时显示。某模型未被官方分类收录时显示“未上榜”，不会用综合分补 0 或推算。单个分类请求失败时，其余成功分类仍会展示，失败项会明确标记为“同步失败”，不会冒充“未上榜”。

每个榜单请求互相隔离，同一榜单不会并发写缓存，快速切换分类时旧响应不会覆盖当前页面。主界面手动“同步数据”和 Arena 页“检查更新”可以触发官方核对；30 秒全局用量轮询不会反复请求 Arena。正常状态 30 分钟复查，联网失败后 5 分钟重试；即使首次离线且尚无成功快照，也会保存失败检查时间以避免高速重复请求。
