# 2026-02-23-WebSocket支持

## 概述

本次更新添加了 WebSocket 传输协议支持，增强了服务器的穿透能力。

## 新增功能

### WebSocket 协议支持
- 新增 `src/ws.rs` 模块，处理 WebSocket 握手和代理转发
- 支持 VLESS over WebSocket，可穿透防火墙和 NAT
- 可配置 WebSocket 路径

### 优雅关闭
- 添加 SIGINT/SIGTERM 信号处理
- Windows 支持 Ctrl+C 关闭
- Unix 支持 SIGINT 和 SIGTERM 信号

### 配置增强
- 新增 `protocol` 配置项（tcp/ws）
- 新增 `ws_path` 配置项（WebSocket 路径）
- 新增 `ws_header_buffer_size` 配置项

## 依赖更新

### 新增依赖
- `tokio-tungstenite`: WebSocket 实现
- `futures-util`: 异步流处理
- `sha1_smol`: SHA1 哈希
- `base64`: Base64 编解码
- `urlencoding`: URL 编码

### 依赖移除
- `sha1` → 替换为 `sha1_smol`

## 文件变更

| 文件 | 变更类型 | 说明 |
|-----|---------|------|
| `src/ws.rs` | 新增 | WebSocket 处理模块 |
| `src/config.rs` | 修改 | 添加协议类型配置 |
| `src/server.rs` | 修改 | 添加 WebSocket 处理 |
| `src/main.rs` | 修改 | 添加信号处理 |
| `src/wizard.rs` | 修改 | 添加协议选择 |
| `Cargo.toml` | 修改 | 添加 WebSocket 依赖 |

## 配置示例

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443,
    "protocol": "ws",
    "ws_path": "/vless"
  },
  "users": [
    {
      "uuid": "your-uuid",
      "email": "user@example.com"
    }
  ],
  "performance": {
    "ws_header_buffer_size": 8192
  }
}
```

## 使用方式

### 交互式配置
运行服务器时选择协议类型：
1. TCP - 原始 VLESS over TCP（推荐）
2. WS - VLESS over WebSocket（可穿透防火墙）
