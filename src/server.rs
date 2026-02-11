use crate::protocol::{VlessRequest, VlessResponse, Command, Address};
use crate::http::is_http_request;
use crate::config::PerformanceConfig;
use crate::buffer_pool::BufferPool;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use socket2::SockRef;

/// 配置TCP socket选项
fn configure_tcp_socket(
    stream: &TcpStream,
    recv_buf: usize,
    send_buf: usize,
    nodelay: bool,
) -> Result<()> {
    // 设置TCP_NODELAY，降低延迟
    if nodelay {
        stream.set_nodelay(true)?;
    }

    // 尝试设置TCP缓冲区大小
    let socket = SockRef::from(stream);

    if recv_buf > 0 {
        if let Err(e) = socket.set_recv_buffer_size(recv_buf) {
            debug!("Failed to set recv buffer size to {}: {}", recv_buf, e);
        } else {
            debug!("Set recv buffer size to {}", recv_buf);
        }
    }

    if send_buf > 0 {
        if let Err(e) = socket.set_send_buffer_size(send_buf) {
            debug!("Failed to set send buffer size to {}: {}", send_buf, e);
        } else {
            debug!("Set send buffer size to {}", send_buf);
        }
    }

    Ok(())
}

/// VLESS服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub users: HashSet<Uuid>,
    pub user_emails: HashMap<Uuid, Option<Arc<str>>>,
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
    performance_config: PerformanceConfig,
    buffer_pool: BufferPool,
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

        Self {
            config: Arc::new(config),
            performance_config,
            buffer_pool,
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
                    let buffer_pool = self.buffer_pool.clone();
                    tokio::spawn(async move {
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
        mut stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: PerformanceConfig,
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

    /// 处理TCP代理
    async fn handle_tcp_proxy(
        client_stream: TcpStream,
        request: VlessRequest,
        initial_data: Bytes,
        perf_config: PerformanceConfig,
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
        perf_config: PerformanceConfig,
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
