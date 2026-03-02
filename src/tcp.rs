//! TCP 协议处理模块
//!
//! 处理原始 TCP 连接上的 VLESS 协议请求

use crate::config::PerformanceConfig;
use crate::protocol::{VlessRequest, VlessResponse, Command, Address};
use crate::socket::configure_tcp_socket;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tracing::{info, warn, debug};

/// 处理 TCP 协议连接
///
/// # Arguments
/// * `stream` - TCP 连接流
/// * `client_addr` - 客户端地址
/// * `config` - 服务器配置引用
/// * `performance_config` - 性能配置
/// * `users` - 有效用户 UUID 集合
/// * `authenticate` - 认证函数，返回用户邮箱
pub async fn handle_tcp_connection<F, Fut>(
    mut stream: TcpStream,
    client_addr: SocketAddr,
    performance_config: PerformanceConfig,
    users: &std::collections::HashSet<uuid::Uuid>,
    authenticate: F,
) -> Result<()>
where
    F: FnOnce(uuid::Uuid) -> Fut,
    Fut: std::future::Future<Output = Option<Arc<str>>>,
{
    // 配置 TCP socket 参数
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

    // 解析 VLESS 请求
    let (request, remaining_data) = VlessRequest::decode(header_bytes)?;

    debug!("Parsed VLESS request: {:?}", request);

    // 验证用户 UUID
    if !users.contains(&request.uuid) {
        warn!("Invalid UUID from {}: {} (not in config)", client_addr, request.uuid);
        return Err(anyhow!("Authentication failed: invalid user UUID (addr: {})", client_addr));
    }

    info!("Authenticated user {} from {}", request.uuid, client_addr);

    // 发送响应头 - 使用与请求相同的版本号
    let response = VlessResponse::new_with_version(request.version);
    stream.write_all(&response.encode()).await?;

    // 获取用户邮箱
    let user_email = authenticate(request.uuid).await;

    // 根据命令类型处理连接
    match request.command {
        Command::Tcp => {
            handle_tcp_proxy(stream, request, remaining_data, performance_config, user_email).await
        }
        Command::Udp => {
            handle_udp_proxy(stream, request, performance_config, user_email).await
        }
        Command::Mux => {
            warn!("Mux command not implemented yet");
            Err(anyhow!("Mux not supported"))
        }
    }
}

/// 处理 TCP 代理
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
            let domain_str = std::str::from_utf8(domain)
                .map_err(|_| anyhow!("Invalid domain encoding"))?;
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

    // 配置目标连接的 TCP 参数
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

    // 使用 tokio::io::copy 进行单向拷贝（比手动 read/write 循环更高效）
    let (mut client_read, mut client_write) = client_stream.into_split();
    let (mut target_read, mut target_write) = target_stream.into_split();

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

/// 处理 UDP 代理（UDP over TCP 机制）
async fn handle_udp_proxy(
    client_stream: TcpStream,
    request: VlessRequest,
    perf_config: PerformanceConfig,
    _user_email: Option<Arc<str>>,
) -> Result<()> {
    // 解析目标地址
    let target_addr = match &request.address {
        Address::Domain(domain) => {
            let domain_str = std::str::from_utf8(domain)
                .map_err(|_| anyhow!("Invalid domain encoding"))?;
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

    // 绑定本地 UDP socket（随机端口）
    let udp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    let local_addr = udp_socket.local_addr()?;
    debug!("UDP socket bound to {}", local_addr);

    let udp_timeout = perf_config.udp_timeout;

    // 分离 TCP 流
    let (mut client_read, mut client_write) = client_stream.into_split();

    // 任务1：客户端 → 目标（读取 TCP 数据，发送 UDP 包）
    let udp_socket_c2t = Arc::clone(&udp_socket);

    let client_to_target = tokio::spawn(async move {
        let mut buffer = [0u8; 4096];
        let timeout_duration = std::time::Duration::from_secs(udp_timeout);

        loop {
            let timeout_result = tokio::time::timeout(
                timeout_duration,
                client_read.read(&mut buffer)
            ).await;

            match timeout_result {
                Ok(Ok(0)) => {
                    debug!("Client closed connection");
                    break;
                }
                Ok(Ok(n)) => {
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

    // 任务2：目标 → 客户端（接收 UDP 包，写入 TCP 流）
    let udp_socket_t2c = Arc::clone(&udp_socket);

    let target_to_client = tokio::spawn(async move {
        let mut buffer = [0u8; 4096];
        let timeout_duration = std::time::Duration::from_secs(udp_timeout);

        loop {
            let timeout_result = tokio::time::timeout(
                timeout_duration,
                udp_socket_t2c.recv_from(&mut buffer)
            ).await;

            match timeout_result {
                Ok(Ok((n, src))) => {
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
