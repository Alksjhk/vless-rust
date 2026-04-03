//! VLESS 服务器核心模块
//!
//! 负责服务器启动和连接调度，具体协议处理委托给子模块

use crate::api::{self, ApiConfig};
use crate::config::{PerformanceConfig, ProtocolType};
use crate::http::is_http_request;
use crate::tcp;
use crate::ws::{self, is_websocket_upgrade, WsConnectionResult};
use anyhow::Result;
use bytes::Bytes;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tracing::{debug, error, info};
use uuid::Uuid;

/// 用户邮箱映射类型别名
type UserEmails = Arc<HashMap<Uuid, Option<Arc<str>>>>;

/// 协议类型提示
#[derive(Debug, Clone)]
pub enum ProtocolHint {
    /// HTTP 请求（用于 API）
    HttpRequest,
    /// WebSocket 升级请求
    WebSocketUpgrade,
    /// VLESS 协议连接
    VlessConnection,
}

/// 检测连接的协议类型
pub fn detect_protocol(data: &[u8]) -> ProtocolHint {
    if is_http_request(data) {
        if is_websocket_upgrade(data) {
            ProtocolHint::WebSocketUpgrade
        } else {
            ProtocolHint::HttpRequest
        }
    } else {
        ProtocolHint::VlessConnection
    }
}

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
    /// 用户邮箱映射（Arc 共享，避免每次 HTTP 请求深拷贝）
    pub user_emails: UserEmails,
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
            user_emails: Arc::new(HashMap::new()),
            public_ip,
            port,
        }
    }

    /// 添加用户（带邮箱）
    pub fn add_user_with_email(&mut self, uuid: Uuid, email: Option<String>) {
        self.users.insert(uuid);
        let email_arc = email.map(|e| Arc::from(e.as_str()));
        Arc::make_mut(&mut self.user_emails).insert(uuid, email_arc);
    }
}

/// VLESS 服务器
pub struct VlessServer {
    config: Arc<ServerConfig>,
    performance_config: PerformanceConfig,
    shutdown: Option<tokio::sync::broadcast::Sender<()>>,
}

impl VlessServer {
    /// 创建新的服务器实例
    pub fn new(config: ServerConfig, performance_config: PerformanceConfig) -> Self {
        Self {
            config: Arc::new(config),
            performance_config,
            shutdown: None,
        }
    }

    /// 设置关闭信号通道
    pub fn with_shutdown(mut self, shutdown: tokio::sync::broadcast::Sender<()>) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    /// 启动服务器
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.config.bind_addr).await?;
        info!("VLESS server listening on {}", self.config.bind_addr);

        // 如果有关闭信号，监听它
        let mut shutdown_rx = self.shutdown.as_ref().map(|s| s.subscribe());

        loop {
            // 使用 tokio::select! 来监听关闭信号
            let accept_result = if let Some(ref mut rx) = shutdown_rx {
                tokio::select! {
                    result = listener.accept() => Some(result),
                    _ = rx.recv() => {
                        info!("Server shutdown signal received, stopping accept loop");
                        break;
                    }
                }
            } else {
                Some(listener.accept().await)
            };

            match accept_result {
                Some(Ok((stream, addr))) => {
                    let config = Arc::clone(&self.config);
                    let performance_config = self.performance_config.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(stream, addr, config, performance_config).await
                        {
                            error!("Error handling connection from {}: {}", addr, e);
                        }
                    });
                }
                Some(Err(e)) => {
                    error!("Failed to accept connection: {}", e);
                }
                None => break, // shutdown signal received
            }
        }

        info!("Server stopped accepting new connections");
        Ok(())
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
                Self::handle_ws_connection(stream, client_addr, config, performance_config).await
            }
            ProtocolType::Tcp => {
                Self::handle_tcp_connection(stream, client_addr, config, performance_config).await
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
        let mut peek_buf = [0u8; 1024];
        let n = stream.peek(&mut peek_buf).await?;
        if n == 0 {
            return Err(anyhow::anyhow!(
                "Connection closed by client (addr: {})",
                client_addr
            ));
        }

        match detect_protocol(&peek_buf[..n]) {
            ProtocolHint::HttpRequest | ProtocolHint::WebSocketUpgrade => {
                debug!("HTTP request detected from {}", client_addr);
                let mut stream = stream;
                // 使用栈上固定缓冲区，避免堆分配（HTTP 头通常远小于 8KB）
                let mut http_buf = [0u8; 8192];
                let read_n = stream.read(&mut http_buf).await?;
                let header_bytes = Bytes::copy_from_slice(&http_buf[..read_n]);
                Self::handle_http_request(stream, header_bytes, &config).await
            }
            ProtocolHint::VlessConnection => {
                // 通过 Arc 共享，避免每连接深拷贝整个 HashSet/HashMap
                let config_ref = Arc::clone(&config);

                tcp::handle_tcp_connection(
                    stream,
                    client_addr,
                    performance_config,
                    &config_ref.users,
                    |uuid| {
                        let config_ref = Arc::clone(&config_ref);
                        async move { config_ref.user_emails.get(&uuid).and_then(|e| e.clone()) }
                    },
                )
                .await
            }
        }
    }

    /// 处理 WebSocket 协议连接
    async fn handle_ws_connection(
        stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: PerformanceConfig,
    ) -> Result<()> {
        let result =
            ws::detect_ws_connection(stream, &config.ws_path, performance_config.clone()).await?;

        match result {
            WsConnectionResult::UpgradeSuccess(ws_stream, first_message) => {
                // 通过 Arc 共享，避免每连接深拷贝
                let config_ref = Arc::clone(&config);

                ws::handle_ws_vless(
                    ws_stream,
                    first_message,
                    &config_ref.users,
                    |uuid| config_ref.user_emails.get(uuid).and_then(|e| e.clone()),
                    performance_config,
                    client_addr,
                )
                .await
            }
            WsConnectionResult::HttpRequest(stream, data) => {
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
            public_ip: config
                .public_ip
                .clone()
                .unwrap_or_else(|| config.bind_addr.ip().to_string()),
            port: config.port,
            protocol: config.protocol,
            ws_path: if config.protocol == ProtocolType::WebSocket {
                Some(config.ws_path.clone())
            } else {
                None
            },
            // Arc::clone 只增加引用计数，不复制 HashMap 数据
            user_emails: Arc::clone(&config.user_emails),
        };

        api::handle_http_request(stream, &data, &api_config).await
    }
}
