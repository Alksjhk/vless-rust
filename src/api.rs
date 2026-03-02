//! HTTP API 处理模块
//!
//! 处理 HTTP 请求，提供 VLESS 链接生成和服务器信息展示

use crate::config::ProtocolType;
use crate::http::{parse_http_request, build_html_response, build_json_response, build_404_response, build_400_response};
use crate::vless_link::{generate_vless_links, VlessLinkConfig};
use crate::version::VERSION_INFO;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{info, debug};
use uuid::Uuid;

/// API 处理配置
pub struct ApiConfig {
    /// 公网 IP 或域名
    pub public_ip: String,
    /// 服务端口
    pub port: u16,
    /// 协议类型
    pub protocol: ProtocolType,
    /// WebSocket 路径（仅 WebSocket 协议）
    pub ws_path: Option<String>,
    /// 用户邮箱映射
    pub user_emails: HashMap<Uuid, Option<Arc<str>>>,
}

/// 处理 HTTP 请求
///
/// # Arguments
/// * `stream` - TCP 连接流
/// * `data` - HTTP 请求数据
/// * `config` - API 配置
pub async fn handle_http_request(
    mut stream: TcpStream,
    data: &[u8],
    config: &ApiConfig,
) -> Result<()> {
    let query = match parse_http_request(data) {
        Some(q) => q,
        None => {
            let response = build_400_response("Invalid HTTP request");
            stream.write_all(&response).await?;
            return Ok(());
        }
    };

    // 只处理根路径
    if query.path != "/" {
        let response = build_404_response();
        stream.write_all(&response).await?;
        return Ok(());
    }

    // 检查是否有 email 参数
    if let Some(email) = query.params.get("email") {
        handle_link_request(&mut stream, email, config).await?;
    } else {
        handle_info_page(&mut stream, config).await?;
    }

    Ok(())
}

/// 处理链接生成请求
async fn handle_link_request(
    stream: &mut TcpStream,
    email: &str,
    config: &ApiConfig,
) -> Result<()> {
    // 根据 email 查找用户
    let user_entry = config.user_emails.iter()
        .find(|(_, e)| e.as_deref() == Some(email));

    match user_entry {
        Some((uuid, _)) => {
            // 生成 VLESS 链接
            let link_config = VlessLinkConfig {
                uuid: *uuid,
                host: config.public_ip.clone(),
                port: config.port,
                ws_path: config.ws_path.clone(),
                alias: email.to_string(),
            };

            let links = generate_vless_links(&link_config);

            // 根据协议类型返回对应的链接
            let response_json = match config.protocol {
                ProtocolType::WebSocket => {
                    if let Some(ws) = links.ws {
                        serde_json::json!({
                            "ws": ws.vless,
                            "ws_b64": ws.base64
                        })
                    } else {
                        serde_json::json!({
                            "error": "WebSocket link not available"
                        })
                    }
                }
                ProtocolType::Tcp => {
                    serde_json::json!({
                        "tcp": links.tcp.vless,
                        "tcp_b64": links.tcp.base64
                    })
                }
            };

            let response = build_json_response(&response_json.to_string());
            stream.write_all(&response).await?;
            info!("Served VLESS links for email: {}", email);
        }
        None => {
            let response_json = serde_json::json!({
                "error": "User not found"
            });
            let response = build_json_response(&response_json.to_string());
            stream.write_all(&response).await?;
        }
    }

    Ok(())
}

/// 处理信息页面请求
async fn handle_info_page(
    stream: &mut TcpStream,
    config: &ApiConfig,
) -> Result<()> {
    let protocol_str = match config.protocol {
        ProtocolType::Tcp => "TCP",
        ProtocolType::WebSocket => "WebSocket",
    };

    let ws_path_info = if config.protocol == ProtocolType::WebSocket {
        format!("<tr><td>WS Path</td><td><code>{}</code></td></tr>",
                config.ws_path.as_deref().unwrap_or("/"))
    } else {
        String::new()
    };

    let html = format!(r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{product_name}</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%); min-height: 100vh; padding: 40px 20px; color: #e0e0e0; }}
        .container {{ max-width: 600px; margin: 0 auto; }}
        .card {{ background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: 16px; padding: 32px; margin-bottom: 24px; backdrop-filter: blur(10px); }}
        h1 {{ font-size: 1.5rem; margin-bottom: 8px; color: #fff; }}
        .version {{ color: #4ecdc4; font-size: 0.875rem; margin-bottom: 24px; }}
        table {{ width: 100%; border-collapse: collapse; }}
        td {{ padding: 12px 0; border-bottom: 1px solid rgba(255,255,255,0.1); }}
        td:first-child {{ color: #888; width: 120px; }}
        td:last-child {{ color: #fff; }}
        code {{ background: rgba(78,205,196,0.2); padding: 4px 8px; border-radius: 4px; font-size: 0.875rem; }}
        .api-section {{ margin-top: 32px; }}
        .api-title {{ font-size: 1rem; color: #fff; margin-bottom: 16px; }}
        .api-box {{ background: rgba(0,0,0,0.3); border-radius: 8px; padding: 16px; }}
        .api-url {{ font-family: monospace; color: #4ecdc4; word-break: break-all; }}
        .api-note {{ font-size: 0.75rem; color: #888; margin-top: 8px; }}
        .footer {{ text-align: center; font-size: 0.75rem; color: #666; margin-top: 24px; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <h1>{product_name}</h1>
            <p class="version">v{version}</p>
            <table>
                <tr><td>Author</td><td>{author}</td></tr>
                <tr><td>IP</td><td><code>{ip}</code></td></tr>
                <tr><td>Port</td><td><code>{port}</code></td></tr>
                <tr><td>Protocol</td><td><code>{protocol}</code></td></tr>
                {ws_path_info}
            </table>
        </div>
        <div class="card api-section">
            <p class="api-title">Get VLESS Link</p>
            <div class="api-box">
                <p class="api-url">http://{ip}:{port}/?email=your_email</p>
            </div>
            <p class="api-note">Replace <code>your_email</code> with the email configured in config.json</p>
        </div>
        <p class="footer">{copyright}</p>
    </div>
</body>
</html>"#,
        product_name = VERSION_INFO.product_name,
        version = VERSION_INFO.version,
        author = VERSION_INFO.author,
        ip = config.public_ip,
        port = config.port,
        protocol = protocol_str,
        ws_path_info = ws_path_info,
        copyright = VERSION_INFO.legal_copyright,
    );

    let response = build_html_response(&html);
    stream.write_all(&response).await?;
    debug!("Served HTML info page");

    Ok(())
}
