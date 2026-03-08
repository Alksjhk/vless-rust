//! WebSocket 协议处理模块
//!
//! 处理 WebSocket 连接上的 VLESS 协议请求

use crate::config::PerformanceConfig;
use crate::protocol::{VlessRequest, VlessResponse, Address, Command};
use crate::socket::configure_tcp_socket;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use tokio_tungstenite::tungstenite::Message;
use sha1_smol::Sha1;
use tracing::{debug, warn, info};

/// WebSocket 握手密钥常量
const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// 检测 HTTP 请求是否是 WebSocket 升级请求
pub fn is_websocket_upgrade(data: &[u8]) -> bool {
    let text = String::from_utf8_lossy(data);
    let mut has_upgrade = false;
    let mut has_connection_upgrade = false;
    let mut has_ws_key = false;

    for line in text.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("upgrade:") {
            if lower.contains("websocket") {
                has_upgrade = true;
            }
        }
        if lower.starts_with("connection:") {
            if lower.contains("upgrade") {
                has_connection_upgrade = true;
            }
        }
        if lower.starts_with("sec-websocket-key:") {
            has_ws_key = true;
        }
    }

    has_upgrade && has_connection_upgrade && has_ws_key
}

/// 解析 HTTP 请求头，获取请求路径
fn parse_http_path(data: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    for line in text.lines() {
        if line.starts_with("GET ") || line.starts_with("POST ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let path = parts[1].to_string();
                // 安全检查：防止路径遍历攻击（包括 URL 编码形式）
                let decoded_path = urlencoding::decode(&path).unwrap_or_default();
                if decoded_path.contains("..") || decoded_path.contains('\\') {
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
            if let Some(value) = line.get(target.len()..) {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// 验证 HTTP 请求头的基本安全性
fn validate_http_headers(headers: &[u8]) -> Result<()> {
    if let Some(content_length) = get_header_value(headers, "Content-Length") {
        let length: usize = content_length.parse().unwrap_or(0);
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

    loop {
        let n = stream.read(&mut temp_buf).await?;
        if n == 0 {
            return Err(anyhow!("Connection closed while reading headers"));
        }
        header_buf.extend_from_slice(&temp_buf[..n]);
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

    let path = parse_http_path(&header_buf).ok_or_else(|| anyhow!("Invalid HTTP request"))?;
    if path != expected_path {
        warn!("WebSocket path mismatch: expected '{}', got '{}'", expected_path, path);
        return Err(anyhow!("Invalid WebSocket path: {}", path));
    }

    validate_http_headers(&header_buf)?;

    let ws_key = get_header_value(&header_buf, "Sec-WebSocket-Key")
        .ok_or_else(|| anyhow!("Missing Sec-WebSocket-Key header"))?;

    let mut sha1 = Sha1::new();
    sha1.update(ws_key.as_bytes());
    sha1.update(WEBSOCKET_GUID.as_bytes());
    let accept_key = BASE64.encode(sha1.digest().bytes());

    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
        Upgrade: websocket\r\n\
        Connection: Upgrade\r\n\
        Sec-WebSocket-Accept: {}\r\n\
        \r\n",
        accept_key
    );

    stream.write_all(response.as_bytes()).await?;

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
    let mut buffer = vec![0u8; header_buffer_size];
    let n = stream.peek(&mut buffer).await?;
    if n == 0 {
        return Err(anyhow!("Empty request"));
    }

    let path = parse_http_path(&buffer[..n]).ok_or_else(|| anyhow!("Invalid HTTP request"))?;

    if path != ws_path {
        warn!("WebSocket path mismatch: expected '{}', got '{}'", ws_path, path);
        return Err(anyhow!("Invalid WebSocket path: {}", path));
    }

    info!("WebSocket path validated: {}", path);

    let mut ws_stream = process_ws_handshake(stream, &path, header_buffer_size).await?;

    let first_message = match ws_stream.next().await {
        Some(Ok(Message::Binary(data))) => {
            debug!("Received first WebSocket message: {} bytes", data.len());
            Bytes::from(data)
        }
        Some(Ok(Message::Text(text))) => {
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

/// WebSocket 连接处理结果
pub enum WsConnectionResult {
    /// WebSocket 升级成功，返回流和首条消息
    UpgradeSuccess(tokio_tungstenite::WebSocketStream<TcpStream>, Bytes),
    /// 普通 HTTP 请求，返回流和已读取的数据
    HttpRequest(TcpStream, Bytes),
}

/// 检测并处理 WebSocket 连接
///
/// 返回连接类型，由调用者决定后续处理
pub async fn detect_ws_connection(
    stream: TcpStream,
    ws_path: &str,
    performance_config: PerformanceConfig,
) -> Result<WsConnectionResult> {
    use crate::http::is_http_request;
    use tokio::io::AsyncReadExt;

    // 配置 TCP socket 参数
    configure_tcp_socket(
        &stream,
        performance_config.tcp_recv_buffer,
        performance_config.tcp_send_buffer,
        performance_config.tcp_nodelay,
    )?;

    // 先 peek 数据检测请求类型
    let mut peek_buf = [0u8; 1024];
    let n = stream.peek(&mut peek_buf).await?;
    if n == 0 {
        return Err(anyhow!("Connection closed by client"));
    }

    // 检测是否是 HTTP 请求
    if is_http_request(&peek_buf[..n]) {
        // 检测是否是 WebSocket 升级请求
        if is_websocket_upgrade(&peek_buf[..n]) {
            debug!("WebSocket upgrade request detected");
            let (ws_stream, first_message) = handle_ws_upgrade(
                stream,
                ws_path,
                performance_config.ws_header_buffer_size,
            ).await?;
            return Ok(WsConnectionResult::UpgradeSuccess(ws_stream, first_message));
        } else {
            // 普通 HTTP 请求
            debug!("Plain HTTP request detected (not WS upgrade)");
            let mut stream = stream;
            let mut http_buf = vec![0u8; performance_config.ws_header_buffer_size];
            let read_n = stream.read(&mut http_buf).await?;
            let header_bytes = Bytes::copy_from_slice(&http_buf[..read_n]);
            return Ok(WsConnectionResult::HttpRequest(stream, header_bytes));
        }
    }

    // 非 HTTP 请求，在 WebSocket 模式下不支持
    Err(anyhow!("Non-HTTP connection not supported in WebSocket mode"))
}

/// 处理已验证的 WebSocket VLESS 连接
pub async fn handle_ws_vless(
    ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
    first_message: Bytes,
    users: &std::collections::HashSet<uuid::Uuid>,
    get_user_email: impl Fn(&uuid::Uuid) -> Option<Arc<str>>,
    performance_config: PerformanceConfig,
    client_addr: SocketAddr,
) -> Result<()> {
    // 解析 VLESS 请求
    let (request, remaining_data) = VlessRequest::decode(first_message)?;

    debug!("Parsed VLESS request from WS: {:?}", request);

    // 验证用户 UUID
    if !users.contains(&request.uuid) {
        warn!("Invalid UUID from {}: {} (not in config)", client_addr, request.uuid);
        return Err(anyhow!("Authentication failed: invalid user UUID (addr: {})", client_addr));
    }

    info!("Authenticated user {} from {} (WS)", request.uuid, client_addr);

    // 创建 VLESS 响应并发送
    let response = VlessResponse::new_with_version(request.version);
    let (mut ws_sender, ws_receiver) = ws_stream.split();

    // 发送响应
    send_ws_response(&mut ws_sender, &response).await?;

    // 获取用户邮箱
    let user_email = get_user_email(&request.uuid);

    // 根据命令类型分发处理
    match request.command {
        Command::Tcp => {
            handle_ws_proxy(
                ws_sender,
                ws_receiver,
                request,
                remaining_data,
                performance_config,
                user_email,
                client_addr,
            ).await
        }
        Command::Udp => {
            warn!("UDP over WebSocket not fully implemented");
            Err(anyhow!("UDP over WebSocket not supported yet"))
        }
        Command::Mux => {
            warn!("Mux command not supported");
            Err(anyhow!("Mux not supported"))
        }
    }
}

/// 处理 WebSocket 代理连接
pub async fn handle_ws_proxy(
    mut ws_sender: SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
    mut ws_receiver: SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
    request: VlessRequest,
    initial_data: Bytes,
    perf_config: PerformanceConfig,
    _user_email: Option<Arc<str>>,
    client_addr: SocketAddr,
) -> Result<()> {
    info!("Starting WebSocket proxy for user {} from {}", request.uuid, client_addr);

    // 解析目标地址
    let target_addr = match &request.address {
        Address::Domain(domain) => {
            let domain_str = std::str::from_utf8(domain)
                .map_err(|_| anyhow!("Invalid domain encoding"))?;
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

    let (mut target_read, mut target_write) = target_stream.into_split();

    // 任务1：WebSocket -> 目标
    let ws_to_target = tokio::spawn(async move {
        loop {
            match ws_receiver.next().await {
                Some(Ok(Message::Binary(data))) => {
                    if target_write.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Text(text))) => {
                    match BASE64.decode(&text) {
                        Ok(data) => {
                            if target_write.write_all(&data).await.is_err() {
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
        let _ = target_write.shutdown().await;
    });

    // 任务2：目标 -> WebSocket
    let target_to_ws = tokio::spawn(async move {
        let mut buffer = [0u8; 8 * 1024];

        loop {
            match target_read.read(&mut buffer).await {
                Ok(0) => {
                    debug!("Target connection closed");
                    break;
                }
                Ok(n) => {
                    let data = buffer[..n].to_vec();
                    if ws_sender.send(Message::Binary(data)).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = ws_sender.send(Message::Close(None)).await;
    });

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
