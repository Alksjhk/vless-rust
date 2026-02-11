# 配置文件说明

## 概述

VLESS-Rust 使用 JSON 格式的配置文件（默认为 `config.json`）。所有配置项都有合理的默认值，可以只配置必需项。

## 快速开始

### 最小配置示例

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443
  },
  "users": [
    {
      "uuid": "615767da-4db9-4df7-9f12-d7d617fc1d96",
      "email": "user@example.com"
    }
  ]
}
```

### 使用示例配置

```bash
# 编辑配置文件
nano config.json  # 或使用其他编辑器

# 启动服务器
cargo run -- config.json
```

## 配置项详解

### 1. 服务器配置 (server)

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `listen` | string | "0.0.0.0" | 监听地址，0.0.0.0 表示所有网卡 |
| `port` | number | 443 | 监听端口，建议使用 443 或 8443 |

**示例：**
```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443
  }
}
```

**端口选择建议：**
- `443` - HTTPS 标准端口，流量混淆度高
- `8443` - HTTPS 备用端口，避免冲突
- 避免使用已被其他服务占用的端口

**注意事项：**
- 需要在防火墙中放行配置的端口
- 端口小于 1024 需要 root/administrator 权限

### 2. 用户配置 (users)

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `uuid` | string | ✅ | 用户唯一标识符 |
| `email` | string | ❌ | 用户邮箱（可选） |

**示例：**
```json
{
  "users": [
    {
      "uuid": "615767da-4db9-4df7-9f12-d7d617fc1d96",
      "email": "user1@example.com"
    },
    {
      "uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
      "email": "user2@example.com"
    },
    {
      "uuid": "11111111-2222-3333-4444-555555555555"
    }
  ]
}
```

**UUID 生成方法：**
- Linux/Mac: `uuidgen`
- Windows PowerShell: `[guid]::NewGuid()`
- 在线工具: https://www.uuidgenerator.net/

**邮箱说明：**
- 用于生成 VLESS 链接别名
- 可省略，省略时使用 UUID 前 8 位

### 3. 性能优化配置 (performance)

所有字段都是可选的，未配置时使用默认值。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `buffer_size` | number | 131072 | 传输缓冲区大小（字节） |
| `tcp_recv_buffer` | number | 262144 | TCP 接收缓冲区大小（字节） |
| `tcp_send_buffer` | number | 262144 | TCP 发送缓冲区大小（字节） |
| `tcp_nodelay` | boolean | true | 是否启用 TCP_NODELAY |
| `udp_timeout` | number | 30 | UDP 会话超时时间（秒） |
| `udp_recv_buffer` | number | 65536 | UDP 接收缓冲区大小（字节） |
| `buffer_pool_size` | number | min(32, CPU核心数*4) | 缓冲区池大小 |

**示例：**
```json
{
  "performance": {
    "buffer_size": 131072,
    "tcp_recv_buffer": 262144,
    "tcp_send_buffer": 262144,
    "tcp_nodelay": true,
    "udp_timeout": 30,
    "udp_recv_buffer": 65536,
    "buffer_pool_size": 32
  }
}
```

**详细说明：**

#### buffer_size（传输缓冲区大小）
- **作用**：每个连接的数据传输缓冲区大小
- **默认**：131072 (128KB)
- **范围**：65536 (64KB) - 524288 (512KB)
- **影响**：
  - 增大：提高高带宽场景性能，但增加内存占用
  - 减小：降低内存占用，适合高并发场景
- **建议**：
  - 千兆网络：131072 (128KB)
  - 高并发（1000+ 连接）：65536 (64KB)
  - 万兆网络：262144 (256KB) 或更大

#### tcp_recv_buffer / tcp_send_buffer（TCP 缓冲区大小）
- **作用**：操作系统层面的 TCP 接收/发送缓冲区
- **默认**：262144 (256KB)
- **设置为 0**：使用系统默认值
- **建议**：与 buffer_size 保持 2:1 比例

#### tcp_nodelay（是否启用 TCP_NODELAY）
- **作用**：禁用 Nagle 算法，立即发送小数据包
- **默认**：true
- **影响**：
  - true：降低延迟，适合交互式应用
  - false：提高吞吐量，适合批量传输
- **建议**：VLESS 场景建议保持 true

#### buffer_pool_size（缓冲区池大小）
- **作用**：预分配的缓冲区数量，减少内存分配开销
- **默认**：min(32, CPU核心数*4)
- **影响**：
  - 增大：提升高并发性能，但增加初始内存占用
  - 减小：降低内存占用
- **建议**：
  - 低并发（<100 连接）：16-32
  - 中并发（100-1000 连接）：32-64
  - 高并发（1000+ 连接）：64-128

#### udp_timeout（UDP 会话超时）
- **作用**：UDP 连接 N 秒无数据则自动关闭
- **默认**：30 秒
- **范围**：10-300 秒
- **建议**：
  - 实时应用（游戏、语音）：10-20 秒
  - 一般应用：30 秒
  - 长连接应用：60-120 秒

#### udp_recv_buffer（UDP 接收缓冲区）
- **作用**：每个 UDP 会话的接收缓冲区
- **默认**：65536 (64KB)
- **范围**：32768 (32KB) - 262144 (256KB)
- **建议**：
  - 低延迟场景：32768 (32KB)
  - 一般场景：65536 (64KB)
  - 高吞吐场景：131072 (128KB)

## 性能调优指南

### 场景 1：高并发（1000+ 连接）

```json
{
  "performance": {
    "buffer_size": 65536,
    "tcp_nodelay": true,
    "buffer_pool_size": 64
  }
}
```

**说明**：
- 减小缓冲区以降低内存占用
- 增加缓冲区池以提升高并发性能

### 场景 2：高带宽（千兆/万兆网络）

```json
{
  "performance": {
    "buffer_size": 262144,
    "tcp_recv_buffer": 524288,
    "tcp_send_buffer": 524288,
    "tcp_nodelay": true
  }
}
```

**说明**：
- 增大传输和 TCP 缓冲区
- 启用 TCP_NODELAY 降低延迟
- 适合大文件传输场景

### 场景 3：低延迟（游戏、实时应用）

```json
{
  "performance": {
    "buffer_size": 65536,
    "tcp_nodelay": true,
    "udp_timeout": 15,
    "udp_recv_buffer": 32768
  }
}
```

**说明**：
- 减小缓冲区以降低延迟
- 启用 TCP_NODELAY
- 减小 UDP 超时和缓冲区

### 场景 4：低配服务器（1GB 内存）

```json
{
  "performance": {
    "buffer_size": 65536,
    "tcp_recv_buffer": 131072,
    "tcp_send_buffer": 131072,
    "buffer_pool_size": 16
  }
}
```

**说明**：
- 减小所有缓冲区
- 减小缓冲区池
- 降低内存占用

## 配置验证

服务器启动时会自动验证配置文件。如果配置文件不存在，会自动启动配置向导。

### 检查配置文件

```bash
# 启动服务器时指定配置文件
cargo run -- /path/to/config.json

# 如果配置文件有误，服务器会显示错误信息
```

### 常见错误

1. **JSON 格式错误**
   - 检查是否有语法错误（缺少逗号、括号不匹配等）
   - 使用 JSON 验证工具检查格式

2. **UUID 格式错误**
   - 确保 UUID 格式正确：xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
   - 使用 UUID 生成工具生成

3. **端口已被占用**
   - 更换端口号
   - 或停止占用该端口的程序

4. **权限不足**
   - 端口小于 1024 需要 root/administrator 权限
   - 使用 1024 以上的端口

## 配置文件位置

- **默认位置**：`config.json`（与可执行文件同目录）
- **指定位置**：`cargo run -- /path/to/config.json`
- **首次运行**：自动启动配置向导，引导创建配置文件

## 完整配置示例

参考项目根目录的 `config.json` 文件。

## 相关文档

- [技术文档](technology.md) - 架构设计和实现细节
- [README](../README.md) - 项目说明和使用指南
