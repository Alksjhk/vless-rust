# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

这是一个基于 Rust 和 Tokio 实现的高性能 VLESS 协议服务器。项目遵循 xray-core 的 VLESS 协议规范，支持版本 0（测试版）和版本 1（正式版）。

## 常用命令

### 编译和运行
```bash
# 编译项目
cargo build

# 编译优化版本
cargo build --release

# 运行服务器（使用默认配置文件 config.json）
cargo run

# 运行服务器（指定配置文件）
cargo run -- /path/to/config.json

# 检查代码（不编译）
cargo check
```

## 架构设计

### 模块职责

**后端核心模块：**

- `src/main.rs`: 程序入口，负责配置加载和服务器启动
- `src/config.rs`: 配置文件解析，支持 JSON 格式的服务器和用户配置
- `src/protocol.rs`: VLESS 协议编解码实现，包含请求/响应结构体和地址类型处理
- `src/server.rs`: 服务器核心逻辑，处理连接、用户认证、TCP/UDP 代理转发
- `src/http.rs`: HTTP 请求检测（用于区分 HTTP 和 VLESS 请求）
- `src/utils.rs`: 工具函数，VLESS URL 生成
- `src/wizard.rs`: 配置向导，交互式生成配置文件
- `src/buffer_pool.rs`: 缓冲区池，复用缓冲区减少内存分配

### 关键设计模式

**协议请求检测：**
- 通过 `is_http_request()` 检测数据包前缀判断请求类型
- 拒绝 HTTP 请求，只处理 VLESS 协议请求

**异步代理转发：**
- 使用 `tokio::spawn` 同时处理双向数据流
- 任一方向关闭时，整个代理连接终止
- **可配置缓冲区**：默认128KB，适配高带宽场景
- **缓冲区池**：复用缓冲区，减少内存分配开销

### 配置文件结构

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443
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
    "buffer_pool_size": 32
  }
}
```

配置文件在服务器启动时加载，不存在时自动启动配置向导。

## 文件与功能映射关系

### 后端核心文件

| 文件路径 | 核心功能 | 主要结构体/函数 |
|---------|---------|---------------|
| `src/main.rs` | 程序入口、服务器启动 | `main()` - 加载配置、启动服务器 |
| `src/config.rs` | 配置管理、JSON解析 | `Config`、`ServerConfig`、`UserConfig`、`PerformanceConfig` |
| `src/protocol.rs` | VLESS 协议编解码 | `VlessRequest`、`VlessResponse`、`Address`、`Command` |
| `src/server.rs` | 服务器核心逻辑、代理转发 | `VlessServer`、`handle_connection()`、`handle_tcp_proxy()`、`handle_udp_proxy()` |
| `src/http.rs` | HTTP 请求检测 | `is_http_request()` |
| `src/utils.rs` | 工具函数、URL生成 | `generate_vless_url()` |
| `src/wizard.rs` | 配置向导 | `ConfigWizard::run()` |
| `src/buffer_pool.rs` | 缓冲区池 | `BufferPool` |

### 配置文件

| 文件路径 | 核心功能 | 说明 |
|---------|---------|------|
| `Cargo.toml` | Rust 项目配置 | 依赖项、编译优化、二进制配置 |
| `config.json` | 运行时配置 | 服务器、用户、性能参数（自动生成） |

### 文档文件

| 文件路径 | 核心功能 | 说明 |
|---------|---------|------|
| `CLAUDE.md` | AI 助手规则 | 项目架构、开发指南、文件映射 |
| `README.md` | 项目说明 | 安装、使用、部署指南 |
| `AGENTS.md` | AI 角色定义 | 项目助手行为规范 |

### 功能快速查找

**需要修改/查找...**

- **服务器启动流程** → `src/main.rs:main()`
- **配置项和默认值** → `src/config.rs:Config`、`PerformanceConfig`
- **VLESS 协议解析** → `src/protocol.rs:VlessRequest::decode()`
- **用户认证逻辑** → `src/server.rs:handle_connection()`
- **TCP 代理转发** → `src/server.rs:handle_tcp_proxy()`
- **UDP 代理转发** → `src/server.rs:handle_udp_proxy()`
- **HTTP 请求检测** → `src/http.rs:is_http_request()`
- **编译优化配置** → `Cargo.toml` - `[profile.release]`
- **性能参数调整** → `config.json` - `performance` 节点

## 开发指南

### 扩展 VLESS 协议

- **新命令类型**：在 `src/protocol.rs` 中添加 `Command` 枚举值
- **新地址类型**：在 `Address` 枚举中添加变体
- **命令处理**：在 `src/server.rs` 的 `handle_connection()` 中添加匹配分支

## 编译优化

Release 版本启用了以下优化（见 Cargo.toml）：
- `lto = "fat"`: 链接时优化
- `codegen-units = 16`: 代码生成单元数量
- `opt-level = 3`: 优化级别
- `panic = "abort"`: 减小二进制大小

## 性能优化说明

- **可配置缓冲区**：默认128KB传输缓冲区
- **缓冲区池**：复用缓冲区，减少内存分配开销
- **TCP_NODELAY**：默认启用，降低延迟
- **大缓冲区**：适配千兆网络

## UDP 协议支持

### 实现机制

VLESS 协议使用 UDP over TCP (UoT) 机制传输 UDP 数据：

- UDP 数据包封装在 TCP 连接中传输
- 为每个 UDP 会话创建独立的 UDP socket
- 支持域名解析
- 30秒超时自动清理空闲会话

### 配置项

在 `config.json` 的 `performance` 节点配置 UDP 参数：

- `udp_timeout`: UDP 会话超时时间（秒），默认 30
- `udp_recv_buffer`: UDP 接收缓冲区大小（字节），默认 65536 (64KB)

## 安全注意事项

- UUID 是唯一的认证凭据，确保配置文件权限正确
- 日志中不记录敏感信息
- 建议生产环境配合 TLS 使用
- 配置防火墙规则限制访问

## 参考资料

- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core 项目](https://github.com/XTLS/Xray-core)
- [Tokio 官方文档](https://tokio.rs/)
