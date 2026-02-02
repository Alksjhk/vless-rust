# 技术文档

## 项目概述

VLESS协议服务器是一个基于Rust和Tokio实现的高性能代理服务器，遵循xray-core的VLESS协议规范。项目支持完整的VLESS协议（版本0和版本1），并提供基于Vue 3的现代化HTTP监控页面。

## 架构设计

### 模块划分

```
src/
├── main.rs         # 程序入口，初始化配置和启动服务器
├── config.rs       # 配置文件解析和管理
├── protocol.rs     # VLESS协议编解码实现
├── server.rs       # 服务器核心逻辑，处理连接和代理
├── stats.rs        # 监控数据统计和持久化
├── http.rs         # HTTP请求检测和静态文件服务
└── ws.rs           # WebSocket连接管理和实时数据广播

frontend/
├── src/
│   ├── App.vue              # 主应用组件
│   ├── main.js              # 应用入口
│   ├── components/          # Vue组件
│   │   ├── StatCard.vue     # 基础统计卡片
│   │   ├── SpeedCard.vue    # 上传速度卡片
│   │   ├── DownloadCard.vue # 下载速度卡片
│   │   ├── TrafficCard.vue  # 总流量卡片
│   │   ├── UptimeCard.vue   # 运行时长卡片
│   │   ├── MemoryCard.vue   # 内存使用卡片
│   │   ├── ConnectionsCard.vue # 活动连接卡片
│   │   ├── TrafficChart.vue # 流量趋势图（Canvas波形）
│   │   └── ThemeToggle.vue  # 主题切换按钮
│   ├── composables/         # Vue组合式函数
│   │   ├── useWebSocket.js  # WebSocket连接和数据处理
│   │   └── useTheme.js      # 主题切换管理
│   └── assets/
│       └── styles/
│           └── main.css     # 全局样式和CSS变量
├── index.html              # HTML模板
├── vite.config.js          # Vite配置
└── package.json            # 前端依赖管理

static/                    # 构建后的静态文件目录
├── index.html
└── assets/
    ├── index-xxx.js
    └── index-xxx.css
```

### 数据流

```
客户端连接 → HTTP检测 → VLESS协议解析 → 用户认证 → 代理转发
                                    ↓
                              流量统计更新
                                    ↓
                              WebSocket广播推送

浏览器 → WebSocket连接 → 接收实时统计数据 → 前端渲染更新
```

## 核心功能实现

### VLESS协议实现

#### 请求格式
```
1 Byte    | 16 Bytes | 1 Byte  | M Bytes | 1 Byte | 2 Bytes | 1 Byte | S Bytes | X Bytes
版本      | UUID     | 附加长度 | 附加数据 | 命令   | 端口    | 地址类型| 地址    | 请求数据
```

#### 响应格式
```
1 Byte    | 1 Byte  | N Bytes | Y Bytes
版本      | 附加长度 | 附加数据 | 响应数据
```

#### 支持的命令类型
- `0x01`: TCP连接
- `0x02`: UDP连接（计划支持）
- `0x03`: Mux多路复用（计划支持）

### HTTP监控功能

#### HTTP请求检测机制
采用混合检测方案：
1. 前缀检测：检查数据开头是否为HTTP方法（GET、POST等）
2. HTTP解析：解析HTTP请求行和头部
3. WebSocket升级检测：检查 Upgrade: websocket 头部
4. 路由处理：嵌入式静态文件服务或 WebSocket 连接升级

#### 静态文件服务
- 使用 `rust-embed` 将静态文件打包进可执行文件
- 编译时嵌入 `static/` 目录的所有资源
- 支持的路由：
  - `GET /` 或 `GET /index.html`: 返回监控页面HTML
  - `GET /assets/**/*`: 返回前端构建资源（CSS、JS、字体等，支持所有子路径）
  - `GET /vite.svg`: 返回网站图标
- 自动识别文件类型并设置正确的Content-Type头
- 支持二进制文件（SVG、字体等）和文本文件（HTML、CSS、JS）

#### WebSocket实时推送
- 连接路径: `GET /api/ws` 或 `GET /ws`
- 消息格式: JSON RPC 风格，包含 type 和 payload 字段
- 推送频率: 每秒推送一次最新统计数据
- 消息类型:
  - `stats`: 当前监控数据（速度、流量、连接数等）
  - `history`: 历史流量数据（最多120秒趋势）
- 连接管理: 最大100个并发连接，60秒心跳超时
- 安全验证: 支持 Origin 头验证，通过 `VLESS_MONITOR_ORIGIN` 环境变量配置
- 自动降级: WebSocket 连接失败时自动降级到 API 轮询

#### 监控数据收集
使用全局状态收集（Arc<Mutex<Stats>>）：
- 实时传输速度：基于时间窗口的流量差值计算
- 运行时长：从服务器启动时间计算
- 总流量使用量：累计流量统计，每10分钟持久化
- 内存占用：使用sysinfo库查询系统内存
- 活动连接数：WebSocket连接建立时增加，关闭时减少
- **用户级统计**：按UUID分组统计各用户流量和连接数

#### 流量统计语义说明
- **upload（上传）**：客户端发送的数据，即服务器从客户端接收的数据
- **download（下载）**：客户端接收的数据，即服务器发送给客户端的数据
- 方向定义：始终从客户端角度出发

#### 用户流量统计
- **数据结构**：`UserStats` 包含 UUID、邮箱、上传/下载流量、活动连接数
- **统计方式**：在代理转发时按用户UUID分别统计
- **持久化**：配置文件 `monitor.users` 字段存储各用户流量数据
- **API接口**：`GET /api/user-stats` 获取所有用户流量统计

#### 前端架构
- **框架**: Vue 3 (Composition API)
- **构建工具**: Vite
- **实时通信**: WebSocket原生API
- **组件化**: 模块化组件设计
- **主题系统**: CSS变量实现深色/浅色主题
- **响应式**: 适配Mobile/Tablet/Desktop
- **组件列表**:
  - `StatCard.vue`: 基础统计卡片
  - `SpeedCard.vue`: 上传速度卡片
  - `DownloadCard.vue`: 下载速度卡片
  - `TrafficCard.vue`: 总流量卡片
  - `UptimeCard.vue`: 运行时长卡片
  - `MemoryCard.vue`: 内存使用卡片
  - `ConnectionsCard.vue`: 活动连接卡片
  - `TrafficChart.vue`: 流量趋势图（Canvas波形）
  - `ThemeToggle.vue`: 主题切换按钮
  - `UserStats.vue`: 用户流量统计表格（支持排序）

### 流量统计实现

#### 速度计算
使用快照机制算法：
1. 维护上一次计算的快照（包含流量和时间戳）
2. 每次计算时创建新快照，与上一次快照对比
3. 计算时间间隔内的流量差值
4. 差值除以时间间隔得到速度
5. 保留120秒的历史快照用于数据分析

#### 前端数据处理
- 维护60个数据点的历史数组
- 支持sessionStorage缓存，刷新页面后恢复数据
- 实现自动降级机制：WebSocket失败时切换到API轮询
- 使用单例模式管理全局WebSocket连接

#### 持久化策略
- 总流量每10分钟自动保存到config.json
- 重启后从配置文件加载历史流量数据
- 使用monitor字段存储统计信息

## 性能优化

### 异步I/O
- 基于Tokio异步运行时，支持高并发连接
- 每个客户端连接在独立的异步任务中处理
- 使用tokio::select!实现双向数据转发

### 内存管理
- 使用Bytes库进行高效的内存管理
- 避免不必要的数据拷贝
- 定期清理过期的速度计算数据

### 连接复用
- 每个客户端连接复用TCP连接
- 支持长连接和短连接

## 配置管理

### 配置文件结构
```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443
  },
  "users": [
    {
      "uuid": "xxx-xxx-xxx",
      "email": "user@example.com"
    }
  ],
  "monitoring": {
    "speed_history_duration": 60,
    "broadcast_interval": 1,
    "websocket_max_connections": 100,
    "websocket_heartbeat_timeout": 60,
    "vless_max_connections": 100
  },
  "monitor": {
    "total_upload_bytes": 0,        // 客户端上传总流量（服务器接收）
    "total_download_bytes": 0,      // 客户端下载总流量（服务器发送）
    "last_update": "2024-01-01T00:00:00Z",
    "users": {                       // 用户级流量统计
      "uuid-string": {
        "total_upload_bytes": 0,
        "total_download_bytes": 0,
        "email": "user@example.com"
      }
    }
  }
}
```

**配置节说明**：
- `server`: 服务器监听配置
- `users`: 用户认证配置
- `monitoring`: 监控功能配置（所有字段可选，有默认值）
- `monitor`: 运行时统计数据（由后端自动维护，手动修改会被覆盖）

### 配置加载
- 启动时从config.json加载配置
- 配置文件不存在时创建默认配置
- 支持命令行参数指定配置文件路径

## 安全考虑

### 用户认证
- UUID作为唯一认证凭据
- 使用HashMap进行O(1)时间复杂度的用户验证
- 认证失败立即关闭连接

### 数据保护
- 日志中不记录敏感信息
- 配置文件中的UUID需要保密
- 建议在生产环境配合TLS使用

## 错误处理

### 连接错误
- 客户端主动关闭连接
- 网络超时
- 目标服务器不可达

### 协议错误
- 无效的VLESS请求格式
- 不支持的命令类型
- 无效的UUID

### HTTP错误
- HTTP解析失败不影响VLESS连接
- API请求失败返回错误响应

## 扩展性

### 待实现功能
- UDP协议支持
- Mux多路复用支持
- XTLS集成
- WebSocket传输层
- gRPC传输层
- 动态配置重载

### 扩展点
- 新增命令类型：在protocol.rs中添加命令枚举
- 新增传输层：在server.rs中添加传输层处理
- 新增监控指标：在stats.rs中添加统计字段
- 新增WebSocket消息类型：在ws.rs中添加WsMessage枚举值
- 新增前端组件：在frontend/src/components/中添加Vue组件
- 新增Composable：在frontend/src/composables/中添加组合式函数

## 依赖说明

### 核心依赖
- `tokio`: 异步运行时
- `bytes`: 高效字节操作
- `uuid`: UUID生成和解析
- `serde`: 序列化和反序列化
- `anyhow`: 错误处理
- `tracing`: 结构化日志
- `rust-embed`: 静态文件嵌入可执行文件

### 监控依赖
- `sysinfo`: 系统信息查询
- `chrono`: 时间处理

### 前端依赖
- `vue`: Vue 3框架
- `@vitejs/plugin-vue`: Vite的Vue插件
- `rolldown-vite`: Vite构建工具

## 编译优化

### Release配置
- 链接时优化（LTO）
- 单代码生成单元
- 优化级别：size
- 移除调试信息
- panic=abort减小体积
- 静态资源嵌入，单文件部署（约934KB）

### 跨平台编译
- **Windows**: CRT 静态链接，生成零依赖单一 exe 文件
- **Linux**: CRT 静态链接，生成静态二进制文件
- **本地编译**: 通过 .cargo/config.toml 配置与 CI 保持一致

### 配置方式
```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

## 参考资料

- [VLESS协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core项目](https://github.com/XTLS/Xray-core)
- [Tokio官方文档](https://tokio.rs/)
- [Rust异步编程](https://rust-lang.github.io/async-book/)
