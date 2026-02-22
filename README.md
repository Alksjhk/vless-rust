# VLESS Protocol Server in Rust

基于Rust和Tokio实现的高性能VLESS协议服务器，遵循xray-core的VLESS协议规范。

## 特性

- 完整的VLESS协议支持（版本0和版本1）
- 异步I/O处理（基于Tokio）
- **多协议传输**：支持 TCP 和 WebSocket (WS) 传输
- TCP代理转发
- UDP over TCP 支持
- 多用户UUID认证
- 配置文件支持
- **交互式配置向导**，首次运行时引导用户完成配置
- 结构化日志记录
- 版本兼容性（支持测试版和正式版）
- **可配置缓冲区**，适配高带宽场景
- **TCP优化**，默认启用TCP_NODELAY降低延迟
- **缓冲区池**，减少内存分配开销

## VLESS协议实现

本实现严格遵循xray-core的VLESS协议规范，支持两个协议版本：

- **版本0**: 测试版本（BETA）
- **版本1**: 正式版本（RELEASE）

服务器会自动检测客户端使用的版本并返回相应的响应版本。

### 请求格式
```
1 Byte    | 16 Bytes | 1 Byte  | M Bytes | 1 Byte | 2 Bytes | 1 Byte | S Bytes | X Bytes
版本      | UUID     | 附加长度 | 附加数据 | 命令   | 端口    | 地址类型| 地址    | 请求数据
```

### 响应格式
```
1 Byte    | 1 Byte  | N Bytes | Y Bytes
版本      | 附加长度 | 附加数据 | 响应数据
```

### 支持的命令类型
- `0x01`: TCP连接
- `0x02`: UDP连接（通过 UDP over TCP 机制）

### 支持的地址类型
- `0x01`: IPv4地址（4字节）
- `0x02`: 域名（1字节长度 + 域名）
- `0x03`: IPv6地址（16字节）

## 快速开始

### 1. 编译项目

**使用 Cargo：**

```bash
cargo build --release
```

编译后的可执行文件位于 `target/release/vless.exe`（Windows）或 `target/release/vless`（Linux/macOS）。

**编译特性**：
- 使用 CRT 静态链接，生成零依赖的可执行文件
- Windows 版本无需 Visual C++ Redistributable
- 体积小，启动快

### 2. 首次运行（配置向导）

如果您是第一次运行服务器，或者 `config.json` 不存在，程序会自动启动**交互式配置向导**：

```bash
cargo run
```

向导会引导您完成以下配置：
- **服务器监听地址**（默认：0.0.0.0）
- **服务器监听端口**（默认：443）
- **协议类型**（默认：TCP）
  - TCP：直接 TCP 连接，推荐用于需要端口转发的场景
  - WebSocket：WS 协议，可绕过防火墙限制
  - WebSocket 路径（仅 WS 模式，默认：/）
- **用户配置**
  - 用户 UUID（自动生成或手动输入）
  - 用户邮箱（可选，用于标识）

配置完成后，向导会自动生成 `config.json` 文件并启动服务器。

### 3. 配置服务器

编辑 `config.json` 文件：

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443,
    "protocol": "tcp",
    "ws_path": "/"
  },
  "users": [
    {
      "uuid": "12345678-1234-1234-1234-123456789abc",
      "email": "user1@example.com"
    }
  ]
}
```

### 4. 运行服务器

```bash
# 使用编译后的可执行文件
./target/release/vless.exe           # Windows
./target/release/vless               # Linux/macOS

# 或使用 Cargo 运行开发版本
cargo run

# 指定配置文件路径
./vless.exe /path/to/config.json
```

### 5. 客户端配置

在支持VLESS协议的客户端中配置：

- **协议**: VLESS
- **地址**: 你的服务器IP
- **端口**: 8443（或配置文件中设置的端口）
- **UUID**: 配置文件中的用户UUID
- **加密**: none
- **传输协议**: TCP 或 WebSocket（根据服务器配置）

**WebSocket 客户端配置**（当服务器使用 WS 协议时）：
- **传输协议**: WebSocket
- **WebSocket 路径**: /（或配置文件中设置的 ws_path）

## 配置说明

### 基本配置

编辑 `config.json` 文件：

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443,
    "protocol": "tcp",
    "ws_path": "/"
  },
  "users": [
    {
      "uuid": "12345678-1234-1234-1234-123456789abc",
      "email": "user1@example.com"
    }
  ]
}
```

### 配置项说明

#### 服务器配置
- `listen`: 监听地址，通常设为 `0.0.0.0` 监听所有接口
- `port`: 监听端口（建议使用 443 或 8443）
- `protocol`: 协议类型（`tcp` 或 `ws`），默认 `tcp`
- `ws_path`: WebSocket 路径（仅 `ws` 协议使用），默认 `/`

#### 用户配置
- `uuid`: 用户唯一标识符，必须是有效的UUID格式
- `email`: 用户邮箱（可选，用于标识）

#### 性能配置（可选）

```json
{
  "performance": {
    "buffer_size": 131072,
    "tcp_nodelay": true,
    "tcp_recv_buffer": 262144,
    "tcp_send_buffer": 262144,
    "udp_timeout": 30,
    "udp_recv_buffer": 65536,
    "buffer_pool_size": 32
  }
}
```

- `buffer_size`: 传输缓冲区大小（字节），默认131072（128KB）
- `tcp_nodelay`: 是否启用TCP_NODELAY，默认true
- `tcp_recv_buffer`: TCP接收缓冲区大小（字节），默认262144（256KB）
- `tcp_send_buffer`: TCP发送缓冲区大小（字节），默认262144（256KB）
- `udp_timeout`: UDP会话超时时间（秒），默认30
- `udp_recv_buffer`: UDP接收缓冲区大小（字节），默认65536（64KB）
- `buffer_pool_size`: 缓冲区池大小，默认 min(32, CPU核心数*4)

## 性能特点

- **异步处理**: 基于Tokio的异步运行时，支持高并发连接
- **零拷贝**: 使用Bytes库进行高效的内存管理
- **快速UUID验证**: 使用HashSet进行O(1)时间复杂度的用户验证
- **连接复用**: 每个客户端连接在独立的异步任务中处理
- **可配置缓冲区**: 默认128KB传输缓冲区，支持高带宽场景（千兆网络）
- **缓冲区池**: 复用缓冲区，减少内存分配开销
- **TCP优化**: 默认启用TCP_NODELAY降低延迟
- **UDP支持**: UDP over TCP 机制，支持30秒超时自动清理空闲会话
- **静态链接**: Windows CRT 静态链接，零依赖运行

## 部署说明

编译后的可执行文件可以直接复制到目标服务器运行。

**部署步骤**：
1. 编译项目：`cargo build --release`
2. 将 `target/release/vless.exe`（或 `vless`）复制到服务器
3. 创建配置文件 `config.json`（或使用自动生成的默认配置）
4. 运行 `./vless.exe` 或 `./vless.exe /path/to/config.json`

**静态链接特性**：
- Windows 版本使用 CRT 静态链接，无需安装 Visual C++ Redistributable
- 可执行文件只依赖系统内核库（KERNEL32.dll），实现零依赖运行

## 安全注意事项

1. **UUID保密**: 确保用户UUID不被泄露，这是唯一的认证凭据
2. **传输加密**: 建议在生产环境中配合TLS使用
3. **访问控制**: 合理配置防火墙规则
4. **日志管理**: 注意日志中可能包含敏感信息

## 协议参考

- [VLESS协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core项目](https://github.com/XTLS/Xray-core)

## 许可证

MIT License
