# VLESS Protocol Server in Rust

基于 Rust 和 Tokio 实现的高性能 VLESS 协议服务器，遵循 xray-core 协议规范。

## 特性

- 完整 VLESS 协议支持（版本 0/1）
- TCP + WebSocket 传输支持
- UDP over TCP
- 多用户 UUID 认证
- 可配置缓冲区（默认 128KB）
- 缓冲区池复用
- TCP_NODELAY 优化
- TUI 终端日志界面

## 快速开始

### 编译

```bash
cargo build --release
```

### 运行

```bash
# 首次运行自动启动配置向导
cargo run

# 指定配置文件
./target/release/vless.exe config.json

# 禁用 TUI（使用传统日志输出）
./target/release/vless.exe --no-tui
# 或
DISABLE_TUI=1 ./target/release/vless.exe
```

### 客户端配置

| 参数 | 值 |
|------|-----|
| 协议 | VLESS |
| 地址 | 服务器 IP |
| 端口 | 配置的端口 |
| UUID | 配置文件中的 UUID |
| 加密 | none |
| 传输 | TCP / WebSocket |

## 配置文件

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443,
    "protocol": "tcp",
    "ws_path": "/vless"
  },
  "users": [
    {
      "uuid": "uuid-string",
      "email": "user@example.com"
    }
  ],
  "performance": {
    "buffer_size": 131072,
    "tcp_nodelay": true,
    "tcp_recv_buffer": 262144,
    "tcp_send_buffer": 262144,
    "udp_timeout": 30,
    "udp_recv_buffer": 65536,
    "buffer_pool_size": 32,
    "ws_header_buffer_size": 8192
  }
}
```

### 配置项说明

**server**
| 参数 | 默认值 | 说明 |
|------|--------|------|
| listen | 0.0.0.0 | 监听地址 |
| port | 443 | 监听端口 |
| protocol | tcp | 传输协议 (tcp/ws) |
| ws_path | /vless | WebSocket 路径 (ws模式) |

**users**
| 参数 | 说明 |
|------|------|
| uuid | 用户唯一标识 |
| email | 用户邮箱 (可选) |

**performance** (可选)
| 参数 | 默认值 | 说明 |
|------|--------|------|
| buffer_size | 131072 | 传输缓冲区 (128KB) |
| tcp_nodelay | true | 启用 TCP_NODELAY |
| tcp_recv_buffer | 262144 | TCP 接收缓冲 (256KB) |
| tcp_send_buffer | 262144 | TCP 发送缓冲 (256KB) |
| udp_timeout | 30 | UDP 会话超时 (秒) |
| udp_recv_buffer | 65536 | UDP 接收缓冲 (64KB) |
| buffer_pool_size | 32 | 缓冲区池大小 |
| ws_header_buffer_size | 8192 | WebSocket 头缓冲 (8KB) |

## 部署

1. 编译：`cargo build --release`
2. 复制 `target/release/vless.exe` 到服务器
3. 创建 `config.json`
4. 运行：`./vless.exe`

## 安全注意

- UUID 是唯一认证凭据，请保密
- 建议生产环境配合 TLS 使用
- 合理配置防火墙规则

## 参考

- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core 项目](https://github.com/XTLS/Xray-core)

## 许可证

MIT
