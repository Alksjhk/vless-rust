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
- `src/server.rs`: 服务器核心逻辑，处理连接、用户认证和 TCP 代理转发
- `src/stats.rs`: 流量统计模块，使用快照机制计算速度，支持持久化到配置文件
- `src/http.rs`: HTTP 请求检测、静态文件服务和监控 API 端点

**前端架构：**

- Vue 3 Composition API
- Vite 构建工具（使用 rolldown-vite 优化）
- 组件化设计，每个统计指标独立组件
- Composables 模式（useStats、useTheme）
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
    "total_bytes_sent": 0,
    "total_bytes_received": 0,
    "last_update": "2024-01-01T00:00:00Z"
  }
}
```

配置文件在服务器启动时加载，不存在时自动创建默认配置。monitor 字段由后端自动维护，手动修改可能被覆盖。

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

- `GET /api/stats`：获取监控数据（包含用户统计数组）
- `GET /api/user-stats`：获取所有用户流量统计
- `GET /api/speed-history`：获取速度历史数据
- `GET /api/config`：获取监控配置
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

**可执行文件大小**：约 814KB（包含所有前端资源）

## 安全注意事项

- UUID 是唯一的认证凭据，确保配置文件权限正确
- 日志中不记录敏感信息
- 建议生产环境配合 TLS 使用
- HTTP 监控页面无认证，应配置防火墙限制访问

## 参考资料

- [VLESS 协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core 项目](https://github.com/XTLS/Xray-core)
- [Tokio 官方文档](https://tokio.rs/)
