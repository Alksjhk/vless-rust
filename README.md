# VLESS Protocol Server in Rust

基于 Rust 和 Tokio 实现的高性能 VLESS 协议服务器，遵循 xray-core 协议规范。

## 特性

- **完整 VLESS 协议支持**: 版本 0/1 双版本兼容
- **双传输模式**: TCP + WebSocket
- **UDP over TCP**: UDP 代理封装传输
- **多用户认证**: UUID 唯一标识，支持邮箱
- **高性能优化**: TCP_NODELAY、可配置缓冲区、mimalloc 分配器
- **TUI 终端界面**: 实时日志显示，支持滚动查看
- **HTTP API**: VLESS 链接自动生成
- **公网 IP 检测**: 自动获取服务器地址
- **Linux 服务管理**: systemd/OpenRC 自动适配

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
./vless config.json

# 禁用 TUI（传统日志输出）
./vless --no-tui
# 或
DISABLE_TUI=1 ./vless
```

**优雅关闭**
- Unix: SIGINT (Ctrl+C)、SIGTERM
- Windows: Ctrl+C
- TUI界面使用`Q`或`q`
## 配置

### 配置文件示例

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
      "uuid": "",
      "email": ""
    }
  ],
  "performance": {
    "buffer_size": 65536,
    "tcp_nodelay": true
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
| ws_path | /vless | WebSocket 路径 |

**users**
| 参数 | 说明 |
|------|------|
| uuid | 用户唯一标识 |
| email | 用户邮箱 (可选) |

**performance**
| 参数 | 默认值 | 说明 |
|------|--------|------|
| buffer_size | 65536 | 传输缓冲区 (64KB) |
| tcp_nodelay | true | 禁用 Nagle 算法 |
| tcp_recv_buffer | 131072 | TCP 接收缓冲 (128KB) |
| tcp_send_buffer | 131072 | TCP 发送缓冲 (128KB) |
| udp_timeout | 30 | UDP 会话超时 (秒) |

## Linux 服务管理

### Systemd（无需 root）

```bash
# 安装并启动服务
./vless --init

# 卸载服务
./vless --remove

# 查看状态
systemctl --user status vless-rust-serve
journalctl --user -u vless-rust-serve -f
```

### OpenRC（需要 root）

```bash
# 安装并启动服务
sudo ./vless --init

# 卸载服务
sudo ./vless --remove

# 查看状态
rc-service vless-rust-serve status
tail -f /var/log/vless-rust-serve.log
```

## HTTP API

### 信息页面

```
http://<server-ip>:8443/
```

### 获取 VLESS 链接

```
http://<server-ip>:8443/?email=user@example.com
```

返回：
```json
{
  "tcp": "vless://uuid@ip:port?encryption=none...",
  "tcp_b64": "base64_encoded_link",
  "ws": "vless://uuid@ip:port?encryption=none&type=ws...",
  "ws_b64": "base64_encoded_link"
}
```

## 客户端配置

| 参数 | 值 |
|------|-----|
| 协议 | VLESS |
| 地址 | 服务器 IP |
| 端口 | 配置的端口 |
| UUID | 配置文件中的 UUID |
| 加密 | none |
| 传输 | TCP / WebSocket |

## 部署

1. 编译: `cargo build --release`
2. 复制 `target/release/vless` 到服务器
3. 运行 `./vless` 生成配置
4. 服务管理: `./vless --init`

## 安全注意

- UUID 是唯一认证凭据，请保密
- 建议生产环境配合 TLS 使用
- 配置文件权限自动设置为 600

## 技术文档

详见 [docs/tech.md](docs\technology.md)

## 参考

- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core](https://github.com/XTLS/Xray-core)

## 许可证

Copyright (C) 2026
