# API 文档

## 概述

VLESS-Rust 提供 RESTful API 和 WebSocket 实时推送接口，用于获取服务器监控数据和配置信息。

## 基础信息

- **Base URL**: `http://your-server:8443`
- **Content-Type**: `application/json`
- **字符编码**: UTF-8

## REST API 端点

### 1. 获取实时监控数据

获取服务器当前的实时监控数据，包括流量统计、连接数、内存使用等信息。

**请求**
```http
GET /api/stats
```

**响应示例**
```json
{
  "upload_speed": "1.23 MB/s",
  "download_speed": "2.34 MB/s",
  "total_traffic": "15.67 GB",
  "uptime": "2d 5h 30m 15s",
  "memory_usage": "45.67 MB",
  "total_memory": "16.00 GB",
  "active_connections": 5,
  "max_connections": 300,
  "users": [
    {
      "uuid": "12345678-1234-1234-1234-123456789abc",
      "email": "user1@example.com",
      "upload_speed": "0 B/s",
      "download_speed": "0 B/s",
      "total_traffic": "5.23 GB",
      "active_connections": 2
    }
  ]
}
```

**字段说明**
| 字段 | 类型 | 说明 |
|------|------|------|
| upload_speed | string | 上传速度（格式化字符串） |
| download_speed | string | 下载速度（格式化字符串） |
| total_traffic | string | 总流量（上传+下载） |
| uptime | string | 服务器运行时长 |
| memory_usage | string | 当前内存使用量 |
| total_memory | string | 系统总内存 |
| active_connections | number | 当前活动连接数 |
| max_connections | number | 最大连接数限制 |
| users | array | 用户统计数组 |

**用户统计对象 (users)**
| 字段 | 类型 | 说明 |
|------|------|------|
| uuid | string | 用户 UUID |
| email | string/null | 用户邮箱 |
| upload_speed | string | 用户上传速度 |
| download_speed | string | 用户下载速度 |
| total_traffic | string | 用户总流量 |
| active_connections | number | 用户活动连接数 |

---

### 2. 获取用户流量统计

获取所有用户的累计流量统计信息。

**请求**
```http
GET /api/user-stats
```

**响应示例**
```json
[
  {
    "uuid": "12345678-1234-1234-1234-123456789abc",
    "email": "user1@example.com",
    "upload_speed": "0 B/s",
    "download_speed": "0 B/s",
    "total_traffic": "5.23 GB",
    "active_connections": 2
  },
  {
    "uuid": "87654321-4321-4321-4321-cba987654321",
    "email": null,
    "upload_speed": "0 B/s",
    "download_speed": "0 B/s",
    "total_traffic": "10.45 GB",
    "active_connections": 3
  }
]
```

**字段说明**: 同 `/api/stats` 中的 `users` 数组项。

---

### 3. 获取速度历史数据

获取服务器最近一段时间（默认 60 秒）的速度历史记录。

**请求**
```http
GET /api/speed-history
```

**响应示例**
```json
{
  "history": [
    {
      "timestamp": "0",
      "upload_speed": "1.23 MB/s",
      "download_speed": "2.34 MB/s"
    },
    {
      "timestamp": "1",
      "upload_speed": "1.45 MB/s",
      "download_speed": "2.56 MB/s"
    }
  ],
  "duration_seconds": 60
}
```

**字段说明**
| 字段 | 类型 | 说明 |
|------|------|------|
| history | array | 历史数据点数组 |
| history[].timestamp | string | 时间戳（相对于服务器启动的秒数） |
| history[].upload_speed | string | 上传速度 |
| history[].download_speed | string | 下载速度 |
| duration_seconds | number | 历史数据时长（秒） |

**用途**: 用于绘制流量趋势图。

---

### 4. 获取监控配置

获取服务器当前的监控参数配置。

**请求**
```http
GET /api/config
```

**响应示例**
```json
{
  "speed_history_duration": 60,
  "broadcast_interval": 1,
  "websocket_max_connections": 300,
  "websocket_heartbeat_timeout": 60,
  "vless_max_connections": 300
}
```

**字段说明**
| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| speed_history_duration | number | 60 | 速度历史保留时长（秒） |
| broadcast_interval | number | 1 | WebSocket 广播间隔（秒） |
| websocket_max_connections | number | 300 | WebSocket 最大连接数 |
| websocket_heartbeat_timeout | number | 60 | WebSocket 心跳超时（秒） |
| vless_max_connections | number | 300 | VLESS 最大连接数 |

---

### 5. 获取性能配置

获取服务器当前的性能优化参数配置。

**请求**
```http
GET /api/performance
```

**响应示例**
```json
{
  "buffer_size": 131072,
  "tcp_nodelay": true,
  "tcp_recv_buffer": 262144,
  "tcp_send_buffer": 262144,
  "stats_batch_size": 65536
}
```

**字段说明**
| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| buffer_size | number | 131072 | 传输缓冲区大小（字节），默认 128KB |
| tcp_nodelay | boolean | true | 是否启用 TCP_NODELAY |
| tcp_recv_buffer | number | 262144 | TCP 接收缓冲区大小（字节），默认 256KB |
| tcp_send_buffer | number | 262144 | TCP 发送缓冲区大小（字节），默认 256KB |
| stats_batch_size | number | 65536 | 流量统计批量大小（字节），默认 64KB |

---

### 6. 获取连接池监控数据

获取连接池的性能统计信息。

**请求**
```http
GET /api/connection-pool-stats
```

**响应示例**
```json
{
  "total_created": 150,
  "total_reused": 320,
  "total_closed": 145,
  "current_active": 5,
  "current_idle": 12,
  "cache_hits": 320,
  "cache_misses": 150,
  "hit_rate": 68.08
}
```

**字段说明**
| 字段 | 类型 | 说明 |
|------|------|------|
| total_created | number | 总创建连接数 |
| total_reused | number | 总复用连接数 |
| total_closed | number | 总关闭连接数 |
| current_active | number | 当前活跃连接数 |
| current_idle | number | 当前空闲连接数 |
| cache_hits | number | 缓存命中次数 |
| cache_misses | number | 缓存未命中次数 |
| hit_rate | number | 缓存命中率（%） |

---

### 7. 获取内存池监控数据

获取内存池的性能统计信息。

**请求**
```http
GET /api/memory-pool-stats
```

**响应示例**
```json
{
  "small_pool": {
    "buffer_size": 4096,
    "current_size": 45,
    "peak_size": 50,
    "total_allocated": 1234,
    "total_returned": 1189
  },
  "medium_pool": {
    "buffer_size": 65536,
    "current_size": 18,
    "peak_size": 20,
    "total_allocated": 567,
    "total_returned": 549
  },
  "large_pool": {
    "buffer_size": 131072,
    "current_size": 8,
    "peak_size": 10,
    "total_allocated": 234,
    "total_returned": 226
  }
}
```

**字段说明**
| 字段 | 类型 | 说明 |
|------|------|------|
| small_pool | object | 小缓冲区池（4KB）统计 |
| medium_pool | object | 中等缓冲区池（64KB）统计 |
| large_pool | object | 大缓冲区池（128KB）统计 |
| *.buffer_size | number | 缓冲区大小（字节） |
| *.current_size | number | 当前池中缓冲区数量 |
| *.peak_size | number | 峰值池大小 |
| *.total_allocated | number | 总分配次数 |
| *.total_returned | number | 总归还次数 |

---

## WebSocket API

### 连接端点

WebSocket 提供实时数据推送，是推荐的数据获取方式。

**连接 URL**
```
ws://your-server:8443/api/ws
wss://your-server:8443/api/ws  (如果使用 TLS)
```

**备用端点**
```
ws://your-server:8443/ws
```

### 连接流程

1. **握手**: 客户端发送 WebSocket 升级请求
2. **验证**: 服务器验证 Origin 头（可通过 `VLESS_MONITOR_ORIGIN` 环境变量配置）
3. **推送历史**: 连接建立后，服务器立即推送历史速度数据
4. **实时推送**: 之后每秒推送一次实时监控数据

### 消息格式

所有消息采用 JSON 格式，包含 `type` 和 `payload` 字段。

#### 1. 历史数据消息

连接建立后的第一条消息。

```json
{
  "type": "history",
  "payload": {
    "history": [
      {
        "timestamp": "0",
        "upload_speed": "1.23 MB/s",
        "download_speed": "2.34 MB/s"
      }
    ],
    "duration_seconds": 60
  }
}
```

**payload 结构**: 同 REST API 的 `/api/speed-history` 响应。

#### 2. 实时数据消息

每秒推送一次。

```json
{
  "type": "stats",
  "payload": {
    "upload_speed": "1.23 MB/s",
    "download_speed": "2.34 MB/s",
    "total_traffic": "15.67 GB",
    "uptime": "2d 5h 30m 15s",
    "memory_usage": "45.67 MB",
    "total_memory": "16.00 GB",
    "active_connections": 5,
    "max_connections": 300,
    "connection_pool": {
      "total_created": 150,
      "total_reused": 320,
      "total_closed": 145,
      "current_active": 5,
      "current_idle": 12,
      "cache_hits": 320,
      "cache_misses": 150,
      "hit_rate": 68.08
    },
    "memory_pool": {
      "small_pool": {
        "buffer_size": 4096,
        "current_size": 45,
        "peak_size": 50,
        "total_allocated": 1234,
        "total_returned": 1189
      },
      "medium_pool": {
        "buffer_size": 65536,
        "current_size": 18,
        "peak_size": 20,
        "total_allocated": 567,
        "total_returned": 549
      },
      "large_pool": {
        "buffer_size": 131072,
        "current_size": 8,
        "peak_size": 10,
        "total_allocated": 234,
        "total_returned": 226
      }
    },
    "users": [...]
  }
}
```

**payload 结构**: 同 REST API 的 `/api/stats` 响应，并包含连接池和内存池统计数据。

### 心跳机制

- **服务器 → 客户端**: 定期发送 Ping 帧
- **客户端 → 服务器**: 应答 Pong 帧
- **超时断开**: 60 秒无活动自动断开连接

### Origin 验证

为了防止 CSRF 攻击，服务器会验证 WebSocket 请求的 Origin 头。

**配置方式**
```bash
# 设置允许的来源
export VLESS_MONITOR_ORIGIN="https://your-domain.com"

# Windows
set VLESS_MONITOR_ORIGIN=https://your-domain.com
```

**未配置行为**: 如果未设置环境变量，允许所有来源（仅开发模式）。

### 连接限制

- **最大连接数**: 300（可通过配置修改）
- **超出限制**: 返回 1011 状态码，原因 "Server full"

### 错误处理

**连接关闭码**
| 状态码 | 说明 |
|--------|------|
| 1000 | 正常关闭 |
| 1001 | 端点关闭 |
| 1006 | 异常关闭 |
| 1011 | 服务器错误（如达到最大连接数） |

---

## 前端集成示例

### REST API 轮询

```javascript
// 获取实时监控数据
async function getStats() {
  const response = await fetch('/api/stats');
  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  const data = await response.json();
  console.log('上传速度:', data.upload_speed);
  console.log('下载速度:', data.download_speed);
  return data;
}

// 每秒轮询
setInterval(getStats, 1000);
```

### WebSocket 实时推送

```javascript
// 连接 WebSocket
const ws = new WebSocket('ws://localhost:8443/api/ws');

ws.onopen = () => {
  console.log('WebSocket 已连接');
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);

  if (msg.type === 'history') {
    console.log('收到历史数据:', msg.payload);
    // 初始化图表
  } else if (msg.type === 'stats') {
    console.log('收到实时数据:', msg.payload);
    // 更新 UI
  }
};

ws.onerror = (error) => {
  console.error('WebSocket 错误:', error);
};

ws.onclose = (event) => {
  console.log('WebSocket 已关闭:', event.code, event.reason);
  // 可实现重连逻辑
};
```

### API 降级策略

```javascript
class MonitorClient {
  constructor() {
    this.ws = null;
    this.pollingInterval = null;
  }

  connect() {
    // 尝试 WebSocket
    this.ws = new WebSocket('ws://localhost:8443/api/ws');

    this.ws.onopen = () => {
      console.log('WebSocket 已连接');
      this.stopPolling();
    };

    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      this.handleData(msg);
    };

    this.ws.onerror = () => {
      console.log('WebSocket 失败，降级到 API 轮询');
      this.startPolling();
    };

    this.ws.onclose = () => {
      if (!this.pollingInterval) {
        console.log('WebSocket 断开，启动 API 轮询');
        this.startPolling();
      }
    };
  }

  startPolling() {
    if (this.pollingInterval) return;

    this.fetchData(); // 立即获取一次
    this.pollingInterval = setInterval(() => {
      this.fetchData();
    }, 1000);
  }

  stopPolling() {
    if (this.pollingInterval) {
      clearInterval(this.pollingInterval);
      this.pollingInterval = null;
    }
  }

  async fetchData() {
    try {
      const response = await fetch('/api/stats');
      const data = await response.json();
      this.handleData({ type: 'stats', payload: data });
    } catch (error) {
      console.error('API 轮询失败:', error);
    }
  }

  handleData(msg) {
    if (msg.type === 'stats') {
      // 更新 UI
      console.log('上传速度:', msg.payload.upload_speed);
    }
  }
}

// 使用
const client = new MonitorClient();
client.connect();
```

---

## 错误码

### HTTP 状态码

| 状态码 | 说明 | 示例 |
|--------|------|------|
| 200 | 成功 | 数据正常返回 |
| 404 | 未找到 | 请求的端点不存在 |
| 500 | 服务器错误 | 内部处理错误 |

### WebSocket 关闭码

| 状态码 | 说明 |
|--------|------|
| 1000 | 正常关闭 |
| 1001 | 端点离开 |
| 1006 | 异常关闭（网络错误） |
| 1011 | 服务器错误（如达到最大连接数） |

---

## 性能建议

### REST API
- **轮询间隔**: 建议不低于 1 秒
- **超时设置**: 建议 5 秒
- **重试策略**: 指数退避，最多 3 次

### WebSocket
- **重连延迟**: 建议从 1 秒开始，指数增长至 30 秒
- **心跳检测**: 客户端应响应服务器的 Ping 帧
- **缓冲限制**: 客户端应限制消息队列大小

---

## 安全建议

1. **使用 TLS**: 生产环境建议使用 WSS (WebSocket Secure)
2. **Origin 验证**: 设置 `VLESS_MONITOR_ORIGIN` 环境变量
3. **访问控制**: 配置防火墙限制访问监控页面
4. **认证机制**: 当前监控页面无认证，建议通过反向代理添加

---

## 版本历史

- **v1.1.0**: 添加 WebSocket 实时推送
- **v1.0.0**: 初始 REST API

---

## 相关文档

- [技术文档](./technology.md)
- [项目 README](../README.md)
- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
