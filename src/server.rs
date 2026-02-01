use crate::protocol::{VlessRequest, VlessResponse, Command, Address};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

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
}

impl VlessServer {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Arc::new(config),
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
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, addr, config).await {
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
    ) -> Result<()> {
        debug!("New connection from {}", client_addr);

        // 读取VLESS请求头
        let mut header_buf = vec![0u8; 1024];
        let n = stream.read(&mut header_buf).await?;
        if n == 0 {
            return Err(anyhow!("Connection closed by client"));
        }

        let header_bytes = Bytes::from(header_buf[..n].to_vec());
        let (request, remaining_data) = VlessRequest::decode(header_bytes)?;

        debug!("Parsed VLESS request: {:?}", request);

        // 验证用户UUID
        if !config.users.contains(&request.uuid) {
            warn!("Invalid UUID from {}: {}", client_addr, request.uuid);
            return Err(anyhow!("Invalid user UUID"));
        }

        info!("Authenticated user {} from {}", request.uuid, client_addr);

        // 发送响应头 - 使用与请求相同的版本号
        let response = VlessResponse::new_with_version(request.version);
        stream.write_all(&response.encode()).await?;

        // 根据命令类型处理连接
        match request.command {
            Command::Tcp => {
                Self::handle_tcp_proxy(stream, request, remaining_data).await?;
            }
            Command::Udp => {
                warn!("UDP command not implemented yet");
                return Err(anyhow!("UDP not supported"));
            }
            Command::Mux => {
                warn!("Mux command not implemented yet");
                return Err(anyhow!("Mux not supported"));
            }
        }

        Ok(())
    }

    /// 处理TCP代理
    async fn handle_tcp_proxy(
        mut client_stream: TcpStream,
        request: VlessRequest,
        initial_data: Bytes,
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

        // 如果有初始数据，先发送给目标服务器
        if !initial_data.is_empty() {
            target_stream.write_all(&initial_data).await?;
        }

        info!("Established proxy connection: {} -> {}", 
               client_stream.peer_addr()?, target_addr);

        // 双向数据转发
        let (mut client_read, mut client_write) = client_stream.split();
        let (mut target_read, mut target_write) = target_stream.split();

        // 等待任一方向的连接关闭
        tokio::select! {
            result = tokio::io::copy(&mut client_read, &mut target_write) => {
                match result {
                    Ok(bytes) => debug!("Client to target: {} bytes transferred", bytes),
                    Err(e) => debug!("Client to target error: {}", e),
                }
            }
            result = tokio::io::copy(&mut target_read, &mut client_write) => {
                match result {
                    Ok(bytes) => debug!("Target to client: {} bytes transferred", bytes),
                    Err(e) => debug!("Target to client error: {}", e),
                }
            }
        }

        debug!("Proxy connection closed");
        Ok(())
    }
}