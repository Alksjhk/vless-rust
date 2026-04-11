# 架构设计

> 描述系统“怎么做”，说明模块职责、运行流程、非功能性设计与未来扩展边界。

## 1. 设计目标

项目当前架构围绕以下目标展开：

- 单二进制、低依赖、便于跨平台部署
- 代理核心路径尽量减少内存复制
- 用清晰模块边界隔离协议、配置、UI、服务安装等职责
- 在没有数据库的前提下保持配置和运行态足够简单

## 2. 总体架构

```text
main.rs
  ├─ config.rs         配置加载与序列化
  ├─ wizard.rs         首次启动配置向导
  ├─ public_ip.rs      公网 IP 探测
  ├─ service.rs        Linux 服务安装/卸载
  ├─ tui.rs            TUI 日志层
  └─ server.rs         连接监听与协议调度
       ├─ tcp.rs       VLESS over TCP
       │   ├─ protocol.rs
       │   ├─ address.rs
       │   └─ socket.rs
       ├─ ws.rs        VLESS over WebSocket
       │   ├─ protocol.rs
       │   ├─ address.rs
       │   ├─ http.rs
       │   └─ socket.rs
       └─ api.rs       HTTP 信息页与链接接口
           ├─ http.rs
           └─ vless_link.rs
```

## 3. 模块职责

| 模块 | 责任 |
| --- | --- |
| `main.rs` | 启动入口，参数解析，日志模式切换，组装服务 |
| `config.rs` | 定义配置结构与默认值 |
| `wizard.rs` | 在配置缺失时交互生成配置 |
| `server.rs` | 创建监听器，接收连接，分发到 TCP / WS / HTTP 处理路径 |
| `protocol.rs` | VLESS 请求与响应编解码、用户认证 |
| `tcp.rs` | TCP 模式下的 VLESS 代理与 UDP over TCP |
| `ws.rs` | WebSocket 握手、首帧解析与 WebSocket 代理转发 |
| `api.rs` | 处理 `/` 与 `/?email=` 两类 HTTP 请求 |
| `http.rs` | HTTP 请求识别、解析与统一响应构建 |
| `address.rs` | 目标地址解析与目标连接建立 |
| `socket.rs` | TCP 套接字调优 |
| `public_ip.rs` | 并发查询外部服务以获取公网 IP |
| `atomic_write.rs` | 原子写文件，避免配置写入中断损坏 |
| `service.rs` | 生成并安装 systemd / OpenRC 服务 |
| `version.rs` | 版本展示、启动横幅、状态信息输出 |

## 4. 关键流程设计

### 4.1 启动流程

```text
读取命令行参数
  -> 处理 --init / --remove
  -> 加载 config.json
  -> 配置不存在时启动 wizard
  -> 获取公网 IP
  -> 初始化 TUI 或 tracing
  -> 构建 ServerConfig
  -> 启动 Tokio 监听循环
```

### 4.2 连接调度流程

#### TCP 模式

```text
accept TCP connection
  -> peek 前 1024 字节
  -> detect_protocol()
     -> HTTP 请求: api.rs
     -> WebSocket Upgrade: 仍按 HTTP 请求处理
     -> 原始 VLESS: tcp.rs
```

说明：

- 在 `TCP` 模式下，同一端口既能处理 VLESS，也能处理 HTTP 信息页
- 由于检测基于请求头特征，HTTP 与 VLESS 不需要独立端口

#### WebSocket 模式

```text
accept TCP connection
  -> detect_ws_connection()
     -> 普通 HTTP 请求: api.rs
     -> WebSocket Upgrade: ws.rs
     -> 非 HTTP 请求: 直接拒绝
```

说明：

- `WebSocket` 模式下不接受原始 VLESS TCP 首包
- 同一端口只复用 HTTP 信息页与 WebSocket Upgrade

### 4.3 TCP 代理流程

```text
读取请求头
  -> VlessRequest::decode()
  -> UUID 认证
  -> 发送 VLESS 响应头
  -> 根据 Command 分发
     -> Tcp: 建立目标 TCP 连接并双向 copy
     -> Udp: 建立本地 UDP socket，做 UDP over TCP
     -> Mux: 返回未实现错误
```

### 4.4 WebSocket 代理流程

```text
读取并验证 HTTP Upgrade
  -> 手动生成 Sec-WebSocket-Accept
  -> 升级为 WebSocketStream
  -> 读取首帧作为 VLESS 请求头
  -> UUID 认证
  -> 建立目标 TCP 连接
  -> WebSocket <-> TCP 双向转发
```

## 5. 并发模型

### 5.1 Tokio 多线程运行时

- 使用 `#[tokio::main]`
- 默认多线程调度器
- 每个入站连接独立 `tokio::spawn`

### 5.2 双向转发模型

TCP 代理：

- `client -> target` 一个任务
- `target -> client` 一个任务
- 使用 `tokio::io::copy` 做流式转发

WebSocket 代理：

- `ws -> target` 一个任务
- `target -> ws` 一个任务
- 以消息与字节流之间的转换为桥接

### 5.3 优雅关闭

当前关闭机制分两层：

- `broadcast::channel<()>`: 通知服务停止接受新连接
- `watch::channel<bool>`: TUI 退出后通知主服务关闭

Unix 下监听：

- `SIGINT`
- `SIGTERM`

非 Unix 下监听：

- `Ctrl+C`

## 6. 性能设计

### 6.1 零拷贝与低分配策略

- VLESS 解析使用 `Bytes` 和 `split_to`
- 用户集合与邮箱映射使用 `Arc` 共享
- HTTP 探测和 VLESS 首包解析优先使用栈缓冲区

### 6.2 缓冲策略

| 场景 | 策略 |
| --- | --- |
| 协议探测 | `1024` 字节栈缓冲区 |
| HTTP 请求头 | `8192` 字节栈缓冲区 |
| WebSocket 代理读写 | `64KB` 堆缓冲区 |
| UDP 转发 | `16KB` 缓冲区 |

### 6.3 TCP 调优

通过 `socket.rs` 配置：

- `TCP_NODELAY`
- 接收缓冲区大小
- 发送缓冲区大小
- Keepalive

### 6.4 编译优化

`release` 配置启用：

- `lto = "thin"`
- `codegen-units = 1`
- `opt-level = 3`
- `panic = "abort"`
- `strip = true`

## 7. 数据库设计

当前无数据库。

架构决策：

- 用户规模较小，优先使用静态配置文件
- 启动时一次性加载全部配置，运行期只读
- 这样能减少外部依赖、简化部署和跨平台适配

如果未来引入数据库，建议优先承载：

- 用户与 UUID 管理
- 流量统计
- 审计日志元数据
- 管理面配置

## 8. 缓存设计

当前无独立缓存系统。

现有“缓存式”对象主要是进程内只读共享数据：

- `Arc<HashSet<Uuid>>`
- `Arc<HashMap<Uuid, Option<Arc<str>>>>`
- 启动时获取一次的公网 IP

设计取舍：

- 读多写少，进程内共享比引入 Redis 更合适
- 当前没有需要跨实例共享的热点数据

## 9. 消息队列设计

当前无消息队列。

原因：

- 代理核心是同步转发，不涉及异步业务编排
- 没有异步任务消费、削峰填谷或事件总线需求

当前仅使用本地通道：

- `mpsc`: TUI 日志传递
- `broadcast`: 服务关闭通知
- `watch`: TUI 退出信号

## 10. 分布式锁设计

当前无分布式锁。

原因：

- 单进程运行
- 没有多实例共享写资源
- 配置文件写入通过原子写规避部分竞争问题

## 11. 错误处理

### 11.1 错误传播

- 统一使用 `anyhow::Result`
- 在关键 I/O 节点补充上下文信息
- 连接级错误由任务边界捕获并记录

### 11.2 错误分层

| 层级 | 典型错误 | 处理方式 |
| --- | --- | --- |
| 输入层 | 非法请求、路径非法、UUID 无效 | 返回错误响应或拒绝连接 |
| 网络层 | 目标连接失败、读写失败、超时 | 记录日志并关闭连接 |
| 启动层 | 配置解析失败、端口绑定失败 | 直接中止启动 |
| 服务安装层 | init 系统不可用、权限不足 | 返回字符串错误并提示用户 |

### 11.3 防御性校验

- 校验 VLESS 版本号
- 校验 VLESS 报文最小长度
- 校验 WebSocket 路径
- 校验 `Content-Length` 不超过 1MB
- 拒绝包含 `..` 与 `\` 的 HTTP 路径

## 12. 日志记录

### 12.1 日志体系

- 默认使用 `tracing`
- TUI 模式下使用自定义 `Layer` 将日志发送到 `mpsc`
- 非 TUI 模式下输出到标准日志

### 12.2 建议日志分层

| 级别 | 用途 |
| --- | --- |
| `ERROR` | 启动失败、严重内部错误 |
| `WARN` | 认证失败、协议不支持、异常连接 |
| `INFO` | 服务启动、连接建立、服务安装 |
| `DEBUG` | 协议解析、目标连接、转发结束 |

### 12.3 TUI 特性

- 独立线程运行
- 最多缓存 `1000` 条日志
- 支持 `q` / `Esc` 退出
- 支持方向键、`j/k`、`PageUp/PageDown` 滚动

## 13. 监控设计

当前没有完整监控系统，只有基础可观测性：

- 启动横幅读取运行信息
- TUI 实时查看服务状态
- tracing 日志输出
- 公网 IP 自动探测结果打印

当前缺口：

- 指标导出
- 请求/连接计数
- 延迟分位数统计
- 追踪链路
- 健康检查专用端点

## 14. 测试策略

### 14.1 当前测试类型

- 协议解析测试
- HTTP / WebSocket 工具函数测试
- 链接生成测试
- 原子写入测试
- 公网 IP 测试
- 服务器配置测试
- 部分 TCP 行为测试

### 14.2 测试目录约定

- 所有集成测试集中在 `tests/`
- 生产代码不混入调试逻辑

### 14.3 后续测试方向

- 端到端代理回归测试
- WebSocket 完整握手与转发测试
- 服务安装脚本测试
- 性能基准测试
- 模糊测试

## 15. 部署设计

### 15.1 本地二进制部署

最小部署单元：

```text
vless(.exe)
config.json
```

### 15.2 Linux 服务部署

支持：

- `systemd` 用户服务
- `OpenRC` 系统服务

服务特征：

- 工作目录为可执行文件所在目录
- 启动命令自动附带 `--no-tui`
- systemd 使用 `Restart=on-failure`

### 15.3 跨平台构建

CI 目标与本地构建目标保持一致：

- Windows x64
- Linux x64 musl
- Linux ARM64 musl
- Linux ARMv7 musl

## 16. 安全设计

### 16.1 认证

- 基于 UUID 白名单
- 运行期使用 `HashSet` 做快速校验

### 16.2 HTTP 面

- 仅暴露简单只读接口
- 统一添加安全响应头
- 限制请求头体积
- 拒绝路径遍历特征

### 16.3 配置文件

- 缺省通过原子写入生成
- Unix 下写入权限为 `0o600`
- 服务安装前检查配置路径冲突和可写状态

### 16.4 平台安全边界

- 非 root 的 systemd 用户服务优先降低权限需求
- OpenRC 安装明确要求 root
- Windows 仅做资源嵌入，不额外引入服务权限逻辑

## 17. 扩展建议

优先级较高的未来扩展方向：

1. 为传输层补充 TLS / WSS
2. 为运行态补充指标与连接统计
3. 引入管理型 API 与动态用户管理
4. 补齐 Mux 与 WebSocket 下 UDP 能力
5. 为多实例场景重新评估数据库与缓存方案
