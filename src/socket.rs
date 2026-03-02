//! TCP Socket 配置模块
//!
//! 提供 TCP socket 参数配置功能

use anyhow::Result;
use socket2::SockRef;
use tokio::net::TcpStream;
use tracing::debug;

/// 配置 TCP socket 选项
///
/// # Arguments
/// * `stream` - TCP 连接流
/// * `recv_buf` - 接收缓冲区大小（0 表示使用系统默认）
/// * `send_buf` - 发送缓冲区大小（0 表示使用系统默认）
/// * `nodelay` - 是否启用 TCP_NODELAY
pub fn configure_tcp_socket(
    stream: &TcpStream,
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
