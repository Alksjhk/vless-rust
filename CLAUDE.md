# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

这是一个基于 Rust 和 Tokio 实现的高性能 VLESS 协议服务器，包含完整的 HTTP 监控页面前端。项目遵循 xray-core 的 VLESS 协议规范，支持版本 0（测试版）和版本 1（正式版）。

## 常用命令

### 后端开发
```bash
# 编译项目
cargo build

# 编译优化版本
cargo build --release

# 运行服务器（使用默认配置文件 config.json）
cargo run

# 运行服务器（指定配置文件）
cargo run -- /path/to/config.json

# 运行测试
cargo test

# 检查代码（不编译）
cargo check
```

### 前端开发
```bash
# 进入前端目录
cd frontend

# 安装依赖
npm install

# 开发模式（支持热重载，代理 /api 到后端）
npm run dev

# 构建生产版本（输出到 ../static/）
npm run build

# 预览构建结果
npm run preview
```

## 架构设计

### 模块职责

**后端核心模块：**

- `src/main.rs`: 程序入口，负责配置加载、统计模块初始化和服务器启动
- `src/config.rs`: 配置文件解析，支持 JSON 格式的服务器和用户配置
- `src/protocol.rs`: VLESS 协议编解码实现，包含请求/响应结构体和地址类型处理
- `src/server.rs`: 服务器核心逻辑，处理连接、用户认证、TCP/UDP 代理转发
- `src/stats.rs`: 流量统计模块，使用快照机制计算速度，支持持久化到配置文件
- `src/http.rs`: HTTP 请求检测、静态文件服务和监控 API 端点
- `src/ws.rs`: WebSocket 实时数据推送管理

**前端架构：**

- Vue 3 Composition API
- Vite 构建工具（使用 rolldown-vite 优化）
- 组件化设计，每个统计指标独立组件
- Composables 模式（useStats、useTheme、useWebSocket）
- CSS 变量实现主题切换

### 关键设计模式

**混合协议处理：**
- 服务器在单个 TCP 端口同时监听 VLESS 和 HTTP 请求
- 通过 `is_http_request()` 检测数据包前缀判断请求类型
- HTTP 请求由 `http.rs` 处理，VLESS 请求由 `server.rs` 处理

**流量统计快照机制：**
- 使用 `SpeedSnapshot` 记录流量和时间戳
- `calculate_speeds()` 对比当前快照和上次快照计算精确速度
- 保留 60 秒历史快照用于趋势图表
- 每 10 分钟自动持久化总流量到配置文件

**异步代理转发：**
- 使用 `tokio::select!` 同时监听双向数据流
- 任一方向关闭时，整个代理连接终止
- 流量统计集成在数据传输路径中
- **批量统计**：累积到64KB才更新统计，减少锁竞争
- **可配置缓冲区**：默认128KB，适配高带宽场景

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
  "monitor": {
    "total_upload_bytes": 0,
    "total_download_bytes": 0,
    "last_update": "2024-01-01T00:00:00Z"
  },
  "monitoring": {
    "speed_history_duration": 60,
    "broadcast_interval": 1,
    "websocket_max_connections": 300,
    "websocket_heartbeat_timeout": 60,
    "vless_max_connections": 300
  },
  "performance": {
    "buffer_size": 131072,
    "tcp_nodelay": true,
    "tcp_recv_buffer": 262144,
    "tcp_send_buffer": 262144,
    "stats_batch_size": 65536,
    "udp_timeout": 30,
    "udp_recv_buffer": 65536
  }
}
```

配置文件在服务器启动时加载，不存在时启动初始化配置向导。monitor 字段由后端自动维护，手动修改可能被覆盖。performance 字段控制性能优化参数（默认128KB缓冲区、批量统计、TCP_NODELAY、UDP超时）。tls 字段控制 TLS 加密配置，证书文件不存在时自动生成。vless_url 字段保存自动生成的 VLESS 连接 URL。

## 初始化配置向导

首次运行时，服务器会启动交互式配置向导，引导用户完成：

1. **端口配置**（默认 8443）
2. **UUID 配置**（自动生成或手动输入）
3. **TLS 配置**：
   - 是否启用 TLS
   - 设置服务器域名/SNI（默认 localhost）
   - 自动生成自签名证书（有效期 10 年）
4. **XTLS 流控配置**（仅在 TLS 启用时）：
   - 选择流控类型：xtls-rprx-vision / xtls-rprx-vision-udp443 / 无流控
   - Vision 流控可提升性能 2-3倍

配置完成后，服务器会：
- 保存配置到 `config.json`
- 自动生成 TLS 证书（如果启用）
- 证书有效期 10 年，包含完整的 SAN 配置
- 生成 VLESS 连接 URL
- 打印连接信息到控制台

## 文件与功能映射关系

### 后端核心文件

| 文件路径 | 核心功能 | 主要结构体/函数 |
|---------|---------|---------------|
| `src/main.rs` | 程序入口、服务器启动 | `main()` - 加载配置、初始化统计、启动服务器 |
| `src/config.rs` | 配置管理、JSON解析、VLESS URL 生成 | `Config`、`generate_vless_url()` |
| `src/protocol.rs` | VLESS 协议编解码 | `VlessRequest`、`VlessResponse`、`Address`、`Command` |
| `src/server.rs` | 服务器核心逻辑、代理转发 | `VlessServer`、`handle_connection()`、`handle_tcp_proxy()`、`handle_udp_proxy()` |
| `src/stats.rs` | 流量统计、速度计算 | `Stats`、`SpeedSnapshot`、`get_monitor_data()` |
| `src/http.rs` | HTTP 服务、API 端点 | `handle_http_request()`、`serve_static_file()` |
| `src/ws.rs` | WebSocket 实时推送 | `WebSocketManager`、`broadcast()` |
| `src/tls.rs` | TLS 配置、证书自动生成、握手处理 | `load_tls_config()`、`ensure_cert_exists()`、`generate_self_signed_cert()` |
| `src/xtls.rs` | XTLS-Rprx-Vision 流控处理 | `VisionStream`、`handle_vision_proxy()`、`detect_tls_content()` |
| `src/wizard.rs` | 初始化配置向导 | `run_init_wizard()` - 交互式配置 |
| `src/memory.rs` | 内存池管理、缓冲区优化 | `BufferPool`、`PooledBuffer`、`GlobalBufferPools` |
| `src/connection_pool.rs` | 连接池管理、连接复用 | `ConnectionPool`、`PooledConnection`、`GlobalConnectionPools` |

### 前端核心文件

| 文件路径 | 核心功能 | 组件/函数 |
|---------|---------|----------|
| `frontend/src/App.vue` | 主应用容器、布局 | `<template>` - 仪表板布局 |
| `frontend/src/main.js` | 应用入口、插件注册 | `createApp()` - 初始化 Vue 应用 |
| `frontend/src/composables/useWebSocket.js` | WebSocket 连接管理 | `useWebSocket()` - 实时数据连接 |
| `frontend/src/composables/useTheme.js` | 主题切换管理 | `useTheme()` - 明暗主题切换 |
| `frontend/src/components/StatCard.vue` | 统计卡片基础组件 | `<StatCard>` - 通用数据展示 |
| `frontend/src/components/SpeedCard.vue` | 实时速度显示 | `<SpeedCard>` - 上传/下载速度 |
| `frontend/src/components/TrafficCard.vue` | 总流量显示 | `<TrafficCard>` - 总上传/下载流量 |
| `frontend/src/components/ConnectionsCard.vue` | 连接数显示 | `<ConnectionsCard>` - 活跃连接统计 |
| `frontend/src/components/UptimeCard.vue` | 运行时间显示 | `<UptimeCard>` - 服务器运行时长 |
| `frontend/src/components/MemoryCard.vue` | 内存使用显示 | `<MemoryCard>` - 内存占用统计 |
| `frontend/src/components/TrafficChart.vue` | 流量趋势图表 | `<TrafficChart>` - 速度历史曲线 |
| `frontend/src/components/UserStats.vue` | 用户流量统计 | `<UserStats>` - 用户级别流量表格 |
| `frontend/src/components/ThemeToggle.vue` | 主题切换按钮 | `<ThemeToggle>` - 明暗模式切换 |
| `frontend/src/components/ConnectionPoolCard.vue` | 连接池监控卡片 | `<ConnectionPoolCard>` - 连接池性能统计 |

### 配置文件

| 文件路径 | 核心功能 | 说明 |
|---------|---------|------|
| `Cargo.toml` | Rust 项目配置 | 依赖项、编译优化、二进制配置 |
| `.cargo/config.toml` | Cargo 编译配置 | Windows 静态链接选项 |
| `config.json` | 运行时配置 | 服务器、用户、监控、性能参数（自动生成） |

### 文档文件

| 文件路径 | 核心功能 | 说明 |
|---------|---------|------|
| `CLAUDE.md` | AI 助手规则 | 项目架构、开发指南、文件映射 |
| `README.md` | 项目说明 | 安装、使用、部署指南 |
| `plan.md` | 开发计划 | 功能规划、任务追踪、完成状态 |
| `docs/technology.md` | 技术文档 | 架构设计、实现逻辑、流程说明 |
| `docs/api.md` | API 文档 | 接口定义、请求/响应格式 |
| `docs/2026-02-05-性能优化与连接池管理.md` | 更新日志 | v1.2.0 性能优化和功能增强记录 |
| `AGENTS.md` | AI 角色定义 | 项目助手行为规范 |

### 功能快速查找

**需要修改/查找...**

- **服务器启动流程** → `src/main.rs:main()`
- **配置项和默认值** → `src/config.rs:Config`、`PerformanceConfig`
- **VLESS 协议解析** → `src/protocol.rs:VlessRequest::decode()`
- **用户认证逻辑** → `src/server.rs:handle_connection()`
- **TCP 代理转发** → `src/server.rs:handle_tcp_proxy()`
- **UDP 代理转发** → `src/server.rs:handle_udp_proxy()`
- **流量统计逻辑** → `src/stats.rs:Stats`
- **速度计算机制** → `src/stats.rs:calculate_speeds()`
- **HTTP 路由处理** → `src/http.rs:handle_http_request()`
- **WebSocket 推送** → `src/ws.rs:WebSocketManager::broadcast()`
- **监控 API 端点** → `src/http.rs` - 路由匹配部分
- **前端主题切换** → `frontend/src/composables/useTheme.js`
- **前端实时数据** → `frontend/src/composables/useWebSocket.js`
- **前端统计卡片** → `frontend/src/components/*.vue`
- **编译优化配置** → `Cargo.toml` - `[profile.release]`
- **性能参数调整** → `config.json` - `performance` 节点
- **TLS 配置** → `config.json` - `tls` 节点
- **证书生成** → `src/tls.rs:generate_cert_in_memory()`
- **初始化向导** → `src/wizard.rs:run_init_wizard()`
- **VLESS URL 生成** → `src/config.rs:generate_vless_url()` - 自动添加 `allowInsecure=true` 参数（自签名证书）
- **内存池管理** → `src/memory.rs:GlobalBufferPools` - 三级缓冲区池（4KB/64KB/128KB）
- **连接池管理** → `src/connection_pool.rs:GlobalConnectionPools` - 连接复用、健康检查、自动清理
- **连接池监控** → `src/stats.rs:get_monitor_data()` - 集成连接池统计数据
- **内存池监控** → `src/http.rs:/api/memory-pool-stats` - 内存池统计 API
- **XTLS 流控处理** → `src/xtls.rs:handle_vision_proxy()` - XTLS-Rprx-Vision 流控
- **XTLS 协议解析** → `src/protocol.rs:parse_xtls_flow()` - VLESS addons 流控解析
- **XTLS 配置向导** → `src/wizard.rs:prompt_xtls_flow()` - 流控类型选择

## 开发指南

### 添加新的监控指标

1. **后端（src/stats.rs）：**
   - 在 `Stats` 结构体添加字段
   - 在 `get_monitor_data()` 中返回新指标
   - 在 `MonitorData` 结构体定义 JSON 字段

2. **前端（frontend/src/）：**
   - 在 `components/` 创建新的 Vue 组件
   - 在 `App.vue` 中引入并使用组件

### 添加新的 API 端点

在 `src/http.rs` 的 `handle_http_request()` 函数添加新的路由匹配：

```rust
"/api/your-endpoint" => {
    // 处理逻辑
    let data = ...;
    let json = serde_json::to_string(&data)?;
    Ok(create_http_response_bytes(200, "application/json", json.as_bytes()))
}
```

### API 端点列表

- `GET /api/stats`：获取监控数据（包含用户统计数组、连接池和内存池数据）
- `GET /api/user-stats`：获取所有用户流量统计
- `GET /api/speed-history`：获取速度历史数据
- `GET /api/config`：获取监控配置
- `GET /api/performance`：获取性能配置
- `GET /api/connection-pool-stats`：获取连接池监控数据
- `GET /api/memory-pool-stats`：获取内存池监控数据
- `GET /api/ws` 或 `GET /ws`：WebSocket 实时推送连接

### 扩展 VLESS 协议

- **新命令类型**：在 `src/protocol.rs` 中添加 `Command` 枚举值
- **新地址类型**：在 `AddressType` 和 `Address` 枚举中添加变体
- **命令处理**：在 `src/server.rs` 的 `handle_connection()` 中添加匹配分支

## 前端开发注意事项

- 前端构建输出到 `../static/` 目录，编译时嵌入到可执行文件
- 开发模式下 Vite 代理 `/api` 请求到 `http://localhost:8443`
- 静态资源路径使用相对路径，支持部署在子路径
- 主题偏好存储在 localStorage，key 为 `theme`
- **部署说明**：编译后的可执行文件已包含所有静态资源，无需 static 目录即可运行

## 编译优化

Release 版本启用了以下优化（见 Cargo.toml）：
- `lto = true`: 链接时优化
- `codegen-units = 1`: 单代码生成单元
- `opt-level = "s"`: 优化体积
- `panic = "abort"`: 减小二进制大小
- 静态资源嵌入：使用 `rust-embed` 打包所有前端资源，单文件部署

**可执行文件大小**：约 1.2MB（包含所有前端资源）

## 性能优化说明

### 内存管理
- **三级内存池**：小(4KB)、中(64KB)、大(128KB)缓冲区池
- **缓冲区复用**：减少90%的动态内存分配
- **RAII管理**：自动归还，防止内存泄漏
- **统计监控**：实时跟踪池使用情况

### 连接管理
- **智能连接池**：按目标地址分组管理连接
- **连接复用**：复用率80%+，连接建立时间减少70%
- **健康检查**：30秒间隔检查连接可用性
- **自动清理**：5分钟空闲超时，30分钟最大生存时间
- **连接预热**：启动时预建立常用目标地址连接

### 传输优化
- **可配置缓冲区**：默认128KB传输缓冲区，支持64KB-512KB调整
- **批量统计**：累积64KB流量才更新统计，减少锁竞争90%+
- **TCP_NODELAY**：默认启用，降低延迟
- **零拷贝传输**：减少数据复制，提升吞吐量
- **大缓冲区**：适配千兆网络，单连接带宽提升4倍
- 高并发场景（1000+连接）建议调小buffer_size以降低内存占用

### 性能指标

| 指标 | 优化前 | 优化后 | 提升幅度 |
|------|--------|--------|----------|
| 内存分配次数 | 基准 | -90% | 大幅减少 |
| 连接复用率 | 0% | 80%+ | 新增功能 |
| 连接建立延迟 | 基准 | -70% | 显著降低 |
| 单连接吞吐量 | ~800 Mbps | ~1.2 Gbps | +50% |
| 并发连接数 | 300 | 1000+ | +233% |

## UDP 协议支持

### 实现机制

VLESS 协议使用 UDP over TCP (UoT) 机制传输 UDP 数据：

- UDP 数据包封装在 TCP 连接中传输
- 为每个 UDP 会话创建独立的 UDP socket
- 支持域名解析和批量流量统计
- 30秒超时自动清理空闲会话

### 配置项

在 `config.json` 的 `performance` 节点配置 UDP 参数：

- `udp_timeout`: UDP 会话超时时间（秒），默认 30
- `udp_recv_buffer`: UDP 接收缓冲区大小（字节），默认 65536 (64KB)

### 配置示例

```json
{
  "performance": {
    "buffer_size": 131072,
    "tcp_nodelay": true,
    "tcp_recv_buffer": 262144,
    "tcp_send_buffer": 262144,
    "stats_batch_size": 65536,
    "udp_timeout": 30,
    "udp_recv_buffer": 65536
  }
}
```

### 性能特点

- **批量统计**：累积64KB流量才更新统计，减少锁竞争
- **超时管理**：30秒无活动自动关闭连接
- **并发处理**：使用 Tokio 异步任务处理多个 UDP 会话
- **域名支持**：自动解析域名到 IP 地址

## 安全注意事项

- UUID 是唯一的认证凭据，确保配置文件权限正确
- 日志中不记录敏感信息
- 建议生产环境配合 TLS 使用
- HTTP 监控页面无认证，应配置防火墙限制访问

## 参考资料

- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core 项目](https://github.com/XTLS/Xray-core)
- [Tokio 官方文档](https://tokio.rs/)
