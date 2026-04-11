# VLESS-Rust

高性能 VLESS 代理协议服务器，使用 Rust 编写。

## 特性

- **双协议支持**: TCP 直连 和 WebSocket（穿透防火墙）
- **高性能**: 异步 I/O，零拷贝解析，优化的内存分配器
- **TUI 仪表盘**: 实时终端界面显示连接和日志
- **HTTP API**: 自动生成 VLESS 订阅链接
- **一键服务化**: 支持 systemd/OpenRC 服务安装
- **跨平台**: Windows、Linux (x64/ARM64/ARMv7)
- **静态链接**: 无需依赖，单文件运行

## 快速开始

### 下载预编译二进制

从 [Releases](https://github.com/DoPlGek/vless-rust/releases) 下载对应平台的二进制文件。

### 运行

```bash
# 直接运行（首次启动会启动配置向导）
./vless

# 指定配置文件路径
./vless /path/to/config.json

# 禁用 TUI，使用传统日志模式
./vless --no-tui
```

### 配置向导

首次运行时会自动启动交互式配置向导：

1. 设置监听地址（默认 0.0.0.0）
2. 设置监听端口（默认 443）
3. 选择传输协议（TCP 或 WebSocket）
4. 添加用户（自动生成 UUID）

配置完成后会生成 `config.json` 文件。

## 配置示例

### TCP 模式

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 443,
    "protocol": "tcp"
  },
  "users": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "email": "user@example.com"
    }
  ]
}
```

### WebSocket 模式

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 443,
    "protocol": "ws",
    "ws_path": "/vless"
  },
  "users": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "email": "user@example.com"
    }
  ]
}
```

## Linux 服务安装

```bash
# 安装为 systemd 用户服务（无需 sudo，安装到用户级服务目录）
./vless --init

# 查看服务状态
systemctl --user status vless-rust-serve

# 停止服务
systemctl --user stop vless-rust-serve

# 卸载服务
./vless --remove
```

## 获取 VLESS 链接

浏览器访问：

```
http://your-server-ip:port/?email=user@example.com
```

返回 JSON 格式的 VLESS 链接（含 Base64 编码版本）。

## 构建

### 环境要求

- Rust 1.70+
- Linux 交叉编译需要 cargo-zigbuild（ARM 目标）

### 构建命令

```bash
# 构建发布版本
make release

# 构建调试版本
make dev

# 清理
cargo clean
```

### 交叉编译

```bash
# Linux ARM64
cargo zigbuild --release --target aarch64-unknown-linux-musl

# Linux ARMv7
cargo zigbuild --release --target armv7-unknown-linux-musleabihf
```

## 项目文档

| 文档 | 说明 |
|------|------|
| [docs/spec.md](docs/spec.md) | 技术规范（协议、API、配置） |
| [docs/architecture.md](docs/architecture.md) | 架构设计（模块、并发、安全） |
| [docs/todo.md](docs/todo.md) | 任务进度追踪 |
| [CLAUDE.md](CLAUDE.md) | 开发指南（供 Claude Code 使用） |

## 客户端配置示例

### v2rayN (Windows)

1. 从服务器获取 VLESS 链接
2. v2rayN → 服务器 → 从剪贴板导入批量 URL
3. 或手动添加：
   - 地址：服务器 IP
   - 端口：443
   - 用户 ID：配置中的 UUID
   - 传输协议：tcp 或 ws

### v2rayNG (Android)

1. 获取 Base64 编码的链接
2. v2rayNG → 右上角 + → 从剪贴板导入

### Shadowrocket (iOS)

1. 扫描二维码（需先生成二维码）
2. 或手动配置类型为 VLESS

## 许可证

Copyright (C) 2026
