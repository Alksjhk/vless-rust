//! VLESS 链接生成模块
//!
//! 生成 VLESS 协议链接，支持 TCP 和 WebSocket 两种类型

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Serialize;
use uuid::Uuid;

/// VLESS 链接生成配置
#[derive(Debug, Clone)]
pub struct VlessLinkConfig {
    /// 用户 UUID
    pub uuid: Uuid,
    /// 公网 IP 或域名
    pub host: String,
    /// 端口
    pub port: u16,
    /// WebSocket 路径（如果支持 WebSocket）
    pub ws_path: Option<String>,
    /// 用户标识（email 或其他）
    pub alias: String,
}

/// 单个链接
#[derive(Debug, Clone, Serialize)]
pub struct VlessLink {
    /// 原始 VLESS 链接
    pub vless: String,
    /// Base64 编码的链接
    pub base64: String,
}

/// 生成的链接结果
#[derive(Debug, Clone, Serialize)]
pub struct VlessLinks {
    /// TCP 链接
    pub tcp: VlessLink,
    /// WebSocket 链接（可选）
    pub ws: Option<VlessLink>,
    /// 公网 IP
    pub public_ip: String,
    /// 端口
    pub port: u16,
}

/// 生成 VLESS 链接
///
/// # Arguments
/// * `config` - 链接生成配置
///
/// # Returns
/// * `VlessLinks` - TCP 和 WebSocket 链接
pub fn generate_vless_links(config: &VlessLinkConfig) -> VlessLinks {
    let uuid_str = config.uuid.to_string();
    let alias_encoded = urlencoding::encode(&config.alias);

    // 生成 TCP 链接
    // vless://{uuid}@{host}:{port}?encryption=none&security=none&type=tcp#{alias}
    let tcp_link = format!(
        "vless://{}@{}:{}?encryption=none&security=none&type=tcp#{}",
        uuid_str, config.host, config.port, alias_encoded
    );

    // 生成 WebSocket 链接（如果有 ws_path）
    let ws_link = config.ws_path.as_ref().map(|ws_path| {
        let path_encoded = urlencoding::encode(ws_path);
        format!(
            "vless://{}@{}:{}?encryption=none&security=none&type=ws&path={}#{}",
            uuid_str, config.host, config.port, path_encoded, alias_encoded
        )
    });

    VlessLinks {
        tcp: VlessLink {
            vless: tcp_link.clone(),
            base64: encode_link(&tcp_link),
        },
        ws: ws_link.map(|link| VlessLink {
            vless: link.clone(),
            base64: encode_link(&link),
        }),
        public_ip: config.host.clone(),
        port: config.port,
    }
}

/// Base64 编码链接
fn encode_link(link: &str) -> String {
    BASE64.encode(link.as_bytes())
}
