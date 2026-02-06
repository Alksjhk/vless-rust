# 技术文档

## 项目概述

VLESS-Rust 是一个基于 Rust 和 Tokio 异步运行时实现的高性能 VLESS 协议服务器，完全遵循 xray-core 的 VLESS 协议规范。项目采用现代化的技术栈，提供完整的 HTTP 监控页面和 WebSocket 实时数据推送功能。

## 技术栈

### 后端
- **语言**: Rust 2021 Edition
- **异步运行时**: Tokio 1.0 (full features)
- **协议**: VLESS (版本 0 和版本 1)
- **序列化**: serde + serde_json
- **WebSocket**: tokio-tungstenite
- **日志**: tracing + tracing-subscriber
- **静态资源嵌入**: rust-embed
- **系统信息**: sysinfo
- **HTTP 客户端**: reqwest (rustls-tls)

### 前端
- **框架**: Vue 3 (Composition API)
- **构建工具**: Vite (rolldown-vite 优化)
- **实时通信**: WebSocket + API 降级
- **状态管理**: Composables 模式
- **样式**: CSS 变量 + 响应式设计

## 架构设计

### 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                         客户端连接                            │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
        ┌────────────────────────┐
        │   TCP 端口 (8443)      │
        └────────────────────────┘
                     │
                     ▼
        ┌────────────────────────┐
        │  协议检测层             │
        │  is_http_request()     │
        └────────┬───────────────┘
                 │
        ┌────────┴────────┐
        │                 │
        ▼                 ▼
┌──────────────┐  ┌──────────────┐
│ HTTP 请求    │  │ VLESS 请求   │
│              │  │              │
├──────────────┤  ├──────────────┤
│ 静态文件     │  │ UUID 验证    │
│ API 端点     │  │ 命令处理     │
│ WebSocket    │  │ TCP 代理     │
└──────────────┘  └──────────────┘
        │                 │
        └────────┬────────┘
                 ▼
        ┌──────────────────┐
        │   统计模块       │
        │   (Stats)        │
        └──────────────────┘
                 │
        ┌────────┴────────┐
        │                 │
        ▼                 ▼
┌──────────────┐  ┌──────────────┐
│ 持久化存储   │  │ 广播推送     │
│ 配置文件     │  │ WebSocket    │
└──────────────┘  └──────────────┘
```


## 文件与功能映射关系

### 后端核心文件

| 文件路径 | 核心功能 | 主要结构体/函数 |
|---------|---------|---------------|
| `src/main.rs` | 程序入口、服务器启动 | `main()` - 加载配置、初始化统计、启动服务器、IP检测、配置向导触发 |
| `src/config.rs` | 配置管理、JSON解析 | `Config`、`ServerConfig`、`UserConfig`、`PerformanceConfig` |
| `src/protocol.rs` | VLESS 协议编解码 | `VlessRequest`、`VlessResponse`、`Address`、`Command` |
| `src/server.rs` | 服务器核心逻辑、代理转发 | `VlessServer`、`handle_connection()`、`handle_tcp_proxy()`、`handle_udp_proxy()` |
| `src/stats.rs` | 流量统计、速度计算 | `Stats`、`SpeedSnapshot`、`get_monitor_data()` |
| `src/http.rs` | HTTP 服务、API 端点 | `handle_http_request()`、`serve_static_file()` |
| `src/ws.rs` | WebSocket 实时推送 | `WebSocketManager`、`broadcast()` |
| `src/utils.rs` | 工具函数、IP检测、URL生成 | `get_public_ip()`、`generate_vless_url()` |
| `src/wizard.rs` | 交互式配置向导 | `ConfigWizard`、`run()`、`prompt_listen_address()`、`prompt_port()`、`prompt_users()` |

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
| `docs/2026-02-02-监控和性能优化.md` | 更新日志 | 监控优化、性能提升记录 |
| `AGENTS.md` | AI 角色定义 | 项目助手行为规范 |

### 功能快速查找

**需要修改/查找...**

- **服务器启动流程** → `src/main.rs:main()`
- **首次配置向导** → `src/wizard.rs:ConfigWizard::run()`
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

### 核心模块

#### 1. 协议处理模块 (protocol.rs)

**职责**: VLESS 协议的编解码实现

**核心结构**:
- `VlessRequest`: 解码后的 VLESS 请求
  - 版本号 (u8)
  - 用户 UUID
  - 命令类型 (TCP/UDP/Mux)
  - 目标地址和端口
  - 附加数据

- `VlessResponse`: VLESS 响应编码
  - 版本号 (与请求保持一致)
  - 附加数据长度
  - 附加数据

**地址类型支持**:
- IPv4 (4字节)
- IPv6 (16字节)
- 域名 (1字节长度 + 域名)

**编解码流程**:
```
请求: [版本][UUID][附加长度][附加数据][命令][端口][地址类型][地址][请求数据]
       ↓ decode()
响应: [版本][附加长度][附加数据]
```

#### 2. 服务器模块 (server.rs)

**职责**: 核心服务器逻辑，连接处理和代理转发

**关键功能**:

**混合协议检测**:
```rust
if is_http_request(&header_bytes) {
    // HTTP 请求处理路径
} else {
    // VLESS 请求处理路径
}
```

**RAII 连接管理**:
- `ConnectionGuard` 自动管理连接计数
- 生命周期结束时自动递减计数
- 确保连接数统计的准确性

**TCP 代理转发**:
- 使用 `tokio::select!` 同时监听双向数据流
- 批量统计机制 (64KB 批次) 减少锁竞争
- 可配置缓冲区大小 (默认 128KB)

**性能优化**:
- TCP_NODELAY 启用 (降低延迟)
- 大缓冲区适配千兆网络
- 批量统计提升并发性能 90%+

#### 3. 统计模块 (stats.rs)

**职责**: 流量统计、速度计算、数据持久化

**快照机制**:
```rust
struct SpeedSnapshot {
    upload_bytes: u64,
    download_bytes: u64,
    timestamp: Instant,
    upload_speed: f64,
    download_speed: f64,
}
```

**速度计算**:
- 对比当前快照和上次快照
- 计算时间差和流量差
- 得出精确的传输速度
- 保留 60 秒历史数据

**批量统计优化**:
- 累积 64KB 流量才更新统计
- 减少锁竞争 90%+
- 高并发场景性能提升显著

**持久化策略**:
- 每 10 分钟自动保存到配置文件
- 包含总流量、用户流量、更新时间
- 服务重启时自动加载

#### 4. HTTP 处理模块 (http.rs)

**职责**: HTTP 请求处理、静态文件服务、API 端点

**安全头**:
```rust
X-Content-Type-Options: nosniff
X-Frame-Options: SAMEORIGIN
Content-Security-Policy: default-src 'self'; ...
Referrer-Policy: strict-origin-when-cross-origin
X-XSS-Protection: 1; mode=block
```

**API 端点**:
- `GET /api/stats`: 实时监控数据
- `GET /api/user-stats`: 用户流量统计
- `GET /api/speed-history`: 速度历史数据
- `GET /api/config`: 监控配置
- `GET /api/performance`: 性能配置

**静态资源嵌入**:
- 使用 `rust-embed` 编译时嵌入
- 单文件部署，无需 static 目录
- 支持所有前端资源

#### 5. WebSocket 模块 (ws.rs)

**职责**: WebSocket 实时数据推送和连接管理

**连接管理**:
- 最大连接数限制 (默认 300)
- 心跳超时检测 (默认 60 秒)
- 自动清理死连接
- Origin 验证防止 CSRF

**广播机制**:
- 每秒推送监控数据
- 连接建立时发送历史数据
- 失败连接自动移除

**消息格式**:
```json
{
  "type": "stats",
  "payload": {
    "upload_speed": "1.23 MB/s",
    "download_speed": "2.34 MB/s",
    ...
  }
}
```

#### 6. 配置模块 (config.rs)

**职责**: 配置文件解析和验证

**配置结构**:
- `server`: 服务器监听配置
- `users`: 用户 UUID 列表
- `monitoring`: 监控参数配置
- `performance`: 性能优化配置

**默认值策略**:
- 使用 serde 默认值
- 配置文件不存在时自动创建
- 支持部分配置缺失

### 前端架构

#### 组件结构

```
App.vue (根组件)
├── ThemeToggle.vue (主题切换)
├── TrafficChart.vue (流量波形图)
├── SpeedCard.vue (上传速度卡片)
├── DownloadCard.vue (下载速度卡片)
├── TrafficCard.vue (总流量卡片)
├── UptimeCard.vue (运行时长卡片)
├── MemoryCard.vue (内存使用卡片)
├── ConnectionsCard.vue (连接数卡片)
└── UserStats.vue (用户流量统计)
```

#### Composables

**useWebSocket**:
- 单例模式管理状态
- WebSocket 实时连接
- API 降级机制
- 历史数据持久化

**useTheme**:
- 主题切换逻辑
- localStorage 持久化
- CSS 变量管理

#### 数据流

```
WebSocket/API
    ↓
useWebSocket (状态管理)
    ↓
Composables (数据处理)
    ↓
Components (视图渲染)
```

## 数据流

### VLESS 代理流程

```
客户端连接
    ↓
协议检测 (HTTP vs VLESS)
    ↓
UUID 验证
    ↓
发送 VLESS 响应
    ↓
连接目标服务器
    ↓
双向数据转发
    ├─ 客户端 → 目标 (上传 + 批量统计)
    └─ 目标 → 客户端 (下载 + 批量统计)
    ↓
连接关闭，清理资源
```

### 监控数据流程

```
服务器事件 (连接/断开/流量)
    ↓
更新 Stats (内存状态)
    ↓
定时任务 (1秒)
    ↓
广播 WebSocket 消息
    ↓
前端接收并更新 UI
```

### 配置持久化流程

```
定时任务 (10分钟)
    ↓
读取当前 Stats 状态
    ↓
序列化为 JSON
    ↓
更新 config.json 的 monitor 字段
    ↓
写入文件系统
```

## 性能优化

### 后端优化

1. **批量统计**:
   - 累积 64KB 流量才更新统计
   - 减少锁竞争 90%+
   - 适用于高并发场景

2. **大缓冲区**:
   - 默认 128KB 传输缓冲区
   - 适配千兆网络
   - 单连接带宽提升 4 倍

3. **TCP 优化**:
   - TCP_NODELAY 启用
   - 降低延迟
   - 改善小包传输

4. **零拷贝传输**:
   - 使用 `Bytes` 库
   - 减少内存复制
   - 提升吞吐量

5. **静态链接**:
   - CRT 静态链接
   - 零依赖运行
   - 可执行文件约 974KB

### 前端优化

1. **单例状态**:
   - 避免重复连接
   - 减少内存占用
   - 提升性能

2. **会话存储**:
   - 历史数据持久化
   - 刷新页面保留数据
   - 改善用户体验

3. **Canvas 渲染**:
   - 高性能波形图
   - 60 FPS 流畅动画
   - 低 CPU 占用

4. **API 降级**:
   - WebSocket 失败自动降级
   - 保证功能可用
   - 用户体验优先

## 安全考虑

### 认证与授权
- UUID 作为唯一认证凭据
- HashSet O(1) 快速验证
- 无密码，简化部署

### 网络安全
- Origin 验证防止 CSRF
- 安全 HTTP 头
- 建议配合 TLS 使用

### 数据保护
- 日志不记录敏感信息
- 配置文件权限管理
- 避免密钥泄露

### 部署安全
- HTTP 监控页面无认证
- 建议配置防火墙
- 限制访问来源

## 扩展性

### 协议扩展
- 支持添加新命令类型
- 支持新地址类型
- 保持向后兼容

### 监控扩展
- 添加新的统计指标
- 新增 API 端点
- 扩展 WebSocket 消息

### 配置扩展
- 添加新的配置项
- 支持环境变量
- 动态配置重载

## 部署架构

### 单文件部署
```
vless.exe (约 974KB)
├── 嵌入前端资源
├── 静态链接 CRT
└── 零依赖运行
```

### 配置文件
```
config.json
├── server: 监听配置
├── users: 用户列表
├── monitor: 自动维护
├── monitoring: 监控参数
└── performance: 性能参数
```

### 运行要求
- 操作系统: Windows / Linux / macOS
- 依赖: 无 (静态链接)
- 权限: 绑定端口需要管理员/root

## 开发工作流

### 后端开发
```bash
# 编译
cargo build --release

# 运行
cargo run

# 测试
cargo test
```

### 前端开发
```bash
cd frontend

# 安装依赖
npm install

# 开发模式
npm run dev

# 构建
npm run build
```

### 完整构建
```bash
# 1. 构建前端
cd frontend && npm run build && cd ..

# 2. 构建后端 (嵌入前端)
cargo build --release

# 3. 运行
./target/release/vless.exe
```

## 参考资料

- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core 项目](https://github.com/XTLS/Xray-core)
- [Tokio 官方文档](https://tokio.rs/)
- [Vue 3 官方文档](https://vuejs.org/)
