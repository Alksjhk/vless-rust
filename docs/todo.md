# 任务进度追踪 (Task Progress Tracker)

> 定义 "Where we are" — 项目当前状态和待办事项

## 项目状态概览

| 模块 | 状态 | 版本 |
|------|------|------|
| VLESS 协议 (TCP) | [done] | v1.0 |
| VLESS 协议 (WebSocket) | [done] | v1.0 |
| HTTP API | [done] | v1.0 |
| TUI 仪表盘 | [done] | v1.0 |
| Linux 服务管理 | [done] | v1.0 |
| 配置向导 | [done] | v1.0 |
| UDP over TCP | [done] | v1.0 |
| 多平台静态构建 | [done] | v1.0 |

---

## 已完成 [done]

### 核心功能

- [done] VLESS 协议版本 0 和 1 实现
- [done] TCP 传输模式
- [done] WebSocket 传输模式
- [done] UUID 用户认证
- [done] 目标地址解析（IPv4/IPv6/域名）
- [done] 双向数据代理
- [done] UDP over TCP 转发（仅 TCP 模式，WS 模式暂不支持）

### 用户体验

- [done] 交互式配置向导（首次启动）
- [done] TUI 实时仪表盘（ratatui）
- [done] 结构化日志输出（tracing）
- [done] 服务器状态横幅显示
- [done] VLESS 链接自动生成

### 系统管理

- [done] Linux systemd 用户服务安装
- [done] Linux OpenRC 服务安装
- [done] 服务自动重启配置
- [done] 配置文件原子写入（带权限控制）
- [done] 优雅关闭（信号处理）

### 网络优化

- [done] TCP_NODELAY 启用
- [done] TCP Keepalive 配置
- [done] TCP 缓冲区大小调优
- [done] 零拷贝数据解析
- [done] mimalloc 内存分配器

### 构建与发布

- [done] Windows x64 静态构建
- [done] Linux x64 musl 静态构建
- [done] Linux ARM64 交叉编译
- [done] Linux ARMv7 交叉编译
- [done] GitHub Actions CI/CD
- [done] 自动版本提取与发布
- [done] Windows 资源嵌入（图标、版本信息）

### 安全

- [done] HTTP 安全响应头
- [done] 路径遍历防护
- [done] Content-Length 限制
- [done] 配置文件权限控制 (600)
- [done] 输入验证与清理

---

## 进行中 [doing]

暂无

---

## 待处理 [pending]

### 功能增强

- [pending] TLS/SSL 支持（原生 HTTPS/WSS）
- [pending] XTLS Reality 支持
- [pending] 多路复用 (Mux) 支持
- [pending] 流量统计与限速
- [pending] 用户级流量统计
- [pending] 日志文件持久化
- [pending] 配置文件热重载
- [pending] 动态用户管理（API 增删用户）

### 性能优化

- [pending] 连接池实现
- [pending] 零拷贝代理（splice/sendfile）
- [pending] 批处理写入
- [pending] 更高效的缓冲区池

### 可观测性

- [pending] Prometheus 指标导出
- [pending] pprof 性能分析支持
- [pending] 结构化日志（JSON 格式）
- [pending] 分布式追踪支持

### 平台支持

- [pending] macOS 原生服务支持（launchd）
- [pending] Windows 服务支持
- [pending] FreeBSD 支持
- [pending] Android 构建支持

### 测试

- [pending] 集成测试套件
- [pending] 性能基准测试
- [pending] 协议兼容性测试
- [pending] 模糊测试（Fuzzing）

### 文档

- [pending] API 文档（OpenAPI/Swagger）
- [pending] 客户端配置示例（v2rayN, v2rayNG 等）
- [pending] 部署指南
- [pending] 故障排查手册

---

## 已归档 [archived]

### v1.7.x 已完成

- [archived] WebSocket 路径可配置
- [archived] 公网 IP 自动检测
- [archived] 双缓冲区池设计
- [archived] TUI 日志滚动优化

### v1.6.x 已完成

- [archived] 初始版本发布
- [archived] 基础 TCP 代理
- [archived] 基础 WebSocket 支持

---

## 版本规划

### v1.8.x (当前)

- 代码重构和性能优化
- 修复潜在的安全问题
- 改进错误处理

### v1.9.0 (计划中)

- TLS/SSL 原生支持
- 配置文件热重载
- 流量统计基础功能

### v2.0.0 (远期)

- XTLS Reality 支持
- 完整 Mux 支持
- 管理 API 完善
