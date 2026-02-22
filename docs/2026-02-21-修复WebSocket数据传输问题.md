# WebSocket 数据传输问题修复

**日期**: 2026-02-21  
**类型**: Bug Fix  
**严重程度**: Critical

## 问题描述

WebSocket 协议无法正常传输数据，连接建立后数据传输失败。

## 根本原因

在 `src/server.rs` 的 `handle_ws_connection` 函数中存在严重的逻辑错误：

1. 第一个 WebSocket 消息被读取用于解析 VLESS 请求头
2. VLESS 协议解析后返回 `remaining_data`（请求头之后的实际数据负载）
3. 但 `remaining_data` 被命名为 `_remaining_data` 并被丢弃
4. 导致第一个数据包中的实际负载丢失

### 问题代码

```rust
let (request, _remaining_data) = VlessRequest::decode(header_bytes)?;
// _remaining_data 被丢弃！

// 客户端 → 目标（通过 WebSocket）
let upload_task = tokio::spawn(async move {
    loop {
        match ws_receiver.next().await {
            // 从第二个消息开始读取，第一个消息的负载已丢失
            ...
        }
    }
});
```

## 解决方案

### 修改内容

1. **保留剩余数据**：
   ```rust
   let (request, remaining_data) = VlessRequest::decode(header_bytes)?;
   ```

2. **检测初始数据**：
   ```rust
   let has_initial_data = !remaining_data.is_empty();
   if has_initial_data {
       debug!("First WebSocket message contains {} bytes of payload data", remaining_data.len());
   }
   ```

3. **优先发送初始数据**：
   ```rust
   let upload_task = tokio::spawn(async move {
       // 首先发送第一个消息中的剩余数据（如果有）
       if has_initial_data && !remaining_data.is_empty() {
           if target_write.write_all(&remaining_data).await.is_err() {
               return;
           }
       }

       // 继续转发后续的 WebSocket 消息
       loop {
           match ws_receiver.next().await {
               ...
           }
       }
   });
   ```

## 影响范围

- **影响功能**: WebSocket 传输模式
- **影响版本**: 所有之前的版本
- **TCP 模式**: 不受影响

## 测试验证

修复后需要验证：

1. WebSocket 连接建立成功
2. 第一个数据包正确传输
3. 后续数据包正常转发
4. 连接稳定性

### 测试命令

```bash
# 编译检查
cargo check

# 运行测试
cargo test

# 实际连接测试
# 使用 v2ray/xray 客户端连接 WebSocket 模式
```

## 技术细节

### VLESS 协议结构

```
[VLESS Header] [Payload Data]
     ↑              ↑
  解析为 request  remaining_data
```

### 数据流

```
客户端 WebSocket 消息
    ↓
[VLESS Header + 初始数据]
    ↓
解析 → request + remaining_data
    ↓
remaining_data → 目标服务器（必须！）
    ↓
后续消息 → 目标服务器
```

## 相关文件

- `src/server.rs`: 修复 `handle_ws_connection` 函数
- `src/protocol.rs`: VLESS 协议解析（无需修改）
- `src/ws.rs`: WebSocket 握手处理（无需修改）

## 经验教训

1. **不要忽略返回值**: `_remaining_data` 的命名暗示它不重要，但实际上包含关键数据
2. **完整的数据流测试**: 需要测试第一个数据包和后续数据包
3. **协议理解**: 深入理解 VLESS 协议的数据封装方式

## 后续工作

- [ ] 添加 WebSocket 数据传输的集成测试
- [ ] 添加日志记录初始数据的大小
- [ ] 性能测试验证修复后的吞吐量
