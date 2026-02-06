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
# 复制示例配置
cp config.example.json config.json

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
- 用于流量统计和用户标识
- 可省略，省略时使用 UUID 前 8 位

### 3. 监控配置 (monitoring)

所有字段都是可选的，未配置时使用默认值。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `speed_history_duration` | number | 60 | 速度历史记录时长（秒） |
| `broadcast_interval` | number | 1 | WebSocket 广播间隔（秒） |
| `websocket_max_connections` | number | 300 | WebSocket 最大连接数 |
| `websocket_heartbeat_timeout` | number | 60 | WebSocket 心跳超时（秒） |
| `vless_max_connections` | number | 300 | VLESS 最大连接数 |

**示例：**
```json
{
  "monitoring": {
    "speed_history_duration": 60,
    "broadcast_interval": 1,
    "websocket_max_connections": 300,
    "websocket_heartbeat_timeout": 60,
    "vless_max_connections": 300
  }
}
```

**详细说明：**

#### speed_history_duration（速度历史记录时长）
- **作用**：保留最近 N 秒的速度数据，用于前端图表展示
- **默认**：60 秒
- **影响**：前端流量趋势图的时间范围
- **建议**：30-120 秒

#### broadcast_interval（WebSocket 广播间隔）
- **作用**：每隔 N 秒向所有连接的前端推送最新监控数据
- **默认**：1 秒
- **影响**：
  - 过小：增加服务器负载
  - 过大：前端数据更新延迟
- **建议**：1-5 秒

#### websocket_max_connections（WebSocket 最大连接数）
- **作用**：允许同时连接的监控页面数量
- **默认**：300
- **建议**：根据监控需求调整，单机建议不超过 1000

#### websocket_heartbeat_timeout（WebSocket 心跳超时）
- **作用**：客户端 N 秒无心跳则断开连接
- **默认**：60 秒
- **建议**：30-120 秒，网络不稳定时可适当增大

#### vless_max_connections（VLESS 最大连接数）
- **作用**：允许同时连接的 VLESS 客户端数量
- **默认**：300
- **建议**：
  - 低配（1GB 内存）：100-200
  - 中配（2-4GB 内存）：300-500
  - 高配（8GB+ 内存）：1000+

### 4. 性能优化配置 (performance)

所有字段都是可选的，未配置时使用默认值。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `buffer_size` | number | 131072 | 传输缓冲区大小（字节） |
| `tcp_recv_buffer` | number | 262144 | TCP 接收缓冲区大小（字节） |
| `tcp_send_buffer` | number | 262144 | TCP 发送缓冲区大小（字节） |
| `tcp_nodelay` | boolean | true | 是否启用 TCP_NODELAY |
| `stats_batch_size` | number | 65536 | 流量统计批量大小（字节） |
| `udp_timeout` | number | 30 | UDP 会话超时时间（秒） |
| `udp_recv_buffer` | number | 65536 | UDP 接收缓冲区大小（字节） |

**示例：**
```json
{
  "performance": {
    "buffer_size": 131072,
    "tcp_recv_buffer": 262144,
    "tcp_send_buffer": 262144,
    "tcp_nodelay": true,
    "stats_batch_size": 65536,
    "udp_timeout": 30,
    "udp_recv_buffer": 65536
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

#### stats_batch_size（流量统计批量大小）
- **作用**：累积 N 字节流量才更新一次统计
- **默认**：65536 (64KB)
- **作用**：减少锁竞争，提高性能 90%+
- **影响**：实时性略有降低，但性能大幅提升
- **建议**：保持默认值即可

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

### 5. 监控数据 (monitor)

此部分由服务器自动维护，**请勿手动修改**。

| 字段 | 类型 | 说明 |
|------|------|------|
| `total_upload_bytes` | number | 总上传字节数（自动统计） |
| `total_download_bytes` | number | 总下载字节数（自动统计） |
| `last_update` | string | 最后更新时间（自动维护） |
| `users` | object | 用户级别统计（自动维护） |

**注意**：手动修改此部分的数据会被服务器覆盖。

## 性能调优指南

### 场景 1：高并发（1000+ 连接）

```json
{
  "performance": {
    "buffer_size": 65536,
    "tcp_nodelay": true,
    "stats_batch_size": 65536
  },
  "monitoring": {
    "vless_max_connections": 1000
  }
}
```

**说明**：
- 减小缓冲区以降低内存占用
- 保持批量统计以提高性能
- 增加最大连接数限制

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
    "tcp_send_buffer": 131072
  },
  "monitoring": {
    "vless_max_connections": 100,
    "websocket_max_connections": 50
  }
}
```

**说明**：
- 减小所有缓冲区
- 限制最大连接数
- 降低内存占用

## 配置验证

服务器启动时会自动验证配置文件。如果配置文件不存在，会自动创建默认配置。

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
- **示例配置**：`config.example.json`（含所有配置项）
- **带注释示例**：`config.example.json5`（含详细注释，JSON5 格式）

## 完整配置示例

参考项目中的 `config.example.json` 文件，包含所有配置项和默认值。

## 相关文档

- [技术文档](technology.md) - 架构设计和实现细节
- [API 文档](api.md) - 前后端接口定义
- [README](../README.md) - 项目说明和使用指南
