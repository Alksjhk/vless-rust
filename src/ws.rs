use anyhow::{Result, anyhow};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;
use futures_util::StreamExt;
use tracing::info;

// 重新导出 configure_tcp_socket 供其他模块使用
pub use crate::utils::configure_tcp_socket;

/// 规范化并验证 WebSocket 路径
///
/// 此函数执行以下安全检查：
/// 1. URL 解码（百分号编码）
/// 2. 路径遍历检测（防止 ../ 攻击）
/// 3. 路径规范化（移除多余斜杠）
/// 4. 大小写规范化（Windows 系统上不区分大小写）
fn normalize_and_validate_path(path: &str) -> Result<String> {
    // 移除查询参数和片段标识符
    let base_path = extract_base_path(path);

    // URL 解码（处理百分号编码）
    let decoded = urlencoding::decode(base_path)
        .map_err(|_| anyhow!("Invalid URL encoding in path"))?;

    // 检查路径遍历攻击
    if decoded.contains("..") || decoded.contains("\\") {
        return Err(anyhow!("Path traversal detected"));
    }

    // 规范化路径：移除多余的斜杠，确保以 / 开头
    let normalized = decoded
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    // 确保路径以 / 开头
    let normalized = format!("/{}", normalized);

    // 在 Windows 系统上进行大小写不敏感比较
    #[cfg(windows)]
    let normalized = normalized.to_lowercase();

    Ok(normalized)
}

/// 从路径中提取基础路径（移除查询参数和片段标识符）
fn extract_base_path(path: &str) -> &str {
    // 移除片段标识符 (#)
    let path = path.split('#').next().unwrap_or(path);
    // 移除查询参数 (?)
    path.split('?').next().unwrap_or(path)
}

/// 处理 WebSocket 握手和升级
pub async fn handle_ws_upgrade(
    mut stream: TcpStream,
    ws_path: &str,
    client_addr: std::net::SocketAddr,
    header_buffer_size: usize,
) -> Result<(WebSocketStream<TcpStream>, Vec<u8>)> {
    // 读取 HTTP 请求头（使用可配置的缓冲区大小）
    let mut header_buf = vec![0u8; header_buffer_size];
    let n = stream.read(&mut header_buf).await?;
    if n == 0 {
        return Err(anyhow!("Connection closed by client"));
    }

    let header_str = String::from_utf8_lossy(&header_buf[..n]);

    // 解析请求行
    let lines: Vec<&str> = header_str.lines().collect();
    if lines.is_empty() {
        return Err(anyhow!("Invalid HTTP request"));
    }

    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid request line"));
    }

    let method = parts[0];
    let path = parts[1];

    // 验证方法
    if method != "GET" {
        send_http_error(&mut stream, 405, "Method Not Allowed").await?;
        return Err(anyhow!("Invalid method: {}", method));
    }

    // 规范化并验证请求路径
    let normalized_path = normalize_and_validate_path(path)?;

    // 规范化配置的路径（仅一次，在启动时或首次使用时）
    let normalized_ws_path = {
        #[cfg(windows)]
        let path = ws_path.to_lowercase();
        #[cfg(not(windows))]
        let path = ws_path.to_string();

        // 确保配置的路径以 / 开头
        if path.starts_with('/') {
            path
        } else {
            format!("/{}", path)
        }
    };

    // 验证路径
    if normalized_path != normalized_ws_path {
        send_http_error(&mut stream, 404, "Not Found").await?;
        return Err(anyhow!(
            "Invalid path: {} (normalized: {}, expected: {})",
            path,
            normalized_path,
            normalized_ws_path
        ));
    }

    // 解析请求头
    let mut headers = std::collections::HashMap::new();
    for line in lines.iter().skip(1) {
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_lowercase();
            let value = line[pos + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    // 验证 WebSocket 升级请求
    let upgrade = headers.get("upgrade").map(|s| s.to_lowercase()).unwrap_or_default();
    let connection = headers.get("connection").map(|s| s.to_lowercase()).unwrap_or_default();
    let ws_key = headers.get("sec-websocket-key").cloned().unwrap_or_default();

    if upgrade != "websocket" || !connection.contains("upgrade") || ws_key.is_empty() {
        send_http_error(&mut stream, 400, "Bad Request").await?;
        return Err(anyhow!("Not a WebSocket upgrade request"));
    }

    // 生成 WebSocket 响应密钥
    let response_key = generate_accept_key(&ws_key);

    // 手动发送 HTTP 101 响应
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\
         \r\n",
        response_key
    );

    stream.write_all(response.as_bytes()).await?;

    // 手动创建 WebSocket 流（不再使用 accept_async，避免双重握手）
    let ws_stream = WebSocketStream::from_raw_socket(stream, tungstenite::protocol::Role::Server, None).await;

    info!("WebSocket connection established from {}", client_addr);

    // 使用 StreamExt 直接读取第一个消息
    read_first_message(ws_stream).await
}

/// 读取第一个 WebSocket 消息
/// 使用 peek 方法，在不消费消息的情况下检查类型
async fn read_first_message(
    mut ws_stream: WebSocketStream<TcpStream>
) -> Result<(WebSocketStream<TcpStream>, Vec<u8>)> {
    // 使用 StreamExt::next 读取第一个消息
    let first_data = match ws_stream.next().await {
        Some(Ok(Message::Binary(data))) => data,
        Some(Ok(Message::Text(text))) => text.into_bytes(),
        Some(Ok(Message::Close(_))) => return Err(anyhow!("WebSocket closed by client")),
        Some(Err(e)) => return Err(anyhow!("WebSocket error: {}", e)),
        None => return Err(anyhow!("WebSocket stream ended")),
        Some(Ok(_)) => return Err(anyhow!("Unexpected WebSocket message type")),
    };

    Ok((ws_stream, first_data))
}

/// 发送 HTTP 错误响应
async fn send_http_error(stream: &mut TcpStream, code: u16, message: &str) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {} {}\r\n\
        Connection: close\r\n\
        Content-Length: 0\r\n\
        \r\n",
        code, message
    );
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

/// 生成 WebSocket 接受密钥
fn generate_accept_key(key: &str) -> String {
    // RFC 6455 规定的 GUID
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    let mut key_bytes = Vec::with_capacity(key.len() + GUID.len());
    key_bytes.extend_from_slice(key.as_bytes());
    key_bytes.extend_from_slice(GUID.as_bytes());

    let mut hasher = sha1_smol::Sha1::new();
    hasher.update(&key_bytes);
    let result = hasher.digest().bytes();

    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(&result)
}
