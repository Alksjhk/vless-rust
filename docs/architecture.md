# 架构设计 (Architecture Design)

> 描述 "How to do it" — 系统如何设计和实现

## 1. 系统架构

### 1.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                           VLESS-Rust Server                         │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │   CLI Args  │  │   Config    │  │  Public IP  │  │   Signals   │ │
│  │   Parser    │  │   Loader    │  │   Fetcher   │  │   Handler   │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘ │
│         └─────────────────┴─────────────────┴─────────────────┘     │
│                               │                                     │
│                         ┌─────┴─────┐                               │
│                         │   main    │                               │
│                         └─────┬─────┘                               │
│                               │                                     │
│                    ┌──────────┴──────────┐                          │
│                    ▼                     ▼                          │
│           ┌─────────────────┐  ┌─────────────────┐                  │
│           │   TUI 模式      │  │  传统日志模式   │                  │
│           │  (ratatui)      │  │  (tracing)      │                  │
│           └────────┬────────┘  └─────────────────┘                  │
│                    │                                                │
│                    ▼                                                │
│           ┌─────────────────┐                                       │
│           │  VlessServer    │                                       │
│           │  (server.rs)    │                                       │
│           └────────┬────────┘                                       │
│                    │                                                │
│         ┌──────────┼──────────┐                                     │
│         ▼          ▼          ▼                                     │
│   ┌─────────┐ ┌─────────┐ ┌─────────┐                              │
│   │ TCP模式 │ │ WS模式  │ │HTTP API │                              │
│   │tcp.rs   │ │ws.rs    │ │api.rs   │                              │
│   └────┬────┘ └────┬────┘ └────┬────┘                              │
│        │           │           │                                    │
│        └───────────┴───────────┘                                    │
│                    │                                                │
│         ┌──────────┴──────────┐                                     │
│         ▼                     ▼                                     │
│   ┌─────────────┐      ┌─────────────┐                              │
│   │  protocol   │      │  address    │                              │
│   │  (编解码)   │      │  (连接目标) │                              │
│   └─────────────┘      └─────────────┘                              │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 模块依赖关系

```
main.rs
├── server (核心调度)
│   ├── tcp ──────┬──> protocol (VLESS 编解码)
│   ├── ws ───────┤    └──> address (目标连接)
│   └── api ──────┤         └──> socket (TCP 调优)
│       └──> http (请求解析/响应构建)
│
├── config (配置管理)
├── wizard (交互式配置)
├── service (Linux 服务安装)
├── public_ip (公网 IP 获取)
├── tui (终端界面)
├── atomic_write (原子文件写入)
└── version (版本信息)
```

## 2. 核心设计模式

### 2.1 协议多路复用

服务器在单一端口上同时处理多种协议：

```rust
// 通过 peek 检测协议类型
let n = stream.peek(&mut peek_buf).await?;
match detect_protocol(&peek_buf[..n]) {
    ProtocolHint::HttpRequest => handle_http_request(...).await,
    ProtocolHint::WebSocketUpgrade => handle_ws_connection(...).await,
    ProtocolHint::VlessConnection => handle_tcp_connection(...).await,
}
```

**优势**:
- 无需多端口监听
- 防火墙配置简单
- 自动协议识别

### 2.2 零拷贝数据流

```rust
// Bytes::split_to 避免数据复制
let addons = if addons_length > 0 {
    buf.split_to(addons_length as usize)  // 零拷贝切片
} else {
    Bytes::new()
};

// Arc 共享配置，避免每连接深拷贝
user_emails: Arc<HashMap<Uuid, Option<Arc<str>>>>,
```

### 2.3 分层缓冲区策略

为减少内存分配，采用分层缓冲策略：
- **栈上小缓冲区**: 1KB，用于 VLESS 协议头解析（头部通常 < 256 字节）
- **栈上中缓冲区**: 8KB，用于 HTTP 请求头读取
- **堆上大缓冲区**: 64KB（默认），用于 WebSocket 代理传输

```rust
// 栈上小缓冲区（VLESS 头通常 < 256 字节）
let mut small_buf = [0u8; 1024];
let n = stream.read(&mut small_buf).await?;
let header_bytes = Bytes::copy_from_slice(&small_buf[..n]);

// 栈上中缓冲区（HTTP 头通常 < 8KB）
let mut http_buf = [0u8; 8192];
let read_n = stream.read(&mut http_buf).await?;
let header_bytes = Bytes::copy_from_slice(&http_buf[..read_n]);

// 堆上大缓冲区（WebSocket 代理传输）
let mut buffer = vec![0u8; 64 * 1024];

// TCP 代理使用 tokio::io::copy（自带内部缓冲管理）
tokio::io::copy(&mut client_read, &mut target_write).await
```

## 3. 并发模型

### 3.1 Tokio Runtime 配置

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 默认使用多线程调度器
    // worker_threads = CPU 核心数
}
```

### 3.2 每连接一任务

```rust
// 每个连接 spawning 一个独立任务
tokio::spawn(async move {
    if let Err(e) = handle_connection(stream, addr, config, perf_config).await {
        error!("Error handling connection: {}", e);
    }
});
```

### 3.3 优雅关闭机制

使用双层信号通道实现优雅关闭：

- **broadcast 通道**：服务器内部使用，通知所有连接任务停止接受新连接
- **watch 通道**：TUI 模式下，TUI 线程退出时通知服务器关闭

```rust
// 服务器内部 broadcast 通道
let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);

// TUI 模式下的 watch 通道
let (shutdown_tx_watch, shutdown_rx_watch) = tokio::sync::watch::channel(false);

// 主循环同时监听三种信号源
tokio::select! {
    result = server.run() => { ... }
    _ = shutdown => {           // Unix: SIGINT/SIGTERM; Windows: Ctrl+C
        shutdown_tx.send(()).ok();
    }
    _ = flag_check => {         // TUI 退出信号
        shutdown_tx.send(()).ok();
    }
}
```

### 3.4 双向代理并发

```rust
// 两个独立任务处理双向流量
let client_to_target = tokio::spawn(async move {
    tokio::io::copy(&mut client_read, &mut target_write).await
});

let target_to_client = tokio::spawn(async move {
    tokio::io::copy(&mut target_read, &mut client_write).await
});

let _ = tokio::join!(client_to_target, target_to_client);
```

## 4. 错误处理策略

### 4.1 错误传播

使用 `anyhow` 进行上下文丰富的错误传播：

```rust
use anyhow::{anyhow, Result, Context};

async fn handle_connection(...) -> Result<()> {
    let stream = TcpStream::connect(addr).await
        .with_context(|| format!("Failed to connect to {}", addr))?;
    
    if n == 0 {
        return Err(anyhow!("Connection closed by client"));
    }
    Ok(())
}
```

### 4.2 错误分类

| 错误类型 | 处理方式 | 日志级别 |
|----------|----------|----------|
| 连接重置/关闭 | 静默处理，关闭连接 | DEBUG |
| 认证失败 | 记录客户端地址 | WARN |
| 协议错误 | 记录错误详情 | ERROR |
| 目标连接失败 | 记录目标地址 | WARN |
| 内部错误 | 记录堆栈信息 | ERROR |

### 4.3 防御性编程

```rust
// 输入验证
if version != VLESS_VERSION_BETA && version != VLESS_VERSION_RELEASE {
    return Err(anyhow!("Unsupported VLESS version: {}", version));
}

// 长度检查
if buf.len() < 18 {
    return Err(anyhow!("Buffer too short for VLESS request"));
}

// 路径安全检查
if decoded_path.contains("..") || decoded_path.contains('\\') {
    return None;  // 防止路径遍历
}
```

## 5. 日志与监控

### 5.1 日志级别使用规范

| 级别 | 使用场景 |
|------|----------|
| ERROR | 服务启动失败、关键内部错误 |
| WARN | 认证失败、连接异常、超时 |
| INFO | 连接建立/关闭、配置加载、用户操作 |
| DEBUG | 协议解析、数据流向、性能指标 |
| TRACE | 详细字节级调试（开发时使用） |

### 5.2 TUI 日志架构

```
┌─────────────────────────────────────────┐
│           TUI Dashboard                 │
│  ┌─────────────────────────────────┐    │
│  │         Server Status           │    │
│  │  Listening: 0.0.0.0:443         │    │
│  │  Protocol: TCP                  │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │     Scrollable Log View         │    │
│  │  [10:23:45] INFO User auth...   │    │
│  │  [10:23:46] DEBUG Proxy conn... │    │
│  │  [10:23:50] INFO Conn closed    │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

实现机制：
- 自定义 `tracing::Layer` 发送日志到 `mpsc` 通道
- TUI 线程接收日志并渲染
- 最大保留 1000 条日志条目

## 6. 安全配置

### 6.1 用户认证

- UUID 白名单机制
- 配置时自动生成随机 UUID
- 运行时 O(1) HashSet 查询

### 6.2 HTTP 安全头

所有 HTTP 响应包含以下安全头：

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Referrer-Policy: no-referrer
Content-Security-Policy: default-src 'none'; style-src 'self' 'unsafe-inline' 'unsafe-hashes'; script-src 'none'
```

### 6.3 配置文件权限

```rust
// Unix: 600 (rw-------)
atomic_write_file_with_perms(&config_path, &json, 0o600)?;
```

### 6.4 输入验证

- HTTP Content-Length 限制 (< 1MB，仅 WebSocket 握手阶段检查)
- WebSocket 路径格式验证（必须与配置匹配）
- UUID 格式验证
- 路径遍历防护（拒绝含 `..` 或 `\` 的路径）

## 7. 性能优化

### 7.1 TCP 调优

```rust
// 启用 TCP_NODELAY 降低延迟
stream.set_nodelay(true)?;

// Keepalive 防止 NAT 超时
let keepalive = TcpKeepalive::new()
    .with_time(Duration::from_secs(60))
    .with_interval(Duration::from_secs(10));
socket.set_tcp_keepalive(&keepalive)?;
```

### 7.2 内存优化

| 优化项 | 实现方式 |
|--------|----------|
| 内存分配器 | mimalloc（替代默认分配器） |
| 零拷贝解析 | Bytes crate 切片共享 |
| 栈上小缓冲 | 1KB 栈数组用于 VLESS 头解析，8KB 栈数组用于 HTTP 头 |
| 配置共享 | Arc<T> 避免每连接克隆 |
| TCP 内部缓冲 | tokio::io::copy 自带缓冲管理 |

### 7.3 编译优化

```toml
[profile.release]
lto = "thin"          # 链接时优化
codegen-units = 1     # 单代码生成单元
opt-level = 3         # 最高优化级别
panic = "abort"       # 移除 panic 处理开销
strip = true          # 剥离符号表
```

## 8. 平台适配

### 8.1 条件编译

```rust
// 信号处理
#[cfg(unix)]
let mut sigint = signal(SignalKind::interrupt())?;

#[cfg(not(unix))]
let _ = signal::ctrl_c().await;

// 文件权限
#[cfg(unix)]
std::fs::set_permissions(path, Permissions::from_mode(0o600))?;

// 内存分配器
#[cfg(not(target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

### 8.2 平台特定依赖

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.52", features = [...] }

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[target.'cfg(windows)'.build-dependencies]
winres = "0.1"
embed-resource = "2"
```

## 9. 部署架构

### 9.1 独立部署

```
┌─────────────────┐
│  vless binary   │
│  config.json    │  <-- 同一目录
└─────────────────┘
```

### 9.2 系统服务部署 (Linux)

**systemd 用户服务**:
```
~/.config/systemd/user/vless-rust-serve.service
```

**OpenRC 系统服务**:
```
/etc/init.d/vless-rust-serve
```

### 9.3 容器化部署

静态链接的二进制可在最小容器（如 scratch、Alpine）中运行：

```dockerfile
FROM scratch
COPY vless /vless
COPY config.json /config.json
EXPOSE 443
ENTRYPOINT ["/vless"]
```

## 10. 扩展点

### 10.1 新增传输协议

1. 在 `config.rs` 添加新协议类型
2. 在 `server.rs` 添加协议检测逻辑
3. 实现新的处理器模块（如 `grpc.rs`）
4. 更新 `detect_protocol()` 函数

### 10.2 新增认证方式

1. 扩展 `ServerConfig` 添加认证配置
2. 修改 `authenticate_request()` 支持多种认证
3. 更新配置向导

### 10.3 新增 API 端点

1. 在 `api.rs` 添加路由处理
2. 更新 `http.rs` 添加响应构建器
3. 实现业务逻辑
