use anyhow::{Result, Context};
use std::time::Duration;
use futures_util::future;

/// 获取外网 IP 地址
/// - 并发请求多个 API，任一成功即返回
/// - 5秒超时，整体10秒超时
/// - 所有 API 失败返回错误
pub async fn get_public_ip() -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("Failed to create HTTP client")?;

    // API 列表（按优先级排序）
    let apis = [
        "https://api.ipify.org",
        "https://icanhazip.com",
        "https://checkip.amazonaws.com",
        "https://ifconfig.me",
        "https://ipecho.net/plain",
    ];

    // 并发请求所有 API
    let tasks: Vec<_> = apis
        .iter()
        .map(|url| fetch_ip_from_api(&client, url))
        .collect();

    // 使用 tokio::select! 等待第一个成功响应或全部失败
    let results = tokio::time::timeout(Duration::from_secs(10), future::join_all(tasks)).await
        .context("IP detection timeout after 10 seconds")?;

    // 查找第一个成功的结果
    if let Some(ip) = results.iter().find_map(|r| r.as_ref().ok()) {
        return Ok(ip.clone());
    }

    // 收集第一个错误原因以便调试
    let first_error = results.iter()
        .find_map(|r| r.as_ref().err())
        .map(|e| e.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Err(anyhow::anyhow!("all attempts failed. First error: {}", first_error))
}

/// 生成 VLESS 协议链接
/// - 格式: vless://uuid@server:port?params#alias
/// - 别名: IP+邮箱（URL编码）
pub fn generate_vless_url(
    uuid: &str,
    server_ip: &str,
    port: u16,
    email: Option<&str>,
) -> String {
    let params = [
        ("encryption", "none"),
        ("security", "none"),
        ("type", "tcp"),
    ];

    let params_str = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    // 生成别名后缀
    let suffix = if let Some(email) = email {
        format!("{}+{}", server_ip, email)
    } else {
        // 无邮箱时使用 UUID 前 8 位
        format!("{}+{}", server_ip, &uuid[..8.min(uuid.len())])
    };

    let alias = url_encode(&suffix);

    format!(
        "vless://{}@{}:{}?{}#{}",
        uuid, server_ip, port, params_str, alias
    )
}

/// 从单个 API 获取 IP（辅助函数）
async fn fetch_ip_from_api(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch from {}", url))?;

    let body = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from {}", url))?;

    let ip = body.trim().to_string();

    // 验证 IP 格式
    validate_ip(&ip)?;

    Ok(ip)
}

/// 验证 IP 地址格式（支持 IPv4 和 IPv6）
fn validate_ip(ip: &str) -> Result<()> {
    // 使用标准库验证 IP 地址格式
    ip.parse::<std::net::IpAddr>()
        .map(|_| ())
        .with_context(|| format!("Invalid IP address: {}", ip))
}

/// URL 编码（辅助函数）
fn url_encode(input: &str) -> String {
    input
        .bytes()
        .flat_map(|b| {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                vec![b]
            } else {
                // 使用小写十六进制（符合 RFC 3986）
                let encoded = format!("%{:02x}", b);
                encoded.into_bytes()
            }
        })
        .map(|b| b as char)
        .collect()
}
