# 技术规范 (Technical Specification)

> 定义 "What it is" — 本项目是什么，包含哪些功能

## 1. 项目概述

VLESS-Rust 是一个高性能的 VLESS 代理协议服务器，使用 Rust 语言编写。它实现了 VLESS 协议的 TCP 和 WebSocket 传输模式，提供安全的网络代理服务。

### 1.1 核心功能

- **VLESS 协议支持**: 实现 VLESS 协议版本 0 (Beta) 和版本 1 (Release)
- **双传输模式**: 支持原始 TCP 模式和 WebSocket 模式（可穿透防火墙）
- **用户认证**: 基于 UUID 的用户身份验证
- **HTTP API**: 提供信息页面和 VLESS 链接生成功能
- **TUI 仪表盘**: 实时终端界面显示服务器状态和日志
- **Linux 服务管理**: 支持 systemd 和 OpenRC 服务安装

### 1.2 技术栈

| 组件 | 技术选择 | 说明 |
|------|----------|------|
| 运行时 | Tokio | 异步运行时，多线程 |
| 内存分配器 | mimalloc | 高性能内存分配（除 musl 目标） |
| 序列化 | serde + serde_json | 配置文件和 API 响应 |
| WebSocket | tokio-tungstenite | WebSocket 协议实现 |
| TUI | ratatui + crossterm | 终端用户界面 |
| HTTP 客户端 | reqwest | 公网 IP 获取 |
| 日志 | tracing | 结构化日志 |
| UUID | uuid | UUID v4 生成和解析 |
| 字节操作 | bytes | 零拷贝字节处理 |
| Socket 配置 | socket2 | TCP 参数调优 |

## 2. 数据模型

### 2.1 配置文件结构

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 443,
    "protocol": "tcp",
    "ws_path": "/vless"
  },
  "users": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "email": "user@example.com"
    }
  ],
  "performance": {
    "buffer_size": 65536,
    "tcp_recv_buffer": 131072,
    "tcp_send_buffer": 131072,
    "tcp_nodelay": true,
    "udp_timeout": 30,
    "udp_recv_buffer": 65536,
    "buffer_pool_size": 32,
    "ws_header_buffer_size": 8192
  }
}
```

### 2.2 核心数据结构

#### ProtocolType
```rust
pub enum ProtocolType {
    Tcp,
    WebSocket,
}
```

#### VlessRequest (协议请求)
```rust
pub struct VlessRequest {
    pub version: u8,           // 0 或 1
    pub uuid: Uuid,            // 16 字节用户标识
    pub addons_length: u8,     // 附加数据长度
    pub addons: Bytes,         // 附加数据（保留字段）
    pub command: Command,      // TCP=1, UDP=2, Mux=3
    pub port: u16,             // 目标端口
    pub address: Address,      // 目标地址（IPv4/IPv6/Domain）
}
```

#### ServerConfig (运行时配置)
```rust
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub protocol: ProtocolType,
    pub ws_path: String,
    pub users: HashSet<Uuid>,
    pub user_emails: Arc<HashMap<Uuid, Option<Arc<str>>>>,
    pub public_ip: Option<String>,
    pub port: u16,
}
```

## 3. 核心业务流程

### 3.1 连接处理流程

```
1. 监听端口接受 TCP 连接
   ↓
2. 协议检测 (peek 前 1024 字节)
   ├── HTTP 请求？
   │   ├── WebSocket 升级？→ WebSocket 处理流程
   │   └── 普通 HTTP → API 处理
   └── 非 HTTP → VLESS 原始 TCP 处理
   ↓
3. VLESS 协议解析
   ├── 解码请求头 (version, uuid, command, address, port)
   ├── 验证 UUID 是否在白名单
   └── 发送响应头
   ↓
4. 建立到目标服务器的连接
   ↓
5. 双向数据转发 (tokio::io::copy 或 WebSocket 帧转换)
   ↓
6. 连接关闭，清理资源
```

### 3.2 WebSocket 处理流程

```
1. 接收 HTTP 升级请求
   ↓
2. 验证 WebSocket 路径
   ↓
3. 完成 WebSocket 握手 (Sec-WebSocket-Key + SHA1 + Base64)
   ↓
4. 读取第一条 WebSocket 消息作为 VLESS 请求头
   ↓
5. 验证 UUID，发送 VLESS 响应
   ↓
6. 建立目标连接，开始帧转换代理
```

### 3.3 首次启动流程

```
1. 检查命令行参数
   ├── --init → 安装系统服务
   ├── --remove → 卸载系统服务
   └── 无参数 → 启动服务器
   ↓
2. 加载/创建配置文件
   ├── 配置文件存在 → 直接加载
   └── 配置文件不存在 → 启动交互式配置向导
   ↓
3. 获取公网 IP（并发查询多个 API）
   ↓
4. 初始化日志系统（TUI 模式或传统模式）
   ↓
5. 启动 VLESS 服务器
```

## 4. API 定义

### 4.1 HTTP API 端点

服务器在监听端口同时提供 HTTP 服务。

#### GET / — 信息页面

返回 HTML 格式的服务器状态页面。

**响应**: `text/html; charset=utf-8`

显示内容：
- 产品名称和版本
- 作者信息
- 服务器 IP 和端口
- 协议类型
- WebSocket 路径（如适用）
- API 使用说明

#### GET /?email={email} — 获取 VLESS 链接

根据用户邮箱返回 VLESS 连接链接。

**参数**:
- `email` (query, required): 用户配置的邮箱地址

**成功响应** (200):

TCP 模式:
```json
{
  "tcp": "vless://uuid@host:port?encryption=none&security=none&type=tcp#alias",
  "tcp_b64": "base64_encoded_link"
}
```

WebSocket 模式:
```json
{
  "ws": "vless://uuid@host:port?encryption=none&security=none&type=ws&path=%2Fws#alias",
  "ws_b64": "base64_encoded_link"
}
```

**错误响应** (200，JSON 格式):
```json
{
  "error": "User not found"
}
```

### 4.2 VLESS 协议

#### 请求格式

```
+------+-------+----------+----------+---------+------+----------+
| 1B   | 16B   | 1B       | Variable | 1B      | 2B   | Variable |
+------+-------+----------+----------+---------+------+----------+
| Ver  | UUID  | Addons   | Addons   | Cmd     | Port | Address  |
|      |       | Len      | Data     |         |      |          |
+------+-------+----------+----------+---------+------+----------+
```

- **Ver**: 协议版本 (0 或 1)
- **UUID**: 16 字节用户标识符
- **Addons Len**: 附加数据长度（当前版本为 0）
- **Cmd**: 命令类型 (1=TCP, 2=UDP, 3=Mux)
- **Port**: 目标端口（大端）
- **Address**: 目标地址
  - 类型 1 (IPv4): 1B type + 4B address
  - 类型 2 (Domain): 1B type + 1B len + domain
  - 类型 3 (IPv6): 1B type + 16B address

#### 响应格式

```
+------+----------+----------+
| 1B   | 1B       | Variable |
+------+----------+----------+
| Ver  | Addons   | Addons   |
|      | Len      | Data     |
+------+----------+----------+
```

## 5. 配置规范

### 5.1 配置项说明

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `server.listen` | string | "0.0.0.0" | 监听地址 |
| `server.port` | u16 | 443 | 监听端口 |
| `server.protocol` | enum | "tcp" | 传输协议: "tcp" 或 "ws" |
| `server.ws_path` | string | "/vless" | WebSocket 路径（仅 WS 模式） |
| `users[].uuid` | string | 必填 | 用户 UUID (8-4-4-4-12 格式) |
| `users[].email` | string | 可选 | 用户邮箱（用于识别，字段可省略） |
| `performance.buffer_size` | usize | 65536 | 传输缓冲区大小 |
| `performance.tcp_recv_buffer` | usize | 131072 | TCP 接收缓冲区 |
| `performance.tcp_send_buffer` | usize | 131072 | TCP 发送缓冲区 |
| `performance.tcp_nodelay` | bool | true | 启用 TCP_NODELAY |
| `performance.udp_timeout` | u64 | 30 | UDP 会话超时（秒） |
| `performance.udp_recv_buffer` | usize | 65536 | UDP 接收缓冲区 |
| `performance.buffer_pool_size` | usize | min(64, CPU核心数×8) | 缓冲区池大小（动态计算） |
| `performance.ws_header_buffer_size` | usize | 8192 | WebSocket 头缓冲区 |

## 6. 构建规范

### 6.1 支持平台

| 平台 | 目标三元组 | 构建工具 |
|------|------------|----------|
| Windows x64 | x86_64-pc-windows-msvc | cargo |
| Linux x64 | x86_64-unknown-linux-musl | cargo |
| Linux ARM64 | aarch64-unknown-linux-musl | cargo-zigbuild |
| Linux ARMv7 | armv7-unknown-linux-musleabihf | cargo-zigbuild |

### 6.2 构建配置

- **静态链接**: 所有平台使用 `+crt-static`
- **优化级别**: Release 模式使用 `opt-level = 3`
- **LTO**: 启用 `lto = "thin"`
- **代码生成单元**: `codegen-units = 1`（最佳优化）
- **Panic 处理**: `panic = "abort"`
- **符号剥离**: `strip = true`

## 7. 版本规范

版本号遵循语义化版本控制 (SemVer)：`MAJOR.MINOR.PATCH`

- **MAJOR**: 不兼容的协议变更
- **MINOR**: 功能添加，向后兼容
- **PATCH**: Bug 修复，向后兼容

版本号定义在 `Cargo.toml` 的 `[package]` 部分。
