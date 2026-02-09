mod protocol;
mod server;
mod config;
mod stats;
mod http;
mod ws;
mod utils;
mod wizard;
mod base64;
mod memory;
mod time;
mod buffer_pool;

use anyhow::Result;
use config::Config;
use server::{ServerConfig, VlessServer};
use stats::{Stats, start_stats_persistence};
use ws::WebSocketManager;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, warn};

// 使用 mimalloc 作为全局内存分配器，提升内存分配性能
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
            info!("Config file not found at {}", config_path);
            info!("Starting configuration wizard...");
            let config = wizard::ConfigWizard::run()?;
            let json = config.to_json()?;
            std::fs::write(&config_path, json)?;
            info!("Config saved to {}", config_path);
            config
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

    // 获取外网 IP（在创建统计模块之前获取）
    info!("Detecting public IP...");

    // 优先使用配置文件中的 IP
    let public_ip = if let Some(configured_ip) = config.server.public_ip.clone() {
        info!("Using configured public IP: {}", configured_ip);
        configured_ip
    } else {
        // 自动检测 IP
        match utils::get_public_ip().await {
            Ok(ip) => {
                info!("Public IP detected: {}", ip);
                ip
            }
            Err(e) => {
                warn!("Failed to detect public IP:");
                warn!("  {}", e);
                info!("Tips:");
                info!("  1. Check network connectivity: curl https://myip.ipip.net");
                info!("  2. Check DNS resolution: nslookup myip.ipip.net");
                info!("  3. Check firewall rules for outbound HTTPS");
                info!("  4. Add 'public_ip' field in config.json to skip auto-detection");
                "YOUR_SERVER_IP".to_string()
            }
        }
    };

    let stats = Arc::new(RwLock::new(Stats::new(config_path.clone(), monitoring_config.clone(), public_ip.clone())));

    // 从配置文件加载统计数据
    if let Err(e) = stats.write().await.load_from_config() {
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

    info!("\n========== VLESS Connection Links ==========");
    for user in &config.users {
        let url = utils::generate_vless_url(
            &user.uuid,
            &public_ip,
            config.server.port,
            user.email.as_deref(),
        );
        info!("{}", url);
    }

    if public_ip == "YOUR_SERVER_IP" {
        info!("");
        info!("⚠ Please replace YOUR_SERVER_IP with your actual public IP");
        info!("  Or add 'public_ip' field to server section in config.json");
    }

    info!("==========================================\n");

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
