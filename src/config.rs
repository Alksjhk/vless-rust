use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use anyhow::Result;

/// 协议类型
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolType {
    /// TCP 直连模式
    Tcp,
    /// WebSocket 模式
    #[serde(rename = "ws")]
    WebSocket,
}

impl Default for ProtocolType {
    fn default() -> Self {
        ProtocolType::Tcp
    }
}

impl std::fmt::Display for ProtocolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolType::Tcp => write!(f, "tcp"),
            ProtocolType::WebSocket => write!(f, "ws"),
        }
    }
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
    /// UDP会话超时时间（秒），默认30秒
    #[serde(default = "default_udp_timeout")]
    pub udp_timeout: u64,
    /// UDP接收缓冲区大小（字节），默认64KB
    #[serde(default = "default_udp_recv_buffer")]
    pub udp_recv_buffer: usize,
    /// 缓冲区池大小（缓冲区数量），默认 min(32, CPU核心数*4)
    #[serde(default = "default_buffer_pool_size")]
    pub buffer_pool_size: usize,
    /// WebSocket HTTP 头缓冲区大小（字节），默认8KB
    #[serde(default = "default_ws_header_buffer_size")]
    pub ws_header_buffer_size: usize,
    /// 最大并发连接数，0表示无限制，默认1024
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
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
fn default_ws_header_buffer_size() -> usize { 8 * 1024 }  // 8KB
fn default_max_connections() -> usize { 1024 }

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
            ws_header_buffer_size: default_ws_header_buffer_size(),
            max_connections: default_max_connections(),
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
    /// 协议类型：tcp 或 ws，默认 tcp
    #[serde(default)]
    pub protocol: ProtocolType,
    /// WebSocket 路径（仅 ws 模式使用），默认 "/"
    #[serde(default = "default_ws_path")]
    pub ws_path: String,
}

fn default_ws_path() -> String {
    "/".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserConfig {
    pub uuid: String,
    pub email: Option<String>,
}

impl Config {
    /// 从JSON字符串加载配置
    pub fn from_json(json: &str) -> Result<Self> {
        let mut config: Config = serde_json::from_str(json)?;
        config.validate()?;
        Ok(config)
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

    /// 验证配置参数的有效性
    fn validate(&mut self) -> Result<()> {
        // 验证 UDP 超时时间（1秒 ~ 1小时）
        const MIN_UDP_TIMEOUT: u64 = 1;
        const MAX_UDP_TIMEOUT: u64 = 3600;

        if self.performance.udp_timeout < MIN_UDP_TIMEOUT
            || self.performance.udp_timeout > MAX_UDP_TIMEOUT {
            return Err(anyhow::anyhow!(
                "UDP timeout must be between {} and {} seconds",
                MIN_UDP_TIMEOUT, MAX_UDP_TIMEOUT
            ));
        }

        // 验证并限制缓冲区大小（已由 BufferPool 处理，但这里记录警告）
        if self.performance.buffer_size > 16 * 1024 * 1024 {
            tracing::warn!("Large buffer size configured: {} bytes", self.performance.buffer_size);
        }

        // 验证 WebSocket 路径格式
        if !self.server.ws_path.starts_with('/') {
            tracing::warn!("WebSocket path should start with '/', auto-correcting");
            self.server.ws_path = format!("/{}", self.server.ws_path);
        }

        // 验证端口号
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Port cannot be 0"));
        }

        // 验证用户配置
        if self.users.is_empty() {
            return Err(anyhow::anyhow!("At least one user must be configured"));
        }

        // 验证 UUID 格式
        for user in &self.users {
            if uuid::Uuid::parse_str(&user.uuid).is_err() {
                return Err(anyhow::anyhow!("Invalid UUID format for user: {}", user.email.as_deref().unwrap_or("unknown")));
            }
        }

        Ok(())
    }
}
