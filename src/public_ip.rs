//! 公网 IP 获取模块
//! 
//! 使用多个 API 接口并发获取公网 IP，返回首个成功结果

use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// 公网 IP 信息
#[derive(Debug, Clone)]
pub struct PublicIp {
    pub ip: String,
    pub source: String,
}

/// 公网 IP API 端点列表
const IP_API_ENDPOINTS: &[&str] = &[
    "https://api.ipify.org",
    "https://ifconfig.me/ip",
    "https://api4.my-ip.io/ip",
    "https://checkip.amazonaws.com",
    "https://icanhazip.com",
];

/// 从单个 API 获取公网 IP
async fn fetch_from_api(client: &reqwest::Client, url: &str) -> Option<String> {
    match client.get(url).timeout(Duration::from_secs(3)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.text().await {
                    Ok(text) => {
                        let ip = text.trim().to_string();
                        // 简单验证是否为有效 IP（IPv4 或 IPv6）
                        if !ip.is_empty() && ip.len() <= 45 && !ip.contains('<') {
                            debug!("Got public IP from {}: {}", url, ip);
                            return Some(ip);
                        }
                        warn!("Invalid IP format from {}: {}", url, ip);
                    }
                    Err(e) => warn!("Failed to read response from {}: {}", url, e),
                }
            } else {
                warn!("API {} returned status: {}", url, response.status());
            }
        }
        Err(e) => debug!("Failed to fetch from {}: {}", url, e),
    }
    None
}

/// 并发获取公网 IP
/// 
/// 同时向多个 API 发送请求，返回首个成功结果
pub async fn fetch_public_ip() -> Option<PublicIp> {
    let client = match reqwest::Client::builder()
        .user_agent("VLESS-Rust/1.0")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create HTTP client: {}", e);
            return None;
        }
    };

    // 使用通道接收首个成功结果
    let (tx, mut rx) = mpsc::channel::<PublicIp>(1);
    
    // 并发请求所有 API
    let mut handles = Vec::with_capacity(IP_API_ENDPOINTS.len());
    
    for url in IP_API_ENDPOINTS {
        let tx = tx.clone();
        let client = client.clone();
        let url = url.to_string();
        
        let handle = tokio::spawn(async move {
            if let Some(ip) = fetch_from_api(&client, &url).await {
                // 发送成功结果，如果接收端已关闭则忽略
                let _ = tx.send(PublicIp {
                    ip,
                    source: url,
                }).await;
            }
        });
        handles.push(handle);
    }
    
    // 丢弃原始发送端，这样当所有任务完成后通道会关闭
    drop(tx);
    
    // 等待首个成功结果
    let result = rx.recv().await;
    
    // 取消其他任务（通过丢弃 handle 自动取消）
    for handle in handles {
        handle.abort();
    }
    
    result
}

/// 带超时的公网 IP 获取
/// 
/// # Arguments
/// * `timeout_secs` - 超时时间（秒）
pub async fn fetch_public_ip_with_timeout(timeout_secs: u64) -> Option<PublicIp> {
    match tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        fetch_public_ip()
    ).await {
        Ok(result) => result,
        Err(_) => {
            warn!("Public IP fetch timed out after {}s", timeout_secs);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_public_ip() {
        let result = fetch_public_ip_with_timeout(10).await;
        if let Some(ip) = result {
            println!("Public IP: {} (from {})", ip.ip, ip.source);
            assert!(!ip.ip.is_empty());
        }
    }
}
