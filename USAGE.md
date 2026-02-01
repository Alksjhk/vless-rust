# VLESS服务器使用指南

## 项目概述

这是一个基于Rust和Tokio实现的高性能VLESS协议服务器，完全遵循xray-core的VLESS协议规范。

## 功能特性

✅ **已实现功能**
- VLESS协议v1完整支持
- TCP代理转发
- 多用户UUID认证
- 异步高并发处理
- 配置文件管理
- 结构化日志记录

🚧 **计划功能**
- UDP协议支持
- Mux多路复用
- XTLS集成
- WebSocket传输

## 快速开始

### 1. 编译项目

```bash
# 开发版本
cargo build

# 生产版本（推荐）
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

**重要配置说明：**
- `listen`: 监听地址，`0.0.0.0` 表示监听所有网络接口
- `port`: 监听端口，建议使用443或8443
- `uuid`: 用户唯一标识符，必须是标准UUID格式
- `email`: 用户标识（可选）

### 3. 启动服务器

```bash
# 使用默认配置文件
cargo run --release --bin vl-222

# 指定配置文件
cargo run --release --bin vl-222 -- /path/to/config.json
```

### 4. 测试连接

运行内置测试客户端：

```bash
cargo run --release --bin test_client
```

## 客户端配置

### 支持的客户端

- v2rayN (Windows)
- v2rayNG (Android)
- Qv2ray (跨平台)
- Clash Meta
- 其他支持VLESS协议的客户端

### 配置参数

```json
{
  "protocol": "vless",
  "settings": {
    "vnext": [
      {
        "address": "your-server-ip",
        "port": 8443,
        "users": [
          {
            "id": "12345678-1234-1234-1234-123456789abc",
            "encryption": "none"
          }
        ]
      }
    ]
  },
  "streamSettings": {
    "network": "tcp"
  }
}
```

### 客户端配置要点

1. **协议**: 选择 `VLESS`
2. **地址**: 服务器IP地址
3. **端口**: 配置文件中设置的端口
4. **UUID**: 配置文件中的用户UUID
5. **加密**: 选择 `none`
6. **传输协议**: 选择 `TCP`

## 性能优化

### 系统级优化

```bash
# 增加文件描述符限制
ulimit -n 65535

# 优化TCP参数
echo 'net.core.somaxconn = 65535' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_max_syn_backlog = 65535' >> /etc/sysctl.conf
sysctl -p
```

### 应用级优化

- 使用 `--release` 模式编译
- 合理设置用户数量
- 监控内存和CPU使用情况

## 安全建议

### 1. UUID管理
- 使用强随机UUID
- 定期更换用户UUID
- 不要在公共场所分享UUID

### 2. 网络安全
- 配置防火墙规则
- 使用非标准端口
- 考虑配合TLS使用

### 3. 服务器安全
- 定期更新系统
- 监控异常连接
- 备份配置文件

## 故障排除

### 常见问题

**1. 连接被拒绝**
```
检查服务器是否正常启动
确认端口是否被占用
验证防火墙设置
```

**2. 认证失败**
```
检查UUID格式是否正确
确认用户是否在配置文件中
验证客户端配置
```

**3. 代理失败**
```
检查目标地址是否可达
确认DNS解析是否正常
验证网络连接
```

### 日志分析

服务器提供详细的结构化日志：

```
INFO: 服务器启动和配置信息
DEBUG: 连接处理详情
WARN: 异常情况警告
ERROR: 错误信息
```

## 监控和维护

### 性能监控

```bash
# 查看连接数
netstat -an | grep :8443 | wc -l

# 监控资源使用
top -p $(pgrep vl-222)

# 查看日志
tail -f /var/log/vless-server.log
```

### 定期维护

- 检查日志文件大小
- 监控内存使用情况
- 更新依赖库版本
- 备份配置文件

## 开发和贡献

### 项目结构

```
src/
├── main.rs          # 主程序入口
├── protocol.rs      # VLESS协议实现
├── server.rs        # 服务器核心逻辑
├── config.rs        # 配置管理
└── bin/
    └── test_client.rs # 测试客户端
```

### 开发环境

```bash
# 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆项目
git clone <repository-url>
cd vless-server

# 开发模式运行
cargo run --bin vl-222
```

## 许可证

MIT License - 详见 LICENSE 文件