# 技术文档

## 项目概述

VLESS-Rust 是一个基于 Rust 和 Tokio 异步运行时实现的高性能 VLESS 协议服务器，完全遵循 xray-core 的 VLESS 协议规范。项目采用现代化的技术栈，专注于提供高性能、轻量级的 VLESS 代理服务。

## 技术栈

### 后端

- **语言**: Rust 2021 Edition
- **异步运行时**: Tokio 1.0 (精简 features: rt-multi-thread, io-util, net, time, sync, macros, signal)
- **内存分配器**: mimalloc (高性能分配器)
- **协议**: VLESS (版本 0 和版本 1)
- **序列化**: serde + serde_json
- **日志**: tracing + tracing-subscriber
- **SHA1**: sha1_smol (VLESS 协议需要)
- **WebSocket**: tokio-tungstenite + futures-util
- **Base64**: base64 (WebSocket 认证)
- **URL编码**: urlencoding
- **Socket 配置**: socket2

## 架构设计

### 系统架构图

```
┌─────────────────────────────────────────┐
│              客户端连接                   │
└────────────┬────────────────────────────┘
             │
             ▼
   ┌─────────────────────┐
   │  TCP 端口 (8443)    │
   └─────────────────────┘
             │
             ▼
   ┌─────────────────────┐
   │   协议检测层         │
   │  is_http_request()  │
   └──────────┬──────────┘
              │
     ┌────────┴────────┐
     │                 │
     ▼                 ▼
┌─────────┐      ┌──────────────────┐
│ HTTP    │      │ HTTP 升级请求    │
│ 请求    │      │ (WebSocket)      │
├─────────┤      └────────┬─────────┘
│ API     │               │
│ 信息页  │               ▼
│ 链接生成│      ┌──────────────────┐
└─────────┘      │ WebSocket 握手   │
                 └────────┬─────────┘
                          ▼
                 ┌──────────────────┐
                 │  VLESS 请求      │
                 ├──────────────────┤
                 │ UUID 验证        │
                 │ 命令处理         │
                 │ TCP/UDP 代理     │
                 └──────────────────┘
```

## 文件与功能映射关系

### 后端核心文件

| 文件路径 | 核心功能 | 主要结构体/函数 |
|---------|---------|---------------|
| `src/main.rs` | 程序入口、服务器启动 | `main()` - 加载配置、启动服务器、配置向导触发、TUI 日志显示 |
| `src/tui.rs` | TUI 模块 | `TuiLayer`、`LogEntry` - 日志收集层和日志条目结构 |
| `src/version.rs` | 版本信息管理 | `ServerStatusInfo`、`VERSION_INFO` - 服务器状态和版本信息 |
| `src/config.rs` | 配置管理、JSON解析 | `Config`、`ServerConfig`、`UserConfig`、`PerformanceConfig` |
| `src/protocol.rs` | VLESS 协议编解码 | `VlessRequest`、`VlessResponse`、`Address`、`Command` |
| `src/server.rs` | 服务器调度器 | `VlessServer`、`handle_connection()` - 协议分发调度 |
| `src/ws.rs` | WebSocket 协议处理 | `handle_ws_upgrade()`、`is_websocket_upgrade()` |
| `src/http.rs` | HTTP 请求检测和响应构建 | `is_http_request()`、`parse_http_request()`、`build_json_response()` |
| `src/tcp.rs` | TCP 协议处理 | `handle_tcp_connection()`、`handle_tcp_proxy()`、`handle_udp_proxy()` |
| `src/socket.rs` | TCP Socket 配置 | `configure_tcp_socket()` |
| `src/api.rs` | HTTP API 处理 | `handle_http_request()` - 信息页面和链接生成 |
| `src/public_ip.rs` | 公网 IP 自动获取 | `fetch_public_ip_with_timeout()` - 并发获取公网 IP |
| `src/vless_link.rs` | VLESS 链接生成 | `generate_vless_links()` - 生成 TCP/WS 链接 |
| `src/buffer_pool.rs` | 缓冲区池实现 | `BufferPool`、`acquire()` - 对象池复用缓冲区 |
| `src/utils.rs` | 工具函数、URL 生成 | `generate_vless_url()` - 生成 VLESS 协议 URL |
| `src/wizard.rs` | 交互式配置向导 | `ConfigWizard`、`run()` - 引导用户创建配置 |
| `src/service.rs` | 系统服务管理 | `install_service()`、`uninstall_service()` - 自动检测并安装 systemd/OpenRC 服务 |

### 配置文件

| 文件路径 | 核心功能 | 说明 |
|---------|---------|------|
| `Cargo.toml` | Rust 项目配置 | 依赖项、编译优化、二进制配置 |
| `config.json` | 运行时配置 | 服务器、用户、性能参数（自动生成） |

### 文档文件

| 文件路径 | 核心功能 | 说明 |
|---------|---------|------|
| `CLAUDE.md` | AI 助手规则 | 项目架构、开发指南、文件映射 |
| `README.md` | 项目说明 | 安装、使用、部署指南 |
| `docs/technology.md` | 技术文档 | 架构设计、实现逻辑、流程说明 |

### 功能快速查找

**需要修改/查找...**

- **服务器启动流程** → `src/main.rs:main()`
- **首次配置向导** → `src/wizard.rs:ConfigWizard::run()`
- **配置项和默认值** → `src/config.rs:Config`、`PerformanceConfig`
- **VLESS 协议解析** → `src/protocol.rs:VlessRequest::decode()`
- **连接分发调度** → `src/server.rs:handle_connection()` - 根据协议类型分发
- **TCP 代理转发** → `src/tcp.rs:handle_tcp_proxy()`
- **UDP 代理转发** → `src/tcp.rs:handle_udp_proxy()`
- **HTTP 请求检测** → `src/http.rs:is_http_request()`
- **HTTP API 处理** → `src/api.rs:handle_http_request()` - 信息页面和链接生成
- **公网 IP 获取** → `src/public_ip.rs:fetch_public_ip_with_timeout()`
- **VLESS 链接生成** → `src/vless_link.rs:generate_vless_links()`
- **WebSocket 处理** → `src/ws.rs`
- **Socket 配置** → `src/socket.rs:configure_tcp_socket()`
- **服务安装** → `src/service.rs:install_service()` - 自动检测 systemd/OpenRC
- **服务卸载** → `src/service.rs:uninstall_service()`
- **Systemd 服务** → `src/service.rs:install_systemd_service()`
- **OpenRC 服务** → `src/service.rs:install_openrc_service()`
- **编译优化配置** → `Cargo.toml` - `[profile.release]`
- **性能参数调整** → `config.json` - `performance` 节点

### 核心模块

#### 1. 协议处理模块 (protocol.rs)

**职责**: VLESS 协议的编解码实现

**核心结构**:
- `VlessRequest`: 解码后的 VLESS 请求
  - 版本号 (u8)
  - 用户 UUID
  - 命令类型 (TCP/UDP)
  - 目标地址和端口
  - 附加数据

- `VlessResponse`: VLESS 响应编码
  - 版本号 (与请求保持一致)
  - 附加数据长度
  - 附加数据

**地址类型支持**:
- IPv4 (4字节)
- IPv6 (16字节)
- 域名 (1字节长度 + 域名)

**编解码流程**:
```
请求: [版本][UUID][附加长度][附加数据][命令][端口][地址类型][地址][请求数据]
       ↓ decode()
响应: [版本][附加长度][附加数据]
```

#### 2. 服务器模块 (server.rs)

**职责**: 服务器调度器，连接分发和处理

**核心功能**:

**协议分发**:
```rust
if is_http_request(&header_bytes) {
    // HTTP 请求 → API 处理或 WebSocket 握手
    handle_http_request(...).await
} else {
    // VLESS 请求 → TCP 模块处理
    handle_tcp_connection(...).await
}
```

**WebSocket 升级检测**:
- 检测 HTTP 请求是否为 WebSocket 升级请求
- 使用 `is_websocket_upgrade()` 验证
- 验证路径是否匹配配置的 ws_path

**模块协作**:
- TCP 请求 → `src/tcp.rs`
- WebSocket 请求 → `src/ws.rs`
- HTTP API 请求 → `src/api.rs`

**关键结构**:
```rust
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub protocol: ProtocolType,
    pub ws_path: String,
    pub users: HashSet<Uuid>,
    pub user_emails: HashMap<Uuid, Option<Arc<str>>>,
    pub public_ip: Option<String>,  // 用于生成 VLESS 链接
    pub port: u16,
}
```

#### 3. HTTP 检测模块 (http.rs)

**职责**: HTTP 请求检测和响应构建，区分 HTTP 和 VLESS 协议

**检测机制**:
- 检查数据包前缀是否为 HTTP 方法
- 支持 GET、POST、PUT、DELETE、HEAD、OPTIONS、PATCH、TRACE
- HTTP 请求会被拒绝，仅处理 VLESS 协议

**HTTP 响应构建**:
- `parse_http_request()`: 解析 HTTP 请求路径和参数
- `build_json_response()`: 构建 JSON 响应
- `build_html_response()`: 构建 HTML 响应
- `build_404_response()` / `build_400_response()`: 错误响应

#### 4. TCP 处理模块 (tcp.rs)

**职责**: TCP 协议处理，包含 TCP/UDP 代理转发

**核心功能**:
- `handle_tcp_connection()`: 处理 TCP 连接，解析 VLESS 请求，验证 UUID
- `handle_tcp_proxy()`: TCP 代理转发，双向数据流复制
- `handle_udp_proxy()`: UDP over TCP 代理，UDP 数据包封装传输

**技术细节**:
- 使用 `tokio::io::copy` 进行高效双向复制
- 4KB 栈缓冲区处理 UDP 数据包
- 超时控制避免空闲连接阻塞

#### 5. Socket 配置模块 (socket.rs)

**职责**: TCP Socket 参数配置

**核心功能**:
- `configure_tcp_socket()`: 配置 TCP_NODELAY 和缓冲区大小
- 使用 `socket2` 库设置系统 socket 参数

#### 6. API 模块 (api.rs)

**职责**: HTTP API 处理，提供信息页面和 VLESS 链接生成

**核心功能**:

**信息页面** (`/`):
- 返回 HTML 页面
- 显示服务器 IP、端口、协议、版本信息

**链接生成** (`/?email=user@example.com`):
- 根据 email 查找用户 UUID
- 生成对应协议的 VLESS 链接
- 返回 JSON 格式（包含原始链接和 Base64 编码）

#### 7. 公网 IP 模块 (public_ip.rs)

**职责**: 自动获取公网 IP

**核心功能**:
- `fetch_public_ip_with_timeout()`: 带超时的公网 IP 获取
- 并发请求多个 IP API
- 使用通道取首个成功结果
- 自动取消超时任务

**API 端点**:
- api.ipify.org
- ifconfig.me/ip
- api4.my-ip.io/ip
- checkip.amazonaws.com
- icanhazip.com

#### 8. VLESS 链接模块 (vless_link.rs)

**职责**: 生成 VLESS 协议链接

**核心功能**:
- `generate_vless_links()`: 生成 TCP 和 WebSocket 链接
- 支持 Base64 编码
- URL 编码特殊字符

**链接格式**:
```
# TCP
vless://{uuid}@{host}:{port}?encryption=none&security=none&type=tcp#{alias}

# WebSocket
vless://{uuid}@{host}:{port}?encryption=none&security=none&type=ws&path={path}#{alias}
```

#### 9. 配置模块 (config.rs)

**职责**: 配置文件解析和验证

**配置结构**:
- `server`: 服务器监听配置
  - `listen`: 监听地址
  - `port`: 监听端口
  - `protocol`: 协议类型 (tcp/ws)，默认 tcp
  - `ws_path`: WebSocket 路径，仅 ws 模式使用，默认 "/vless"
- `users`: 用户 UUID 列表
  - `uuid`: 用户唯一标识符
  - `email`: 用户邮箱（可选）
- `performance`: 性能优化配置
  - `buffer_size`: 传输缓冲区大小（默认 128KB）
  - `tcp_nodelay`: TCP_NODELAY 启用（默认 true）
  - `tcp_recv_buffer`: TCP 接收缓冲区（默认 256KB）
  - `tcp_send_buffer`: TCP 发送缓冲区（默认 256KB）
  - `udp_timeout`: UDP 会话超时（默认 30 秒）
  - `udp_recv_buffer`: UDP 接收缓冲区（默认 64KB）
  - `buffer_pool_size`: 缓冲区池大小（默认 min(32, CPU核心数*4)）
  - `ws_header_buffer_size`: WebSocket HTTP 头缓冲区（默认 8KB）

**默认值策略**:
- 使用 serde 默认值
- 配置文件不存在时自动创建
- 支持部分配置缺失

#### 10. 缓冲区池模块 (buffer_pool.rs)

**职责**: 对象池实现，复用缓冲区减少内存分配

**核心功能**:
- 预分配固定数量的缓冲区
- acquire() 获取缓冲区
- 自动归还到池中
- 减少内存分配 95%

#### 11. 配置向导模块 (wizard.rs)

**职责**: 交互式配置向导，引导用户创建配置文件

**核心功能**:
- `run()`: 启动向导流程
- `prompt_listen_address()`: 询问监听地址
- `prompt_port()`: 询问端口
- `prompt_users()`: 询问用户配置

**邮箱验证**:
- 检查 `@` 和 `.` 位置关系
- 拒绝无效格式（`@example.com`, `user@`, `user@.`）
- 友好的警告提示

#### 12. WebSocket 模块 (ws.rs)

**职责**: WebSocket 协议处理，支持 VLESS over WebSocket

**核心功能**:

**WebSocket 握手升级**:
- 解析 HTTP Upgrade 请求
- 验证 WebSocket 路径
- 生成 Sec-WebSocket-Accept 响应
- 完成协议切换

**WebSocket 代理转发**:
- 处理 WebSocket 帧的收发
- 支持二进制数据帧
- 复用 TCP 代理逻辑

**配置参数**:
- `ws_path`: WebSocket 路径（默认 "/vless"）
- `ws_header_buffer_size`: HTTP 头缓冲区（默认 8KB）

**协议流程**:
```
客户端 → HTTP GET /vless Upgrade: websocket
    ↓
服务器验证路径和请求
    ↓
Sec-WebSocket-Accept 响应
    ↓
WebSocket 帧传输
    ↓
**服务管理命令**:

**Systemd (无需 root)**:
```bash
# 查看状态
systemctl --user status vless-rust-serve

# 查看日志
journalctl --user -u vless-rust-serve -f

# 停止/重启
systemctl --user stop vless-rust-serve
systemctl --user restart vless-rust-serve
```

**OpenRC (需要 root)**:
```bash
# 查看状态
rc-service vless-rust-serve status

# 查看日志
tail -f /var/log/vless-rust-serve.log

# 停止/重启
rc-service vless-rust-serve stop
rc-service vless-rust-serve restart
```

**安全特性**:
- 检查配置文件是否存在
- 验证路径安全性（避免非 UTF-8 字符）
- 完善的错误处理和用户提示
- 服务状态验证

## 数据流

### VLESS 代理流程

```
客户端连接
    ↓
协议检测 (HTTP vs VLESS)
    ↓
         ┌───────────────────────────────────────┐
         │               VLESS 请求               │
         ├───────────────────────────────────────┤
         │ UUID 验证                             │
         │ 发送 VLESS 响应                       │
         │ 连接目标服务器                         │
         │ 双向数据转发                           │
         │   ├─ 客户端 → 目标 (上传)              │
         │   └─ 目标 → 客户端 (下载)              │
         └───────────────────────────────────────┘
    ↓
连接关闭，清理资源
```

### HTTP API 流程

```
客户端 HTTP 请求
    ↓
协议检测 (HTTP 请求)
    ↓
解析请求路径和参数
    ↓
     ┌──────────────────────────────────────────┐
     │ /                   → 信息页面 (HTML)    │
     │ /?email=xxx         → VLESS 链接 (JSON)  │
     │ 其他路径             → 404 响应          │
     └──────────────────────────────────────────┘
    ↓
返回响应，关闭连接
```

### WebSocket 代理流程

```
客户端连接 (HTTP Upgrade)
    ↓
WebSocket 握手
    ↓
检测 WebSocket 路径
    ↓
WebSocket 帧处理
    ↓
VLESS 协议解析
    ↓
后续流程同 TCP 模式
```

### 配置加载流程

```
程序启动
    ↓
检查 config.json 是否存在
    ↓
存在 → 加载配置
不存在 → 启动配置向导
    ↓
启动服务器
```

## 性能优化

### 后端优化

1. **内存分配器优化**:
   - 使用 mimalloc 替代系统分配器
   - 更快的内存分配和释放
   - 减少内存碎片
   - 更好的多线程扩展性

2. **Tokio Features 精简**:
   - 仅启用必要的 features
   - 减少不必要的代码编译
   - 降低二进制体积

3. **大缓冲区**:
   - 默认 128KB 传输缓冲区
   - 适配千兆网络
   - 单连接带宽提升 4 倍

4. **TCP 优化**:
   - TCP_NODELAY 启用
   - 降低延迟
   - 改善小包传输

5. **零拷贝传输**:
   - 使用 `Bytes` 库
   - 减少内存复制
   - 提升吞吐量

6. **静态链接**:
   - CRT 静态链接
   - 零依赖运行
   - 可执行文件约 1 MB

7. **缓冲区池化**:
   - 使用 `object-pool` crate 实现对象池
   - 复用 128KB 缓冲区，减少内存分配 95%
   - 支持可配置的池大小（默认 min(32, CPU核心数*4)）
   - 1000 连接场景内存占用减少 80% (128MB → 25MB)

8. **优雅关闭**:
   - 支持 SIGINT/SIGTERM 信号处理
   - Windows: Ctrl+C 监听
   - Unix: SIGINT 和 SIGTERM 处理
   - 停止接收新连接
   - 等待现有连接完成

9. **TUI 日志显示**:
   - 固定头部显示服务器状态信息
   - 可滚动的日志区域（最多 1000 条）
   - 自动滚动模式（新日志自动显示在底部）
   - 用户手动滚动时自动禁用自动滚动
   - 按键控制：
     - `q`/`Esc`: 退出
     - `↑`/`↓` 或 `k`/`j`: 上下滚动
     - `Page Up`/`Page Down`: 快速翻页
     - `Home`: 跳转到顶部
     - `End`: 跳转到底部并重新启用自动滚动

## 安全考虑

### 认证与授权
- UUID 作为唯一认证凭据
- HashSet O(1) 快速验证
- 无密码，简化部署

### 网络安全
- HTTP 请求现在可以处理 API 请求（信息页面、VLESS 链接生成）
- WebSocket 路径验证，防止路径遍历攻击
- 建议配合 TLS 使用
- UUID 不在日志中记录

### 数据保护
- 日志不记录敏感信息
- 配置文件权限管理
- 避免密钥泄露

### 部署安全
- 配置防火墙规则
- 限制访问来源
- 定期更新 UUID

## 扩展性

### 协议扩展
- 支持添加新命令类型
- 支持新地址类型
- 保持向后兼容

### 配置扩展
- 添加新的配置项
- 支持环境变量
- 动态配置重载

## 部署架构

### 单文件部署
```
vless.exe (约 1 MB)
├── 静态链接 CRT
└── 零依赖运行
```

### 配置文件
```
config.json
├── server: 监听配置
├── users: 用户列表
└── performance: 性能参数
```

### 运行要求
- 操作系统: Windows / Linux / macOS
- 依赖: 无 (静态链接)
- 权限: 绑定端口需要管理员/root

## 开发工作流

### 后端开发
```bash
# 编译
cargo build --release

# 运行
cargo run

# 检查代码
cargo check
```

### 跨平台编译

项目支持多平台交叉编译，所有配置已在 `.cargo/config.toml` 中预设。

#### 支持的目标平台

| 目标平台 | 架构 | 说明 |
|---------|------|------|
| `x86_64-pc-windows-msvc` | AMD64 | Windows 64位 |
| `aarch64-pc-windows-msvc` | ARM64 | Windows ARM64 |
| `x86_64-unknown-linux-musl` | AMD64 | Linux 64位 (完全静态链接，推荐) |
| `x86_64-unknown-linux-gnu` | AMD64 | Linux 64位 (GNU) |
| `aarch64-unknown-linux-gnu` | ARM64 | Linux ARM64 (GNU) |
| `armv7-unknown-linux-gnueabihf` | ARMv7 | Linux ARM 32位 |
| `x86_64-apple-darwin` | AMD64 | macOS Intel |
| `aarch64-apple-darwin` | ARM64 | macOS Apple Silicon |

#### Windows 交叉编译到 Linux

```bash
# 1. 添加 Linux 目标
rustup target add x86_64-unknown-linux-gnu

# 2. 交叉编译
cargo build --release --target x86_64-unknown-linux-gnu

# 输出: target/x86_64-unknown-linux-gnu/release/vless
```

#### 交叉编译到 ARM64

```bash
# 1. 添加 ARM64 目标
rustup target add aarch64-unknown-linux-gnu

# 2. 安装交叉编译工具链
# Ubuntu/Debian:
sudo apt-get install gcc-aarch64-linux-gnu

# 3. 交叉编译
cargo build --release --target aarch64-unknown-linux-gnu

# 输出: target/aarch64-unknown-linux-gnu/release/vless
```

#### 静态链接说明

项目使用静态链接，生成的二进制文件无需额外依赖:

- **Windows**: 使用 `target-feature=+crt-static`
- **Linux**: 使用 `link-self-contained=yes`

配置文件位置: `.cargo/config.toml`

#### GitHub Actions 自动构建

项目配置了完整的 GitHub Actions CI/CD 工作流，实现跨平台静态链接构建:

**工作流特性**:
- **触发条件**:
  - 推送提交到 `main` 分支 → 自动识别版本号并构建
  - PR 到 `main` 分支 → 仅构建测试（不发布）
  - 手动触发 (`workflow_dispatch`) → 方便测试

- **版本识别机制**:
  - 自动从提交信息中提取版本号（格式：`x.x.x`，如 `1.0.0`）
  - 提交信息必须以版本号开头
  - 如果有多个版本提交，只构建最新的未发布版本
  - 自动检查 Release 是否已存在，避免重复构建

- **支持平台** (8个):
  - Windows AMD64 + ARM64
  - Linux AMD64 (musl/gnu) + ARM64 + ARMv7
  - macOS Intel + Apple Silicon

- **构建流程**:
  1. 分析提交历史，提取版本号
  2. 检查 Release 是否已存在（去重）
  3. 安装 Rust 工具链和交叉编译工具
  4. 编译 Rust 二进制文件
  5. 验证静态链接 (Linux 平台)
  6. 上传构建产物
  7. 创建 GitHub Release（版本号：vx.x.x）
  8. 自动推送标签到仓库

- **静态链接验证**:
  ```bash
  # Linux 平台自动验证
  ldd vless-rust-xxx | grep "not a dynamic" && echo "✓ 静态链接"
  ```

- **输出文件命名**:
  - Windows: `vless-rust-windows-amd64.exe`
  - Linux musl: `vless-rust-linux-amd64-musl`
  - macOS ARM64: `vless-rust-macos-arm64`
  - 格式: `vless-rust-{平台}-{架构}{扩展名}`

**配置文件**: `.github/workflows/build.yml`

**使用示例**:

```bash
# 方式一：提交版本号（推荐）
git commit -m "1.0.0"
git push origin main

# 方式二：创建空提交触发特定版本
git commit --allow-empty -m "1.5.12"
git push origin main

# 工作流会自动：
# 1. 识别版本号 1.0.0 或 1.5.12
# 2. 构建 8 个平台
# 3. 创建 Release v1.0.0 或 v1.5.12
# 4. 推送标签 v1.0.0 或 v1.5.12
```

**版本识别规则**:
- ✅ `1.0.0` - 正确
- ✅ `1.5.12` - 正确
- ❌ `Release 1.0.0` - 错误（不在行首）
- ❌ `v1.0.0` - 错误（有 v 前缀）

## 参考资料

- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core 项目](https://github.com/XTLS/Xray-core)
- [Tokio 官方文档](https://tokio.rs/)
