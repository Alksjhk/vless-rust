//! VLESS 链接生成模块测试

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use uuid::Uuid;
use vless_rust::vless_link::{generate_vless_links, VlessLinkConfig};

#[test]
fn test_generate_tcp_link() {
    let uuid = Uuid::parse_str("7fa8b8a5-e2d4-44dc-b3b4-0b72f04397d8").unwrap();
    let config = VlessLinkConfig {
        uuid,
        host: "1.2.3.4".to_string(),
        port: 443,
        ws_path: None,
        alias: "user@example.com".to_string(),
    };

    let links = generate_vless_links(&config);

    assert!(links.tcp.vless.contains("vless://"));
    assert!(links.tcp.vless.contains("type=tcp"));
    assert!(links.tcp.vless.contains("security=none"));
    assert!(links.ws.is_none());
}

#[test]
fn test_generate_ws_link() {
    let uuid = Uuid::parse_str("7fa8b8a5-e2d4-44dc-b3b4-0b72f04397d8").unwrap();
    let config = VlessLinkConfig {
        uuid,
        host: "1.2.3.4".to_string(),
        port: 443,
        ws_path: Some("/vless".to_string()),
        alias: "user@example.com".to_string(),
    };

    let links = generate_vless_links(&config);

    assert!(links.ws.is_some());
    let ws = links.ws.unwrap();
    assert!(ws.vless.contains("type=ws"));
    assert!(ws.vless.contains("path=%2Fvless"));
}

#[test]
fn test_vless_link_structure() {
    let uuid = Uuid::new_v4();
    let config = VlessLinkConfig {
        uuid,
        host: "example.com".to_string(),
        port: 8443,
        ws_path: Some("/ws".to_string()),
        alias: "test_user".to_string(),
    };

    let links = generate_vless_links(&config);

    assert_eq!(links.public_ip, "example.com");
    assert_eq!(links.port, 8443);
    assert!(links.tcp.vless.starts_with("vless://"));
    assert!(links.tcp.vless.contains(&uuid.to_string()));
    assert!(!links.tcp.base64.is_empty());
}

#[test]
fn test_base64_encoding() {
    let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
    let config = VlessLinkConfig {
        uuid,
        host: "localhost".to_string(),
        port: 443,
        ws_path: None,
        alias: "test".to_string(),
    };

    let links = generate_vless_links(&config);

    let decoded = String::from_utf8(STANDARD.decode(&links.tcp.base64).unwrap()).unwrap();

    assert_eq!(decoded, links.tcp.vless);
}

#[test]
fn test_alias_encoding() {
    let uuid = Uuid::new_v4();
    let config = VlessLinkConfig {
        uuid,
        host: "1.2.3.4".to_string(),
        port: 443,
        ws_path: None,
        alias: "user with spaces".to_string(),
    };

    let links = generate_vless_links(&config);

    // 别名应该被 URL 编码
    assert!(links.tcp.vless.contains("user%20with%20spaces"));
}
