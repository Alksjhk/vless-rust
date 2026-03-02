//! VLESS 服务器核心模块
//!
//! 负责服务器启动和连接调度，具体协议处理委托给子模块

use crate::api::{self, ApiConfig};
use crate::config::{PerformanceConfig, ProtocolType};
use crate::http::is_http_request;
use crate::tcp;
use crate::ws::{self, WsConnectionResult};
use anyhow::Result;
use bytes::Bytes;
use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tracing::{info, error, debug};
use uuid::Uuid;

/// VLESS 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 绑定地址
    pub bind_addr: SocketAddr,
    /// 协议类型
    pub protocol: ProtocolType,
    /// WebSocket 路径
    pub ws_path: String,
    /// 有效用户 UUID 集合
    pub users: HashSet<Uuid>,
    /// 用户邮箱映射
    pub user_emails: HashMap<Uuid, Option<Arc<str>>>,
    /// 公网 IP（用于生成 VLESS 链接）
    pub public_ip: Option<String>,
    /// 服务端口
    pub port: u16,
}

impl ServerConfig {
    /// 创建新的服务器配置
    pub fn new(
        bind_addr: SocketAddr,
        protocol: ProtocolType,
        ws_path: String,
        public_ip: Option<String>,
        port: u16,
    ) -> Self {
        Self {
            bind_addr,
            protocol,
            ws_path,
            users: HashSet::new(),
            user_emails: HashMap::new(),
            public_ip,
            port,
        }
    }

    /// 添加用户（带邮箱）
    pub fn add_user_with_email(&mut self, uuid: Uuid, email: Option<String>) {
        self.users.insert(uuid);
        let email_arc = email.map(|e| Arc::from(e.as_str()));
        self.user_emails.insert(uuid, email_arc);
    }

}

/// VLESS 服务器
pub struct VlessServer {
    config: Arc<ServerConfig>,
    performance_config: PerformanceConfig,
}

impl VlessServer {
    /// 创建新的服务器实例
    pub fn new(
        config: ServerConfig,
        performance_config: PerformanceConfig,
    ) -> Self {
        Self {
            config: Arc::new(config),
            performance_config,
        }
    }

    /// 启动服务器
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.config.bind_addr).await?;
        info!("VLESS server listening on {}", self.config.bind_addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let config = Arc::clone(&self.config);
                    let performance_config = self.performance_config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            stream,
                            addr,
                            config,
                            performance_config,
                        ).await {
                            error!("Error handling connection from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// 处理客户端连接（调度器）
    async fn handle_connection(
        stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: PerformanceConfig,
    ) -> Result<()> {
        debug!("New connection from {}", client_addr);

        // 根据协议类型分发处理
        match config.protocol {
            ProtocolType::WebSocket => {
                Self::handle_ws_connection(
                    stream,
                    client_addr,
                    config,
                    performance_config,
                ).await
            }
            ProtocolType::Tcp => {
                Self::handle_tcp_connection(
                    stream,
                    client_addr,
                    config,
                    performance_config,
                ).await
            }
        }
    }

    /// 处理 TCP 协议连接
    async fn handle_tcp_connection(
        stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: PerformanceConfig,
    ) -> Result<()> {
        // TCP 模式下，需要先检测是否是 HTTP 请求
        // 使用 peek 来检测，不消费数据
        let mut peek_buf = [0u8; 1024];
        let n = stream.peek(&mut peek_buf).await?;
        if n == 0 {
            return Err(anyhow::anyhow!("Connection closed by client (addr: {})", client_addr));
        }

        // 如果是 HTTP 请求，交给 API 处理
        if is_http_request(&peek_buf[..n]) {
            debug!("HTTP request detected from {}", client_addr);
            // 需要先从流中实际读取数据（peek 不会消费数据）
            let mut stream = stream;
            let mut http_buf = vec![0u8; performance_config.ws_header_buffer_size];
            let read_n = stream.read(&mut http_buf).await?;
            let header_bytes = Bytes::copy_from_slice(&http_buf[..read_n]);
            return Self::handle_http_request(stream, header_bytes, &config).await;
        }

        // 委托给 TCP 模块处理
        let users = config.users.clone();
        let user_emails = config.user_emails.clone();

        tcp::handle_tcp_connection(
            stream,
            client_addr,
            performance_config,
            &users,
            |uuid| {
                let user_emails = user_emails.clone();
                async move {
                    user_emails.get(&uuid).and_then(|e| e.clone())
                }
            },
        ).await
    }

    /// 处理 WebSocket 协议连接
    async fn handle_ws_connection(
        stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: PerformanceConfig,
    ) -> Result<()> {
        // 检测连接类型
        let result = ws::detect_ws_connection(
            stream,
            &config.ws_path,
            performance_config.clone(),
        ).await?;

        match result {
            WsConnectionResult::UpgradeSuccess(ws_stream, first_message) => {
                // 处理 WebSocket VLESS 连接
                let users = config.users.clone();
                let user_emails = config.user_emails.clone();

                ws::handle_ws_vless(
                    ws_stream,
                    first_message,
                    &users,
                    |uuid| user_emails.get(&uuid).and_then(|e| e.clone()),
                    performance_config,
                    client_addr,
                ).await
            }
            WsConnectionResult::HttpRequest(stream, data) => {
                // 处理 HTTP 请求
                Self::handle_http_request(stream, data, &config).await
            }
        }
    }

    /// 处理 HTTP 请求（API 和信息页面）
    async fn handle_http_request(
        stream: TcpStream,
        data: Bytes,
        config: &ServerConfig,
    ) -> Result<()> {
        let api_config = ApiConfig {
            public_ip: config.public_ip.clone().unwrap_or_else(|| {
                config.bind_addr.ip().to_string()
            }),
            port: config.port,
            protocol: config.protocol,
            ws_path: if config.protocol == ProtocolType::WebSocket {
                Some(config.ws_path.clone())
            } else {
                None
            },
            user_emails: config.user_emails.clone(),
        };

        api::handle_http_request(stream, &data, &api_config).await
    }
}
