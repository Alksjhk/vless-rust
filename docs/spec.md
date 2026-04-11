# 技术规格书

> 定义系统“是什么”，描述当前版本已经提供的能力、约束、数据模型与接口契约。

## 1. 文档范围

本文档描述 `vless-rust` 当前代码实现所对应的技术规格，覆盖：

- 技术栈
- 配置与数据模型
- 核心业务逻辑
- HTTP API 定义
- VLESS 协议支持边界
- 平台与构建规格

当前版本信息：

- 包名：`vless-rust`
- 当前版本：`1.7.9`
- 运行形态：单进程、单二进制、本地配置文件驱动

## 2. 产品定义

### 2.1 产品目标

`vless-rust` 是一个轻量级 VLESS 服务端，目标是：

- 提供易部署的 TCP / WebSocket 代理服务
- 在不引入数据库的前提下完成本地配置与用户管理
- 通过较低运行时开销处理高并发网络连接
- 为用户提供简单的状态展示与链接分发能力

### 2.2 功能边界

当前版本已支持：

- VLESS 协议版本 `0` 与 `1`
- `TCP` 传输
- `WebSocket` 传输
- 用户 UUID 白名单认证
- 通过邮箱查询并生成 VLESS 链接
- 首次启动配置向导
- TUI 模式与传统日志模式
- Linux `systemd` / `OpenRC` 服务安装

当前版本未支持：

- TLS / WSS
- XTLS / Reality
- Mux 多路复用
- WebSocket 下的 UDP 代理
- 管理型 API
- 配置热重载
- 数据库存储

## 3. 技术栈

| 类别 | 选型 | 用途 |
| --- | --- | --- |
| 语言 | Rust 2021 | 系统实现 |
| 异步运行时 | Tokio | 网络 I/O、任务调度、信号处理 |
| 内存分配 | mimalloc | 非 musl 目标下的全局分配器 |
| 协议编解码 | bytes | VLESS 报文零拷贝切片处理 |
| 序列化 | serde、serde_json | 配置文件与 JSON 响应 |
| WebSocket | tokio-tungstenite | WebSocket 流处理 |
| 哈希与编码 | sha1_smol、base64、urlencoding | WebSocket 握手与链接编码 |
| 日志 | tracing、tracing-subscriber | 结构化日志 |
| TUI | ratatui、crossterm | 终端界面 |
| 网络调优 | socket2 | TCP 参数设置 |
| 公网 IP 获取 | reqwest | 调用外部 HTTP 服务 |
| 用户目录探测 | dirs | Linux 服务安装路径定位 |
| 测试辅助 | tempfile | 原子写入测试 |

## 4. 数据模型

### 4.1 持久化模型

当前项目没有数据库，持久化仅依赖本地 `config.json`。

因此：

- 数据库 Schema：无
- 持久化来源：本地 JSON 配置文件
- 一致性策略：配置启动时全量加载，运行中以内存副本为准

### 4.2 配置文件 Schema

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
    "buffer_pool_size": 64,
    "ws_header_buffer_size": 8192
  }
}
```

### 4.3 配置字段定义

#### `server`

| 字段 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `listen` | `string` | 无 | 监听地址，如 `0.0.0.0` |
| `port` | `u16` | 无 | 监听端口 |
| `protocol` | `tcp \| ws` | `tcp` | 主传输模式 |
| `ws_path` | `string` | `/vless` | WebSocket 路径 |

#### `users[]`

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `uuid` | `string` | 是 | 用户 UUID |
| `email` | `string \| null` | 否 | 用户标识，用于链接查询 |

#### `performance`

| 字段 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `buffer_size` | `usize` | `65536` | 主传输缓冲区大小 |
| `tcp_recv_buffer` | `usize` | `131072` | TCP 接收缓冲区 |
| `tcp_send_buffer` | `usize` | `131072` | TCP 发送缓冲区 |
| `tcp_nodelay` | `bool` | `true` | 是否启用 `TCP_NODELAY` |
| `udp_timeout` | `u64` | `30` | UDP 会话超时，单位秒 |
| `udp_recv_buffer` | `usize` | `65536` | UDP 接收缓冲区 |
| `buffer_pool_size` | `usize` | `min(64, CPU*8)` | 预估缓冲池规模配置 |
| `ws_header_buffer_size` | `usize` | `8192` | WebSocket HTTP 头大小上限 |

### 4.4 运行时核心结构

#### `ProtocolType`

```rust
pub enum ProtocolType {
    Tcp,
    WebSocket,
}
```

#### `ServerConfig`

运行时只保留代理逻辑所需字段：

- 绑定地址
- 传输协议
- WebSocket 路径
- 用户 UUID 集合
- UUID 到邮箱的映射
- 公网 IP
- 服务端口

#### `VlessRequest`

```rust
pub struct VlessRequest {
    pub version: u8,
    pub uuid: Uuid,
    pub addons_length: u8,
    pub addons: Bytes,
    pub command: Command,
    pub port: u16,
    pub address: Address,
}
```

说明：

- `addons` 已解析但当前不参与业务处理
- `command` 支持 `Tcp`、`Udp`、`Mux`
- 当前只有 `Tcp` 与部分 `Udp` 有业务实现

## 5. 核心业务逻辑

### 5.1 启动流程

1. 读取命令行参数
2. 处理 `--init` 或 `--remove`
3. 加载指定配置文件，默认 `config.json`
4. 若配置不存在，则启动交互式向导并原子写入配置
5. 尝试获取公网 IP
6. 根据 `--no-tui` 决定进入 TUI 或传统日志模式
7. 构建 `ServerConfig` 并启动监听

### 5.2 用户认证

- 认证依据：VLESS 请求头中的 UUID
- 校验方式：在内存 `HashSet<Uuid>` 中做 O(1) 查找
- 认证失败：拒绝连接并记录日志

### 5.3 代理逻辑

#### TCP 模式

- 在同一监听端口上通过 `peek()` 检测请求类型
- HTTP 请求进入 API/信息页处理
- VLESS 原始流进入 TCP 代理处理
- 若客户端命令为 `UDP`，使用 `UDP over TCP` 机制转发

#### WebSocket 模式

- 仅接受 HTTP 请求或 WebSocket Upgrade
- 普通 HTTP 请求进入 API/信息页处理
- WebSocket 成功升级后，首帧作为 VLESS 请求头解析
- 后续数据在 WebSocket 与目标 TCP 连接之间双向转发

### 5.4 链接生成逻辑

- 输入：用户邮箱
- 查询：从 `user_emails` 中找到对应 UUID
- 输出：
  - TCP 模式返回 `tcp` 与 `tcp_b64`
  - WebSocket 模式返回 `ws` 与 `ws_b64`

## 6. API 定义

HTTP 服务与代理服务共用同一监听端口。

### 6.1 `GET /`

用途：

- 返回服务信息页

成功响应：

- 状态码：`200`
- 内容类型：`text/html; charset=utf-8`

页面内容：

- 产品名
- 版本号
- 作者
- 公网 IP 或监听地址
- 端口
- 协议类型
- WebSocket 路径（如果启用）

### 6.2 `GET /?email={email}`

用途：

- 根据邮箱生成用户对应的 VLESS 链接

请求参数：

| 参数 | 位置 | 必填 | 说明 |
| --- | --- | --- | --- |
| `email` | query | 是 | 用户邮箱 |

TCP 模式响应示例：

```json
{
  "tcp": "vless://uuid@host:port?encryption=none&security=none&type=tcp#alias",
  "tcp_b64": "base64_encoded_link"
}
```

WebSocket 模式响应示例：

```json
{
  "ws": "vless://uuid@host:port?encryption=none&security=none&type=ws&path=%2Fvless#alias",
  "ws_b64": "base64_encoded_link"
}
```

用户不存在时：

```json
{
  "error": "User not found"
}
```

说明：

- 业务错误当前仍返回 `200`
- 非法请求返回 `400`
- 非根路径返回 `404`

### 6.3 HTTP 响应安全头

所有 HTTP 响应统一附带：

```text
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Referrer-Policy: no-referrer
Content-Security-Policy: default-src 'none'; style-src 'self' 'unsafe-inline' 'unsafe-hashes'; script-src 'none'
```

## 7. VLESS 协议支持

### 7.1 请求格式

```text
+------+-------+----------+----------+---------+------+----------+
| 1B   | 16B   | 1B       | Variable | 1B      | 2B   | Variable |
+------+-------+----------+----------+---------+------+----------+
| Ver  | UUID  | Addons   | Addons   | Cmd     | Port | Address  |
|      |       | Len      | Data     |         |      |          |
+------+-------+----------+----------+---------+------+----------+
```

### 7.2 支持范围

- 版本：`0`、`1`
- 命令：
  - `1` = TCP，已实现
  - `2` = UDP，TCP 模式下已实现 `UDP over TCP`
  - `3` = Mux，已识别但未实现
- 地址类型：
  - IPv4
  - 域名
  - IPv6

### 7.3 响应格式

```text
+------+----------+----------+
| 1B   | 1B       | Variable |
+------+----------+----------+
| Ver  | Addons   | Addons   |
|      | Len      | Data     |
+------+----------+----------+
```

当前响应行为：

- 使用客户端请求中的同版本号回写
- 当前不附带 addons 数据

## 8. 平台与构建规格

### 8.1 支持平台

| 平台 | 目标三元组 | 说明 |
| --- | --- | --- |
| Windows x64 | `x86_64-pc-windows-msvc` | 支持资源嵌入 |
| Linux x64 | `x86_64-unknown-linux-musl` | 静态链接 |
| Linux ARM64 | `aarch64-unknown-linux-musl` | 需 `cargo-zigbuild` |
| Linux ARMv7 | `armv7-unknown-linux-musleabihf` | 需 `cargo-zigbuild` |

### 8.2 构建特性

- `release` 模式启用 `lto = "thin"`
- `codegen-units = 1`
- `opt-level = 3`
- `panic = "abort"`
- `strip = true`

### 8.3 平台条件编译

- 非 `musl` 目标使用 `mimalloc`
- Unix 平台启用文件权限设置与信号处理
- Windows 平台通过 `build.rs` 嵌入资源信息

## 9. 测试规格

当前测试覆盖：

- 协议编解码
- HTTP 与 WebSocket 工具函数
- 链接生成
- 原子写入
- 公网 IP 获取
- 服务器配置
- 部分 TCP 行为与地址解析

当前缺口：

- 端到端代理回归测试
- 服务安装流程自动化测试
- WebSocket 代理真实链路测试
- 压测与基准测试
