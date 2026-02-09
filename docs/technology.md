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
- **内存信息**: 自研跨平台模块 (Linux/Windows)
- **时间处理**: 自研 RFC3339 格式化模块
- **Base64 编码**: 自研实现 (RFC 4648)
- **HTTP 客户端**: reqwest (rustls-tls)

### 前端
- **框架**: React 18 (Hooks)
- **构建工具**: Vite (rolldown-vite 优化)
- **状态管理**: Zustand
- **图表库**: Victory
- **样式**: Tailwind CSS (玻璃态 UI)
- **实时通信**: WebSocket + API 降级

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
| `src/utils.rs` | 工具函数、IP检测、URL生成 | `get_public_ip()`、`get_public_ip_with_diagnostic()`、`generate_vless_url()`、`fetch_ip_from_api()`、`validate_ipv4()` |
| `src/wizard.rs` | 交互式配置向导 | `ConfigWizard`、`run()`、`prompt_listen_address()`、`prompt_port()`、`prompt_users()` |

### 前端核心文件

| 文件路径 | 核心功能 | 组件/函数 |
|---------|---------|----------|
| `frontend/src/App.jsx` | 主应用容器、布局 | `<App>` - 仪表板布局 |
| `frontend/src/main.jsx` | 应用入口、React 挂载 | `ReactDOM.createRoot()` - 初始化 React 应用 |
| `frontend/src/components/metrics/ConnectionsMetric.jsx` | 活跃连接显示 | `<ConnectionsMetric>` - 连接数统计（React.memo优化） |
| `frontend/src/components/metrics/SpeedMetric.jsx` | 实时速度显示 | `<SpeedMetric>` - 上传/下载速度（React.memo优化） |
| `frontend/src/components/metrics/TrafficMetric.jsx` | 总流量显示 | `<TrafficMetric>` - 总流量统计（React.memo优化） |
| `frontend/src/components/metrics/MemoryMetric.jsx` | 内存使用显示 | `<MemoryMetric>` - 内存使用量（React.memo优化） |
| `frontend/src/components/metrics/UptimeMetric.jsx` | 运行时间显示 | `<UptimeMetric>` - 服务器运行时长（React.memo优化） |
| `frontend/src/components/charts/SpeedChart.jsx` | 流量趋势图表 | `<SpeedChart>` - Victory 实现，动态Y轴，ResizeObserver响应式宽度 |
| `frontend/src/components/charts/TrafficChartSection.jsx` | 图表容器组件 | `<TrafficChartSection>` - 连接状态、历史时长显示 |
| `frontend/src/components/ResourceCard.jsx` | 资源使用显示 | `<ResourceCard>` - 内存和连接数（React.memo优化） |
| `frontend/src/components/SystemInfo.jsx` | 系统信息面板 | `<SystemInfo>` - 服务器状态总览（React.memo优化） |
| `frontend/src/utils/debounce.js` | 防抖和节流工具 | `debounce()`, `throttle()` - 性能优化工具函数 |
| `frontend/src/components/UserTable.jsx` | 用户流量统计 | `<UserTable>` - 用户级别流量表格 |

### 配置文件

| 文件路径 | 核心功能 | 说明 |
|---------|---------|------|
| `Cargo.toml` | Rust 项目配置 | 依赖项、编译优化、二进制配置 |
| `.cargo/config.toml` | Cargo 编译配置 | 静态链接选项、跨平台编译配置（Windows/Linux/macOS） |
| `.github/workflows/build.yml` | CI/CD 工作流 | 跨平台自动构建、Release 发布、静态链接验证 |
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
- 保留 120 秒历史数据
- 支持总体速度和用户级别速度计算

**批量统计优化**:
- 累积 64KB 流量才更新统计
- 减少锁竞争 90%+
- 高并发场景性能提升显著

**流量统计策略**:
- **仅统计VLESS代理流量**：只有通过VLESS协议的TCP/UDP代理流量会被统计
- **排除HTTP请求**：前端监控页面的HTTP请求（API、静态资源、WebSocket）不计入流量统计
- HTTP请求在 `handle_connection` 的早期阶段被识别并分流，不进入流量统计逻辑

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
  - `listen`: 监听地址
  - `port`: 监听端口
  - `public_ip`: 公网 IP（可选）
- `users`: 用户 UUID 列表
- `monitoring`: 监控参数配置
  - `speed_history_duration`: 速度历史时长（默认 120 秒，2 分钟）
  - `broadcast_interval`: 广播间隔（默认 1 秒）
  - `websocket_max_connections`: WebSocket 最大连接数（默认 300）
  - `websocket_heartbeat_timeout`: WebSocket 心跳超时（默认 60 秒）
  - `vless_max_connections`: VLESS 最大连接数（默认 300）
- `performance`: 性能优化配置
  - `buffer_size`: 传输缓冲区大小（默认 128KB）
  - `tcp_nodelay`: TCP_NODELAY 启用（默认 true）
  - `tcp_recv_buffer`: TCP 接收缓冲区（默认 256KB）
  - `tcp_send_buffer`: TCP 发送缓冲区（默认 256KB）
  - `stats_batch_size`: 流量统计批量大小（默认 64KB）
  - `udp_timeout`: UDP 会话超时（默认 30 秒）
  - `udp_recv_buffer`: UDP 接收缓冲区（默认 64KB）

**默认值策略**:
- 使用 serde 默认值
- 配置文件不存在时自动创建
- 支持部分配置缺失

#### 7. 内存信息模块 (memory.rs)

**职责**: 跨平台内存信息获取，替代 sysinfo 库

**支持平台**:
- Linux: 读取 `/proc/self/status` 和 `/proc/meminfo`
- Windows: 使用 `GetProcessMemoryInfo` 和 `GlobalMemoryStatusEx` API

**核心函数**:
- `get_process_memory()`: 获取当前进程内存使用（字节）
- `get_total_memory()`: 获取系统总内存（字节）

**优化特点**:
- 使用迭代器避免中间 Vec 分配
- 详细的错误日志（`tracing::error!` 和 `tracing::warn!`）
- 解析失败时返回 0 并记录警告

#### 8. 时间工具模块 (time.rs)

**职责**: RFC3339 时间格式化，替代 chrono 库

**核心结构**:
- `UtcTime`: UTC 时间结构体
  - `timestamp`: Unix 时间戳（秒）

**核心函数**:
- `UtcTime::now()`: 获取当前 UTC 时间
- `to_rfc3339()`: 格式化为 RFC3339 字符串
- `signed_duration_since()`: 计算时间差
- `utc_now_rfc3339()`: 快捷函数

**优化特点**:
- 手动实现日期算法（无需外部依赖）
- 负时间戳边界处理
- 支持负数秒数调整

#### 9. Base64 编码模块 (base64.rs)

**职责**: Base64 编码实现（RFC 4648），用于 WebSocket 握手

**核心函数**:
- `encode(input: &[u8]) -> String`: 标准 Base64 编码

**优化特点**:
- 仅实现编码功能（节省代码体积）
- 无 unsafe 代码（使用 `String::from_utf8()`）
- 高性能位运算实现

**安全性**:
- 移除 `unsafe` 块，使用安全 API
- 符合 Rust 最小权限原则

#### 10. 配置向导模块 (wizard.rs)

**职责**: 交互式配置向导，引导用户创建配置文件

**核心功能**:
- `run()`: 启动向导流程
- `prompt_listen_address()`: 询问监听地址
- `prompt_port()`: 询问端口
- `prompt_users()`: 询问用户配置

**邮箱验证**:
- 检查 `@` 和 `.` 位置关系
- 拒绝无效格式（`@example.com`, `user@`, `user@.`）
- 友好的警告提示

### 前端架构

#### 组件结构

```
App.jsx (根组件)
├── layouts/
│   └── DashboardLayout.jsx (仪表板布局)
├── components/
│   ├── ThemeToggle.jsx (主题切换)
│   ├── TrafficChart.jsx (流量波形图 - Recharts)
│   ├── SpeedCard.jsx (上传速度卡片)
│   ├── TrafficCard.jsx (总流量卡片)
│   ├── UptimeCard.jsx (运行时长卡片)
│   ├── ResourceCard.jsx (资源使用卡片)
│   ├── SystemInfo.jsx (系统信息面板)
│   └── UserTable.jsx (用户流量统计)
├── store/
│   └── monitorStore.js (Zustand 全局状态)
└── api/
    ├── websocket.js (WebSocket 连接管理)
    └── rest.js (REST API 封装)
```

#### 状态管理

**Zustand Store**:
- `useMonitorStore()` - 全局监控状态
- WebSocket 实时连接状态
- API 降级机制
- 历史数据持久化

#### 数据流

```
WebSocket/API
    ↓
monitorStore (Zustand 状态管理)
    ↓
React Components (视图渲染)
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

1. **流量图表优化**:
   - Y轴默认上界设置为 200KB/s (0.2 MB/s)
   - 支持数据超出时动态扩展Y轴范围
   - 小于1MB/s时自动显示为KB/s单位
   - 2分钟历史数据（120秒），每秒一个数据点

2. **Zustand状态管理**:
   - 轻量级全局状态管理
   - 避免重复连接
   - 减少内存占用

3. **API降级机制**:
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

# 检查代码
cargo check
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

### 跨平台编译

项目支持多平台交叉编译,所有配置已在 `.cargo/config.toml` 中预设。

#### 支持的目标平台

| 目标平台 | 架构 | 说明 |
|---------|------|------|
| `x86_64-pc-windows-msvc` | AMD64 | Windows 64位 |
| `aarch64-pc-windows-msvc` | ARM64 | Windows ARM64 |
| `x86_64-unknown-linux-musl` | AMD64 | Linux 64位 (完全静态链接，推荐) |
| `x86_64-unknown-linux-gnu` | AMD64 | Linux 64位 (GNU) |
| `aarch64-unknown-linux-gnu` | ARM64 | Linux ARM64 (GNU) |
| `armv7-unknown-linux-gnueabihf` | ARMv7 | Linux ARM 32位 |
| `x86_64-apple-darwin` | AMD64 | macOS Intel |
| `aarch64-apple-darwin` | ARM64 | macOS Apple Silicon |

#### Windows 交叉编译到 Linux

```bash
# 1. 添加 Linux 目标
rustup target add x86_64-unknown-linux-gnu

# 2. 构建前端
cd frontend && npm run build && cd ..

# 3. 交叉编译
cargo build --release --target x86_64-unknown-linux-gnu

# 输出: target/x86_64-unknown-linux-gnu/release/vless
```

#### 交叉编译到 ARM64

```bash
# 1. 添加 ARM64 目标
rustup target add aarch64-unknown-linux-gnu

# 2. 安装交叉编译工具链
# Ubuntu/Debian:
sudo apt-get install gcc-aarch64-linux-gnu

# 3. 构建前端
cd frontend && npm run build && cd ..

# 4. 交叉编译
cargo build --release --target aarch64-unknown-linux-gnu

# 输出: target/aarch64-unknown-linux-gnu/release/vless
```

#### 静态链接说明

项目使用静态链接,生成的二进制文件无需额外依赖:

- **Windows**: 使用 `target-feature=+crt-static`
- **Linux**: 使用 `link-self-contained=yes`
- **TLS**: 使用 `rustls-tls` (纯 Rust 实现,无 OpenSSL 依赖)

配置文件位置: `.cargo/config.toml`

#### GitHub Actions 自动构建

项目配置了完整的 GitHub Actions CI/CD 工作流,实现跨平台静态链接构建:

**工作流特性**:
- **触发条件**:
  - 推送提交到 `main` 分支 → 自动识别版本号并构建
  - PR 到 `main` 分支 → 仅构建测试（不发布）
  - 手动触发 (`workflow_dispatch`) → 方便测试

- **版本识别机制**:
  - 自动从提交信息中提取版本号（格式：`x.x.x`，如 `1.0.0`）
  - 提交信息必须以版本号开头
  - 如果有多个版本提交，只构建最新的未发布版本
  - 自动检查 Release 是否已存在，避免重复构建

- **支持平台** (8个):
  - Windows AMD64 + ARM64
  - Linux AMD64 (musl/gnu) + ARM64 + ARMv7
  - macOS Intel + Apple Silicon

- **构建流程**:
  1. 分析提交历史，提取版本号
  2. 检查 Release 是否已存在（去重）
  3. 安装 Rust 工具链和交叉编译工具
  4. 构建前端资源 (Node.js 20)
  5. 编译 Rust 二进制文件
  6. 验证静态链接 (Linux 平台)
  7. 上传构建产物
  8. 创建 GitHub Release（版本号：vx.x.x）
  9. 自动推送标签到仓库

- **静态链接验证**:
  ```bash
  # Linux 平台自动验证
  ldd vless-rust-xxx | grep "not a dynamic" && echo "✓ 静态链接"
  ```

- **输出文件命名**:
  - Windows: `vless-rust-windows-amd64.exe`
  - Linux musl: `vless-rust-linux-amd64-musl`
  - macOS ARM64: `vless-rust-macos-arm64`
  - 格式: `vless-rust-{平台}-{架构}{扩展名}`

**配置文件**: `.github/workflows/build.yml`

**使用示例**:

```bash
# 方式一：提交版本号（推荐）
git commit -m "1.0.0"
git push origin main

# 方式二：创建空提交触发特定版本
git commit --allow-empty -m "1.5.12"
git push origin main

# 工作流会自动：
# 1. 识别版本号 1.0.0 或 1.5.12
# 2. 构建 8 个平台
# 3. 创建 Release v1.0.0 或 v1.5.12
# 4. 推送标签 v1.0.0 或 v1.5.12
```

**版本识别规则**:
- ✅ `1.0.0` - 正确
- ✅ `1.5.12` - 正确
- ❌ `Release 1.0.0` - 错误（不在行首）
- ❌ `v1.0.0` - 错误（有 v 前缀）

## 参考资料

- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core 项目](https://github.com/XTLS/Xray-core)
- [Tokio 官方文档](https://tokio.rs/)
- [Vue 3 官方文档](https://vuejs.org/)
