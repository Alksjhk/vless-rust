mod config;
mod connection_pool;
mod http;
mod memory;
mod protocol;
mod server;
mod stats;
mod tls;
mod wizard;
mod ws;

use anyhow::Result;
use config::Config;
use rustls::ServerConfig as RustlsServerConfig;
use server::{ServerConfig, VlessServer};
use stats::{start_stats_persistence, Stats};
use std::env;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};
use ws::WebSocketManager;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 读取配置文件路径
    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "config.json".to_string());

    // 检查配置文件是否存在
    let config_exists = std::path::Path::new(&config_path).exists();

    // 加载配置
    let mut config = if config_exists {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => {
                info!("Loading config from {}", config_path);
                Config::from_json(&content)?
            }
            Err(e) => {
                error!("Failed to read config file: {}", e);
                return Err(e.into());
            }
        }
    } else {
        // 首次运行，启动初始化向导
        info!("Config file not found, starting initialization wizard...");
        let wizard_config = wizard::run_init_wizard()?;
        let json = wizard_config.to_json()?;
        std::fs::write(&config_path, json)?;
        println!("\n✅ 配置已保存到: {}", config_path);
        wizard_config
    };

    // 生成并保存 VLESS URL
    let vless_url = config.generate_vless_url();
    config.vless_url = Some(vless_url.clone());

    // 更新配置文件（包含 vless_url）
    let json = config.to_json()?;
    std::fs::write(&config_path, json)?;

    // 打印服务器信息和 VLESS 连接 URL
    print_server_info(&config);

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
            info!(
                "  Added user: {} ({})",
                uuid,
                email.as_deref().unwrap_or("no email")
            );
        }
    }

    // 创建统计模块
    let config_path = config_path.clone();
    let monitoring_config = config.monitoring.clone();
    let stats = Arc::new(Mutex::new(Stats::new(
        config_path.clone(),
        monitoring_config.clone(),
    )));

    // 从配置文件加载统计数据
    if let Err(e) = stats.lock().await.load_from_config() {
        info!("No existing stats found: {}", e);
    }

    // 创建 WebSocket 管理器
    let ws_manager = Arc::new(RwLock::new(WebSocketManager::new(
        monitoring_config.clone(),
    )));
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

    // 加载 TLS 配置
    let tls_config: Option<Arc<RustlsServerConfig>> = if config.tls.enabled {
        info!("TLS is enabled, loading certificates...");
        // 确保证书文件存在（如果不存在则自动生成）
        if let Err(e) = tls::ensure_cert_exists(&config.tls) {
            error!("Failed to ensure TLS certificates exist: {}", e);
            return Err(e);
        }
        // 加载 TLS 配置
        match tls::load_tls_config(&config.tls).await {
            Ok(cfg) => {
                info!("TLS configuration loaded successfully");
                info!("  Certificate: {}", config.tls.cert_file);
                info!("  Private key: {}", config.tls.key_file);
                Some(cfg)
            }
            Err(e) => {
                error!("Failed to load TLS configuration: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("TLS is disabled");
        None
    };

    // 启动服务器
    let performance_config = config.performance.clone();
    let server = VlessServer::new(
        server_config,
        stats.clone(),
        ws_manager,
        monitoring_config,
        performance_config,
        tls_config,
    );

    // 设置连接池引用到统计模块
    let connection_pools = server.get_connection_pools();
    stats.lock().await.set_connection_pools(connection_pools.clone());

    // 预热连接池（如果配置了常用目标地址）
    if let Some(warmup_targets) = get_warmup_targets(&config) {
        info!("Warming up connection pools for {} targets", warmup_targets.len());
        if let Err(e) = connection_pools.warmup(warmup_targets).await {
            warn!("Failed to warmup connection pools: {}", e);
        }
    }

    // 设置优雅关闭处理
    let connection_pools_shutdown = connection_pools.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        info!("Received shutdown signal, cleaning up...");
        connection_pools_shutdown.shutdown().await;
        std::process::exit(0);
    });

    info!("Starting VLESS server...");
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e);
    }

    Ok(())
}

/// 打印服务器信息和 VLESS 连接 URL
fn print_server_info(config: &Config) {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║              VLESS Rust 服务器已启动                      ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("\n📋 服务器信息:");
    println!(
        "  监听地址: {}:{}",
        config.server.listen, config.server.port
    );
    println!(
        "  TLS 状态: {}",
        if config.tls.enabled {
            "启用"
        } else {
            "禁用"
        }
    );
    if config.tls.enabled {
        println!("  证书文件: {}", config.tls.cert_file);
        println!("  服务器名称: {}", config.tls.server_name);
    }
    println!("  用户数量: {}", config.users.len());

    if let Some(vless_url) = &config.vless_url {
        println!("\n🔗 VLESS 连接 URL:");
        println!("  ┌─────────────────────────────────────────────────────────┐");
        println!("  │ {}", vless_url);
        println!("  └─────────────────────────────────────────────────────────┘");
        println!("\n  💡 提示: 复制上方 URL 到 VLESS 客户端即可连接");
    }

    println!("\n📊 监控面板:");
    let protocol = if config.tls.enabled { "https" } else { "http" };
    println!(
        "  {}://{}:{}/",
        protocol, config.server.listen, config.server.port
    );
    println!("\n按 Ctrl+C 停止服务器\n");
}

/// 获取连接池预热目标地址
fn get_warmup_targets(_config: &Config) -> Option<Vec<std::net::SocketAddr>> {
    // 从配置中获取常用的目标地址进行预热
    // 这里可以配置一些常用的目标服务器，比如 DNS 服务器、CDN 等
    let common_targets = vec![
        "8.8.8.8:53",      // Google DNS
        "1.1.1.1:53",      // Cloudflare DNS
        "208.67.222.222:53", // OpenDNS
    ];

    let mut targets = Vec::new();
    for target_str in common_targets {
        if let Ok(addr) = target_str.parse::<std::net::SocketAddr>() {
            targets.push(addr);
        }
    }

    // 如果配置中有特定的预热目标，也可以添加
    // 这里暂时使用默认的常用目标
    if targets.is_empty() {
        None
    } else {
        Some(targets)
    }
}
