//! TCP Socket 配置模块
//!
//! 提供 TCP socket 参数配置功能

use anyhow::Result;
use socket2::{SockRef, TcpKeepalive};
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::debug;

/// TCP Keepalive 参数：60s 空闲后开始探测，每 10s 一次，最多 3 次
const KEEPALIVE_IDLE: Duration = Duration::from_secs(60);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
const KEEPALIVE_RETRIES: u32 = 3;

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

    let socket = SockRef::from(stream);

    // 启用 TCP Keepalive，防止 NAT 超时导致的僵尸连接
    let keepalive = TcpKeepalive::new()
        .with_time(KEEPALIVE_IDLE)
        .with_interval(KEEPALIVE_INTERVAL);

    // retries 仅在支持的平台上设置（Linux、macOS；Windows 忽略）
    #[cfg(not(windows))]
    let keepalive = keepalive.with_retries(KEEPALIVE_RETRIES);
    #[cfg(windows)]
    let _ = KEEPALIVE_RETRIES;

    if let Err(e) = socket.set_tcp_keepalive(&keepalive) {
        debug!("Failed to set TCP keepalive: {}", e);
    } else {
        debug!("TCP keepalive enabled (idle=60s, interval=10s)");
    }

    // 尝试设置 TCP 缓冲区大小
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
