# VLESS Protocol Server in Rust

基于Rust和Tokio实现的高性能VLESS协议服务器，遵循xray-core的VLESS协议规范。

## 特性

- 完整的VLESS协议支持（版本0和版本1）
- 异步I/O处理（基于Tokio）
- TCP代理转发
- 多用户UUID认证
- 配置文件支持
- 结构化日志记录
- 版本兼容性（支持测试版和正式版）
- **WebSocket实时数据推送**，每秒更新监控数据
- **现代化HTTP监控页面**，包含实时波形图和统计卡片
- **深色/浅色主题切换**，支持localStorage持久化
- **响应式设计**，适配桌面、平板、移动设备
- **自动降级机制**，WebSocket失败时切换到API轮询
- 流量统计持久化（每10分钟自动保存）
- 单文件部署（静态资源嵌入可执行文件）
- 计划中：UDP支持、Mux多路复用、XTLS支持

## VLESS协议实现

本实现严格遵循xray-core的VLESS协议规范，支持两个协议版本：

- **版本0**: 测试版本（BETA）
- **版本1**: 正式版本（RELEASE）

服务器会自动检测客户端使用的版本并返回相应的响应版本。

### 请求格式
```
1 Byte    | 16 Bytes | 1 Byte  | M Bytes | 1 Byte | 2 Bytes | 1 Byte | S Bytes | X Bytes
版本      | UUID     | 附加长度 | 附加数据 | 命令   | 端口    | 地址类型| 地址    | 请求数据
```

### 响应格式
```
1 Byte    | 1 Byte  | N Bytes | Y Bytes
版本      | 附加长度 | 附加数据 | 响应数据
```

### 支持的命令类型
- `0x01`: TCP连接
- `0x02`: UDP连接（计划支持）
- `0x03`: Mux多路复用（计划支持）

### 支持的地址类型
- `0x01`: IPv4地址（4字节）
- `0x02`: 域名（1字节长度 + 域名）
- `0x03`: IPv6地址（16字节）

## 快速开始

### 1. 安装前端依赖

```bash
cd frontend
npm install
```

### 2. 构建前端

```bash
cd frontend
npm run build
```

前端构建完成后，静态文件将输出到 `static/` 目录。

### 3. 编译Rust后端

```bash
cargo build --release
```

**编译特性**：
- 使用 CRT 静态链接，生成零依赖的可执行文件
- 编译后的程序已嵌入所有静态资源，单文件即可运行
- 本地编译配置与 CI 环境一致（通过 `.cargo/config.toml`）

**注意**：编译时需要 `static/` 目录存在（包含前端构建产物）。编译后的可执行文件已嵌入所有静态资源，单文件即可运行。

### 4. 配置服务器

编辑 `config.json` 文件：

```json
{
  "server": {
    "listen": "0.0.0.0",
    "port": 8443
  },
  "users": [
    {
      "uuid": "12345678-1234-1234-1234-123456789abc",
      "email": "user1@example.com"
    }
  ]
}
```

### 5. 运行服务器

```bash
# 使用默认配置文件 config.json
cargo run

# 或指定配置文件路径
cargo run -- /path/to/your/config.json

# 运行发布版本
./target/release/vless.exe
```

### 4. 客户端配置

在支持VLESS协议的客户端中配置：

- **协议**: VLESS
- **地址**: 你的服务器IP
- **端口**: 8443（或配置文件中设置的端口）
- **UUID**: 配置文件中的用户UUID
- **加密**: none
- **传输协议**: TCP

### 6. 访问监控页面

服务器支持HTTP监控页面，直接在浏览器中访问服务器地址即可：

```
http://your-server-ip:8443/
```

监控页面提供以下功能：

- **实时传输速度**: 上传/下载速度实时显示
- **运行时长**: 服务器运行时间统计
- **总流量使用量**: 累计流量统计（每10分钟自动保存到配置文件）
- **内存占用**: 当前内存使用情况
- **活动连接数**: 当前活跃连接数量
- **实时波形图**: Canvas绘制的流量趋势图，支持鼠标悬停查看详细数据
- **深色/浅色主题**: 支持主题切换并持久化

技术特性：
- WebSocket实时推送，每秒更新数据
- 响应式设计，适配各种设备
- 自动降级机制，WebSocket失败时切换到API轮询
- 使用Vue 3 Composition API和Vite构建

### 前端开发模式

如需开发前端监控页面，可以使用Vite开发服务器：

```bash
cd frontend
npm run dev
```

开发服务器会自动代理 `/api` 请求和 WebSocket 连接到后端服务器（默认端口8443）。

## 配置说明

### 服务器配置

- `listen`: 监听地址，通常设为 `0.0.0.0` 监听所有接口
- `port`: 监听端口

### 用户配置

- `uuid`: 用户唯一标识符，必须是有效的UUID格式
- `email`: 用户邮箱（可选，仅用于标识）

## 性能特点

- **异步处理**: 基于Tokio的异步运行时，支持高并发连接
- **零拷贝**: 使用Bytes库进行高效的内存管理
- **快速UUID验证**: 使用HashSet进行O(1)时间复杂度的用户验证
- **连接复用**: 每个客户端连接在独立的异步任务中处理
- **单文件部署**: 静态资源嵌入可执行文件，约934KB，无需额外依赖
- **静态链接**: Windows CRT 静态链接，零依赖运行，无需 VC++ 运行时库
- **WebSocket广播**: 支持最大100个并发WebSocket连接，每秒推送更新
- **智能降级**: WebSocket连接失败时自动切换到API轮询模式

## 部署说明

编译后的 `vless.exe` 已包含所有前端静态资源，可以直接复制到目标服务器运行，无需 `static/` 目录。

**部署步骤**：
1. 将 `target/release/vless.exe` 复制到服务器
2. 创建配置文件 `config.json`（或使用自动生成的默认配置）
3. 运行 `./vless.exe` 或 `./vless.exe /path/to/config.json`

**静态链接特性**：
- Windows 版本使用 CRT 静态链接，无需安装 Visual C++ Redistributable
- 可执行文件只依赖系统内核库（KERNEL32.dll），实现零依赖运行
- 本地编译和 CI 编译结果一致，确保可移植性

## 安全注意事项

1. **UUID保密**: 确保用户UUID不被泄露，这是唯一的认证凭据
2. **传输加密**: 建议在生产环境中配合TLS使用
3. **访问控制**: 合理配置防火墙规则
4. **日志管理**: 注意日志中可能包含敏感信息
5. **WebSocket安全**: 生产环境建议设置 `VLESS_MONITOR_ORIGIN` 环境变量限制访问来源
   ```bash
   export VLESS_MONITOR_ORIGIN="https://your-domain.com"
   ```

## 开发计划

- [ ] UDP协议支持
- [ ] Mux多路复用支持
- [ ] XTLS集成
- [ ] WebSocket传输层
- [ ] gRPC传输层
- [ ] 动态配置重载

## 协议参考

- [VLESS协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core项目](https://github.com/XTLS/Xray-core)

## 许可证

MIT License