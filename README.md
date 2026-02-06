# VLESS Protocol Server in Rust

基于Rust和Tokio实现的高性能VLESS协议服务器，遵循xray-core的VLESS协议规范。

## 特性

- 完整的VLESS协议支持（版本0和版本1）
- 异步I/O处理（基于Tokio）
- TCP代理转发
- 多用户UUID认证
- 配置文件支持
- **交互式配置向导**，首次运行时引导用户完成配置
- 结构化日志记录
- 版本兼容性（支持测试版和正式版）
- **WebSocket实时数据推送**，每秒更新监控数据
- **现代化HTTP监控页面**，包含实时波形图和统计卡片
- **深色/浅色主题切换**，支持localStorage持久化
- **响应式设计**，适配桌面、平板、移动设备
- **自动降级机制**，WebSocket失败时切换到API轮询
- 流量统计持久化（每10分钟自动保存）
- 单文件部署（静态资源嵌入可执行文件）
- UDP over TCP 支持
- **外网IP自动检测**，启动时自动生成VLESS连接链接

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
- `0x02`: UDP连接（已支持，通过 UDP over TCP 机制）

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

**方式一：使用构建脚本（推荐）**

```bash
make                     # 编译 release 版本并输出到项目根目录
make debug               # 编译 debug 版本
make clean               # 清理编译产物
```

**方式二：使用 Cargo**

```bash
cargo build --release
```

**编译特性**：
- 使用 CRT 静态链接，生成零依赖的可执行文件
- 编译后的程序已嵌入所有静态资源，单文件即可运行
- 本地编译配置与 CI 环境一致（通过 `.cargo/config.toml`）
- 使用构建脚本时，可执行文件会自动复制到项目根目录（`vless-rust.exe`）
- 体积约 3MB（包含完整前端资源）

**注意**：编译时需要 `static/` 目录存在（包含前端构建产物）。编译后的可执行文件已嵌入所有静态资源，单文件即可运行。

### 4. 首次运行（配置向导）

如果您是第一次运行服务器，或者 `config.json` 不存在，程序会自动启动**交互式配置向导**：

```bash
cargo run
```

向导会引导您完成以下配置：
- **服务器监听地址**（默认：0.0.0.0）
- **服务器监听端口**（默认：443）
- **用户配置**
  - 用户 UUID（自动生成或手动输入）
  - 用户邮箱（可选，用于标识）

配置完成后，向导会自动生成 `config.json` 文件并启动服务器。

### 5. 配置服务器

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

### 6. 运行服务器

```bash
# 使用构建脚本编译后，直接运行根目录的可执行文件
./vless-rust.exe           # Windows
./vless-rust               # Linux/macOS

# 或使用 Cargo 运行开发版本
cargo run

# 指定配置文件路径
./vless-rust.exe /path/to/config.json
```

**启动输出示例**：

服务器启动时会自动检测外网 IP 并为每个用户生成 VLESS 连接链接：

```
[INFO] Detecting public IP...
[INFO] Public IP: 123.182.3.236
[INFO]
[INFO] ========== VLESS Connection Links ==========
[INFO] vless://......
[INFO] ==========================================
```

直接复制链接到 VLESS 客户端即可导入使用。

### 7. 客户端配置

在支持VLESS协议的客户端中配置：

- **协议**: VLESS
- **地址**: 你的服务器IP
- **端口**: 8443（或配置文件中设置的端口）
- **UUID**: 配置文件中的用户UUID
- **加密**: none
- **传输协议**: TCP

### 8. 访问监控页面

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

### 快速配置

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

**完整配置说明**：参考 [配置指南](config-guide.md) 或查看 [示例配置文件](config.json)

### 配置项说明

所有配置项都有合理的默认值，可以只配置必需项。

#### 服务器配置
- `listen`: 监听地址，通常设为 `0.0.0.0` 监听所有接口
- `port`: 监听端口（建议使用 443 或 8443）
- `public_ip`: 公网 IP 地址（可选），配置后将不再自动检测，适用于无法访问外网 API 的环境

#### 用户配置
- `uuid`: 用户唯一标识符，必须是有效的UUID格式
- `email`: 用户邮箱（可选，用于生成 VLESS 链接别名）

#### 监控配置（可选）
- `speed_history_duration`: 速度历史保留时长（秒），默认60
- `broadcast_interval`: WebSocket广播间隔（秒），默认1
- `websocket_max_connections`: WebSocket最大连接数，默认300
- `websocket_heartbeat_timeout`: WebSocket心跳超时（秒），默认60
- `vless_max_connections`: VLESS最大连接数，默认300

#### 性能配置（可选）
- `buffer_size`: 传输缓冲区大小（字节），默认131072（128KB）
- `tcp_nodelay`: 是否启用TCP_NODELAY，默认true
- `tcp_recv_buffer`: TCP接收缓冲区大小（字节），默认262144（256KB）
- `tcp_send_buffer`: TCP发送缓冲区大小（字节），默认262144（256KB）
- `stats_batch_size`: 流量统计批量大小（字节），默认65536（64KB）
- `udp_timeout`: UDP会话超时时间（秒），默认30
- `udp_recv_buffer`: UDP接收缓冲区大小（字节），默认65536（64KB）

**详细说明**：查看 [完整配置指南](config-guide.md)

## 性能特点

- **异步处理**: 基于Tokio的异步运行时，支持高并发连接
- **零拷贝**: 使用Bytes库进行高效的内存管理
- **快速UUID验证**: 使用HashSet进行O(1)时间复杂度的用户验证
- **连接复用**: 每个客户端连接在独立的异步任务中处理
- **可配置缓冲区**: 默认128KB传输缓冲区，支持高带宽场景（千兆网络）
- **批量统计**: 减少锁竞争，提升多连接并发性能
- **TCP优化**: 默认启用TCP_NODELAY降低延迟
- **UDP支持**: UDP over TCP 机制，支持30秒超时自动清理空闲会话
- **单文件部署**: 静态资源嵌入可执行文件，约3MB，无需额外依赖
- **静态链接**: Windows CRT 静态链接，零依赖运行，无需 VC++ 运行时库
- **WebSocket广播**: 支持最大300个并发WebSocket连接，每秒推送更新
- **智能降级**: WebSocket连接失败时自动切换到API轮询模式
- **IP自动检测**: 启动时并发请求3个IPv4专用API获取公网地址
  - 使用 ipv4.icanhazip.com、checkip.amazonaws.com、v4.ident.me
  - 只接受 IPv4 地址（自动过滤 IPv6）
  - 使用 native-tls 确保网络兼容性
  - 支持配置文件手动指定 IP，跳过自动检测
  - 检测失败时提供详细诊断提示

## 部署说明

编译后的可执行文件已包含所有前端静态资源，可以直接复制到目标服务器运行。

**部署步骤**：
1. 使用构建脚本编译：`.\build.ps1` 或 `make`
2. 将项目根目录的 `vless-rust.exe` 复制到服务器
3. 创建配置文件 `config.json`（或使用自动生成的默认配置）
4. 运行 `./vless-rust.exe` 或 `./vless-rust.exe /path/to/config.json`

**静态链接特性**：
- Windows 版本使用 CRT 静态链接，无需安装 Visual C++ Redistributable
- 可执行文件只依赖系统内核库（KERNEL32.dll），实现零依赖运行
- 本地编译和 CI 编译结果一致，确保可移植性

## 安全注意事项

1. **UUID保密**: 确保用户UUID不被泄露，这是唯一的认证凭据
2. **传输加密**: 建议在生产环境中配合TLS使用
3. **访问控制**: 合理配置防火墙规则
4. **日志管理**: 注意日志中可能包含敏感信息
5. **WebSocket安全**: 生产环境建议配置防火墙限制监控页面访问

## 协议参考

- [VLESS协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core项目](https://github.com/XTLS/Xray-core)

## 许可证

MIT License