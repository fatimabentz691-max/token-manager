# Token Manager v0.11.11

## 新增功能

- 新增 DeepSeek Harness 本地 Agent：自动识别 `DSH_HOME` 或当前用户目录下的 `.dsh` 配置与会话目录。
- DeepSeek API 账户可一键把 Harness 的 `llm-deepseek.baseURL` 接入 Token Manager 本机代理；使用官方热更新配置，不写系统全局环境变量。
- 接入前自动备份 Harness 原始 `settings.yaml`，关闭自动接入时可恢复；API Key 仍由 Harness 自己管理。
- DeepSeek Harness 使用 DeepSeek 官方品牌资源，并可作为本地 Agent 添加到独立仪表盘与悬浮窗。

## 修复问题

- 修复“代理端口已启动”被误显示成“调用工具已自动接入”的状态错误。
- 实时监控链路现在明确区分“代理监听中”“Agent 已接入”“已捕获请求”三个阶段。
- DeepSeek 余额变化但本机没有捕获请求时，不再暗示已经完成 Token 统计；界面会给出 Harness 专属接入指引。
- 扩大余额变化与已捕获代理请求的关联时间窗，降低流式请求结束后余额接口延迟造成的误报。
- 一键自动接入优先选择已检测到的 DeepSeek Harness；接入失败时不会保留虚假的已连接状态。

## 数据边界

- 代理只在请求真实经过 Token Manager 时统计请求与 Token。
- 余额减少无法反推出输入、输出或缓存 Token，因此不会生成虚假用量。
- DeepSeek Harness 默认压缩会话不做正文解压读取；精确用量优先来自本机代理返回的真实 `usage` 字段。

