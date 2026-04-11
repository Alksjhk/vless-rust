# 任务进度表

> 记录项目当前“做到哪了”。所有任务尽量保持原子化，并使用 `[pending]`、`[doing]`、`[done]` 标记。

## 当前状态

- 当前版本：`1.7.9`
- 当前主能力：TCP / WebSocket VLESS 服务、HTTP 信息页、链接生成、TUI、Linux 服务化
- 当前文档结构：已重构为 `README.md`、`docs/spec.md`、`docs/architecture.md`、`docs/todo.md`

## 已完成 `[done]`

### 运行与配置

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [done] | 实现命令行启动入口 | 支持默认启动、指定配置文件、`--no-tui` |
| [done] | 实现首次启动配置向导 | 配置缺失时自动进入交互式向导 |
| [done] | 实现配置文件 JSON 解析 | 支持 `server`、`users`、`performance` 三段配置 |
| [done] | 实现配置文件原子写入 | 避免配置文件写入中断损坏 |
| [done] | 实现 Unix 配置权限控制 | Unix 下按 `0o600` 写入配置 |

### 核心代理能力

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [done] | 实现 VLESS 协议版本 0 解码 | 支持 Beta 版本请求 |
| [done] | 实现 VLESS 协议版本 1 解码 | 支持 Release 版本请求 |
| [done] | 实现 UUID 白名单认证 | 基于内存集合校验用户 |
| [done] | 实现 TCP 模式 VLESS 代理 | 支持目标 TCP 连接与双向转发 |
| [done] | 实现 TCP 模式 UDP over TCP | 支持 `Command::Udp` 的基本转发 |
| [done] | 实现 WebSocket 握手与升级 | 手动计算 `Sec-WebSocket-Accept` |
| [done] | 实现 WebSocket 模式 VLESS 代理 | 使用首帧作为 VLESS 请求头 |
| [done] | 实现 IPv4 / IPv6 / 域名地址解析 | 支持三类目标地址 |

### HTTP 与用户体验

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [done] | 实现根路径信息页 | 返回 HTML 运行信息页面 |
| [done] | 实现按邮箱生成 VLESS 链接接口 | 通过 `GET /?email=` 返回 JSON |
| [done] | 实现 TCP 模式下 HTTP 与代理端口复用 | 单端口区分 HTTP 与 VLESS |
| [done] | 实现 WebSocket 模式下 HTTP 与升级复用 | 同端口处理信息页与 WS Upgrade |
| [done] | 实现公网 IP 自动探测 | 并发请求多个外部接口 |
| [done] | 实现 TUI 状态面板 | 显示服务状态与滚动日志 |
| [done] | 实现传统日志模式 | `--no-tui` 下输出 tracing 日志 |

### 平台与部署

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [done] | 实现 Linux `systemd` 服务安装 | 用户级服务模式 |
| [done] | 实现 Linux `OpenRC` 服务安装 | 系统服务模式 |
| [done] | 实现 Linux 服务卸载流程 | 支持 `--remove` |
| [done] | 实现 Windows 资源嵌入 | 由 `build.rs` 生成版本资源 |
| [done] | 实现 x64 / ARM 交叉构建配置 | 支持 musl 与 zigbuild 目标 |
| [done] | 实现发布模式体积优化 | 启用 LTO、strip、panic abort |

### 安全与稳定性

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [done] | 实现 HTTP 安全响应头 | 统一附加浏览器安全头 |
| [done] | 实现 HTTP 路径遍历防护 | 拒绝 `..` 与 `\` 路径 |
| [done] | 实现 WebSocket 请求头大小限制 | 防止超大头部请求 |
| [done] | 实现信号驱动的优雅关闭 | Unix 监听 SIGINT/SIGTERM，其他平台监听 Ctrl+C |
| [done] | 实现 TCP socket 基础调优 | 支持 `TCP_NODELAY` 与缓冲区设置 |

### 测试与文档

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [done] | 编写协议编解码测试 | 覆盖 VLESS 版本、命令、地址类型 |
| [done] | 编写 HTTP / WebSocket 工具函数测试 | 覆盖 Upgrade 检测与请求解析 |
| [done] | 编写链接生成测试 | 覆盖 TCP / WS 链接与 Base64 |
| [done] | 编写原子写入测试 | 覆盖覆盖写与权限写入 |
| [done] | 编写服务器配置测试 | 覆盖协议、用户、公网 IP、端口 |
| [done] | 重构项目文档结构 | 四份核心文档职责重新划分 |
| [done] | 重写 README 用户指南 | 面向使用者整理上手路径 |
| [done] | 重写技术规格书 | 按源码对齐配置、协议、接口与限制 |
| [done] | 重写架构设计文档 | 明确模块职责、运行流程与非功能设计 |

## 进行中 `[doing]`

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [doing] | 无 | 当前没有进行中的独立任务，请在启动新任务时补充 |

## 待处理 `[pending]`

### 传输层增强

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [pending] | 为 TCP 模式引入 TLS | 支持原生 TLS 入站 |
| [pending] | 为 WebSocket 模式引入 WSS | 支持加密的 WebSocket 代理 |
| [pending] | 实现 `Command::Mux` | 补齐多路复用能力 |
| [pending] | 完成 WebSocket 下的 UDP 代理 | 补齐协议支持边界 |
| [pending] | 评估并实现 Reality / XTLS | 面向更完整的 VLESS 生态兼容 |

### 运维与可观测性

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [pending] | 增加 Prometheus 指标导出 | 暴露连接数、失败数、流量统计 |
| [pending] | 增加结构化 JSON 日志输出 | 便于日志采集与分析 |
| [pending] | 增加健康检查端点 | 用于部署探活 |
| [pending] | 增加日志落盘与轮转策略 | 支持长期运维 |
| [pending] | 增加性能基准测试 | 度量吞吐、延迟、内存占用 |

### 配置与管理

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [pending] | 实现配置热重载 | 避免重启生效 |
| [pending] | 实现动态用户管理 API | 支持新增、删除、查询用户 |
| [pending] | 实现流量统计模型 | 为用户或连接维度统计流量 |
| [pending] | 评估持久化存储方案 | 为管理面能力预留数据层 |

### 平台支持

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [pending] | 增加 Windows 服务安装能力 | 补齐 Windows 运维体验 |
| [pending] | 增加 macOS `launchd` 支持 | 补齐 macOS 服务化部署 |
| [pending] | 评估 FreeBSD 支持 | 扩展服务端平台范围 |

### 测试与文档

| 状态 | 任务 | 说明 |
| --- | --- | --- |
| [pending] | 补充端到端代理测试 | 覆盖真实代理链路 |
| [pending] | 补充 WebSocket 转发集成测试 | 覆盖握手与消息转发 |
| [pending] | 补充服务安装测试 | 覆盖 `systemd` / `OpenRC` 生成逻辑 |
| [pending] | 编写部署指南 | 输出系统化部署步骤 |
| [pending] | 编写故障排查手册 | 覆盖常见连接与配置问题 |

## 维护规则

- 新任务写入 `docs/todo.md` 时，优先拆成独立可完成的原子项
- 任务状态变更时，同步更新对应文档中的实现描述
- 如果功能边界发生变化，同时更新 `README.md`、`docs/spec.md`、`docs/architecture.md`
