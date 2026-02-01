mod protocol;
mod server;
mod config;

use anyhow::Result;
use config::Config;
use server::{ServerConfig, VlessServer};
use std::env;
use tracing::{info, error};
use tracing_subscriber;

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
    let mut server_config = ServerConfig::new(bind_addr);
    
    // 添加用户
    for uuid in config.user_uuids()? {
        server_config.add_user(uuid);
        info!("  Added user: {}", uuid);
    }

    // 启动服务器
    let server = VlessServer::new(server_config);
    
    info!("Starting VLESS server...");
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e);
    }

    Ok(())
}
