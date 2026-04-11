# VLESS-Rust

Rust 编写的高性能 VLESS 服务端，提供原生 TCP 与 WebSocket 两种传输模式，并在同一监听端口上暴露简洁的 HTTP 信息页与 VLESS 链接生成接口。

## 项目定位

- 面向单机场景的轻量级 VLESS 服务端
- 优先关注性能、可移植性和部署简单度
- 配置文件与二进制放在同目录，无外部数据库依赖
- 支持交互式配置向导、TUI 运行界面和 Linux 服务化安装

## 功能特性

- 支持 VLESS 协议版本 `0` 与 `1`
- 支持 `TCP` 直连代理
- 支持 `WebSocket` 传输代理
- 支持基于 `UUID` 的用户认证
- 支持按邮箱生成 VLESS 分享链接
- 支持首次启动自动生成 `config.json`
- 支持 TUI 实时日志面板与传统日志模式
- 支持 Linux `systemd` / `OpenRC` 服务安装
- 支持 Windows、Linux x64、Linux ARM64、Linux ARMv7 构建

## 快速开始

### 1. 获取程序

可以直接下载 Release，或在本地构建：

```bash
cargo build --release
```

生成的可执行文件位于：

- Windows: `target/release/vless.exe`
- Linux/macOS: `target/release/vless`

### 2. 首次运行

```bash
./vless
```

如果当前目录下不存在 `config.json`，程序会自动启动交互式向导，引导你完成：

1. 监听地址
2. 监听端口
3. 传输协议选择
4. 用户 UUID 与邮箱配置

完成后会在程序同目录生成 `config.json`。

### 3. 常用启动方式

```bash
# 使用默认 config.json
./vless

# 指定配置文件
./vless ./config.json

# 关闭 TUI，使用传统日志输出
./vless --no-tui
```

## 配置文件

配置文件由三部分组成：

- `server`: 服务监听与传输协议
- `users`: 可认证的用户列表
- `performance`: 网络与缓冲区调优参数

### TCP 模式示例

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 443,
    "protocol": "tcp",
    "ws_path": "/vless"
  },
  "users": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "email": "user@example.com"
    }
  ],
  "performance": {
    "buffer_size": 65536,
    "tcp_recv_buffer": 131072,
    "tcp_send_buffer": 131072,
    "tcp_nodelay": true,
    "udp_timeout": 30,
    "udp_recv_buffer": 65536,
    "buffer_pool_size": 64,
    "ws_header_buffer_size": 8192
  }
}
```

### WebSocket 模式示例

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
  ],
  "performance": {
    "buffer_size": 65536,
    "tcp_recv_buffer": 131072,
    "tcp_send_buffer": 131072,
    "tcp_nodelay": true,
    "udp_timeout": 30,
    "udp_recv_buffer": 65536,
    "buffer_pool_size": 64,
    "ws_header_buffer_size": 8192
  }
}
```

## HTTP 接口

程序监听端口除了处理代理流量，也提供简单的 HTTP 页面与链接接口。

### 信息页

```text
GET /
```

返回一个 HTML 页面，展示：

- 产品名与版本号
- 公网 IP 或监听地址
- 端口
- 当前协议类型
- WebSocket 路径（启用时）


## Linux 服务化

```bash
# 安装服务
./vless --init

# 卸载服务
./vless --remove
```

当前支持：

- `systemd` 用户级服务
- `OpenRC` 系统服务

说明：

- `systemd` 以用户服务方式安装
- `OpenRC` 需要 root 权限
- 服务启动时会自动附带 `--no-tui`

## 构建与测试

```bash
# 调试构建
cargo build

# 发布构建
make release

# 全量测试
cargo test

# 单个测试文件
cargo test --test protocol_test
```


## 当前限制

- 未内置 TLS / WSS
- `Mux` 命令尚未实现
- `UDP over WebSocket` 尚未实现
- 无管理后台、无数据库、无配置热重载

## 文档导航

- `README.md`: 用户使用指南
- `docs/spec.md`: 技术规格书，定义系统“是什么”
- `docs/architecture.md`: 架构设计，说明系统“怎么做”
- `docs/todo.md`: 原子化任务进度表

## 许可证

Copyright (C) 2026
