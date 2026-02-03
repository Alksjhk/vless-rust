use crate::protocol::{VlessRequest, VlessResponse, Command, Address};
use crate::stats::SharedStats;
use crate::http::{is_http_request, parse_http_request, handle_http_request};
use crate::ws::{self, SharedWsManager};
use crate::config::{MonitoringConfig, PerformanceConfig};
use crate::tls;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use rustls::ServerConfig as RustlsServerConfig;

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
    tls_config: Option<Arc<RustlsServerConfig>>,
}

impl VlessServer {
    pub fn new(
        config: ServerConfig,
        stats: SharedStats,
        ws_manager: SharedWsManager,
        monitoring_config: MonitoringConfig,
        performance_config: PerformanceConfig,
        tls_config: Option<Arc<RustlsServerConfig>>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            stats,
            ws_manager,
            monitoring_config,
            performance_config,
            tls_config,
        }
    }

    /// 启动服务器
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.config.bind_addr).await?;
        let tls_enabled = self.tls_config.is_some();
        info!("VLESS server listening on {} (TLS: {})", self.config.bind_addr, tls_enabled);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let config = Arc::clone(&self.config);
                    let stats = Arc::clone(&self.stats);
                    let ws_manager = Arc::clone(&self.ws_manager);
                    let monitoring_config = self.monitoring_config.clone();
                    let performance_config = self.performance_config.clone();
                    let tls_config = self.tls_config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            stream,
                            addr,
                            config,
                            stats,
                            ws_manager,
                            monitoring_config,
                            performance_config,
                            tls_config,
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
        tls_config: Option<Arc<RustlsServerConfig>>,
    ) -> Result<()> {
        debug!("New connection from {}", client_addr);

        // 配置TCP socket参数
        configure_tcp_socket(
            &stream,
            performance_config.tcp_recv_buffer,
            performance_config.tcp_send_buffer,
            performance_config.tcp_nodelay,
        ).await?;

        // Peek 首字节检测协议类型
        let mut peek_buf = [0u8; 1];
        let first_byte = match stream.peek(&mut peek_buf).await {
            Ok(n) => {
                if n == 0 {
                    return Err(anyhow!("Connection closed by client"));
                }
                peek_buf[0]
            }
            Err(e) => {
                warn!("Failed to peek connection from {}: {}", client_addr, e);
                return Err(e.into());
            }
        };

        // TLS Handshake (0x16)
        if first_byte == 0x16 {
            debug!("TLS handshake detected from {}", client_addr);
            if let Some(tls_cfg) = tls_config {
                match tls::accept_tls(stream, tls_cfg).await {
                    Ok(tls_stream) => {
                        // TLS 握手成功，处理连接
                        return Self::handle_connection_after_handshake(
                            tls_stream,
                            client_addr,
                            config,
                            stats,
                            ws_manager,
                            monitoring_config,
                            performance_config,
                        ).await;
                    }
                    Err(e) => {
                        warn!("TLS handshake failed from {}: {}", client_addr, e);
                        return Err(e);
                    }
                }
            } else {
                warn!("TLS connection received but TLS is not enabled");
                return Err(anyhow!("TLS not enabled"));
            }
        }

        // 明文连接（HTTP 或 VLESS）
        // 读取初始数据检测协议类型
        let mut initial_buf = vec![0u8; 4096];
        let n = stream.peek(&mut initial_buf).await?;
        if n == 0 {
            return Err(anyhow!("Connection closed by client"));
        }

        // 检测 HTTP 请求
        if is_http_request(&initial_buf[..n]) {
            // 读取完整请求数据
            let n = stream.read(&mut initial_buf).await?;
            let request_data = &initial_buf[..n];

            match parse_http_request(request_data) {
                Ok(request) => {
                    // 检测 WebSocket 升级请求
                    if ws::is_websocket_upgrade(&request) {
                        debug!("WebSocket upgrade request detected from {}", client_addr);
                        // 将所有权转移给 WebSocket 处理函数
                        return ws::handle_websocket_connection(
                            stream,
                            ws_manager,
                            stats,
                            client_addr,
                            Some(request_data.to_vec()),
                        ).await;
                    }

                    let response = handle_http_request(&request, stats.clone(), monitoring_config.clone(), performance_config.clone()).await?;
                    let mut stream = stream;
                    stream.write_all(&response).await?;
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to parse HTTP request from {}: {}", client_addr, e);
                    return Err(e);
                }
            }
        }

        Self::handle_connection_after_handshake(
            stream,
            client_addr,
            config,
            stats,
            ws_manager,
            monitoring_config,
            performance_config,
        ).await
    }

    /// 连接建立后处理（通用逻辑）
    async fn handle_connection_after_handshake<S>(
        mut stream: S,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        stats: SharedStats,
        _ws_manager: SharedWsManager,
        monitoring_config: MonitoringConfig,
        performance_config: PerformanceConfig,
    ) -> Result<()>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
    {
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
                        // 对于 WebSocket，我们需要原始的 TcpStream
                        // 这里暂时不支持 TLS over WebSocket
                        warn!("WebSocket over TLS not yet supported");
                        return Ok(());
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
                let user_email = config.get_user_email(&request.uuid);
                Self::handle_udp_proxy(stream, request, remaining_data, stats.clone(), performance_config, user_email).await
            }
            Command::Mux => {
                warn!("Mux command not implemented yet");
                Err(anyhow!("Mux not supported"))
            }
        };

        result
    }

    /// 处理TCP代理
    async fn handle_tcp_proxy<S>(
        client_stream: S,
        request: VlessRequest,
        initial_data: Bytes,
        stats: SharedStats,
        perf_config: PerformanceConfig,
        user_email: Option<String>,
    ) -> Result<()>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
    {
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

        info!("Established TCP proxy connection");

        // 使用tokio的io::copy_bidirectional实现零拷贝传输
        let (mut client_read, mut client_write) = tokio::io::split(client_stream);
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

        debug!("TCP proxy connection closed");
        Ok(())
    }

    /// 处理UDP代理（UDP over TCP机制）
    async fn handle_udp_proxy<S>(
        client_stream: S,
        request: VlessRequest,
        _initial_data: Bytes,
        stats: SharedStats,
        perf_config: PerformanceConfig,
        user_email: Option<String>,
    ) -> Result<()>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
    {
        let uuid_str = request.uuid.to_string();

        // 解析目标地址
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

        info!("Establishing UDP proxy to {}", target_addr);

        // 绑定本地UDP socket（随机端口）
        let udp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
        let local_addr = udp_socket.local_addr()?;
        debug!("UDP socket bound to {}", local_addr);

        let batch_size = perf_config.stats_batch_size as u64;
        let udp_timeout = perf_config.udp_timeout;
        let udp_recv_buffer = perf_config.udp_recv_buffer;

        // 分离TCP流
        let (mut client_read, mut client_write) = tokio::io::split(client_stream);

        // 任务1：客户端 → 目标（读取TCP数据，发送UDP包）
        let udp_socket_c2t = Arc::clone(&udp_socket);
        let stats_c2t = stats.clone();
        let uuid_c2t = uuid_str.clone();
        let email_c2t = user_email.clone();

        let client_to_target = tokio::spawn(async move {
            let mut buffer = vec![0u8; udp_recv_buffer];
            let mut total = 0u64;
            let mut batch_total = 0u64;

            loop {
                // 超时检测
                let timeout_duration = std::time::Duration::from_secs(udp_timeout);
                let timeout_result = tokio::time::timeout(timeout_duration, client_read.read(&mut buffer)).await;

                match timeout_result {
                    Ok(Ok(0)) => {
                        debug!("Client closed connection");
                        break;
                    }
                    Ok(Ok(n)) => {
                        total += n as u64;
                        batch_total += n as u64;

                        // 发送UDP包到目标
                        if let Err(e) = udp_socket_c2t.send_to(&buffer[..n], target_addr).await {
                            warn!("Failed to send UDP packet: {}", e);
                            break;
                        }

                        // 批量更新统计
                        if batch_total >= batch_size {
                            stats_c2t.lock().await.add_upload_bytes(batch_total);
                            stats_c2t.lock().await.add_user_upload_bytes(&uuid_c2t, batch_total, email_c2t.clone());
                            batch_total = 0;
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("Error reading from client: {}", e);
                        break;
                    }
                    Err(_) => {
                        debug!("UDP session timeout after {}s of inactivity", udp_timeout);
                        break;
                    }
                }
            }

            // 处理剩余的批量统计
            if batch_total > 0 {
                stats_c2t.lock().await.add_upload_bytes(batch_total);
                stats_c2t.lock().await.add_user_upload_bytes(&uuid_c2t, batch_total, email_c2t);
            }

            total
        });

        // 任务2：目标 → 客户端（接收UDP包，写入TCP流）
        let udp_socket_t2c = Arc::clone(&udp_socket);
        let stats_t2c = stats.clone();
        let uuid_t2c = uuid_str.clone();
        let email_t2c = user_email;

        let target_to_client = tokio::spawn(async move {
            let mut buffer = vec![0u8; udp_recv_buffer];
            let mut total = 0u64;
            let mut batch_total = 0u64;

            loop {
                match udp_socket_t2c.recv_from(&mut buffer).await {
                    Ok((n, src)) => {
                        // 只接收来自目标地址的UDP包
                        if src == target_addr {
                            total += n as u64;
                            batch_total += n as u64;

                            if client_write.write_all(&buffer[..n]).await.is_err() {
                                break;
                            }

                            // 批量更新统计
                            if batch_total >= batch_size {
                                stats_t2c.lock().await.add_download_bytes(batch_total);
                                stats_t2c.lock().await.add_user_download_bytes(&uuid_t2c, batch_total, email_t2c.clone());
                                batch_total = 0;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error receiving UDP packet: {}", e);
                        break;
                    }
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
        let _ = tokio::join!(client_to_target, target_to_client);

        debug!("UDP proxy session closed");
        Ok(())
    }
}
