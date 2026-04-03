//! 地址解析工具模块
//!
//! 提供统一的地址解析功能，供 TCP 和 WebSocket 模块复用

use crate::config::PerformanceConfig;
use crate::socket::configure_tcp_socket;
use anyhow::{anyhow, Result};
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// 解析目标地址
///
/// 统一处理域名解析和 IP 地址转换
///
/// # Arguments
/// * `domain` - 域名字节
/// * `port` - 目标端口
///
/// # Returns
/// * `SocketAddr` - 解析后的地址
pub async fn resolve_target_address(domain: &[u8], port: u16) -> Result<SocketAddr> {
    let domain_str = std::str::from_utf8(domain).map_err(|_| anyhow!("Invalid domain encoding"))?;
    resolve_address(domain_str, port).await
}

/// 解析地址字符串
///
/// # Arguments
/// * `addr` - 地址字符串（域名或 IP）
/// * `port` - 目标端口
///
/// # Returns
/// * `SocketAddr` - 解析后的地址
pub async fn resolve_address(addr: &str, port: u16) -> Result<SocketAddr> {
    let addr_str = format!("{}:{}", addr, port);
    // 直接使用迭代器的 next()，避免收集所有地址到Vec
    let mut addrs = tokio::net::lookup_host(&addr_str).await?;
    addrs
        .next()
        .ok_or_else(|| anyhow!("Failed to resolve address: {}", addr))
}

/// 从协议地址解析目标
///
/// 供 TCP/WS 代理使用，统一处理 Address 枚举
///
/// # Arguments
/// * `address` - 协议层地址
/// * `port` - 目标端口
///
/// # Returns
/// * `SocketAddr` - 解析后的地址
pub async fn resolve_protocol_address(
    address: &crate::protocol::Address,
    port: u16,
) -> Result<SocketAddr> {
    use crate::protocol::Address;

    match address {
        Address::Domain(domain) => resolve_target_address(domain, port).await,
        _ => address.to_socket_addr(port),
    }
}

/// 连接到目标服务器
///
/// 统一处理地址解析、TCP 连接和 socket 配置
///
/// # Arguments
/// * `address` - 目标地址（协议层）
/// * `port` - 目标端口
/// * `perf_config` - 性能配置
///
/// # Returns
/// * `Result<TcpStream>` - 连接成功返回 TCP 流
pub async fn connect_target(
    address: &crate::protocol::Address,
    port: u16,
    perf_config: &PerformanceConfig,
) -> Result<TcpStream> {
    let target_addr = resolve_protocol_address(address, port).await?;
    let stream = TcpStream::connect(target_addr).await?;
    configure_tcp_socket(
        &stream,
        perf_config.tcp_recv_buffer,
        perf_config.tcp_send_buffer,
        perf_config.tcp_nodelay,
    )?;
    Ok(stream)
}
