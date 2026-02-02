# API 文档

## 概述

VLESS监控页面提供RESTful API接口，用于获取服务器运行状态和监控数据。

## 基础信息

- **Base URL**: `http://your-server-ip:port`
- **协议**: HTTP/1.1
- **响应格式**: JSON
- **字符编码**: UTF-8

## 接口列表

### 1. 获取监控数据

获取服务器实时监控数据，包括传输速度、流量统计、运行时长等信息。

#### 请求

```http
GET /api/stats HTTP/1.1
Host: your-server-ip:port
Accept: application/json
```

#### 响应

**状态码**: 200 OK

**响应体**:
```json
{
  "upload_speed": "1.2 MB/s",
  "download_speed": "3.5 MB/s",
  "total_traffic": "10.5 GB",
  "uptime": "2 days 3 hours 45 minutes",
  "memory_usage": "45.2 MB",
  "active_connections": 12
}
```

#### 字段说明

| 字段 | 类型 | 说明 | 示例 |
|------|------|------|------|
| upload_speed | string | 上传速度，自动格式化 | "1.2 MB/s" |
| download_speed | string | 下载速度，自动格式化 | "3.5 MB/s" |
| total_traffic | string | 总流量（上传+下载），自动格式化 | "10.5 GB" |
| uptime | string | 服务器运行时长 | "2d 3h 45m 30s" |
| memory_usage | string | 内存使用量，自动格式化 | "45.2 MB" |
| total_memory | string | 总内存容量，自动格式化 | "16.0 GB" |
| active_connections | number | 当前活动连接数 | 12 |
| max_connections | number | 最大连接数 | 300 |

#### 错误响应

**状态码**: 500 Internal Server Error

```json
{
  "error": "Internal server error"
}
```

### 2. 获取速度历史

获取服务器速度历史数据，用于绘制趋势图。

#### 请求

```http
GET /api/speed-history HTTP/1.1
Host: your-server-ip:port
Accept: application/json
```

#### 响应

**状态码**: 200 OK

**响应体**:
```json
{
  "history": [
    {
      "timestamp": "0",
      "upload_speed": "1.2 MB/s",
      "download_speed": "3.5 MB/s"
    }
  ],
  "duration_seconds": 60
}
```

#### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| history | array | 历史数据数组，最多120个数据点 |
| history[].timestamp | string | 时间戳（相对值，秒） |
| history[].upload_speed | string | 上传速度 |
| history[].download_speed | string | 下载速度 |
| duration_seconds | number | 历史数据覆盖的时长（秒） |

### 3. 获取监控配置

获取服务器的监控配置参数，包括历史时长、广播间隔等可配置项。

#### 请求

```http
GET /api/config HTTP/1.1
Host: your-server-ip:port
Accept: application/json
```

#### 响应

**状态码**: 200 OK

**响应体**:
```json
{
  "speed_history_duration": 60,
  "broadcast_interval": 1,
  "websocket_max_connections": 300,
  "websocket_heartbeat_timeout": 60,
  "vless_max_connections": 300
}
```

#### 字段说明

| 字段 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| speed_history_duration | number | 流量历史保留时长（秒） | 60 |
| broadcast_interval | number | WebSocket广播间隔（秒） | 1 |
| websocket_max_connections | number | WebSocket最大连接数 | 300 |
| websocket_heartbeat_timeout | number | WebSocket心跳超时（秒） | 60 |
| vless_max_connections | number | VLESS最大连接数 | 300 |

### 4. 获取性能配置

获取服务器的性能优化配置参数。

#### 请求

```http
GET /api/performance HTTP/1.1
Host: your-server-ip:port
Accept: application/json
```

#### 响应

**状态码**: 200 OK

**响应体**:
```json
{
  "buffer_size": 131072,
  "tcp_nodelay": true,
  "tcp_recv_buffer": 262144,
  "tcp_send_buffer": 262144,
  "stats_batch_size": 65536
}
```

#### 字段说明

| 字段 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| buffer_size | number | 传输缓冲区大小（字节） | 131072 (128KB) |
| tcp_nodelay | boolean | 是否启用TCP_NODELAY | true |
| tcp_recv_buffer | number | TCP接收缓冲区大小（字节） | 262144 (256KB) |
| tcp_send_buffer | number | TCP发送缓冲区大小（字节） | 262144 (256KB) |
| stats_batch_size | number | 流量统计批量大小（字节） | 65536 (64KB) |

### 5. WebSocket 实时推送

建立 WebSocket 连接接收实时监控数据推送。

#### 连接

```javascript
const ws = new WebSocket('ws://your-server-ip:port/api/ws');
```

#### 消息格式

所有消息采用 JSON 格式，包含 `type` 和 `payload` 字段：

```json
{
  "type": "stats|history",
  "payload": { /* 数据对象 */ }
}
```

#### 消息类型

##### stats 消息

推送当前实时统计数据，格式与 `/api/stats` 响应相同。

```json
{
  "type": "stats",
  "payload": {
    "upload_speed": "1.2 MB/s",
    "download_speed": "3.5 MB/s",
    "total_traffic": "10.5 GB",
    "uptime": "2d 3h 45m 30s",
    "memory_usage": "45.2 MB",
    "total_memory": "16.0 GB",
    "active_connections": 12,
    "max_connections": 300
  }
}
```

##### history 消息

连接建立后推送一次历史数据，格式与 `/api/speed-history` 响应相同。

```json
{
  "type": "history",
  "payload": {
    "history": [
      {
        "timestamp": "0",
        "upload_speed": "1.2 MB/s",
        "download_speed": "3.5 MB/s"
      }
    ],
    "duration_seconds": 120
  }
}
```

#### 推送频率

- `stats` 消息：每秒推送一次
- `history` 消息：仅在连接建立时推送一次

#### 连接管理

- 最大并发连接数：300
- 心跳超时：60秒
- 支持 Origin 验证（通过 `VLESS_MONITOR_ORIGIN` 环境变量配置）

#### 使用示例

```javascript
const ws = new WebSocket('ws://localhost:8443/api/ws');

ws.onopen = () => {
  console.log('WebSocket connected');
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);

  if (msg.type === 'stats') {
    console.log('Upload:', msg.payload.upload_speed);
    console.log('Download:', msg.payload.download_speed);
  } else if (msg.type === 'history') {
    console.log('History data:', msg.payload.history);
  }
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('WebSocket disconnected');
};
```

### 6. 获取监控页面

获取监控页面的HTML内容，用于在浏览器中显示监控仪表盘。

#### 请求

```http
GET / HTTP/1.1
Host: your-server-ip:port
Accept: text/html
```

#### 响应

**状态码**: 200 OK

**Content-Type**: text/html; charset=utf-8

**响应体**: 完整的HTML页面，包含：
- 监控仪表盘UI
- 内联CSS样式
- JavaScript轮询逻辑
- 深色/浅色主题切换

#### 页面特性

- **自动刷新**: 每2秒自动更新数据
- **主题切换**: 支持深色/浅色主题，使用localStorage持久化
- **响应式设计**: 适配桌面、平板、移动设备
- **数据可视化**: 进度条、图表展示

#### 错误响应

**状态码**: 404 Not Found

```html
Not Found
```

## 数据格式说明

### 速度格式

速度值自动格式化，支持以下单位：
- B/s: 字节/秒
- KB/s: 千字节/秒
- MB/s: 兆字节/秒
- GB/s: 吉字节/秒

示例：
- "1024 B/s"
- "1.5 MB/s"
- "2.3 GB/s"

### 流量格式

流量值自动格式化，支持以下单位：
- B: 字节
- KB: 千字节
- MB: 兆字节
- GB: 吉字节
- TB: 太字节

示例：
- "1024 B"
- "1.5 MB"
- "10.5 GB"

### 时间格式

运行时长根据长度自动调整格式：
- 小于1分钟: "30s"
- 小于1小时: "45m 30s"
- 小于1天: "2h 45m 30s"
- 大于1天: "2d 3h 45m 30s"

## 使用示例

### JavaScript (浏览器)

```javascript
// 获取监控数据
async function fetchStats() {
  try {
    const response = await fetch('/api/stats');
    const data = await response.json();
    console.log('Upload speed:', data.upload_speed);
    console.log('Download speed:', data.download_speed);
    console.log('Total traffic:', data.total_traffic);
    console.log('Uptime:', data.uptime);
    console.log('Memory usage:', data.memory_usage);
    console.log('Active connections:', data.active_connections);
  } catch (error) {
    console.error('Failed to fetch stats:', error);
  }
}

// 每2秒更新一次
setInterval(fetchStats, 2000);
```

### Python

```python
import requests
import time

def fetch_stats():
    try:
        response = requests.get('http://your-server-ip:8443/api/stats')
        data = response.json()
        print(f"Upload speed: {data['upload_speed']}")
        print(f"Download speed: {data['download_speed']}")
        print(f"Total traffic: {data['total_traffic']}")
        print(f"Uptime: {data['uptime']}")
        print(f"Memory usage: {data['memory_usage']}")
        print(f"Active connections: {data['active_connections']}")
    except requests.RequestException as e:
        print(f"Failed to fetch stats: {e}")

# 每2秒更新一次
while True:
    fetch_stats()
    time.sleep(2)
```

### cURL

```bash
# 获取监控数据
curl http://your-server-ip:8443/api/stats

# 获取监控页面
curl http://your-server-ip:8443/

# 格式化JSON输出
curl http://your-server-ip:8443/api/stats | jq
```

### Go

```go
package main

import (
    "encoding/json"
    "fmt"
    "net/http"
    "time"
)

type MonitorData struct {
    UploadSpeed       string `json:"upload_speed"`
    DownloadSpeed     string `json:"download_speed"`
    TotalTraffic      string `json:"total_traffic"`
    Uptime            string `json:"uptime"`
    MemoryUsage       string `json:"memory_usage"`
    ActiveConnections int    `json:"active_connections"`
}

func fetchStats() (*MonitorData, error) {
    resp, err := http.Get("http://your-server-ip:8443/api/stats")
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var data MonitorData
    if err := json.NewDecoder(resp.Body).Decode(&data); err != nil {
        return nil, err
    }

    return &data, nil
}

func main() {
    for {
        data, err := fetchStats()
        if err != nil {
            fmt.Printf("Error: %v\n", err)
        } else {
            fmt.Printf("Upload: %s, Download: %s, Total: %s\n",
                data.UploadSpeed, data.DownloadSpeed, data.TotalTraffic)
        }
        time.Sleep(5 * time.Second)
    }
}
```

## 注意事项

### 性能考虑
- API响应时间通常在10ms以内
- 建议轮询间隔不小于2秒
- 避免频繁请求以减少服务器负载

### 数据准确性
- 速度数据基于最近10秒的流量计算
- 总流量数据每10分钟持久化一次
- 内存使用量为服务器进程的内存占用

### 错误处理
- 网络错误应进行重试
- 建议实现超时机制
- 解析错误应记录日志

### 安全建议
- 在生产环境建议使用HTTPS
- 限制API访问频率
- 考虑添加认证机制

## 版本历史

### v1.0.0 (2026-02-01)
- 初始版本
- 支持获取监控数据
- 支持获取监控页面
