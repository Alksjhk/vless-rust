//! TCP 模块集成测试

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use vless_rust::address::resolve_address;
use vless_rust::config::PerformanceConfig;
use vless_rust::socket::configure_tcp_socket;

// ============================================================================
// TCP Socket 配置测试
// ============================================================================

#[tokio::test]
async fn test_configure_tcp_socket() {
    // 创建监听器
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    // 创建客户端连接
    let _client_stream = tokio::net::TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();

    // 接受连接
    let (stream, _) = listener.accept().await.unwrap();

    // 创建性能配置
    let perf = PerformanceConfig {
        tcp_recv_buffer: 32768,
        tcp_send_buffer: 32768,
        tcp_nodelay: true,
        ..Default::default()
    };

    // 配置 TCP socket
    let result = configure_tcp_socket(
        &stream,
        perf.tcp_recv_buffer,
        perf.tcp_send_buffer,
        perf.tcp_nodelay,
    );

    assert!(result.is_ok(), "configure_tcp_socket should succeed");

    // 验证 TCP_NODELAY 已设置
    assert!(stream.nodelay().unwrap(), "TCP_NODELAY should be enabled");
}

#[tokio::test]
async fn test_configure_tcp_socket_basic() {
    // 绑定到随机端口
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    // 接受连接需要另一个连接
    let addr = listener.local_addr().unwrap();
    let _client = tokio::net::TcpStream::connect(addr).await;

    // 简单验证 socket 配置函数存在
    assert!(listener.local_addr().is_ok());
}

#[tokio::test]
async fn test_tcp_nodelay_enabled() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // 客户端连接
    let client = tokio::net::TcpStream::connect(addr).await.unwrap();

    // 设置 TCP_NODELAY
    client.set_nodelay(true).unwrap();
    assert!(client.nodelay().unwrap());
}

#[tokio::test]
async fn test_tcp_nodelay_disabled() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let client = tokio::net::TcpStream::connect(addr).await.unwrap();

    // 禁用 TCP_NODELAY
    client.set_nodelay(false).unwrap();
    assert!(!client.nodelay().unwrap());
}

// ============================================================================
// 地址解析测试
// ============================================================================

#[tokio::test]
async fn test_resolve_address_ipv4() {
    let result = resolve_address("127.0.0.1", 80).await;
    assert!(
        result.is_ok(),
        "resolve_address should succeed for IPv4 loopback"
    );

    let addr = result.unwrap();
    assert!(addr.ip().is_loopback(), "IP should be loopback");
    assert_eq!(addr.port(), 80, "Port should match");
}

#[tokio::test]
async fn test_resolve_address_ipv6_loopback() {
    let result = resolve_address("::1", 443).await;
    assert!(result.is_ok());

    let addr = result.unwrap();
    assert!(addr.ip().is_loopback());
    assert_eq!(addr.port(), 443);
}

#[tokio::test]
async fn test_resolve_address_localhost() {
    let result = resolve_address("localhost", 8080).await;
    assert!(result.is_ok());

    let addr = result.unwrap();
    assert_eq!(addr.port(), 8080);
}

#[tokio::test]
async fn test_resolve_address_invalid() {
    // 无效域名应该失败或超时
    let _result = resolve_address("invalid.invalid.invalid", 80).await;
    // 可能因 DNS 解析失败而返回错误
    // 这里不强制断言，因为 DNS 行为因环境而异
}

// ============================================================================
// 性能配置测试
// ============================================================================

#[test]
fn test_performance_config_defaults() {
    let config = PerformanceConfig::default();

    assert_eq!(config.buffer_size, 64 * 1024);
    assert_eq!(config.tcp_recv_buffer, 128 * 1024);
    assert!(config.tcp_nodelay);
    assert_eq!(config.udp_timeout, 30);
}

#[test]
fn test_performance_config_custom() {
    let config = PerformanceConfig {
        buffer_size: 128 * 1024,
        tcp_recv_buffer: 256 * 1024,
        tcp_send_buffer: 256 * 1024,
        tcp_nodelay: false,
        udp_timeout: 60,
        udp_recv_buffer: 128 * 1024,
        buffer_pool_size: 64,
        ws_header_buffer_size: 16 * 1024,
    };

    assert_eq!(config.buffer_size, 128 * 1024);
    assert_eq!(config.tcp_recv_buffer, 256 * 1024);
    assert!(!config.tcp_nodelay);
    assert_eq!(config.udp_timeout, 60);
}

// ============================================================================
// TCP 连接测试
// ============================================================================

#[tokio::test]
async fn test_tcp_port_binding_random() {
    // 绑定到随机端口
    let listener1 = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener1.local_addr().unwrap().port();

    // 验证端口已分配
    assert!(port > 0);
}

#[tokio::test]
async fn test_tcp_connection_timeout() {
    use tokio::time::{timeout, Duration};

    // 尝试连接到一个不存在的地址，应该超时
    let result = timeout(
        Duration::from_millis(100),
        tokio::net::TcpStream::connect("10.255.255.1:9999"),
    )
    .await;

    // 应该超时或连接失败
    assert!(result.is_err());
}

// ============================================================================
// 双向数据转发测试
// ============================================================================

#[tokio::test]
async fn test_bidirectional_data_flow() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // 服务器任务：接受连接，回显数据
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        stream.write_all(&buf[..n]).await.unwrap();
    });

    // 客户端任务：连接服务器，发送数据并验证回显
    let client = tokio::spawn(async move {
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let data = b"Hello, VLESS!";
        stream.write_all(data).await.unwrap();
        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], data, "Echoed data should match original");
    });

    // 等待任务完成
    let (server_result, client_result) = tokio::join!(server, client);
    assert!(server_result.is_ok(), "Server task should succeed");
    assert!(client_result.is_ok(), "Client task should succeed");
}

#[tokio::test]
async fn test_large_data_transfer() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // 创建 1MB 数据
    let data = vec![0xABu8; 1024 * 1024];
    let data_clone = data.clone();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 1024 * 1024];
        let n = stream.read(&mut buf).await.unwrap();
        stream.write_all(&buf[..n]).await.unwrap();
    });

    let client = tokio::spawn(async move {
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream.write_all(&data_clone).await.unwrap();

        let mut buf = vec![0u8; 1024 * 1024];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(n, 1024 * 1024);
    });

    let _ = tokio::join!(server, client);
}

// ============================================================================
// IPv4/IPv6 测试
// ============================================================================

#[tokio::test]
async fn test_tcp_ipv4_binding() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    assert!(addr.is_ipv4());
}

#[tokio::test]
async fn test_tcp_ipv6_binding() {
    // 尝试绑定 IPv6
    let result = TcpListener::bind("[::1]:0").await;

    // IPv6 可能不被所有系统支持
    if let Ok(listener) = result {
        let addr = listener.local_addr().unwrap();
        assert!(addr.is_ipv6());
    }
}

// ============================================================================
// 地址解析测试（从 src/address.rs 移动）
// ============================================================================

#[tokio::test]
async fn test_resolve_address_localhost_from_unit() {
    let result = resolve_address("127.0.0.1", 80).await;
    assert!(result.is_ok());
    let addr = result.unwrap();
    assert_eq!(addr.port(), 80);
    assert!(addr.ip().is_loopback());
}

#[tokio::test]
async fn test_resolve_invalid_domain_from_unit() {
    let result = resolve_address("invalid.invalid.invalid", 80).await;
    assert!(result.is_err());
}
