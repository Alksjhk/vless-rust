use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;
use anyhow::Result;

/// 监控配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitoringConfig {
    #[serde(default = "default_history_duration")]
    pub speed_history_duration: u64,
    #[serde(default = "default_broadcast_interval")]
    pub broadcast_interval: u64,
    #[serde(default = "default_ws_max_connections")]
    pub websocket_max_connections: usize,
    #[serde(default = "default_ws_heartbeat_timeout")]
    pub websocket_heartbeat_timeout: u64,
    #[serde(default = "default_vless_max_connections")]
    pub vless_max_connections: usize,
}

/// 性能优化配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceConfig {
    /// 传输缓冲区大小（字节），默认128KB
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    /// TCP接收缓冲区大小（字节），0表示使用系统默认，默认256KB
    #[serde(default = "default_tcp_recv_buffer")]
    pub tcp_recv_buffer: usize,
    /// TCP发送缓冲区大小（字节），0表示使用系统默认，默认256KB
    #[serde(default = "default_tcp_send_buffer")]
    pub tcp_send_buffer: usize,
    /// 是否启用TCP_NODELAY，默认true
    #[serde(default = "default_tcp_nodelay")]
    pub tcp_nodelay: bool,
    /// 流量统计批量大小（字节数），累积到此数量才更新统计，默认64KB
    #[serde(default = "default_stats_batch_size")]
    pub stats_batch_size: usize,
    /// UDP会话超时时间（秒），默认30秒
    #[serde(default = "default_udp_timeout")]
    pub udp_timeout: u64,
    /// UDP接收缓冲区大小（字节），默认64KB
    #[serde(default = "default_udp_recv_buffer")]
    pub udp_recv_buffer: usize,
}

fn default_history_duration() -> u64 { 60 }
fn default_broadcast_interval() -> u64 { 1 }
fn default_ws_max_connections() -> usize { 300 }
fn default_ws_heartbeat_timeout() -> u64 { 60 }
fn default_vless_max_connections() -> usize { 300 }

// Performance config defaults
fn default_buffer_size() -> usize { 128 * 1024 }  // 128KB
fn default_tcp_recv_buffer() -> usize { 256 * 1024 }  // 256KB
fn default_tcp_send_buffer() -> usize { 256 * 1024 }  // 256KB
fn default_tcp_nodelay() -> bool { true }
fn default_stats_batch_size() -> usize { 64 * 1024 }  // 64KB
fn default_udp_timeout() -> u64 { 30 }
fn default_udp_recv_buffer() -> usize { 64 * 1024 }  // 64KB

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            speed_history_duration: default_history_duration(),
            broadcast_interval: default_broadcast_interval(),
            websocket_max_connections: default_ws_max_connections(),
            websocket_heartbeat_timeout: default_ws_heartbeat_timeout(),
            vless_max_connections: default_vless_max_connections(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            buffer_size: default_buffer_size(),
            tcp_recv_buffer: default_tcp_recv_buffer(),
            tcp_send_buffer: default_tcp_send_buffer(),
            tcp_nodelay: default_tcp_nodelay(),
            stats_batch_size: default_stats_batch_size(),
            udp_timeout: default_udp_timeout(),
            udp_recv_buffer: default_udp_recv_buffer(),
        }
    }
}

/// 服务器配置文件格式
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerSettings,
    pub users: Vec<UserConfig>,
    #[serde(default)]
    pub monitoring: MonitoringConfig,
    #[serde(default)]
    pub performance: PerformanceConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerSettings {
    pub listen: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserConfig {
    pub uuid: String,
    pub email: Option<String>,
}

impl Config {
    /// 从JSON字符串加载配置
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// 转换为JSON字符串
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 获取绑定地址
    pub fn bind_addr(&self) -> Result<SocketAddr> {
        let addr_str = format!("{}:{}", self.server.listen, self.server.port);
        Ok(addr_str.parse()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = Config {
            server: ServerSettings {
                listen: "0.0.0.0".to_string(),
                port: 443,
            },
            users: vec![
                UserConfig {
                    uuid: Uuid::new_v4().to_string(),
                    email: Some("user@example.com".to_string()),
                }
            ],
            monitoring: MonitoringConfig::default(),
            performance: PerformanceConfig::default(),
        };
        let json = config.to_json().unwrap();
        let parsed = Config::from_json(&json).unwrap();

        assert_eq!(config.server.listen, parsed.server.listen);
        assert_eq!(config.server.port, parsed.server.port);
        assert_eq!(config.users.len(), parsed.users.len());
    }

    #[test]
    fn test_udp_config_defaults() {
        let config = PerformanceConfig::default();
        assert_eq!(config.udp_timeout, 30);
        assert_eq!(config.udp_recv_buffer, 64 * 1024);
    }
}