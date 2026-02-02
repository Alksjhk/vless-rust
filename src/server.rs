use crate::protocol::{VlessRequest, VlessResponse, Command, Address};
use crate::stats::SharedStats;
use crate::http::{is_http_request, parse_http_request, handle_http_request};
use crate::ws::{self, SharedWsManager};
use crate::config::{MonitoringConfig, PerformanceConfig};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// 配置TCP socket选项
async fn configure_tcp_socket(
    stream: &TcpStream,
    _recv_buf: usize,
    _send_buf: usize,
    nodelay: bool,
) -> Result<()> {
    // 设置TCP_NODELAY
    if nodelay {
        stream.set_nodelay(true)?;
    }

    // 注意：socket缓冲区大小通常由系统自动调优
    // 在大多数情况下，系统默认值已经足够好
    // 如果需要手动设置，可以使用socket2库，但会增加复杂度

    Ok(())
}

/// RAII guard for connection counting
struct ConnectionGuard {
    stats: SharedStats,
    uuid: String,
    released: Arc<AtomicBool>,
}

impl ConnectionGuard {
    async fn new(stats: SharedStats, uuid: String, email: Option<String>) -> Self {
        stats.lock().await.increment_connections();
        stats.lock().await.increment_user_connection(&uuid, email);
        Self {
            stats,
            uuid,
            released: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        if !self.released.load(Ordering::SeqCst) {
            let stats = self.stats.clone();
            let uuid = self.uuid.clone();
            tokio::spawn(async move {
                stats.lock().await.decrement_connections();
                stats.lock().await.decrement_user_connection(&uuid);
            });
        }
    }
}

/// VLESS服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub users: HashSet<Uuid>,
    pub user_emails: HashMap<Uuid, Option<String>>,
}

impl ServerConfig {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            users: HashSet::new(),
            user_emails: HashMap::new(),
        }
    }

    pub fn add_user_with_email(&mut self, uuid: Uuid, email: Option<String>) {
        self.users.insert(uuid);
        self.user_emails.insert(uuid, email);
    }

    pub fn get_user_email(&self, uuid: &Uuid) -> Option<String> {
        self.user_emails.get(uuid).and_then(|e| e.clone())
    }
}

/// VLESS服务器
pub struct VlessServer {
    config: Arc<ServerConfig>,
    stats: SharedStats,
    ws_manager: SharedWsManager,
    monitoring_config: MonitoringConfig,
    performance_config: PerformanceConfig,
}

impl VlessServer {
    pub fn new(
        config: ServerConfig,
        stats: SharedStats,
        ws_manager: SharedWsManager,
        monitoring_config: MonitoringConfig,
        performance_config: PerformanceConfig,
    ) -> Self {
        Self {
            config: Arc::new(config),
            stats,
            ws_manager,
            monitoring_config,
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
                    let stats = Arc::clone(&self.stats);
                    let ws_manager = Arc::clone(&self.ws_manager);
                    let monitoring_config = self.monitoring_config.clone();
                    let performance_config = self.performance_config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            stream,
                            addr,
                            config,
                            stats,
                            ws_manager,
                            monitoring_config,
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

    /// 处理客户端连接
    async fn handle_connection(
        mut stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        stats: SharedStats,
        ws_manager: SharedWsManager,
        monitoring_config: MonitoringConfig,
        performance_config: PerformanceConfig,
    ) -> Result<()> {
        debug!("New connection from {}", client_addr);

        // 配置TCP socket参数
        configure_tcp_socket(
            &stream,
            performance_config.tcp_recv_buffer,
            performance_config.tcp_send_buffer,
            performance_config.tcp_nodelay,
        ).await?;

        // 读取请求数据
        let mut header_buf = vec![0u8; performance_config.buffer_size.min(4096)];
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

                    let response = handle_http_request(&request, stats.clone(), monitoring_config.clone(), performance_config.clone()).await?;
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

        let uuid_str = request.uuid.to_string();
        let user_email = config.get_user_email(&request.uuid);

        // RAII guard for connection counting
        let _guard = ConnectionGuard::new(stats.clone(), uuid_str.clone(), user_email).await;

        // 发送响应头 - 使用与请求相同的版本号
        let response = VlessResponse::new_with_version(request.version);
        stream.write_all(&response.encode()).await?;

        // 根据命令类型处理连接
        let result = match request.command {
            Command::Tcp => {
                let user_email = config.get_user_email(&request.uuid);
                Self::handle_tcp_proxy(stream, request, remaining_data, stats.clone(), performance_config, user_email).await
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
        client_stream: TcpStream,
        request: VlessRequest,
        initial_data: Bytes,
        stats: SharedStats,
        perf_config: PerformanceConfig,
        user_email: Option<String>,
    ) -> Result<()> {
        let uuid_str = request.uuid.to_string();
        let email_opt = user_email;

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

        // 配置目标连接的TCP参数
        configure_tcp_socket(
            &target_stream,
            perf_config.tcp_recv_buffer,
            perf_config.tcp_send_buffer,
            perf_config.tcp_nodelay,
        ).await?;

        // 如果有初始数据，先发送给目标服务器并统计上传流量
        let initial_len = initial_data.len();
        if initial_len > 0 {
            target_stream.write_all(&initial_data).await?;
            stats.lock().await.add_upload_bytes(initial_len as u64);
            stats.lock().await.add_user_upload_bytes(&uuid_str, initial_len as u64, email_opt.clone());
        }

        info!("Established proxy connection: {} -> {}",
               client_stream.peer_addr()?, target_addr);

        // 使用tokio的io::copy_bidirectional实现零拷贝传输
        let (mut client_read, mut client_write) = client_stream.into_split();
        let (mut target_read, mut target_write) = target_stream.into_split();

        let stats_c2t = stats.clone();
        let stats_t2c = stats.clone();
        let uuid_c2t = uuid_str.clone();
        let uuid_t2c = uuid_str.clone();
        let email_c2t = email_opt.clone();
        let email_t2c = email_opt;
        let batch_size = perf_config.stats_batch_size as u64;

        // 客户端→目标（上传）任务 - 使用批量统计
        let upload_task = tokio::spawn(async move {
            let mut buffer = vec![0u8; perf_config.buffer_size];
            let mut total = 0u64;
            let mut batch_total = 0u64;

            loop {
                match client_read.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        total += n as u64;
                        batch_total += n as u64;

                        if target_write.write_all(&buffer[..n]).await.is_err() {
                            break;
                        }

                        // 批量更新统计，减少锁竞争
                        if batch_total >= batch_size {
                            stats_c2t.lock().await.add_upload_bytes(batch_total);
                            stats_c2t.lock().await.add_user_upload_bytes(&uuid_c2t, batch_total, email_c2t.clone());
                            batch_total = 0;
                        }
                    }
                    Err(_) => break,
                }
            }

            // 处理剩余的批量统计
            if batch_total > 0 {
                stats_c2t.lock().await.add_upload_bytes(batch_total);
                stats_c2t.lock().await.add_user_upload_bytes(&uuid_c2t, batch_total, email_c2t);
            }

            total
        });

        // 目标→客户端（下载）任务 - 使用批量统计
        let download_task = tokio::spawn(async move {
            let mut buffer = vec![0u8; perf_config.buffer_size];
            let mut total = 0u64;
            let mut batch_total = 0u64;

            loop {
                match target_read.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        total += n as u64;
                        batch_total += n as u64;

                        if client_write.write_all(&buffer[..n]).await.is_err() {
                            break;
                        }

                        // 批量更新统计，减少锁竞争
                        if batch_total >= batch_size {
                            stats_t2c.lock().await.add_download_bytes(batch_total);
                            stats_t2c.lock().await.add_user_download_bytes(&uuid_t2c, batch_total, email_t2c.clone());
                            batch_total = 0;
                        }
                    }
                    Err(_) => break,
                }
            }

            // 处理剩余的批量统计
            if batch_total > 0 {
                stats_t2c.lock().await.add_download_bytes(batch_total);
                stats_t2c.lock().await.add_user_download_bytes(&uuid_t2c, batch_total, email_t2c);
            }

            total
        });

        // 等待两个任务完成
        let _ = tokio::join!(upload_task, download_task);

        debug!("Proxy connection closed");
        Ok(())
    }
}