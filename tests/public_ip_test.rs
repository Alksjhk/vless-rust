//! 公网 IP 获取模块测试

use vless_rust::public_ip::{fetch_public_ip_with_timeout, PublicIp};

#[tokio::test]
async fn test_fetch_public_ip_with_timeout() {
    let result = fetch_public_ip_with_timeout(10).await;

    // 这个测试依赖网络，可能成功也可能超时
    if let Some(ip) = result {
        println!("Public IP: {} (from {})", ip.ip, ip.source);
        assert!(!ip.ip.is_empty());
        assert!(!ip.source.is_empty());
    }
}

#[tokio::test]
async fn test_fetch_public_ip_timeout_behavior() {
    // 测试超时参数是否生效
    use std::time::Instant;

    let start = Instant::now();
    let _ = fetch_public_ip_with_timeout(2).await;
    let elapsed = start.elapsed();

    // 应该在 2 秒 + 一些余量内完成
    assert!(elapsed.as_secs() < 5);
}

#[test]
fn test_public_ip_structure() {
    let ip = PublicIp {
        ip: "1.2.3.4".to_string(),
        source: "https://api.example.com/ip".to_string(),
    };

    assert_eq!(ip.ip, "1.2.3.4");
    assert_eq!(ip.source, "https://api.example.com/ip");
}
