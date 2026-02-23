use crate::protocol::{VlessRequest, VlessResponse, Address};
use crate::config::PerformanceConfig;
use crate::buffer_pool::BufferPool;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpStream, tcp::OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use tokio_tungstenite::tungstenite::Message;
use sha1_smol::Sha1;
use tracing::{debug, warn, info};

/// WebSocket 握手密钥常量
const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// 解析 HTTP 请求头，获取请求路径
fn parse_http_path(data: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    for line in text.lines() {
        if line.starts_with("GET ") || line.starts_with("POST ") {
            // 解析 GET /path HTTP/1.1
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let path = parts[1].to_string();
                // 安全检查：防止路径遍历攻击
                if path.contains("..") || path.contains('\\') {
                    return None;
                }
                return Some(path);
            }
        }
    }
    None
}

/// 从 HTTP 头中提取指定头的值
fn get_header_value(headers: &[u8], header_name: &str) -> Option<String> {
    let text = String::from_utf8_lossy(headers);
    let target = format!("{}:", header_name);
    for line in text.lines() {
        let lower_line = line.to_lowercase();
        let target_lower = target.to_lowercase();
        if lower_line.starts_with(&target_lower) {
            // 找到header，提取值
            if let Some(value) = line.get(target.len()..) {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// 验证 HTTP 请求头的基本安全性
fn validate_http_headers(headers: &[u8]) -> Result<()> {
    // 验证 Content-Length（如果存在）不超过合理限制
    if let Some(content_length) = get_header_value(headers, "Content-Length") {
        let length: usize = content_length.parse().unwrap_or(0);
        // 限制最大请求体大小为 1MB，防止请求走私
        if length > 1024 * 1024 {
            return Err(anyhow!("Content-Length too large"));
        }
    }
    Ok(())
}

/// 验证并处理 WebSocket 升级请求，手动完成握手
async fn process_ws_handshake(
    mut stream: TcpStream,
    expected_path: &str,
    header_buffer_size: usize,
) -> Result<tokio_tungstenite::WebSocketStream<TcpStream>> {
    let mut header_buf = Vec::new();
    let mut temp_buf = [0u8; 1024];

    // 读取 HTTP 请求头直到空行（\r\n\r\n）
    loop {
        let n = stream.read(&mut temp_buf).await?;
        if n == 0 {
            return Err(anyhow!("Connection closed while reading headers"));
        }
        header_buf.extend_from_slice(&temp_buf[..n]);
        // 检查是否到达空行
        if header_buf.len() >= 4 {
            let end = header_buf.len() - 4;
            if &header_buf[end..] == b"\r\n\r\n" {
                break;
            }
        }
        if header_buf.len() >= header_buffer_size {
            return Err(anyhow!("HTTP header too long"));
        }
    }

    // 解析路径并验证
    let path = parse_http_path(&header_buf).ok_or_else(|| anyhow!("Invalid HTTP request"))?;
    if path != expected_path {
        warn!("WebSocket path mismatch: expected '{}', got '{}'", expected_path, path);
        return Err(anyhow!("Invalid WebSocket path: {}", path));
    }

    // 验证 HTTP 头安全性
    validate_http_headers(&header_buf)?;

    // 提取 Sec-WebSocket-Key
    let ws_key = get_header_value(&header_buf, "Sec-WebSocket-Key")
        .ok_or_else(|| anyhow!("Missing Sec-WebSocket-Key header"))?;

    // 提取并验证 Host 头（可选但推荐）
    let _host = get_header_value(&header_buf, "Host");

    // 提取 Origin（可选，用于验证）
    let _origin = get_header_value(&header_buf, "Origin");

    // 生成 Sec-WebSocket-Accept
    let mut sha1 = Sha1::new();
    sha1.update(ws_key.as_bytes());
    sha1.update(WEBSOCKET_GUID.as_bytes());
    let accept_key = BASE64.encode(sha1.digest().bytes());

    // 构建 WebSocket 升级响应
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
        Upgrade: websocket\r\n\
        Connection: Upgrade\r\n\
        Sec-WebSocket-Accept: {}\r\n\
        \r\n",
        accept_key
    );

    // 发送响应
    stream.write_all(response.as_bytes()).await?;

    // 创建 WebSocket 流
    let ws_stream = tokio_tungstenite::WebSocketStream::from_raw_socket(
        stream,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    ).await;

    info!("WebSocket handshake completed for path: {}", path);

    Ok(ws_stream)
}

/// 处理 WebSocket 升级请求
pub async fn handle_ws_upgrade(
    stream: TcpStream,
    ws_path: &str,
    header_buffer_size: usize,
) -> Result<(tokio_tungstenite::WebSocketStream<TcpStream>, Bytes)> {
    // 先 peek HTTP 请求头，验证路径
    let mut buffer = vec![0u8; header_buffer_size];
    let n = stream.peek(&mut buffer).await?;
    if n == 0 {
        return Err(anyhow!("Empty request"));
    }

    // 解析请求路径
    let path = parse_http_path(&buffer[..n]).ok_or_else(|| anyhow!("Invalid HTTP request"))?;

    // 验证路径是否匹配
    if path != ws_path {
        warn!("WebSocket path mismatch: expected '{}', got '{}'", ws_path, path);
        return Err(anyhow!("Invalid WebSocket path: {}", path));
    }

    info!("WebSocket path validated: {}", path);

    // 处理握手（在握手内部验证路径）
    let mut ws_stream = process_ws_handshake(stream, &path, header_buffer_size).await?;

    // 读取第一个消息（VLESS 请求）
    let first_message = match ws_stream.next().await {
        Some(Ok(Message::Binary(data))) => {
            debug!("Received first WebSocket message: {} bytes", data.len());
            Bytes::from(data)
        }
        Some(Ok(Message::Text(text))) => {
            // 尝试解析为二进制（Base64 编码）
            match BASE64.decode(&text) {
                Ok(data) => {
                    debug!("Received first WebSocket message (Base64): {} bytes", data.len());
                    Bytes::from(data)
                }
                Err(_) => return Err(anyhow!("First WebSocket message must be binary"))
            }
        }
        Some(Ok(Message::Close(_))) | None => {
            return Err(anyhow!("WebSocket closed by client"));
        }
        _ => {
            return Err(anyhow!("Unexpected WebSocket message type"));
        }
    };

    Ok((ws_stream, first_message))
}

/// 处理 WebSocket 代理连接
pub async fn handle_ws_proxy(
    mut ws_sender: SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
    mut ws_receiver: SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
    request: VlessRequest,
    initial_data: Bytes,
    perf_config: PerformanceConfig,
    _user_email: Option<Arc<str>>,
    buffer_pool: BufferPool,
    client_addr: SocketAddr,
) -> Result<()> {
    use crate::server::configure_tcp_socket;

    info!("Starting WebSocket proxy for user {} from {}", request.uuid, client_addr);

    // 解析目标地址
    let target_addr = match &request.address {
        Address::Domain(domain) => {
            let domain_str = std::str::from_utf8(domain).map_err(|_| anyhow!("Invalid domain encoding"))?;
            let addr_str = format!("{}:{}", domain_str, request.port);
            let resolved = tokio::net::lookup_host(&addr_str)
                .await?
                .next()
                .ok_or_else(|| anyhow!("Failed to resolve domain: {}", domain_str))?;
            resolved
        }
        _ => request.address.to_socket_addr(request.port)?,
    };

    debug!("Connecting to target: {}", target_addr);
    let mut target_stream = tokio::net::TcpStream::connect(target_addr).await?;

    // 配置目标连接的 TCP 参数
    configure_tcp_socket(
        &target_stream,
        perf_config.tcp_recv_buffer,
        perf_config.tcp_send_buffer,
        perf_config.tcp_nodelay,
    )?;

    // 如果有初始数据，先发送给目标服务器
    if !initial_data.is_empty() {
        target_stream.write_all(&initial_data).await?;
    }

    info!("Established WebSocket proxy connection: {} -> {}",
          client_addr, target_addr);

    // 创建目标流的读写 halves
    let (mut target_read, mut target_write) = target_stream.into_split();

    // 为两个任务分别创建缓冲区池副本
    let buffer_pool_w2t = buffer_pool.clone();
    let buffer_pool_t2w = buffer_pool;

    // 辅助函数：将数据通过缓冲区分块转发到目标
    async fn forward_to_target(
        target_write: &mut OwnedWriteHalf,
        buffer: &mut [u8],
        data: &[u8],
    ) -> bool {
        let mut offset = 0;
        while offset < data.len() {
            let chunk_size = std::cmp::min(buffer.len(), data.len() - offset);
            buffer[..chunk_size].copy_from_slice(&data[offset..offset + chunk_size]);
            if target_write.write_all(&buffer[..chunk_size]).await.is_err() {
                return false;
            }
            offset += chunk_size;
        }
        true
    }

    // 任务1：WebSocket -> 目标（使用缓冲区池）
    let ws_to_target = tokio::spawn(async move {
        let mut buffer = buffer_pool_w2t.acquire();

        loop {
            match ws_receiver.next().await {
                Some(Ok(Message::Binary(data))) => {
                    if !forward_to_target(&mut target_write, &mut buffer, &data).await {
                        break;
                    }
                }
                Some(Ok(Message::Text(text))) => {
                    // 只接受 Base64 编码的二进制数据
                    match BASE64.decode(&text) {
                        Ok(data) => {
                            if !forward_to_target(&mut target_write, &mut buffer, &data).await {
                                break;
                            }
                        }
                        Err(_) => {
                            warn!("Received non-binary Text message that is not valid Base64, skipping");
                        }
                    }
                }
                Some(Ok(Message::Close(_))) | None => {
                    debug!("WebSocket closed by client");
                    break;
                }
                Some(Err(e)) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        // 释放缓冲区
        drop(buffer);
        // 关闭目标连接
        let _ = target_write.shutdown().await;
    });

    // 任务2：目标 -> WebSocket（使用缓冲区池）
    let target_to_ws = tokio::spawn(async move {
        let mut buffer = buffer_pool_t2w.acquire();

        loop {
            match target_read.read(&mut buffer[..]).await {
                Ok(0) => {
                    debug!("Target connection closed");
                    break;
                }
                Ok(n) => {
                    // 发送二进制消息
                    if ws_sender.send(Message::Binary(buffer[..n].to_vec())).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        // 释放缓冲区
        drop(buffer);
        // 发送 WebSocket 关闭帧
        let _ = ws_sender.send(Message::Close(None)).await;
    });

    // 等待两个任务完成
    let _ = tokio::join!(ws_to_target, target_to_ws);

    debug!("WebSocket proxy session closed");
    Ok(())
}

/// 发送 WebSocket 响应
pub async fn send_ws_response<S>(ws_sender: &mut S, response: &VlessResponse) -> Result<()>
where
    S: SinkExt<Message> + Unpin,
{
    let data = response.encode();
    if let Err(_e) = ws_sender.send(Message::Binary(data.to_vec())).await {
        return Err(anyhow!("WebSocket send error"));
    }
    Ok(())
}
