//! VLESS 协议模块集成测试

use bytes::Bytes;
use std::net::{Ipv4Addr, Ipv6Addr};
use uuid::Uuid;
use vless_rust::protocol::{
    Address, AddressType, Command, VlessRequest, VlessResponse, VLESS_VERSION_BETA,
    VLESS_VERSION_RELEASE,
};

// ============================================================================
// 版本常量测试
// ============================================================================

#[test]
fn test_vless_version_constants() {
    assert_eq!(VLESS_VERSION_BETA, 0);
    assert_eq!(VLESS_VERSION_RELEASE, 1);
}

// ============================================================================
// 命令类型测试
// ============================================================================

#[test]
fn test_command_from_u8_valid() {
    assert!(matches!(Command::try_from(1), Ok(Command::Tcp)));
    assert!(matches!(Command::try_from(2), Ok(Command::Udp)));
    assert!(matches!(Command::try_from(3), Ok(Command::Mux)));
}

#[test]
fn test_command_from_u8_invalid() {
    assert!(Command::try_from(0).is_err());
    assert!(Command::try_from(4).is_err());
    assert!(Command::try_from(255).is_err());
}

// ============================================================================
// 地址类型测试
// ============================================================================

#[test]
fn test_address_type_from_u8_valid() {
    assert!(matches!(AddressType::try_from(1), Ok(AddressType::Ipv4)));
    assert!(matches!(AddressType::try_from(2), Ok(AddressType::Domain)));
    assert!(matches!(AddressType::try_from(3), Ok(AddressType::Ipv6)));
}

#[test]
fn test_address_type_from_u8_invalid() {
    assert!(AddressType::try_from(0).is_err());
    assert!(AddressType::try_from(4).is_err());
}

// ============================================================================
// 地址解码测试
// ============================================================================

#[test]
fn test_address_decode_ipv4() {
    let mut buf = Bytes::from(vec![1, 127, 0, 0, 1]);
    let addr = Address::decode(&mut buf).unwrap();

    match addr {
        Address::Ipv4(ip) => {
            assert_eq!(ip, Ipv4Addr::new(127, 0, 0, 1));
        }
        _ => panic!("Expected IPv4 address"),
    }
}

#[test]
fn test_address_decode_ipv6() {
    let ip_bytes: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let mut data = vec![3];
    data.extend_from_slice(&ip_bytes);

    let mut buf = Bytes::from(data);
    let addr = Address::decode(&mut buf).unwrap();

    match addr {
        Address::Ipv6(ip) => {
            assert_eq!(ip, Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
        }
        _ => panic!("Expected IPv6 address"),
    }
}

#[test]
fn test_address_decode_domain() {
    let domain = b"example.com";
    let mut data = vec![2, domain.len() as u8];
    data.extend_from_slice(domain);

    let mut buf = Bytes::from(data);
    let addr = Address::decode(&mut buf).unwrap();

    match addr {
        Address::Domain(d) => {
            assert_eq!(&d[..], b"example.com");
        }
        _ => panic!("Expected Domain address"),
    }
}

#[test]
fn test_address_decode_empty_buffer() {
    let mut buf = Bytes::from(vec![]);
    assert!(Address::decode(&mut buf).is_err());
}

// ============================================================================
// VLESS 请求解码测试
// ============================================================================

#[test]
fn test_vless_request_decode_version_0() {
    let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

    let mut data = vec![
        0, // 版本 0
    ];
    data.extend_from_slice(uuid.as_bytes());
    data.push(0); // addons 长度
    data.push(1); // 命令 TCP
    data.extend_from_slice(&80u16.to_be_bytes()); // 端口 80
    data.push(1); // IPv4 类型
    data.extend_from_slice(&[127, 0, 0, 1]); // 127.0.0.1

    let buf = Bytes::from(data);
    let (request, _remaining) = VlessRequest::decode(buf).unwrap();

    assert_eq!(request.version, 0);
    assert_eq!(request.uuid, uuid);
    assert_eq!(request.command, Command::Tcp);
    assert_eq!(request.port, 80);
}

#[test]
fn test_vless_request_decode_version_1() {
    let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

    let mut data = vec![
        1, // 版本 1
    ];
    data.extend_from_slice(uuid.as_bytes());
    data.push(0); // addons 长度
    data.push(2); // 命令 UDP
    data.extend_from_slice(&443u16.to_be_bytes()); // 端口 443
    data.push(2); // 域名类型
    data.push(11); // 域名长度
    data.extend_from_slice(b"example.com");

    let buf = Bytes::from(data);
    let (request, _remaining) = VlessRequest::decode(buf).unwrap();

    assert_eq!(request.version, 1);
    assert_eq!(request.uuid, uuid);
    assert_eq!(request.command, Command::Udp);
    assert_eq!(request.port, 443);
}

#[test]
fn test_vless_request_decode_with_addons() {
    let uuid = Uuid::new_v4();

    let mut data = vec![
        1, // 版本 1
    ];
    data.extend_from_slice(uuid.as_bytes());
    data.push(4); // addons 长度 4
    data.extend_from_slice(b"test"); // addons 数据
    data.push(1); // 命令 TCP
    data.extend_from_slice(&8080u16.to_be_bytes());
    data.push(1); // IPv4
    data.extend_from_slice(&[192, 168, 1, 1]);

    let buf = Bytes::from(data);
    let (request, _remaining) = VlessRequest::decode(buf).unwrap();

    assert_eq!(request.addons_length, 4);
}

// ============================================================================
// VLESS 响应编码测试
// ============================================================================

#[test]
fn test_vless_response_encode_version_0() {
    let response = VlessResponse::new_with_version(0);
    let encoded = response.encode();

    assert_eq!(encoded.len(), 2);
    assert_eq!(encoded[0], 0); // 版本
    assert_eq!(encoded[1], 0); // addons 长度
}

#[test]
fn test_vless_response_encode_version_1() {
    let response = VlessResponse::new_with_version(1);
    let encoded = response.encode();

    assert_eq!(encoded.len(), 2);
    assert_eq!(encoded[0], 1); // 版本
    assert_eq!(encoded[1], 0); // addons 长度
}

// ============================================================================
// 错误处理测试
// ============================================================================

#[test]
fn test_vless_request_decode_invalid_version() {
    let data = vec![99]; // 无效版本
    let result = VlessRequest::decode(Bytes::from(data));
    assert!(result.is_err());
}

#[test]
fn test_vless_request_decode_buffer_too_short() {
    let data = vec![1]; // 只有版本，太短
    let result = VlessRequest::decode(Bytes::from(data));
    assert!(result.is_err());
}

#[test]
fn test_vless_request_decode_missing_uuid() {
    let data = vec![1]; // 版本后没有 UUID
    let result = VlessRequest::decode(Bytes::from(data));
    assert!(result.is_err());
}

// ============================================================================
// 地址转换测试
// ============================================================================

#[test]
fn test_address_to_socket_addr_ipv4() {
    let addr = Address::Ipv4(Ipv4Addr::new(127, 0, 0, 1));
    let socket_addr = addr.to_socket_addr(8080).unwrap();

    assert_eq!(socket_addr.port(), 8080);
    assert!(socket_addr.ip().is_loopback());
}

#[test]
fn test_address_to_socket_addr_ipv6() {
    let addr = Address::Ipv6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
    let socket_addr = addr.to_socket_addr(443).unwrap();

    assert_eq!(socket_addr.port(), 443);
    assert!(socket_addr.ip().is_loopback());
}

#[test]
fn test_address_to_socket_addr_domain_fails() {
    let addr = Address::Domain(Bytes::from(&b"example.com"[..]));
    let result = addr.to_socket_addr(80);

    assert!(result.is_err());
}
