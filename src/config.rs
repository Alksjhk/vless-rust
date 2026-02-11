use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use anyhow::Result;

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
    /// UDP会话超时时间（秒），默认30秒
    #[serde(default = "default_udp_timeout")]
    pub udp_timeout: u64,
    /// UDP接收缓冲区大小（字节），默认64KB
    #[serde(default = "default_udp_recv_buffer")]
    pub udp_recv_buffer: usize,
    /// 缓冲区池大小（缓冲区数量），默认 min(32, CPU核心数*4)
    #[serde(default = "default_buffer_pool_size")]
    pub buffer_pool_size: usize,
}

fn default_buffer_size() -> usize { 128 * 1024 }  // 128KB
fn default_tcp_recv_buffer() -> usize { 256 * 1024 }  // 256KB
fn default_tcp_send_buffer() -> usize { 256 * 1024 }  // 256KB
fn default_tcp_nodelay() -> bool { true }
fn default_udp_timeout() -> u64 { 30 }
fn default_udp_recv_buffer() -> usize { 64 * 1024 }  // 64KB
fn default_buffer_pool_size() -> usize {
    std::cmp::min(32, std::thread::available_parallelism().map_or(4, |n| n.get() * 4))
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            buffer_size: default_buffer_size(),
            tcp_recv_buffer: default_tcp_recv_buffer(),
            tcp_send_buffer: default_tcp_send_buffer(),
            tcp_nodelay: default_tcp_nodelay(),
            udp_timeout: default_udp_timeout(),
            udp_recv_buffer: default_udp_recv_buffer(),
            buffer_pool_size: default_buffer_pool_size(),
        }
    }
}

/// 服务器配置文件格式
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerSettings,
    pub users: Vec<UserConfig>,
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
