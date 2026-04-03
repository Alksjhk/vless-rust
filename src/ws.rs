//! WebSocket 协议处理模块
//!
//! 处理 WebSocket 连接上的 VLESS 协议请求

use crate::address::connect_target;
use crate::config::PerformanceConfig;
use crate::http::{extract_header_value, extract_http_path, validate_http_headers};
use crate::protocol::{
    authenticate_request, Command, VlessRequest, VlessResponse, VlessResponseSender,
};
use crate::socket::configure_tcp_socket;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bytes::Bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use sha1_smol::Sha1;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};

/// WebSocket 握手密钥常量
const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// 检测 HTTP 请求是否是 WebSocket 升级请求
pub fn is_websocket_upgrade(data: &[u8]) -> bool {
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut has_upgrade = false;
    let mut has_connection_upgrade = false;
    let mut has_ws_key = false;

    for line in text.lines() {
        // 使用 eq_ignore_ascii_case 避免 to_lowercase() 堆分配
        if let Some(pos) = line.find(':') {
            let name = line[..pos].trim();
            let value = line[pos + 1..].trim();
            if name.eq_ignore_ascii_case("upgrade") && value.eq_ignore_ascii_case("websocket") {
                has_upgrade = true;
            } else if name.eq_ignore_ascii_case("connection")
                && value.to_ascii_lowercase().contains("upgrade")
            {
                has_connection_upgrade = true;
            } else if name.eq_ignore_ascii_case("sec-websocket-key") && !value.is_empty() {
                has_ws_key = true;
            }
        }
        if has_upgrade && has_connection_upgrade && has_ws_key {
            break;
        }
    }

    has_upgrade && has_connection_upgrade && has_ws_key
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

    let path = extract_http_path(&header_buf).ok_or_else(|| anyhow!("Invalid HTTP request"))?;
    if path != expected_path {
        warn!(
            "WebSocket path mismatch: expected '{}', got '{}'",
            expected_path, path
        );
        return Err(anyhow!("Invalid WebSocket path: {}", path));
    }

    if let Some(error) = validate_http_headers(&header_buf) {
        return Err(anyhow!("{}", error));
    }

    let ws_key = extract_header_value(&header_buf, "Sec-WebSocket-Key")
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
    )
    .await;

    info!("WebSocket handshake completed for path: {}", path);

    Ok(ws_stream)
}

/// 处理 WebSocket 升级请求（已确认是 WS 升级，直接握手）
pub async fn handle_ws_upgrade(
    stream: TcpStream,
    ws_path: &str,
    header_buffer_size: usize,
) -> Result<(tokio_tungstenite::WebSocketStream<TcpStream>, Bytes)> {
    // detect_ws_connection 已验证是 WS 升级请求，直接握手，无需再 peek
    let mut ws_stream = process_ws_handshake(stream, ws_path, header_buffer_size).await?;

    let first_message = match ws_stream.next().await {
        Some(Ok(Message::Binary(data))) => {
            debug!("Received first WebSocket message: {} bytes", data.len());
            Bytes::from(data)
        }
        Some(Ok(Message::Text(text))) => match BASE64.decode(&text) {
            Ok(data) => {
                debug!(
                    "Received first WebSocket message (Base64): {} bytes",
                    data.len()
                );
                Bytes::from(data)
            }
            Err(_) => return Err(anyhow!("First WebSocket message must be binary")),
        },
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
#[allow(clippy::large_enum_variant)]
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
            let (ws_stream, first_message) =
                handle_ws_upgrade(stream, ws_path, performance_config.ws_header_buffer_size)
                    .await?;
            return Ok(WsConnectionResult::UpgradeSuccess(ws_stream, first_message));
        } else {
            // 普通 HTTP 请求：使用栈上固定缓冲区，避免堆分配
            debug!("Plain HTTP request detected (not WS upgrade)");
            let mut stream = stream;
            let mut http_buf = [0u8; 8192];
            let read_n = stream.read(&mut http_buf).await?;
            let header_bytes = Bytes::copy_from_slice(&http_buf[..read_n]);
            return Ok(WsConnectionResult::HttpRequest(stream, header_bytes));
        }
    }

    // 非 HTTP 请求，在 WebSocket 模式下不支持
    Err(anyhow!(
        "Non-HTTP connection not supported in WebSocket mode"
    ))
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
    authenticate_request(&request, users, client_addr)?;
    info!(
        "Authenticated user {} from {} (WS)",
        request.uuid, client_addr
    );

    let response = VlessResponse::new_with_version(request.version);
    let (mut ws_sender, ws_receiver) = ws_stream.split();

    ws_sender.send_response(&response).await?;

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
            )
            .await
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
    info!(
        "Starting WebSocket proxy for user {} from {}",
        request.uuid, client_addr
    );

    let mut target_stream = connect_target(&request.address, request.port, &perf_config).await?;
    let target_addr = target_stream.peer_addr()?;

    debug!("Connected to target: {}", target_addr);

    if !initial_data.is_empty() {
        target_stream.write_all(&initial_data).await?;
    }

    info!(
        "Established WebSocket proxy connection: {} -> {}",
        client_addr, target_addr
    );

    let (mut target_read, mut target_write) = target_stream.into_split();

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
        debug!("WebSocket receive loop ended");
        let _ = target_write.shutdown().await;
    });

    let target_to_ws = tokio::spawn(async move {
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB，与 TCP 模式对齐

        loop {
            match target_read.read(&mut buffer).await {
                Ok(0) => {
                    debug!("Target connection closed");
                    break;
                }
                Ok(n) => {
                    // 使用 Bytes::copy_from_slice 避免 to_vec() 的额外分配语义混淆；
                    // 注意 tungstenite Message::Binary 接受 Vec<u8>，此处仍需一次拷贝，
                    // 但语义更清晰，且 buffer 可继续复用
                    let payload = buffer[..n].to_vec();
                    if ws_sender.send(Message::Binary(payload)).await.is_err() {
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
