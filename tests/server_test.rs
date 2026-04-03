//! 服务器模块集成测试

use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;
use vless_rust::config::ProtocolType;
use vless_rust::server::ServerConfig;

// ============================================================================
// 服务器配置创建测试
// ============================================================================

#[test]
fn test_server_config_new_tcp() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let config = ServerConfig::new(
        addr,
        ProtocolType::Tcp,
        "/vless".to_string(),
        Some("1.2.3.4".to_string()),
        8080,
    );

    assert_eq!(config.bind_addr, addr);
    assert_eq!(config.protocol, ProtocolType::Tcp);
    assert_eq!(config.ws_path, "/vless");
    assert_eq!(config.public_ip, Some("1.2.3.4".to_string()));
    assert_eq!(config.port, 8080);
    assert!(config.users.is_empty());
}

#[test]
fn test_server_config_new_websocket() {
    let addr: SocketAddr = "0.0.0.0:443".parse().unwrap();
    let config = ServerConfig::new(
        addr,
        ProtocolType::WebSocket,
        "/custom-ws".to_string(),
        None,
        443,
    );

    assert_eq!(config.protocol, ProtocolType::WebSocket);
    assert_eq!(config.ws_path, "/custom-ws");
    assert_eq!(config.public_ip, None);
    assert_eq!(config.port, 443);
}

// ============================================================================
// 用户管理测试
// ============================================================================

#[test]
fn test_server_config_add_user_with_email() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut config = ServerConfig::new(addr, ProtocolType::Tcp, "/vless".to_string(), None, 8080);

    let uuid = Uuid::new_v4();
    config.add_user_with_email(uuid, Some("user@example.com".to_string()));

    assert!(config.users.contains(&uuid));
    assert_eq!(config.users.len(), 1);

    let email = config.user_emails.get(&uuid).and_then(|e| e.clone());
    assert_eq!(email, Some(Arc::from("user@example.com")));
}

#[test]
fn test_server_config_add_user_without_email() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut config = ServerConfig::new(addr, ProtocolType::Tcp, "/vless".to_string(), None, 8080);

    let uuid = Uuid::new_v4();
    config.add_user_with_email(uuid, None);

    assert!(config.users.contains(&uuid));
    assert_eq!(config.user_emails.get(&uuid).and_then(|e| e.clone()), None);
}

#[test]
fn test_server_config_multiple_users() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut config = ServerConfig::new(addr, ProtocolType::WebSocket, "/ws".to_string(), None, 443);

    let uuid1 = Uuid::new_v4();
    let uuid2 = Uuid::new_v4();
    let uuid3 = Uuid::new_v4();

    config.add_user_with_email(uuid1, Some("user1@example.com".to_string()));
    config.add_user_with_email(uuid2, None);
    config.add_user_with_email(uuid3, Some("user3@example.com".to_string()));

    assert_eq!(config.users.len(), 3);
    assert!(config.users.contains(&uuid1));
    assert!(config.users.contains(&uuid2));
    assert!(config.users.contains(&uuid3));
}

// ============================================================================
// 协议类型测试
// ============================================================================

#[test]
fn test_protocol_type_tcp() {
    let addr: SocketAddr = "0.0.0.0:443".parse().unwrap();
    let config = ServerConfig::new(addr, ProtocolType::Tcp, "/vless".to_string(), None, 443);

    assert_eq!(config.protocol, ProtocolType::Tcp);
}

#[test]
fn test_protocol_type_websocket() {
    let addr: SocketAddr = "0.0.0.0:8443".parse().unwrap();
    let config = ServerConfig::new(
        addr,
        ProtocolType::WebSocket,
        "/custom-path".to_string(),
        None,
        8443,
    );

    assert_eq!(config.protocol, ProtocolType::WebSocket);
    assert_eq!(config.ws_path, "/custom-path");
}

// ============================================================================
// IPv6 绑定测试
// ============================================================================

#[test]
fn test_server_config_ipv6() {
    let addr: SocketAddr = "[::1]:8080".parse().unwrap();
    let config = ServerConfig::new(addr, ProtocolType::Tcp, "/vless".to_string(), None, 8080);

    assert!(config.bind_addr.is_ipv6());
    assert!(config.bind_addr.ip().is_loopback());
}

#[test]
fn test_server_config_ipv6_any() {
    let addr: SocketAddr = "[::]:443".parse().unwrap();
    let config = ServerConfig::new(
        addr,
        ProtocolType::WebSocket,
        "/ws".to_string(),
        Some("2001:db8::1".to_string()),
        443,
    );

    assert!(config.bind_addr.is_ipv6());
    assert!(config.bind_addr.ip().is_unspecified());
}

// ============================================================================
// 公网 IP 测试
// ============================================================================

#[test]
fn test_server_config_public_ip_ipv4() {
    let addr: SocketAddr = "0.0.0.0:443".parse().unwrap();

    let config = ServerConfig::new(
        addr,
        ProtocolType::Tcp,
        "/vless".to_string(),
        Some("203.0.113.1".to_string()),
        443,
    );

    assert_eq!(config.public_ip, Some("203.0.113.1".to_string()));
}

#[test]
fn test_server_config_no_public_ip() {
    let addr: SocketAddr = "0.0.0.0:443".parse().unwrap();

    let config = ServerConfig::new(addr, ProtocolType::Tcp, "/vless".to_string(), None, 443);

    assert_eq!(config.public_ip, None);
}

// ============================================================================
// 端口测试
// ============================================================================

#[test]
fn test_server_config_various_ports() {
    let test_ports = vec![80, 443, 8080, 8443, 10000];

    for port in test_ports {
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        let config = ServerConfig::new(addr, ProtocolType::Tcp, "/vless".to_string(), None, port);

        assert_eq!(config.port, port);
    }
}

// ============================================================================
// 配置克隆测试
// ============================================================================

#[test]
fn test_server_config_clone() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut config = ServerConfig::new(
        addr,
        ProtocolType::Tcp,
        "/vless".to_string(),
        Some("1.2.3.4".to_string()),
        8080,
    );

    let uuid = Uuid::new_v4();
    config.add_user_with_email(uuid, Some("test@example.com".to_string()));

    let cloned = config.clone();

    assert_eq!(cloned.bind_addr, config.bind_addr);
    assert_eq!(cloned.protocol, config.protocol);
    assert_eq!(cloned.users.len(), config.users.len());
    assert!(cloned.users.contains(&uuid));
}
