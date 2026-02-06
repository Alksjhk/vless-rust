use anyhow::{Result, Context};
use std::time::Duration;
use futures_util::future;

/// 获取外网 IP 地址
/// - 同时请求 3 个可靠的 API
/// - 任一成功即返回
/// - 5秒超时，整体8秒超时
pub async fn get_public_ip() -> Result<String> {
    get_public_ip_with_diagnostic().await
}

/// 获取外网 IP 地址（带详细诊断信息）
pub async fn get_public_ip_with_diagnostic() -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("Failed to create HTTP client")?;

    // 使用 3 个可靠的 IPv4 专用 API
    let apis = [
        "https://ipv4.icanhazip.com",    // 强制 IPv4
        "https://checkip.amazonaws.com", // AWS IPv4
        "https://v4.ident.me",           // 强制 IPv4
    ];

    // 并发请求所有 API
    let tasks: Vec<_> = apis
        .iter()
        .map(|url| fetch_ip_from_api(&client, url))
        .collect();

    // 等待第一个成功响应或全部失败
    let results = tokio::time::timeout(Duration::from_secs(8), future::join_all(tasks)).await
        .context("IP detection timeout after 8 seconds")?;

    // 查找第一个成功的结果
    let mut failed_apis = Vec::new();

    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(ip) => {
                // 返回第一个成功的 IP
                return Ok(ip.clone());
            }
            Err(e) => {
                failed_apis.push(format!("{}: {}", apis[i], e));
            }
        }
    }

    // 所有 API 都失败
    Err(anyhow::anyhow!(
        "All 3 API attempts failed:\n  - {}\n\nPlease check:\n  1. Network connectivity\n  2. DNS resolution\n  3. Firewall rules\n  4. Add 'public_ip' to config.json to skip detection",
        failed_apis.join("\n  - ")
    ))
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

/// 从单个 API 获取 IP
async fn fetch_ip_from_api(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to {}", url))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP status: {}", response.status()));
    }

    let body = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from {}", url))?;

    let ip = body.trim().to_string();

    // 验证 IPv4 格式（只接受 IPv4）
    validate_ipv4(&ip)?;

    Ok(ip)
}

/// 验证 IPv4 地址格式
fn validate_ipv4(ip: &str) -> Result<()> {
    ip.parse::<std::net::Ipv4Addr>()
        .map(|_| ())
        .with_context(|| format!("Invalid IPv4 address: {}", ip))
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
