use crate::stats::{MonitorData, SharedStats, SpeedHistoryResponse};
use crate::config::MonitoringConfig;
use crate::time::UtcTime;
use anyhow::{Result, anyhow};
use tokio::sync::mpsc::UnboundedSender;
use futures_util::{stream::StreamExt, sink::SinkExt};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_tungstenite::{
    tungstenite::protocol::{Message, WebSocketConfig},
};
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
pub(crate) enum WsMessage {
    #[serde(rename = "stats")]
    Stats(MonitorData),
    #[serde(rename = "history")]
    History(SpeedHistoryResponse),
}

pub type WsSender = UnboundedSender<Message>;

pub struct WebSocketConnection {
    pub tx: WsSender,
    pub last_activity: Arc<tokio::sync::Mutex<UtcTime>>,
}

impl WebSocketConnection {
    pub fn new(tx: WsSender) -> Self {
        Self {
            tx,
            last_activity: Arc::new(tokio::sync::Mutex::new(UtcTime::now())),
        }
    }
}

pub type SharedWsManager = Arc<RwLock<WebSocketManager>>;

pub struct WebSocketManager {
    connections: HashMap<usize, WebSocketConnection>,
    next_id: usize,
    config: MonitoringConfig,
}

impl WebSocketManager {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            connections: HashMap::new(),
            next_id: 0,
            config,
        }
    }

    pub async fn add_connection(&mut self, conn: WebSocketConnection) -> Result<usize> {
        if self.connections.len() >= self.config.websocket_max_connections {
            return Err(anyhow!("Maximum WebSocket connections reached ({})", self.config.websocket_max_connections));
        }

        let id = self.next_id;
        self.connections.insert(id, conn);
        self.next_id += 1;
        tracing::info!("WebSocket connection added: id={}, total={}", id, self.connections.len());
        Ok(id)
    }

    pub async fn remove_connection(&mut self, id: usize) {
        if self.connections.remove(&id).is_some() {
            tracing::info!("WebSocket connection removed: id={}, total={}", id, self.connections.len());
        }
    }

    pub(crate) async fn broadcast(&self, msg: &WsMessage) -> Result<Vec<usize>> {
        let json = serde_json::to_string(msg)?;
        let mut dead_connections = Vec::new();

        for (id, conn) in &self.connections {
            if conn.tx.send(Message::Text(json.clone())).is_err() {
                dead_connections.push(*id);
            }
        }

        Ok(dead_connections)
    }

    pub async fn cleanup_stale_connections(&mut self) -> Vec<usize> {
        let mut dead_ids = Vec::new();
        let now = UtcTime::now();

        for (id, conn) in &self.connections {
            if conn.tx.send(Message::Ping(vec![])).is_err() {
                dead_ids.push(*id);
                continue;
            }

            // Check heartbeat timeout
            if let Ok(last_activity) = conn.last_activity.try_lock() {
                let duration = now.signed_duration_since(*last_activity);
                if duration > self.config.websocket_heartbeat_timeout as i64 {
                    tracing::warn!("WebSocket connection {} timeout after {}s", id, duration);
                    dead_ids.push(*id);
                }
            }
        }

        for id in &dead_ids {
            self.remove_connection(*id).await;
        }

        dead_ids
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new(MonitoringConfig::default())
    }
}

pub async fn start_broadcasting_task(ws_manager: SharedWsManager, stats: SharedStats, config: MonitoringConfig) {
    let mut interval = tokio::time::interval(Duration::from_secs(config.broadcast_interval));
    let mut cleanup_interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut stats_guard = stats.lock().await;
                let monitor_data = stats_guard.get_monitor_data();
                drop(stats_guard);

                let msg = WsMessage::Stats(monitor_data);

                let manager = ws_manager.write().await;
                match manager.broadcast(&msg).await {
                    Ok(dead_connections) => {
                        if !dead_connections.is_empty() {
                            drop(manager);
                            let mut manager = ws_manager.write().await;
                            for id in dead_connections {
                                tracing::warn!("Removing dead WebSocket connection: {}", id);
                                manager.remove_connection(id).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to broadcast stats: {}", e);
                    }
                }
            }
            _ = cleanup_interval.tick() => {
                let mut manager = ws_manager.write().await;
                let dead_ids = manager.cleanup_stale_connections().await;
                if !dead_ids.is_empty() {
                    tracing::info!("Cleaned up {} stale WebSocket connections", dead_ids.len());
                }
            }
        }
    }
}

use crate::http::HttpRequest;

fn is_allowed_origin(origin: &str) -> bool {
    // Allow same-origin connections
    if let Ok(expected) = std::env::var("VLESS_MONITOR_ORIGIN") {
        return origin == expected;
    }

    // If no origin is configured, allow all (development mode)
    // In production, always set VLESS_MONITOR_ORIGIN
    true
}

pub fn is_websocket_upgrade(request: &HttpRequest) -> bool {
    if request.method.to_uppercase() != "GET" {
        tracing::debug!("Not WebSocket: method is {}", request.method);
        return false;
    }

    // 检查必需的 WebSocket 握手头
    let has_upgrade_header = request
        .headers
        .iter()
        .any(|(k, v)| k.to_lowercase() == "upgrade" && v.to_lowercase() == "websocket");

    if !has_upgrade_header {
        tracing::debug!("Not WebSocket: missing or invalid Upgrade header");
        return false;
    }

    let has_connection_header = request
        .headers
        .iter()
        .any(|(k, v)| {
            k.to_lowercase() == "connection" &&
            (v.to_lowercase().contains("upgrade") || v == "Upgrade")
        });

    if !has_connection_header {
        tracing::debug!("Not WebSocket: missing or invalid Connection header");
        return false;
    }

    // 检查是否是 WebSocket 路径
    let is_ws_path = request.path == "/api/ws" || request.path == "/ws";

    if !is_ws_path {
        tracing::debug!("Not WebSocket: path is {}", request.path);
        return false;
    }

    tracing::info!("WebSocket upgrade request validated for path: {}", request.path);

    // 验证 Origin header 防止 CSRF 攻击
    if let Some(origin) = request
        .headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "origin")
        .map(|(_, v)| v.as_str())
    {
        if !is_allowed_origin(origin) {
            tracing::warn!("Rejected WebSocket connection from disallowed origin: {}", origin);
            return false;
        }
    }

    true
}

pub async fn handle_websocket_connection(
    mut stream: TcpStream,
    ws_manager: SharedWsManager,
    stats: SharedStats,
    client_addr: std::net::SocketAddr,
    initial_data: Option<Vec<u8>>,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // 使用传入的初始数据，如果没有则读取
    let request_str = if let Some(data) = initial_data {
        std::str::from_utf8(&data)?.to_string()
    } else {
        let mut buffer = vec![0u8; 4096];
        let n = stream.read(&mut buffer).await?;
        std::str::from_utf8(&buffer[..n])?.to_string()
    };

    // Parse the HTTP request to extract Sec-WebSocket-Key
    let ws_key = extract_websocket_key(&request_str)?;

    // Send WebSocket upgrade response with security headers
    let accept_key = compute_accept_key(&ws_key);

    // Build raw WebSocket upgrade response
    let header = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\
         \r\n",
        accept_key
    );

    let mut stream = stream;
    stream.write_all(header.as_bytes()).await?;

    tracing::info!("WebSocket connection established from {}", client_addr);

    // Wrap in WebSocket
    let config = WebSocketConfig::default();
    let ws_stream = tokio_tungstenite::WebSocketStream::from_raw_socket(
        stream,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        Some(config),
    )
    .await;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Create channel for sending messages
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // Add connection to manager
    let conn = WebSocketConnection::new(tx);
    let mut manager = ws_manager.write().await;

    let conn_id = match manager.add_connection(conn).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to add WebSocket connection: {}", e);
            let _ = ws_sender.send(Message::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Policy,
                reason: "Server full".into(),
            }))).await;
            return Err(e);
        }
    };

    // Get connection reference for activity updates
    let conn_ref = {
        if let Some(c) = manager.connections.get(&conn_id) {
            c.last_activity.clone()
        } else {
            tracing::error!("Connection {} not found after adding", conn_id);
            return Err(anyhow!("Connection not found"));
        }
    };

    drop(manager);

    // Send initial history data
    {
        let stats_guard = stats.lock().await;
        let history = stats_guard.get_speed_history_response();
        drop(stats_guard);

        let history_msg = WsMessage::History(history);
        if let Ok(json) = serde_json::to_string(&history_msg) {
            if ws_sender.send(Message::Text(json)).await.is_err() {
                tracing::error!("Failed to send history data to connection {}", conn_id);
                let mut manager = ws_manager.write().await;
                manager.remove_connection(conn_id).await;
                return Ok(());
            }
        }
    }

    // Update initial activity
    {
        let mut last_activity = conn_ref.lock().await;
        *last_activity = UtcTime::now();
    }

    // Handle incoming messages and channel messages simultaneously
    loop {
        tokio::select! {
            // Handle messages from the broadcast channel
            Some(msg) = rx.recv() => {
                if ws_sender.send(msg).await.is_err() {
                    tracing::warn!("Failed to send message to connection {}, closing", conn_id);
                    break;
                }
            }
            // Handle incoming WebSocket messages (ping/pong/close)
            msg_result = ws_receiver.next() => {
                match msg_result {
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_sender.send(Message::Pong(data)).await;
                        // Update activity on ping
                        {
                            let mut last_activity = conn_ref.lock().await;
                            *last_activity = UtcTime::now();
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Update activity on pong
                        {
                            let mut last_activity = conn_ref.lock().await;
                            *last_activity = UtcTime::now();
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("WebSocket connection {} requested close", conn_id);
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error on connection {}: {}", conn_id, e);
                        break;
                    }
                    None => {
                        // Connection closed
                        tracing::info!("WebSocket connection {} closed by client", conn_id);
                        break;
                    }
                    Some(Ok(_)) => {}
                }
            }
        }
    }

    // Cleanup
    let mut manager = ws_manager.write().await;
    manager.remove_connection(conn_id).await;

    Ok(())
}

fn extract_websocket_key(request: &str) -> Result<String> {
    for line in request.lines() {
        if line.to_lowercase().starts_with("sec-websocket-key:") {
            let key = line.split(':').nth(1)
                .ok_or_else(|| anyhow!("Invalid Sec-WebSocket-Key header"))?
                .trim();
            return Ok(key.to_string());
        }
    }
    Err(anyhow!("Sec-WebSocket-Key header not found"))
}

fn compute_accept_key(key: &str) -> String {
    use sha1::{Digest, Sha1};
    use crate::base64::encode;
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(GUID.as_bytes());
    let result = hasher.finalize();

    encode(&result)
}
