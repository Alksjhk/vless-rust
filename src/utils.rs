//! 工具函数模块
//!
//! 提供通用的辅助功能

use anyhow::Result;
use socket2::SockRef;
use tracing::debug;
use uuid::Uuid;

/// 配置 TCP socket 参数
///
/// 设置 TCP socket 的性能优化参数
///
/// # 参数
/// - `stream`: TCP 流引用
/// - `recv_buf`: 接收缓冲区大小（0表示不修改）
/// - `send_buf`: 发送缓冲区大小（0表示不修改）
/// - `nodelay`: 是否启用 TCP_NODELAY
pub fn configure_tcp_socket(
    stream: &tokio::net::TcpStream,
    recv_buf: usize,
    send_buf: usize,
    nodelay: bool,
) -> Result<()> {
    // 设置 TCP_NODELAY，降低延迟
    if nodelay {
        stream.set_nodelay(true)?;
    }

    // 尝试设置 TCP 缓冲区大小
    let socket = SockRef::from(stream);

    if recv_buf > 0 {
        if let Err(e) = socket.set_recv_buffer_size(recv_buf) {
            debug!("Failed to set recv buffer size to {}: {}", recv_buf, e);
        } else {
            debug!("Set recv buffer size to {}", recv_buf);
        }
    }

    if send_buf > 0 {
        if let Err(e) = socket.set_send_buffer_size(send_buf) {
            debug!("Failed to set send buffer size to {}: {}", send_buf, e);
        } else {
            debug!("Set send buffer size to {}", send_buf);
        }
    }

    Ok(())
}

/// 生成 VLESS URL
///
/// 根据 RFC 标准生成 VLESS 协议 URL，用于客户端配置导入
///
/// # 参数
/// - `server`: 服务器地址
/// - `port`: 服务器端口
/// - `uuid`: 用户 UUID
/// - `email`: 用户邮箱（可选）
/// - `ws_path`: WebSocket 路径（可选）
///
/// # 返回
/// VLESS URL 字符串
pub fn generate_vless_url(
    server: &str,
    port: u16,
    uuid: &Uuid,
    email: Option<&str>,
    ws_path: Option<&str>,
) -> String {
    let uuid_str = uuid.to_string();
    let remarks = email.unwrap_or("vless-rust");

    // VLESS URL 格式: vless://uuid@server:port?params#remarks
    let mut url = format!("vless://{}@{}:{}", uuid_str, server, port);

    // 添加查询参数
    let mut params = Vec::new();
    params.push(format!("encryption=none"));
    params.push(format!("type=tcp"));
    params.push(format!("security=none"));

    if let Some(path) = ws_path {
        params.push(format!("transport=ws"));
        params.push(format!("path={}", path));
    }

    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    url.push('#');
    url.push_str(&url_encode(remarks));

    url
}

/// URL 编码
///
/// 对字符串进行 URL 编码，用于 VLESS URL 的备注部分
fn url_encode(input: &str) -> String {
    input.chars().flat_map(|c| {
        if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
            c.to_string().chars().collect::<Vec<_>>()
        } else {
            format!("%{:02X}", c as u8).chars().collect::<Vec<_>>()
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_vless_url() {
        let uuid = Uuid::new_v4();
        let url = generate_vless_url("example.com", 443, &uuid, Some("user@example.com"), Some("/ws"));

        assert!(url.starts_with("vless://"));
        assert!(url.contains("example.com"));
        assert!(url.contains("/ws"));
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("test@example.com"), "test%40example.com");
        assert_eq!(url_encode("simple"), "simple");
    }
}
