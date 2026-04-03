# 技术文档

## 项目概述

VLESS-Rust 是基于 Rust 和 Tokio 实现的高性能 VLESS 协议服务器，遵循 xray-core 协议规范，支持 TCP 和 WebSocket 传输。

## 技术栈

- **语言**: Rust 2021 Edition
- **异步运行时**: Tokio 1.0 (rt-multi-thread)
- **内存分配器**: mimalloc（高性能，musl 除外）
- **协议**: VLESS (v0/v1)
- **错误处理**: anyhow
- **序列化**: serde + serde_json
- **日志**: tracing + tracing-subscriber
- **WebSocket**: tokio-tungstenite
- **TUI**: ratatui + crossterm

## 架构设计

### 系统架构

```
客户端连接 → TCP端口 → 协议检测层 → [HTTP请求 | VLESS请求]
                                     ↓
                              HTTP API / WebSocket握手 / VLESS处理
                                     ↓
                              UUID验证 → 目标连接 → 双向转发
```

### 模块结构

```
src/
├── main.rs          # 入口点，TUI，信号处理
├── lib.rs           # 库导出
├── server.rs        # 连接调度器
├── protocol.rs      # VLESS 协议编解码
├── tcp.rs           # TCP 协议处理
├── ws.rs            # WebSocket 处理
├── http.rs          # HTTP 检测
├── config.rs        # 配置解析
├── api.rs           # HTTP API 端点
├── address.rs       # DNS 解析
├── socket.rs        # TCP socket 配置
├── wizard.rs        # 交互式配置
├── vless_link.rs    # VLESS 链接生成
├── public_ip.rs     # IP 检测
├── service.rs       # Linux 服务管理
├── atomic_write.rs  # 原子文件写入
├── tui.rs           # 终端 UI
├── version.rs       # 版本显示
└── version_info.rs  # 生成常量
```

## 核心模块

| 模块 | 文件 | 核心功能 |
|------|------|---------|
| **协议处理** | `protocol.rs` | VLESS 协议编解码，支持 IPv4/IPv6/域名 |
| **服务器调度** | `server.rs` | 连接分发、协议检测、优雅关闭 |
| **TCP 处理** | `tcp.rs` | TCP/UDP 代理转发，双向数据流 |
| **WebSocket** | `ws.rs` | WebSocket 握手升级和帧处理 |
| **HTTP API** | `api.rs` | 信息页面展示、VLESS 链接生成 |
| **配置管理** | `config.rs` | JSON 配置解析和验证 |
| **地址解析** | `address.rs` | 统一地址解析，支持 DNS |
| **服务管理** | `service.rs` | systemd/OpenRC 服务管理 |

## 辅助模块

| 模块 | 文件 | 核心功能 |
|------|------|---------|
| **HTTP 工具** | `http.rs` | HTTP 请求检测、路径解析、响应构建 |
| **链接生成** | `vless_link.rs` | VLESS 链接生成、Base64 编码 |
| **TUI 日志** | `tui.rs` | tracing Layer、日志通道 |
| **配置向导** | `wizard.rs` | 交互式配置、UUID 生成 |
| **Socket 配置** | `socket.rs` | TCP 参数优化、缓冲区配置 |
| **原子写入** | `atomic_write.rs` | 安全文件写入、权限控制 |
| **公网 IP** | `public_ip.rs` | 多 API 并发、自动获取 |
| **版本显示** | `version.rs` | 状态横幅、版本信息 |
| **入口点** | `main.rs` | 参数解析、服务管理、信号处理 |

## 协议实现

### VLESS 协议格式

支持版本 0 (Beta) 和版本 1 (Release)：

```
请求: [版本: 1字节] [UUID: 16字节] [Addons长度: 1字节] [Addons数据] [命令: 1字节] [端口: 2字节] [地址]
响应: [版本: 1字节] [Addons长度: 1字节] [Addons数据]
```

### 地址类型

| 类型 | 编码 |
|------|------|
| IPv4 | 4 字节 |
| IPv6 | 16 字节 |
| Domain | 1字节长度 + 域名数据 |

### 命令类型

| 命令 | 值 | 说明 |
|------|-----|------|
| TCP | 1 | TCP 代理 |
| UDP | 2 | UDP over TCP |
| Mux | 3 | 多路复用（未实现） |

## 核心流程

### VLESS 代理流程
1. 协议检测（HTTP vs VLESS）
2. UUID 验证
3. VLESS 响应发送
4. 目标服务器连接
5. 双向数据转发（上传/下载）

### WebSocket 流程
1. HTTP Upgrade 检测
2. WebSocket 路径验证
3. 握手响应（Sec-WebSocket-Accept）
4. WebSocket 帧传输
5. VLESS 协议处理

### HTTP API 流程
- `/` → 服务器信息页面（HTML）
- `/?email=xxx` → VLESS 链接生成（JSON）

## 性能优化

### 缓冲区配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| buffer_size | 64KB | 传输缓冲区 |
| tcp_recv_buffer | 128KB | TCP 接收缓冲 |
| tcp_send_buffer | 128KB | TCP 发送缓冲 |
| udp_recv_buffer | 64KB | UDP 接收缓冲 |
| ws_header_buffer_size | 8KB | WebSocket 头缓冲 |

### 系统优化
- **mimalloc**: 高性能内存分配器
- **TCP_NODELAY**: 禁用 Nagle 算法，降低延迟
- **零拷贝**: 使用 Bytes 库减少内存复制
- **Arc 共享**: 配置和用户数据共享

### 编译优化
- LTO thin 优化
- 代码生成单元优化（codegen-units = 1）
- 静态链接 CRT（Windows）
- 二进制文件约 1 MB

### 运行优化
- Tokio 多线程运行时
- 异步 I/O 高效并发
- 优雅关闭信号处理

## 配置结构

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443,
    "protocol": "tcp",
    "ws_path": "/vless"
  },
  "users": [
    { "uuid": "...", "email": "user@example.com" }
  ],
  "performance": {
    "buffer_size": 65536,
    "tcp_nodelay": true,
    "tcp_recv_buffer": 131072,
    "tcp_send_buffer": 131072
  }
}
```

配置向导：首次运行时自动启动交互式配置向导，引导用户完成配置创建。

## 安全特性

- UUID 作为唯一认证凭据
- WebSocket 路径验证防路径遍历
- HTTP 请求安全检查
- 配置文件 600 权限
- 原子文件写入
- 不记录敏感信息日志

## 服务管理

### Systemd（无需 root）

```bash
./vless --init      # 安装服务
./vless --remove    # 卸载服务
```

### OpenRC（需要 root）

```bash
sudo ./vless --init
sudo ./vless --remove
```

## 平台支持

| 平台 | 支持 |
|------|------|
| Windows 7+ | ✓ |
| Linux (glibc/musl) | ✓ |
| macOS 10.15+ | ✓ |
| x86_64 | ✓ |
| ARM64 | ✓ |
| ARMv7 | ✓ |

## 构建配置

### Release 优化

```toml
[profile.release]
lto = "thin"
codegen-units = 1
opt-level = 3
panic = "abort"
strip = true
```

### 平台特定

- Windows: 资源文件嵌入
- Unix: signal 处理（SIGINT/SIGTERM）
- musl: 禁用 mimalloc

## 扩展能力

- 支持新命令类型扩展
- 支持新地址类型
- 模块化设计易扩展
- 配置项可灵活调整

## 参考资料

- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core 项目](https://github.com/XTLS/Xray-core)
- [Tokio 官方文档](https://tokio.rs/)
