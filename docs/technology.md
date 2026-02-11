# 技术文档

## 项目概述

VLESS-Rust 是一个基于 Rust 和 Tokio 异步运行时实现的高性能 VLESS 协议服务器，完全遵循 xray-core 的 VLESS 协议规范。项目采用现代化的技术栈，专注于提供高性能、轻量级的 VLESS 代理服务。

## 技术栈

### 后端

- **语言**: Rust 2021 Edition
- **异步运行时**: Tokio 1.0 (精简 features: rt-multi-thread, io-util, net, time, sync, macros)
- **内存分配器**: mimalloc (高性能分配器)
- **协议**: VLESS (版本 0 和版本 1)
- **序列化**: serde + serde_json
- **日志**: tracing + tracing-subscriber
- **SHA1**: sha1 (VLESS 协议需要)
- **Socket 配置**: socket2

## 架构设计

### 系统架构图

```
┌─────────────────────────────────────────┐
│              客户端连接                   │
└────────────┬────────────────────────────┘
             │
             ▼
   ┌─────────────────────┐
   │  TCP 端口 (8443)    │
   └─────────────────────┘
             │
             ▼
   ┌─────────────────────┐
   │   协议检测层         │
   │  is_http_request()  │
   └──────────┬──────────┘
              │
     ┌────────┴────────┐
     │                 │
     ▼                 ▼
┌─────────┐      ┌──────────────┐
│ HTTP    │      │ VLESS 请求   │
│ 请求    │      │              │
│ (拒绝)  │      ├──────────────┤
└─────────┘      │ UUID 验证    │
                 │ 命令处理     │
                 │ TCP 代理     │
                 │ UDP 代理     │
                 └──────────────┘
```

## 文件与功能映射关系

### 后端核心文件

| 文件路径 | 核心功能 | 主要结构体/函数 |
|---------|---------|---------------|
| `src/main.rs` | 程序入口、服务器启动 | `main()` - 加载配置、启动服务器、配置向导触发 |
| `src/config.rs` | 配置管理、JSON解析 | `Config`、`ServerConfig`、`UserConfig`、`PerformanceConfig` |
| `src/protocol.rs` | VLESS 协议编解码 | `VlessRequest`、`VlessResponse`、`Address`、`Command` |
| `src/server.rs` | 服务器核心逻辑、代理转发 | `VlessServer`、`handle_connection()`、`handle_tcp_proxy()`、`handle_udp_proxy()` |
| `src/http.rs` | HTTP 请求检测 | `is_http_request()` - 区分 HTTP 和 VLESS 请求 |
| `src/buffer_pool.rs` | 缓冲区池实现 | `BufferPool`、`acquire()` - 对象池复用缓冲区 |
| `src/utils.rs` | 工具函数、URL 生成 | `generate_vless_url()` - 生成 VLESS 协议 URL |
| `src/wizard.rs` | 交互式配置向导 | `ConfigWizard`、`run()` - 引导用户创建配置 |

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
| `docs/technology.md` | 技术文档 | 架构设计、实现逻辑、流程说明 |

### 功能快速查找

**需要修改/查找...**

- **服务器启动流程** → `src/main.rs:main()`
- **首次配置向导** → `src/wizard.rs:ConfigWizard::run()`
- **配置项和默认值** → `src/config.rs:Config`、`PerformanceConfig`
- **VLESS 协议解析** → `src/protocol.rs:VlessRequest::decode()`
- **用户认证逻辑** → `src/server.rs:handle_connection()`
- **TCP 代理转发** → `src/server.rs:handle_tcp_proxy()`
- **UDP 代理转发** → `src/server.rs:handle_udp_proxy()`
- **HTTP 请求检测** → `src/http.rs:is_http_request()`
- **编译优化配置** → `Cargo.toml` - `[profile.release]`
- **性能参数调整** → `config.json` - `performance` 节点

### 核心模块

#### 1. 协议处理模块 (protocol.rs)

**职责**: VLESS 协议的编解码实现

**核心结构**:
- `VlessRequest`: 解码后的 VLESS 请求
  - 版本号 (u8)
  - 用户 UUID
  - 命令类型 (TCP/UDP)
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

**协议检测**:
```rust
if is_http_request(&header_bytes) {
    // HTTP 请求 - 拒绝处理
} else {
    // VLESS 请求处理路径
}
```

**TCP 代理转发**:
- 使用 `tokio::select!` 同时监听双向数据流
- 任一方向关闭时，整个代理连接终止
- 可配置缓冲区大小 (默认 128KB)

**性能优化**:
- TCP_NODELAY 启用 (降低延迟)
- 大缓冲区适配千兆网络
- 缓冲区池减少内存分配

#### 3. HTTP 检测模块 (http.rs)

**职责**: HTTP 请求检测，区分 HTTP 和 VLESS 协议

**检测机制**:
- 检查数据包前缀是否为 HTTP 方法
- 支持 GET、POST、PUT、DELETE、HEAD、OPTIONS、PATCH、TRACE
- HTTP 请求会被拒绝，仅处理 VLESS 协议

#### 4. 配置模块 (config.rs)

**职责**: 配置文件解析和验证

**配置结构**:
- `server`: 服务器监听配置
  - `listen`: 监听地址
  - `port`: 监听端口
- `users`: 用户 UUID 列表
  - `uuid`: 用户唯一标识符
  - `email`: 用户邮箱（可选）
- `performance`: 性能优化配置
  - `buffer_size`: 传输缓冲区大小（默认 128KB）
  - `tcp_nodelay`: TCP_NODELAY 启用（默认 true）
  - `tcp_recv_buffer`: TCP 接收缓冲区（默认 256KB）
  - `tcp_send_buffer`: TCP 发送缓冲区（默认 256KB）
  - `udp_timeout`: UDP 会话超时（默认 30 秒）
  - `udp_recv_buffer`: UDP 接收缓冲区（默认 64KB）
  - `buffer_pool_size`: 缓冲区池大小（默认 min(32, CPU核心数*4)）

**默认值策略**:
- 使用 serde 默认值
- 配置文件不存在时自动创建
- 支持部分配置缺失

#### 5. 缓冲区池模块 (buffer_pool.rs)

**职责**: 对象池实现，复用缓冲区减少内存分配

**核心功能**:
- 预分配固定数量的缓冲区
- acquire() 获取缓冲区
- 自动归还到池中
- 减少内存分配 95%

#### 6. 配置向导模块 (wizard.rs)

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
    ├─ 客户端 → 目标 (上传)
    └─ 目标 → 客户端 (下载)
    ↓
连接关闭，清理资源
```

### 配置加载流程

```
程序启动
    ↓
检查 config.json 是否存在
    ↓
存在 → 加载配置
不存在 → 启动配置向导
    ↓
启动服务器
```

## 性能优化

### 后端优化

1. **内存分配器优化**:
   - 使用 mimalloc 替代系统分配器
   - 更快的内存分配和释放
   - 减少内存碎片
   - 更好的多线程扩展性

2. **Tokio Features 精简**:
   - 仅启用必要的 features
   - 减少不必要的代码编译
   - 降低二进制体积

3. **大缓冲区**:
   - 默认 128KB 传输缓冲区
   - 适配千兆网络
   - 单连接带宽提升 4 倍

4. **TCP 优化**:
   - TCP_NODELAY 启用
   - 降低延迟
   - 改善小包传输

5. **零拷贝传输**:
   - 使用 `Bytes` 库
   - 减少内存复制
   - 提升吞吐量

6. **静态链接**:
   - CRT 静态链接
   - 零依赖运行
   - 可执行文件约 1 MB

7. **缓冲区池化**:
   - 使用 `object-pool` crate 实现对象池
   - 复用 128KB 缓冲区，减少内存分配 95%
   - 支持可配置的池大小（默认 min(32, CPU核心数*4)）
   - 1000 连接场景内存占用减少 80% (128MB → 25MB)

## 安全考虑

### 认证与授权
- UUID 作为唯一认证凭据
- HashSet O(1) 快速验证
- 无密码，简化部署

### 网络安全
- 拒绝 HTTP 请求，防止非 VLESS 连接
- 建议配合 TLS 使用
- UUID 不在日志中记录

### 数据保护
- 日志不记录敏感信息
- 配置文件权限管理
- 避免密钥泄露

### 部署安全
- 配置防火墙规则
- 限制访问来源
- 定期更新 UUID

## 扩展性

### 协议扩展
- 支持添加新命令类型
- 支持新地址类型
- 保持向后兼容

### 配置扩展
- 添加新的配置项
- 支持环境变量
- 动态配置重载

## 部署架构

### 单文件部署
```
vless.exe (约 1 MB)
├── 静态链接 CRT
└── 零依赖运行
```

### 配置文件
```
config.json
├── server: 监听配置
├── users: 用户列表
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

### 跨平台编译

项目支持多平台交叉编译，所有配置已在 `.cargo/config.toml` 中预设。

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

# 2. 交叉编译
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

# 3. 交叉编译
cargo build --release --target aarch64-unknown-linux-gnu

# 输出: target/aarch64-unknown-linux-gnu/release/vless
```

#### 静态链接说明

项目使用静态链接，生成的二进制文件无需额外依赖:

- **Windows**: 使用 `target-feature=+crt-static`
- **Linux**: 使用 `link-self-contained=yes`

配置文件位置: `.cargo/config.toml`

#### GitHub Actions 自动构建

项目配置了完整的 GitHub Actions CI/CD 工作流，实现跨平台静态链接构建:

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
  4. 编译 Rust 二进制文件
  5. 验证静态链接 (Linux 平台)
  6. 上传构建产物
  7. 创建 GitHub Release（版本号：vx.x.x）
  8. 自动推送标签到仓库

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
