use crate::protocol::{VlessRequest, VlessResponse, Command, Address};
use crate::http::is_http_request;
use crate::config::{PerformanceConfig, ProtocolType};
use crate::ws;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use futures_util::StreamExt;
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use socket2::SockRef;

/// 配置TCP socket选项（公开供 ws 模块使用）
pub fn configure_tcp_socket(
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
    pub protocol: ProtocolType,
    pub ws_path: String,
    pub users: HashSet<Uuid>,
    pub user_emails: HashMap<Uuid, Option<Arc<str>>>,
}

impl ServerConfig {
    pub fn new(bind_addr: SocketAddr, protocol: ProtocolType, ws_path: String) -> Self {
        Self {
            bind_addr,
            protocol,
            ws_path,
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
}

impl VlessServer {
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

    /// 处理客户端连接
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
        mut stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: PerformanceConfig,
    ) -> Result<()> {
        // 配置TCP socket参数
        configure_tcp_socket(
            &stream,
            performance_config.tcp_recv_buffer,
            performance_config.tcp_send_buffer,
            performance_config.tcp_nodelay,
        )?;

        // 读取请求数据（使用栈上小缓冲区避免堆分配）
        // VLESS 头部通常小于 256 字节，使用 1KB 栈缓冲区足够
        let mut small_buf = [0u8; 1024];
        let n = stream.read(&mut small_buf).await?;
        if n == 0 {
            return Err(anyhow!("Connection closed by client (addr: {})", client_addr));
        }

        // 从栈缓冲区创建 Bytes（一次拷贝，避免堆分配）
        let header_bytes = Bytes::copy_from_slice(&small_buf[..n]);

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
                Self::handle_tcp_proxy(stream, request, remaining_data, performance_config, user_email).await
            }
            Command::Udp => {
                let user_email = config.get_user_email_arc(&request.uuid);
                Self::handle_udp_proxy(stream, request, remaining_data, performance_config, user_email).await
            }
            Command::Mux => {
                warn!("Mux command not implemented yet");
                Err(anyhow!("Mux not supported"))
            }
        };

        result
    }

    /// 处理 WebSocket 协议连接
    async fn handle_ws_connection(
        stream: TcpStream,
        client_addr: SocketAddr,
        config: Arc<ServerConfig>,
        performance_config: PerformanceConfig,
    ) -> Result<()> {
        // 配置TCP socket参数
        configure_tcp_socket(
            &stream,
            performance_config.tcp_recv_buffer,
            performance_config.tcp_send_buffer,
            performance_config.tcp_nodelay,
        )?;

        // 处理 WebSocket 升级
        let (ws_stream, first_message) = ws::handle_ws_upgrade(
            stream,
            &config.ws_path,
            performance_config.ws_header_buffer_size,
        ).await?;

        // 解析 VLESS 请求
        let (request, remaining_data) = VlessRequest::decode(first_message)?;

        debug!("Parsed VLESS request from WS: {:?}", request);

        // 验证用户UUID
        if !config.users.contains(&request.uuid) {
            warn!("Invalid UUID from {}: {} (not in config)", client_addr, request.uuid);
            return Err(anyhow!("Authentication failed: invalid user UUID (addr: {})", client_addr));
        }

        info!("Authenticated user {} from {} (WS)", request.uuid, client_addr);

        // 创建 VLESS 响应并发送
        let response = VlessResponse::new_with_version(request.version);

        // 需要通过 WebSocket 发送响应
        let (mut ws_sender, ws_receiver) = ws_stream.split();

        // 发送响应
        ws::send_ws_response(&mut ws_sender, &response).await?;

        // 获取用户邮箱（在移动 request 之前）
        let user_email = config.get_user_email_arc(&request.uuid);

        // 根据命令类型分发处理
        match request.command {
            Command::Tcp => {
                // 处理 TCP 代理（直接使用 remaining_data，无需复制）
                ws::handle_ws_proxy(
                    ws_sender,
                    ws_receiver,
                    request,
                    remaining_data,
                    performance_config,
                    user_email,
                    client_addr,
                ).await
            }
            Command::Udp => {
                // 处理 UDP over WebSocket
                warn!("UDP over WebSocket not fully implemented, falling back to TCP");
                // UDP over WebSocket 需要更复杂的实现：每个 UDP 包需要独立处理
                // 目前简化为返回错误
                Err(anyhow!("UDP over WebSocket not supported yet"))
            }
            Command::Mux => {
                warn!("Mux command not supported");
                Err(anyhow!("Mux not supported"))
            }
        }
    }

    /// 处理TCP代理
    async fn handle_tcp_proxy(
        client_stream: TcpStream,
        request: VlessRequest,
        initial_data: Bytes,
        perf_config: PerformanceConfig,
        _user_email: Option<Arc<str>>,
    ) -> Result<()> {
        // 连接到目标服务器
        let target_addr = match &request.address {
            Address::Domain(domain) => {
                let domain_str = std::str::from_utf8(domain).map_err(|_| anyhow!("Invalid domain encoding"))?;
                let addr_str = format!("{}:{}", domain_str, request.port);
                let resolved = tokio::net::lookup_host(&addr_str)
                    .await?
                    .next()
                    .ok_or_else(|| anyhow!("Failed to resolve domain: {}", domain_str))?;
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

        // 使用 tokio::io::copy_bidirectional 实现零拷贝双向转发
        // 这是 Tokio 提供的高度优化实现，比手动循环更高效
        let (mut client_read, mut client_write) = client_stream.into_split();
        let (mut target_read, mut target_write) = target_stream.into_split();

        // 使用 tokio::io::copy 进行单向拷贝（比手动 read/write 循环更高效）
        let client_to_target = tokio::spawn(async move {
            let _ = tokio::io::copy(&mut client_read, &mut target_write).await;
        });

        let target_to_client = tokio::spawn(async move {
            let _ = tokio::io::copy(&mut target_read, &mut client_write).await;
        });

        // 等待两个任务完成
        let _ = tokio::join!(client_to_target, target_to_client);

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
    ) -> Result<()> {
        // 解析目标地址
        let target_addr = match &request.address {
            Address::Domain(domain) => {
                let domain_str = std::str::from_utf8(domain).map_err(|_| anyhow!("Invalid domain encoding"))?;
                let addr_str = format!("{}:{}", domain_str, request.port);
                let resolved = tokio::net::lookup_host(&addr_str)
                    .await?
                    .next()
                    .ok_or_else(|| anyhow!("Failed to resolve domain: {}", domain_str))?;
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
        // UDP 包通常较小（<1500字节），使用栈缓冲区避免堆分配
        let udp_socket_c2t = Arc::clone(&udp_socket);

        let client_to_target = tokio::spawn(async move {
            // 使用栈缓冲区（增大到 4KB 以支持 jumbo frames 或非标准 MTU）
            let mut buffer = [0u8; 4096];
            let timeout_duration = std::time::Duration::from_secs(udp_timeout);

            loop {
                // 超时检测
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

        let target_to_client = tokio::spawn(async move {
            // 使用栈缓冲区（增大到 4KB 以支持 jumbo frames 或非标准 MTU）
            let mut buffer = [0u8; 4096];
            let timeout_duration = std::time::Duration::from_secs(udp_timeout);

            loop {
                // 添加超时检测，防止客户端断开时任务永远阻塞
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
