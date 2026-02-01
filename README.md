# VLESS Protocol Server in Rust

基于Rust和Tokio实现的高性能VLESS协议服务器，遵循xray-core的VLESS协议规范。

## 特性

- ✅ 完整的VLESS协议支持（版本0和版本1）
- ✅ 异步I/O处理（基于Tokio）
- ✅ TCP代理转发
- ✅ 多用户UUID认证
- ✅ 配置文件支持
- ✅ 结构化日志记录
- ✅ 版本兼容性（支持测试版和正式版）
- 🚧 UDP支持（计划中）
- 🚧 Mux多路复用（计划中）
- 🚧 XTLS支持（计划中）

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

### 1. 编译项目

```bash
cargo build --release
```

### 2. 配置服务器

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

### 3. 运行服务器

```bash
# 使用默认配置文件 config.json
cargo run

# 或指定配置文件路径
cargo run -- /path/to/your/config.json
```

### 4. 客户端配置

在支持VLESS协议的客户端中配置：

- **协议**: VLESS
- **地址**: 你的服务器IP
- **端口**: 8443（或配置文件中设置的端口）
- **UUID**: 配置文件中的用户UUID
- **加密**: none
- **传输协议**: TCP

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
- **快速UUID验证**: 使用HashMap进行O(1)时间复杂度的用户验证
- **连接复用**: 每个客户端连接在独立的异步任务中处理

## 安全注意事项

1. **UUID保密**: 确保用户UUID不被泄露，这是唯一的认证凭据
2. **传输加密**: 建议在生产环境中配合TLS使用
3. **访问控制**: 合理配置防火墙规则
4. **日志管理**: 注意日志中可能包含敏感信息

## 开发计划

- [ ] UDP协议支持
- [ ] Mux多路复用支持
- [ ] XTLS集成
- [ ] WebSocket传输层
- [ ] gRPC传输层
- [ ] 流量统计
- [ ] 动态配置重载
- [ ] 性能监控接口

## 协议参考

- [VLESS协议规范](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core项目](https://github.com/XTLS/Xray-core)

## 许可证

MIT License