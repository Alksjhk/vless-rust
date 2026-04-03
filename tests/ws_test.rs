//! WebSocket 集成测试

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sha1_smol::Sha1;
use vless_rust::http::{extract_http_path, is_http_request, parse_http_request};
use vless_rust::ws::is_websocket_upgrade;

/// WebSocket 升级检测测试
#[test]
fn test_is_websocket_upgrade_valid() {
    let valid_upgrade = b"GET /vless HTTP/1.1\r\n\
        Host: example.com\r\n\
        Upgrade: websocket\r\n\
        Connection: Upgrade\r\n\
        Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
        \r\n";

    assert!(is_websocket_upgrade(valid_upgrade));
}

#[test]
fn test_is_websocket_upgrade_missing_key() {
    let missing_key = b"GET /vless HTTP/1.1\r\n\
        Host: example.com\r\n\
        Upgrade: websocket\r\n\
        Connection: Upgrade\r\n\
        \r\n";

    assert!(!is_websocket_upgrade(missing_key));
}

#[test]
fn test_is_websocket_upgrade_missing_upgrade() {
    let missing_upgrade = b"GET /vless HTTP/1.1\r\n\
        Host: example.com\r\n\
        Connection: Upgrade\r\n\
        Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
        \r\n";

    assert!(!is_websocket_upgrade(missing_upgrade));
}

#[test]
fn test_is_websocket_upgrade_missing_connection() {
    let missing_connection = b"GET /vless HTTP/1.1\r\n\
        Host: example.com\r\n\
        Upgrade: websocket\r\n\
        Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
        \r\n";

    assert!(!is_websocket_upgrade(missing_connection));
}

#[test]
fn test_is_websocket_upgrade_case_insensitive() {
    let mixed_case = b"GET /vless HTTP/1.1\r\n\
        Host: example.com\r\n\
        UPGRADE: websocket\r\n\
        CONNECTION: Upgrade\r\n\
        sec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
        \r\n";

    assert!(is_websocket_upgrade(mixed_case));
}

#[test]
fn test_is_websocket_upgrade_with_additional_headers() {
    let extra_headers = b"GET /vless HTTP/1.1\r\n\
        Host: example.com\r\n\
        User-Agent: TestClient\r\n\
        Upgrade: websocket\r\n\
        Connection: Upgrade\r\n\
        Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
        Sec-WebSocket-Version: 13\r\n\
        \r\n";

    assert!(is_websocket_upgrade(extra_headers));
}

/// HTTP 工具函数测试
#[test]
fn test_extract_http_path() {
    let request = b"GET /vless HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let path = extract_http_path(request);
    assert_eq!(path, Some("/vless".to_string()));
}

#[test]
fn test_extract_http_path_with_query() {
    let request = b"GET /vless?email=test@example.com HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let path = extract_http_path(request);
    // extract_http_path 应该返回完整路径，包含查询参数
    assert_eq!(path, Some("/vless?email=test@example.com".to_string()));
}

#[test]
fn test_extract_http_path_post_request() {
    let request = b"POST /api HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let path = extract_http_path(request);
    assert_eq!(path, Some("/api".to_string()));
}

#[test]
fn test_extract_http_path_malformed() {
    let request = b"Invalid request line";
    let path = extract_http_path(request);
    assert_eq!(path, None);
}

#[test]
fn test_is_http_request() {
    assert!(is_http_request(b"GET / HTTP/1.1\r\n"));
    assert!(is_http_request(b"POST /api HTTP/1.1\r\n"));
    assert!(is_http_request(b"HEAD / HTTP/1.1\r\n"));
    assert!(is_http_request(b"PUT / HTTP/1.1\r\n"));
    assert!(is_http_request(b"DELETE / HTTP/1.1\r\n"));
    assert!(is_http_request(b"OPTIONS / HTTP/1.1\r\n"));
    assert!(is_http_request(b"PATCH / HTTP/1.1\r\n"));
    assert!(is_http_request(b"CONNECT / HTTP/1.1\r\n"));
    assert!(is_http_request(b"TRACE / HTTP/1.1\r\n"));
    assert!(!is_http_request(b"Not HTTP"));
    assert!(!is_http_request(b""));
}

#[test]
fn test_is_http_request_http2_pri() {
    // HTTP/2 连接前言
    assert!(is_http_request(b"PRI * HTTP/2.0\r\n"));
}

#[test]
fn test_is_http_request_partial() {
    assert!(!is_http_request(b"GE"));
    assert!(!is_http_request(b"GET"));
    assert!(is_http_request(b"GET "));
}

#[test]
fn test_parse_http_request_with_params() {
    let data = b"GET /?email=user@example.com HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let query = parse_http_request(data).unwrap();
    assert_eq!(query.path, "/");
    assert_eq!(query.params.get("email").unwrap(), "user@example.com");
}

#[test]
fn test_parse_http_request_root() {
    let data = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let query = parse_http_request(data).unwrap();
    assert_eq!(query.path, "/");
    assert!(query.params.is_empty());
}

#[test]
fn test_parse_http_request_multiple_params() {
    let data = b"GET /test?foo=bar&baz=qux HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let query = parse_http_request(data).unwrap();
    assert_eq!(query.path, "/test");
    assert_eq!(query.params.get("foo").unwrap(), "bar");
    assert_eq!(query.params.get("baz").unwrap(), "qux");
}

#[test]
fn test_parse_http_request_urlencoded() {
    let data =
        b"GET /test?email=user%40example.com&name=John%20Doe HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let query = parse_http_request(data).unwrap();
    assert_eq!(query.path, "/test");
    assert_eq!(query.params.get("email").unwrap(), "user@example.com");
    assert_eq!(query.params.get("name").unwrap(), "John Doe");
}

#[test]
fn test_parse_http_request_invalid() {
    // 空数据应该返回 None
    let data = b"";
    let query = parse_http_request(data);
    assert!(query.is_none());
}

#[test]
fn test_parse_http_request_missing_path() {
    // 只有方法没有路径的数据
    let data = b"GET\r\n";
    let query = parse_http_request(data);
    assert!(query.is_none());
}

/// Base64 和 SHA1 测试
#[test]
fn test_base64_encode_decode() {
    let original = b"Hello, WebSocket!";
    let encoded = BASE64.encode(original);
    let decoded = BASE64.decode(&encoded).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn test_base64_encode_decode_empty() {
    let original = b"";
    let encoded = BASE64.encode(original);
    let decoded = BASE64.decode(&encoded).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn test_base64_encode_decode_binary() {
    let original: Vec<u8> = (0..255).collect();
    let encoded = BASE64.encode(&original);
    let decoded = BASE64.decode(&encoded).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn test_sha1_hash() {
    let mut sha1 = Sha1::new();
    sha1.update(b"test-string");
    let digest = sha1.digest();
    assert_eq!(digest.bytes().len(), 20);
}

#[test]
fn test_sha1_hash_empty() {
    let mut sha1 = Sha1::new();
    sha1.update(b"");
    let digest = sha1.digest();
    assert_eq!(digest.bytes().len(), 20);
    // 已知的空字符串 SHA1 哈希值
    let expected = [
        0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60, 0x18,
        0x90, 0xaf, 0xd8, 0x07, 0x09,
    ];
    assert_eq!(digest.bytes(), expected);
}

#[test]
fn test_sha1_hash_consistency() {
    let input = b"The quick brown fox jumps over the lazy dog";
    let mut sha1 = Sha1::new();
    sha1.update(input);
    let digest1 = sha1.digest().bytes();

    let mut sha2 = Sha1::new();
    sha2.update(input);
    let digest2 = sha2.digest().bytes();

    assert_eq!(digest1, digest2);

    // 已知的标准测试向量
    let expected = [
        0x2f, 0xd4, 0xe1, 0xc6, 0x7a, 0x2d, 0x28, 0xfc, 0xed, 0x84, 0x9e, 0xe1, 0xbb, 0x76, 0xe7,
        0x39, 0x1b, 0x93, 0xeb, 0x12,
    ];
    assert_eq!(digest1, expected);
}

#[test]
fn test_sha1_hash_incremental() {
    let input1 = b"The quick brown fox ";
    let input2 = b"jumps over the lazy dog";

    let mut sha1 = Sha1::new();
    sha1.update(input1);
    sha1.update(input2);
    let digest_incremental = sha1.digest().bytes();

    let mut sha2 = Sha1::new();
    sha2.update(b"The quick brown fox jumps over the lazy dog");
    let digest_single = sha2.digest().bytes();

    assert_eq!(digest_incremental, digest_single);
}
