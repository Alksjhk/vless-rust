use crate::protocol::{VlessRequest, VlessResponse, Command, Address};
use crate::http::is_http_request;
use crate::config::{PerformanceConfig, ProtocolType};
use crate::buffer_pool::BufferPool;
use crate::ws;
use crate::utils::configure_tcp_socket;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Semaphore;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// VLESS服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub users: HashSet<Uuid>,
    pub user_emails: HashMap<Uuid, Option<Arc<str>>>,
    pub protocol: ProtocolType,
    pub ws_path: String,
}

impl ServerConfig {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            users: HashSet::new(),
            user_emails: HashMap::new(),
            protocol: ProtocolType::Tcp,
            ws_path: "/".to_string(),
        }
    }

    pub fn add_user_with_email(&mut self, uuid: Uuid, email: Option<String>) {
        self.users.insert(uuid);
        let email_arc = email.map(|e| Arc::from(e.as_str()));
        self.user_emails.insert(uuid, email_arc);
    }

    /// 获取用户邮箱（返回 Arc<str>，推荐使用）
    pub fn get_user_email_arc(&self, uuid: &Uuid) -> Option<Arc<str>> {
        self.user_emails.get(uuid).and_then(|e| e.clone())
    }
}

/// VLESS服务器
pub struct VlessServer {
    config: Arc<ServerConfig>,
    performance_config: Arc<PerformanceConfig>,
    buffer_pool: BufferPool,
    connection_semaphore: Arc<Semaphore>,
}

impl VlessServer {
    pub fn new(
        config: ServerConfig,
        performance_config: PerformanceConfig,
    ) -> Self {
        let buffer_pool = BufferPool::new(
            performance_config.buffer_size,
            performance_config.buffer_pool_size,
        );

        // 创建连接数限制信号量
        let max_connections = performance_config.max_connections;
        let connection_semaphore = if max_connections == 0 {
            // 无限制：创建一个永远不会耗尽的信号量
            Arc::new(Semaphore::new(usize::MAX))
        } else {
            Arc::new(Semaphore::new(max_connections))
        };

        Self {
            config: Arc::new(config),
            performance_config: Arc::new(performance_config),
            buffer_pool,
            connection_semaphore,
        }
    }

    /// 启动服务器
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.config.bind_addr).await?;
        let protocol_str = match self.config.protocol {
            ProtocolType::Tcp => "TCP",
            ProtocolType::WebSocket => "WebSocket",
        };
        let ws_path_info = if self.config.protocol == ProtocolType::WebSocket {
            format!(" (path: {})", self.config.ws_path)
        } else {
            String::new()
        };

        let max_conn_info = if self.performance_config.max_connections == 0 {
            "unlimited".to_string()
        } else {
            format!("{}", self.performance_config.max_connections)
        };

        info!("VLESS server listening on {} [{}{}]",
              self.config.bind_addr, protocol_str, ws_path_info);
        info!("Maximum connections: {}", max_conn_info);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let config = Arc::clone(&self.config);
                    let performance_config = self.performance_config.clone();
                    let buffer_pool = self.buffer_pool.clone();
                    let semaphore = Arc::clone(&self.connection_semaphore);

                    tokio::spawn(async move {
                        // 尝试获取连接许可
                        let permit = match semaphore.try_acquire() {
                            Ok(p) => p,
                            Err(_) => {
                                warn!("Connection limit reached, rejecting from {}", addr);
                                return;
                            }
                        };

                        // permit 会在任务结束时自动释放
                        let _permit = permit;

                        if let Err(e) = Self::handle_connection(
                            stream,
                            addr,
                            config,
                            performance_config,
                            buffer_pool,
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
        stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: Arc<PerformanceConfig>,
        buffer_pool: BufferPool,
    ) -> Result<()> {
        debug!("New connection from {}", client_addr);

        // 配置TCP socket参数
        configure_tcp_socket(
            &stream,
            performance_config.tcp_recv_buffer,
            performance_config.tcp_send_buffer,
            performance_config.tcp_nodelay,
        )?;

        // 根据协议类型处理
        match config.protocol {
            ProtocolType::Tcp => {
                Self::handle_tcp_connection(
                    stream,
                    client_addr,
                    config,
                    performance_config,
                    buffer_pool,
                ).await
            }
            ProtocolType::WebSocket => {
                Self::handle_ws_connection(
                    stream,
                    client_addr,
                    config,
                    performance_config,
                    buffer_pool,
                ).await
            }
        }
    }

    /// 处理 TCP 连接
    async fn handle_tcp_connection(
        mut stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: Arc<PerformanceConfig>,
        buffer_pool: BufferPool,
    ) -> Result<()> {
        // 读取请求数据
        let mut header_buf = vec![0u8; performance_config.buffer_size.min(4096)];
        let n = stream.read(&mut header_buf).await?;
        if n == 0 {
            return Err(anyhow!("Connection closed by client (addr: {})", client_addr));
        }

        let header_bytes = Bytes::from(header_buf[..n].to_vec());

        // 检测HTTP请求 - 拒绝HTTP请求
        if is_http_request(&header_bytes) {
            warn!("HTTP request detected from {} - rejecting (only VLESS protocol supported)", client_addr);
            return Err(anyhow!("HTTP requests not supported (addr: {})", client_addr));
        }

        // 解析VLESS请求
        let (request, remaining_data) = VlessRequest::decode(header_bytes)?;

        debug!("Parsed VLESS request: {:?}", request);

        // 验证用户UUID
        if !config.users.contains(&request.uuid) {
            warn!("Invalid UUID from {}: {} (not in config)", client_addr, request.uuid);
            return Err(anyhow!("Authentication failed: invalid user UUID (addr: {})", client_addr));
        }

        info!("Authenticated user {} from {}", request.uuid, client_addr);

        // 发送响应头 - 使用与请求相同的版本号
        let response = VlessResponse::new_with_version(request.version);
        stream.write_all(&response.encode()).await?;

        // 根据命令类型处理连接
        let result = match request.command {
            Command::Tcp => {
                let user_email = config.get_user_email_arc(&request.uuid);
                Self::handle_tcp_proxy(stream, request, remaining_data, performance_config, user_email, buffer_pool.clone()).await
            }
            Command::Udp => {
                let user_email = config.get_user_email_arc(&request.uuid);
                Self::handle_udp_proxy(stream, request, remaining_data, performance_config, user_email, buffer_pool).await
            }
            Command::Mux => {
                warn!("Mux command not implemented yet");
                Err(anyhow!("Mux not supported"))
            }
        };

        result
    }

    /// 处理 WebSocket 连接
    async fn handle_ws_connection(
        stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: Arc<PerformanceConfig>,
        buffer_pool: BufferPool,
    ) -> Result<()> {
        // 执行 WebSocket 握手并读取第一个消息
        let (ws_stream, first_data) = ws::handle_ws_upgrade(
            stream,
            &config.ws_path,
            client_addr,
            performance_config.ws_header_buffer_size,
        ).await?;

        // 解析VLESS请求
        let header_bytes = Bytes::from(first_data);
        let (request, remaining_data) = VlessRequest::decode(header_bytes)?;

        debug!("Parsed VLESS request via WebSocket: {:?}", request);

        // 验证用户UUID
        if !config.users.contains(&request.uuid) {
            warn!("Invalid UUID from {}: {} (not in config)", client_addr, request.uuid);
            return Err(anyhow!("Authentication failed: invalid user UUID (addr: {})", client_addr));
        }

        info!("Authenticated user {} via WebSocket from {}", request.uuid, client_addr);

        // 发送 VLESS 响应头（必须发送，否则客户端无法建立连接）
        let response = VlessResponse::new_with_version(request.version);

        use futures_util::{SinkExt, StreamExt};
        use tungstenite::Message;

        // 将响应头封装为 WebSocket 消息发送给客户端
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        if let Err(e) = ws_sender.send(Message::Binary(response.encode().to_vec())).await {
            return Err(anyhow!("Failed to send VLESS response via WebSocket: {}", e));
        }

        debug!("Sent VLESS response via WebSocket to {}", client_addr);

        // 检查是否有剩余数据（第一个消息中的实际负载）
        let has_initial_data = !remaining_data.is_empty();
        if has_initial_data {
            debug!("First WebSocket message contains {} bytes of payload data", remaining_data.len());
        }

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

        debug!("Connecting to target via WebSocket: {}", target_addr);

        // 连接到目标服务器
        let target_stream = TcpStream::connect(target_addr).await?;

        // 配置目标连接的 TCP 参数
        ws::configure_tcp_socket(
            &target_stream,
            performance_config.tcp_recv_buffer,
            performance_config.tcp_send_buffer,
            performance_config.tcp_nodelay,
        )?;

        info!("Established WebSocket proxy connection: {:?} -> {}",
               request.address, target_addr);

        // 分离目标流并转发数据
        let (mut target_read, mut target_write) = target_stream.into_split();

        let buffer_pool_t2c = buffer_pool;

        // 客户端 → 目标（通过 WebSocket）
        let upload_task = tokio::spawn(async move {
            // 首先发送第一个消息中的剩余数据（如果有）
            if has_initial_data && !remaining_data.is_empty() {
                if target_write.write_all(&remaining_data).await.is_err() {
                    return;
                }
            }

            // 继续转发后续的 WebSocket 消息
            loop {
                // 使用 StreamExt::next 读取 WebSocket 消息
                match ws_receiver.next().await {
                    Some(Ok(Message::Binary(data))) => {
                        if target_write.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        if target_write.write_all(text.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    _ => {}
                }
            }
        });

        // 目标 → 客户端（通过 WebSocket）
        let download_task = tokio::spawn(async move {
            let mut buffer = buffer_pool_t2c.acquire();

            loop {
                match target_read.read(&mut buffer[..]).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if ws_sender.send(Message::Binary(buffer[..n].to_vec())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // 等待两个任务完成
        let _ = tokio::join!(upload_task, download_task);

        debug!("WebSocket proxy connection closed");
        Ok(())
    }

    /// 处理TCP代理
    async fn handle_tcp_proxy(
        client_stream: TcpStream,
        request: VlessRequest,
        initial_data: Bytes,
        perf_config: Arc<PerformanceConfig>,
        _user_email: Option<Arc<str>>,
        buffer_pool: BufferPool,
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

        // 配置目标连接的TCP参数
        configure_tcp_socket(
            &target_stream,
            perf_config.tcp_recv_buffer,
            perf_config.tcp_send_buffer,
            perf_config.tcp_nodelay,
        )?;

        // 如果有初始数据，先发送给目标服务器
        let initial_len = initial_data.len();
        if initial_len > 0 {
            target_stream.write_all(&initial_data).await?;
        }

        info!("Established proxy connection: {} -> {}",
               client_stream.peer_addr()?, target_addr);

        // 使用tokio的io::copy_bidirectional实现零拷贝传输
        let (mut client_read, mut client_write) = client_stream.into_split();
        let (mut target_read, mut target_write) = target_stream.into_split();

        let buffer_pool_c2t = buffer_pool.clone();
        let buffer_pool_t2c = buffer_pool;

        // 客户端→目标任务
        let upload_task = tokio::spawn(async move {
            let mut buffer = buffer_pool_c2t.acquire();

            loop {
                match client_read.read(&mut buffer[..]).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if target_write.write_all(&buffer[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // 目标→客户端任务
        let download_task = tokio::spawn(async move {
            let mut buffer = buffer_pool_t2c.acquire();

            loop {
                match target_read.read(&mut buffer[..]).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if client_write.write_all(&buffer[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // 等待两个任务完成
        let _ = tokio::join!(upload_task, download_task);

        debug!("Proxy connection closed");
        Ok(())
    }

    /// 处理UDP代理（UDP over TCP机制）
    async fn handle_udp_proxy(
        client_stream: TcpStream,
        request: VlessRequest,
        _initial_data: Bytes,
        perf_config: Arc<PerformanceConfig>,
        _user_email: Option<Arc<str>>,
        buffer_pool: BufferPool,
    ) -> Result<()> {
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

        info!("Establishing UDP proxy: {:?} -> {}", client_stream.peer_addr(), target_addr);

        // 绑定本地UDP socket（随机端口）
        let udp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
        let local_addr = udp_socket.local_addr()?;
        debug!("UDP socket bound to {}", local_addr);

        let udp_timeout = perf_config.udp_timeout;

        // 分离TCP流
        let (mut client_read, mut client_write) = client_stream.into_split();

        // 任务1：客户端 → 目标（读取TCP数据，发送UDP包）
        let udp_socket_c2t = Arc::clone(&udp_socket);
        let buffer_pool_c2t = buffer_pool.clone();

        let client_to_target = tokio::spawn(async move {
            let mut buffer = buffer_pool_c2t.acquire();

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
                        // 发送UDP包到目标
                        if let Err(e) = udp_socket_c2t.send_to(&buffer[..n], target_addr).await {
                            warn!("Failed to send UDP packet: {}", e);
                            break;
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
        });

        // 任务2：目标 → 客户端（接收UDP包，写入TCP流）
        let udp_socket_t2c = Arc::clone(&udp_socket);
        let buffer_pool_t2c = buffer_pool;

        let target_to_client = tokio::spawn(async move {
            let mut buffer = buffer_pool_t2c.acquire();

            loop {
                // 添加超时检测，防止客户端断开时任务永远阻塞
                let timeout_duration = std::time::Duration::from_secs(udp_timeout);
                let timeout_result = tokio::time::timeout(
                    timeout_duration,
                    udp_socket_t2c.recv_from(&mut buffer)
                ).await;

                match timeout_result {
                    Ok(Ok((n, src))) => {
                        // 只接收来自目标地址的UDP包，忽略其他源
                        if src != target_addr {
                            debug!("Ignoring UDP packet from unexpected source: {}", src);
                            continue;
                        }
                        if client_write.write_all(&buffer[..n]).await.is_err() {
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("Error receiving UDP packet: {}", e);
                        break;
                    }
                    Err(_) => {
                        debug!("UDP receive timeout after {}s of inactivity", udp_timeout);
                        break;
                    }
                }
            }
        });

        // 等待两个任务完成
        let _ = tokio::join!(client_to_target, target_to_client);

        debug!("UDP proxy session closed");
        Ok(())
    }
}
