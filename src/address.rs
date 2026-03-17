//! 地址解析工具模块
//!
//! 提供统一的地址解析功能，供 TCP 和 WebSocket 模块复用

use anyhow::{Result, anyhow};
use std::net::SocketAddr;

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
    let domain_str = std::str::from_utf8(domain)
        .map_err(|_| anyhow!("Invalid domain encoding"))?;
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
    addrs.next()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_address_localhost() {
        let result = resolve_address("127.0.0.1", 80).await;
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert_eq!(addr.port(), 80);
        assert!(addr.ip().is_loopback());
    }

    #[tokio::test]
    async fn test_resolve_invalid_domain() {
        let result = resolve_address("invalid.invalid.invalid", 80).await;
        // 应该失败，因为域名不存在
        assert!(result.is_err());
    }
}
