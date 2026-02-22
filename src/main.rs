mod protocol;
mod server;
mod config;
mod http;
mod wizard;
mod buffer_pool;
mod ws;
mod utils;

use anyhow::Result;
use config::Config;
use server::{ServerConfig, VlessServer};
use std::env;
use tracing::{info, error};

// 使用 mimalloc 作为全局内存分配器，提升内存分配性能
// musl 目标不使用 mimalloc，因为与静态链接存在兼容性问题（__memcpy_chk、__memset_chk）
#[cfg(not(target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志（默认 info 级别，可通过 RUST_LOG 环境变量配置）
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

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

            // 在 Unix 系统上设置配置文件权限为只有所有者可读写
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&config_path) {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o600); // rw-------
                    if let Err(e) = std::fs::set_permissions(&config_path, perms) {
                        error!("Failed to set config file permissions: {}", e);
                    } else {
                        info!("Config file permissions set to 600 (rw-------)");
                    }
                }
            }

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
    server_config.protocol = config.server.protocol;
    server_config.ws_path = config.server.ws_path.clone();

    for user in &config.users {
        if let Ok(uuid) = uuid::Uuid::parse_str(&user.uuid) {
            let email = user.email.clone();
            server_config.add_user_with_email(uuid, email.clone());
            info!("  Added user: {} ({})", uuid, email.as_deref().unwrap_or("no email"));
        }
    }

    // 启动服务器
    let performance_config = config.performance.clone();
    let server = VlessServer::new(server_config, performance_config);

    info!("Starting VLESS server...");
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e);
    }

    Ok(())
}
