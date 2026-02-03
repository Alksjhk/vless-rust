//! 初始化向导模块
//!
//! 提供首次运行时的交互式配置向导

use anyhow::Result;
use crate::config::{Config, ServerSettings, UserConfig, TlsConfig};
use uuid::Uuid;
use std::io::{self, Write};

/// 初始化向导
///
/// 引导用户完成首次配置，包括：
/// - 设置监听端口（默认 8443）
/// - 生成或设置 UUID
/// - 配置 TLS（启用/禁用，自动生成证书）
pub fn run_init_wizard() -> Result<Config> {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║        VLESS Rust 服务器 - 初始化配置向导                  ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("\n欢迎使用 VLESS Rust 服务器！");
    println!("请按照提示完成初始化配置：\n");

    // 1. 配置端口
    let port = prompt_port()?;

    // 2. 配置 UUID
    let uuid = prompt_uuid()?;

    // 3. 配置 TLS
    let (tls_enabled, tls_server_name) = prompt_tls()?;

    // 构建配置
    let tls_config = TlsConfig {
        enabled: tls_enabled,
        cert_file: "certs/server.crt".to_string(),
        key_file: "certs/server.key".to_string(),
        server_name: tls_server_name.clone(),
    };

    // 如果启用 TLS，生成证书
    if tls_enabled {
        println!("\n正在生成 TLS 证书...");
        crate::tls::ensure_cert_exists(&tls_config)?;
        println!("✅ TLS 证书已生成: {}", tls_config.cert_file);
    }

    let config = Config {
        server: ServerSettings {
            listen: "0.0.0.0".to_string(),
            port,
        },
        users: vec![UserConfig {
            uuid: uuid.clone(),
            email: Some(format!("user@{}", tls_server_name)),
        }],
        monitoring: Default::default(),
        performance: Default::default(),
        tls: tls_config,
        vless_url: None,
    };

    println!("\n✅ 配置已完成！");
    println!("\n配置摘要：");
    println!("  监听地址: {}:{}", config.server.listen, port);
    println!("  UUID: {}", uuid);
    println!("  TLS: {}", if tls_enabled { "启用" } else { "禁用" });

    Ok(config)
}

/// 提示用户输入端口
fn prompt_port() -> Result<u16> {
    println!("\n【步骤 1/3】配置监听端口");
    println!("  默认端口: 8443");
    print!("  请输入端口 (直接回车使用默认): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let port = input.trim();
    if port.is_empty() {
        Ok(8443)
    } else {
        match port.parse::<u16>() {
            Ok(p) if p > 0 => Ok(p),
            _ => {
                println!("  ⚠️  无效端口号，使用默认值 8443");
                Ok(8443)
            }
        }
    }
}

/// 提示用户输入或生成 UUID
fn prompt_uuid() -> Result<String> {
    println!("\n【步骤 2/3】配置 UUID");
    print!("  选项: [1] 自动生成  [2] 手动输入\n");
    print!("  请选择 (默认: 1): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim() {
        "2" => {
            print!("  请输入 UUID: ");
            io::stdout().flush()?;
            let mut uuid_input = String::new();
            io::stdin().read_line(&mut uuid_input)?;

            let uuid_str = uuid_input.trim();
            match Uuid::parse_str(uuid_str) {
                Ok(uuid) => Ok(uuid.to_string()),
                Err(_) => {
                    println!("  ⚠️  无效 UUID，已自动生成");
                    Ok(Uuid::new_v4().to_string())
                }
            }
        }
        _ => {
            let new_uuid = Uuid::new_v4();
            println!("  已生成 UUID: {}", new_uuid);
            Ok(new_uuid.to_string())
        }
    }
}

/// 提示用户配置 TLS
fn prompt_tls() -> Result<(bool, String)> {
    println!("\n【步骤 3/3】配置 TLS 加密");
    println!("  TLS 可以加密传输流量，提高安全性");
    print!("  是否启用 TLS? [Y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let enabled = match input.trim().to_lowercase().as_str() {
        "n" | "no" => false,
        _ => true,
    };

    if !enabled {
        return Ok((false, "localhost".to_string()));
    }

    print!("  请输入服务器域名/SNI (默认: localhost): ");
    io::stdout().flush()?;
    let mut server = String::new();
    io::stdin().read_line(&mut server)?;
    let server_name = server.trim().if_empty("localhost");

    println!("  将自动生成自签名证书，SNI: {}", server_name);

    Ok((true, server_name.to_string()))
}

/// 扩展 trait：如果字符串为空则返回默认值
trait IfEmpty {
    fn if_empty(self, default: &str) -> String;
}

impl IfEmpty for &str {
    fn if_empty(self, default: &str) -> String {
        if self.is_empty() {
            default.to_string()
        } else {
            self.to_string()
        }
    }
}
