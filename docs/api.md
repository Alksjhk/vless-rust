# API 接口文档

## 概述

VLESS-Rust 服务器提供 HTTP API 接口，用于获取服务器信息和生成 VLESS 链接。API 通过 HTTP 协议访问，默认监听配置的服务器端口。

**基础 URL**: `http://<server-ip>:<port>`

## 接口列表

### 1. 获取服务器信息

**接口路径**: `/`

**请求方法**: `GET`

**功能说明**: 获取服务器基本信息和状态，返回 HTML 格式的信息页面。

#### 请求示例

```http
GET / HTTP/1.1
Host: <server-ip>:<port>
```

#### 响应示例

```http
HTTP/1.1 200 OK
Content-Type: text/html; charset=utf-8
Content-Length: 1234

<!DOCTYPE html>
<html>
<head>
    <title>VLESS Server Info</title>
</head>
<body>
    <h1>VLESS Server Status</h1>
    <p>Server IP: xxx.xxx.xxx.xxx</p>
    <p>Port: 8443</p>
    <p>Protocol: TCP</p>
    <p>Version: 1.7.8</p>
</body>
</html>
```

#### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| Server IP | String | 服务器公网 IP 地址 |
| Port | Integer | 服务器监听端口 |
| Protocol | String | 协议类型（TCP/WebSocket） |
| Version | String | 服务器版本号 |

---

### 2. 生成 VLESS 链接

**接口路径**: `/`

**请求方法**: `GET`

**功能说明**: 根据用户邮箱生成对应的 VLESS 链接，返回 JSON 格式的链接数据。

#### 请求参数

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| email | String | 是 | 用户邮箱地址 |

#### 请求示例

```http
GET /?email=user@example.com HTTP/1.1
Host: <server-ip>:<port>
```

#### 响应示例

**TCP 协议模式**:

```json
{
  "tcp": "vless://550e8400-e29b-41d4-a716-446655440000@example.com:8443?encryption=none&security=none&type=tcp#user@example.com",
  "tcp_b64": "dmxlc3M6Ly81NTBlODQwMC1lMjliLTQxZDQtYTcxNi00NDY2NTU0NDAwMDBAZXhhbXBsZS5jb206ODQ0Mz9lbmNyeXB0aW9uPW5vbmUmc2VjdXJpdHk9bm9uZSZ0eXBlPXRjcCN1c2VyQGV4YW1wbGUuY29t"
}
```

**WebSocket 协议模式**:

```json
{
  "ws": "vless://550e8400-e29b-41d4-a716-446655440000@example.com:8443?encryption=none&security=none&type=ws&path=%2Fvless#user@example.com",
  "ws_b64": "dmxlc3M6Ly81NTBlODQwMC1lMjliLTQxZDQtYTcxNi00NDY2NTU0NDAwMDBAZXhhbXBsZS5jb206ODQ0Mz9lbmNyeXB0aW9uPW5vbmUmc2VjdXJpdHk9bm9uZSZ0eXBlPXdzJnBhdGg9JTJGdmxlc3MjdXNlckBleGFtcGxlLmNvbQ=="
}
```

#### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| tcp | String | TCP 协议的 VLESS 链接（仅 TCP 模式） |
| tcp_b64 | String | TCP 链接的 Base64 编码（仅 TCP 模式） |
| ws | String | WebSocket 协议的 VLESS 链接（仅 WS 模式） |
| ws_b64 | String | WS 链接的 Base64 编码（仅 WS 模式） |

#### VLESS 链接格式

**TCP 协议**:
```
vless://{uuid}@{host}:{port}?encryption=none&security=none&type=tcp#{alias}
```

**WebSocket 协议**:
```
vless://{uuid}@{host}:{port}?encryption=none&security=none&type=ws&path={path}#{alias}
```

**链接参数说明**:

| 参数 | 说明 |
|------|------|
| uuid | 用户唯一标识符 |
| host | 服务器 IP 地址 |
| port | 服务器端口 |
| encryption | 加密方式（固定为 none） |
| security | 安全类型（固定为 none） |
| type | 传输类型（tcp 或 ws） |
| path | WebSocket 路径（仅 WS 模式） |
| alias | 用户别名（email） |

---

### 3. 错误响应

#### 404 Not Found

**触发条件**: 访问不存在的路径

**响应示例**:

```http
HTTP/1.1 404 Not Found
Content-Type: text/plain; charset=utf-8
Content-Length: 9

Not Found
```

#### 400 Bad Request

**触发条件**: 请求参数错误或格式不正确

**响应示例**:

```http
HTTP/1.1 400 Bad Request
Content-Type: application/json
Content-Length: 35

{"error": "Invalid email parameter"}
```

#### 常见错误码

| 状态码 | 说明 | 解决方法 |
|--------|------|----------|
| 400 | 请求参数错误 | 检查请求参数格式 |
| 404 | 路径不存在 | 使用正确的 API 路径 |
| 500 | 服务器内部错误 | 查看服务器日志 |

---

## 使用示例

### cURL 示例

**获取服务器信息**:

```bash
curl http://example.com:8443/
```

**生成 VLESS 链接**:

```bash
curl "http://example.com:8443/?email=user@example.com"
```

### JavaScript 示例

**获取服务器信息**:

```javascript
fetch('http://example.com:8443/')
  .then(response => response.text())
  .then(html => console.log(html));
```

**生成 VLESS 链接**:

```javascript
const email = 'user@example.com';
fetch(`http://example.com:8443/?email=${encodeURIComponent(email)}`)
  .then(response => response.json())
  .then(data => {
    console.log('TCP Link:', data.tcp);
    console.log('WS Link:', data.ws);
  });
```

### Python 示例

**获取服务器信息**:

```python
import requests

response = requests.get('http://example.com:8443/')
print(response.text)
```

**生成 VLESS 链接**:

```python
import requests

email = 'user@example.com'
response = requests.get(f'http://example.com:8443/', params={'email': email})
data = response.json()

print('TCP Link:', data.get('tcp'))
print('WS Link:', data.get('ws'))
```

---

## 注意事项

1. **安全性**: API 接口无认证机制，建议配合防火墙规则限制访问来源
2. **邮箱匹配**: 邮箱参数必须与配置文件中的用户邮箱完全匹配
3. **协议模式**: 响应数据根据服务器配置的协议类型返回对应的链接
4. **URL 编码**: WebSocket 路径中的特殊字符已进行 URL 编码
5. **Base64 编码**: Base64 编码的链接可直接用于部分客户端导入

---

## 客户端配置

获取 VLESS 链接后，可配置到支持 VLESS 协议的客户端：

| 客户端 | 平台 | 链接导入支持 |
|--------|------|-------------|
| v2rayN | Windows | ✓ |
| v2rayNG | Android | ✓ |
| Shadowrocket | iOS | ✓ |
| Quantumult X | iOS | ✓ |
| Clash | 跨平台 | 需手动配置 |
