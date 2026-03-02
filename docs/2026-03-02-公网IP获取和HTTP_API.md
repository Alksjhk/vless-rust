# 2026-03-02 更新日志

## 新增功能

### 公网 IP 自动获取
- 服务器启动时自动检测公网 IP
- 并发请求多个 IP API，取首个成功结果
- 用于状态显示和 VLESS 链接生成

### HTTP API
- 访问根路径 `/` 显示服务器信息页面（HTML）
- 通过 `/?email=user@example.com` 获取用户 VLESS 链接（JSON）
- 支持 TCP 和 WebSocket 两种链接格式

## 代码重构

### 模块化拆分
将 server.rs 拆分为多个子模块：

| 文件 | 功能 |
|------|------|
| `src/server.rs` | 调度器，根据协议类型分发请求 |
| `src/tcp.rs` | TCP 协议处理，包含 TCP/UDP 代理 |
| `src/socket.rs` | TCP Socket 配置 |
| `src/api.rs` | HTTP API 处理（信息页面、链接生成） |
| `src/public_ip.rs` | 公网 IP 获取 |
| `src/vless_link.rs` | VLESS 链接生成 |
| `src/http.rs` | HTTP 请求解析和响应构建 |

### WebSocket 增强
- 增加 WebSocket 升级请求检测 `is_websocket_upgrade()`
- HTTP 请求区分：WebSocket 升级请求 vs 普通 API 请求
