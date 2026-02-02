use crate::protocol::{VlessRequest, VlessResponse, Command, Address};
use crate::stats::SharedStats;
use crate::http::{is_http_request, parse_http_request, handle_http_request};
use crate::ws::{self, SharedWsManager};
use crate::config::MonitoringConfig;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// RAII guard for connection counting
struct ConnectionGuard {
    stats: SharedStats,
    released: Arc<AtomicBool>,
}

impl ConnectionGuard {
    async fn new(stats: SharedStats) -> Self {
        stats.lock().await.increment_connections();
        Self {
            stats,
            released: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        if !self.released.load(Ordering::SeqCst) {
            let stats = self.stats.clone();
            tokio::spawn(async move {
                stats.lock().await.decrement_connections();
            });
        }
    }
}

/// VLESS服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub users: HashSet<Uuid>,
}

impl ServerConfig {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            users: HashSet::new(),
        }
    }

    pub fn add_user(&mut self, uuid: Uuid) {
        self.users.insert(uuid);
    }
}

/// VLESS服务器
pub struct VlessServer {
    config: Arc<ServerConfig>,
    stats: SharedStats,
    ws_manager: SharedWsManager,
    monitoring_config: MonitoringConfig,
}

impl VlessServer {
    pub fn new(config: ServerConfig, stats: SharedStats, ws_manager: SharedWsManager, monitoring_config: MonitoringConfig) -> Self {
        Self {
            config: Arc::new(config),
            stats,
            ws_manager,
            monitoring_config,
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
                    let stats = Arc::clone(&self.stats);
                    let ws_manager = Arc::clone(&self.ws_manager);
                    let monitoring_config = self.monitoring_config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, addr, config, stats, ws_manager, monitoring_config).await {
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

    /// 处理客户端连接
    async fn handle_connection(
        mut stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        stats: SharedStats,
        ws_manager: SharedWsManager,
        monitoring_config: MonitoringConfig,
    ) -> Result<()> {
        debug!("New connection from {}", client_addr);

        // 读取请求数据
        let mut header_buf = vec![0u8; 1024];
        let n = stream.read(&mut header_buf).await?;
        if n == 0 {
            return Err(anyhow!("Connection closed by client"));
        }

        let header_bytes = Bytes::from(header_buf[..n].to_vec());

        // 检测HTTP请求
        if is_http_request(&header_bytes) {
            debug!("HTTP request detected from {}", client_addr);
            match parse_http_request(&header_bytes) {
                Ok(request) => {
                    // 检测 WebSocket 升级请求
                    if ws::is_websocket_upgrade(&request) {
                        debug!("WebSocket upgrade request detected from {} to {}", client_addr, request.path);
                        return ws::handle_websocket_connection(
                            stream,
                            ws_manager,
                            stats,
                            client_addr,
                            Some(header_bytes.to_vec())
                        ).await;
                    }

                    let response = handle_http_request(&request, stats.clone(), monitoring_config.clone()).await?;
                    stream.write_all(&response).await?;
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to parse HTTP request from {}: {}", client_addr, e);
                    return Err(e);
                }
            }
        }

        // 解析VLESS请求
        let (request, remaining_data) = VlessRequest::decode(header_bytes)?;

        debug!("Parsed VLESS request: {:?}", request);

        // 验证用户UUID
        if !config.users.contains(&request.uuid) {
            warn!("Invalid UUID from {}: {}", client_addr, request.uuid);
            return Err(anyhow!("Invalid user UUID"));
        }

        info!("Authenticated user {} from {}", request.uuid, client_addr);

        // RAII guard for connection counting
        let _guard = ConnectionGuard::new(stats.clone()).await;

        // 发送响应头 - 使用与请求相同的版本号
        let response = VlessResponse::new_with_version(request.version);
        stream.write_all(&response.encode()).await?;

        // 根据命令类型处理连接
        let result = match request.command {
            Command::Tcp => {
                Self::handle_tcp_proxy(stream, request, remaining_data, stats.clone()).await
            }
            Command::Udp => {
                warn!("UDP command not implemented yet");
                Err(anyhow!("UDP not supported"))
            }
            Command::Mux => {
                warn!("Mux command not implemented yet");
                Err(anyhow!("Mux not supported"))
            }
        };

        result
    }

    /// 处理TCP代理
    async fn handle_tcp_proxy(
        mut client_stream: TcpStream,
        request: VlessRequest,
        initial_data: Bytes,
        stats: SharedStats,
    ) -> Result<()> {
        // 连接到目标服务器
        let target_addr = match &request.address {
            Address::Domain(domain) => {
                let addr_str = format!("{}:{}", domain, request.port);
                let resolved = tokio::net::lookup_host(&addr_str)
                    .await?
                    .next()
                    .ok_or_else(|| anyhow!("Failed to resolve domain: {}", domain))?;
                resolved
            }
            _ => request.address.to_socket_addr(request.port)?,
        };

        debug!("Connecting to target: {}", target_addr);
        let mut target_stream = TcpStream::connect(target_addr).await?;

        // 如果有初始数据，先发送给目标服务器并统计上传流量
        if !initial_data.is_empty() {
            let len = initial_data.len() as u64;
            target_stream.write_all(&initial_data).await?;
            stats.lock().await.add_sent_bytes(len);
        }

        info!("Established proxy connection: {} -> {}", 
               client_stream.peer_addr()?, target_addr);

        // 双向数据转发
        let (mut client_read, mut client_write) = client_stream.split();
        let (mut target_read, mut target_write) = target_stream.split();

        let stats_client_to_target = stats.clone();
        let stats_target_to_client = stats.clone();

        // 等待任一方向的连接关闭
        tokio::select! {
            result = tokio::io::copy(&mut client_read, &mut target_write) => {
                match result {
                    Ok(bytes) => {
                        debug!("Client to target: {} bytes transferred", bytes);
                        stats_client_to_target.lock().await.add_sent_bytes(bytes);
                    }
                    Err(e) => debug!("Client to target error: {}", e),
                }
            }
            result = tokio::io::copy(&mut target_read, &mut client_write) => {
                match result {
                    Ok(bytes) => {
                        debug!("Target to client: {} bytes transferred", bytes);
                        stats_target_to_client.lock().await.add_received_bytes(bytes);
                    }
                    Err(e) => debug!("Target to client error: {}", e),
                }
            }
        }

        debug!("Proxy connection closed");
        Ok(())
    }
}