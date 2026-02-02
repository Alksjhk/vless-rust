mod protocol;
mod server;
mod config;
mod stats;
mod http;
mod ws;

use anyhow::Result;
use config::Config;
use server::{ServerConfig, VlessServer};
use stats::{Stats, start_stats_persistence};
use ws::WebSocketManager;
use std::env;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 读取配置文件路径
    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "config.json".to_string());

    // 加载配置
    let config = match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            info!("Loading config from {}", config_path);
            Config::from_json(&content)?
        }
        Err(_) => {
            info!("Config file not found, creating default config at {}", config_path);
            let default_config = Config::default();
            let json = default_config.to_json()?;
            std::fs::write(&config_path, json)?;
            default_config
        }
    };

    info!("Server configuration loaded:");
    info!("  Listen: {}:{}", config.server.listen, config.server.port);
    info!("  Users: {}", config.users.len());

    // 创建服务器配置
    let bind_addr = config.bind_addr()?;

    // 添加用户及邮箱信息
    let mut server_config = ServerConfig::new(bind_addr);

    for user in &config.users {
        if let Ok(uuid) = uuid::Uuid::parse_str(&user.uuid) {
            let email = user.email.clone();
            server_config.add_user_with_email(uuid, email.clone());
            info!("  Added user: {} ({})", uuid, email.as_deref().unwrap_or("no email"));
        }
    }

    // 创建统计模块
    let config_path = config_path.clone();
    let monitoring_config = config.monitoring.clone();
    let stats = Arc::new(Mutex::new(Stats::new(config_path.clone(), monitoring_config.clone())));

    // 从配置文件加载统计数据
    if let Err(e) = stats.lock().await.load_from_config() {
        info!("No existing stats found: {}", e);
    }

    // 创建 WebSocket 管理器
    let ws_manager = Arc::new(RwLock::new(WebSocketManager::new(monitoring_config.clone())));
    let ws_manager_clone = Arc::clone(&ws_manager);
    let stats_clone = Arc::clone(&stats);
    let monitoring_config_clone = monitoring_config.clone();

    // 启动 WebSocket 广播任务
    tokio::spawn(async move {
        ws::start_broadcasting_task(ws_manager_clone, stats_clone, monitoring_config_clone).await;
    });

    // 启动统计持久化任务
    let stats_persistence = Arc::clone(&stats);
    tokio::spawn(async move {
        start_stats_persistence(stats_persistence, config_path).await;
    });

    // 启动服务器
    let performance_config = config.performance.clone();
    let server = VlessServer::new(server_config, stats, ws_manager, monitoring_config, performance_config);
    
    info!("Starting VLESS server...");
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e);
    }

    Ok(())
}
