//! 初始化向导模块
//!
//! 提供首次运行时的交互式配置向导

use crate::config::{Config, ServerSettings, TlsConfig, UserConfig};
use anyhow::Result;
use std::io::{self, Write};
use uuid::Uuid;

/// 证书配置模式
#[derive(Debug, Clone)]
enum CertMode {
    /// 自动生成自签名证书
    Generate,
    /// 使用现有证书文件
    UseExisting { cert_file: String, key_file: String },
}

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
    let (tls_enabled, tls_server_name, cert_mode) = prompt_tls()?;

    // 根据证书模式确定证书路径
    let (cert_file, key_file) = match &cert_mode {
        CertMode::Generate => (
            "certs/server.crt".to_string(),
            "certs/server.key".to_string(),
        ),
        CertMode::UseExisting {
            cert_file,
            key_file,
        } => (cert_file.clone(), key_file.clone()),
    };

    // 构建配置
    let tls_config = TlsConfig {
        enabled: tls_enabled,
        cert_file,
        key_file,
        server_name: tls_server_name.clone(),
        xtls_flow: if tls_enabled {
            "xtls-rprx-vision".to_string()
        } else {
            "".to_string()
        },
    };

    // 根据证书模式处理证书
    if tls_enabled {
        match cert_mode {
            CertMode::Generate => {
                println!("\n正在生成 TLS 证书...");
                crate::tls::ensure_cert_exists(&tls_config)?;
                println!("✅ TLS 证书已生成: {}", tls_config.cert_file);
            }
            CertMode::UseExisting { .. } => {
                println!("\n✅ 已配置使用现有证书");
                println!("   请确保证书文件存在并有效");
            }
        }
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
    print!("  选项: [1] 自动生成  [2] 手动输入");
    println!();
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
fn prompt_tls() -> Result<(bool, String, CertMode)> {
    println!("\n【步骤 3/3】配置 TLS 加密");
    println!("  TLS 可以加密传输流量，提高安全性");
    print!("  是否启用 TLS? [Y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let enabled = !matches!(input.trim().to_lowercase().as_str(), "n" | "no");

    if !enabled {
        return Ok((false, "localhost".to_string(), CertMode::Generate));
    }

    // 选择证书配置方式
    println!("\n  证书配置选项:");
    println!("    [1] 自动生成自签名证书 (适合测试环境)");
    println!("    [2] 使用现有证书文件 (适合生产环境)");
    print!("  请选择 (默认: 1): ");
    io::stdout().flush()?;

    let mut cert_choice = String::new();
    io::stdin().read_line(&mut cert_choice)?;

    match cert_choice.trim() {
        "2" => prompt_existing_cert(),
        _ => prompt_generate_cert(),
    }
}

/// 提示用户配置现有证书
fn prompt_existing_cert() -> Result<(bool, String, CertMode)> {
    println!("\n  使用现有证书文件");

    // 获取证书文件路径
    print!("  证书文件路径 (默认: certs/server.crt): ");
    io::stdout().flush()?;
    let mut cert_path = String::new();
    io::stdin().read_line(&mut cert_path)?;
    let cert_file = cert_path.trim().if_empty("certs/server.crt");

    // 获取私钥文件路径
    print!("  私钥文件路径 (默认: certs/server.key): ");
    io::stdout().flush()?;
    let mut key_path = String::new();
    io::stdin().read_line(&mut key_path)?;
    let key_file = key_path.trim().if_empty("certs/server.key");

    // 获取服务器名称
    print!("  服务器域名/SNI (默认: localhost): ");
    io::stdout().flush()?;
    let mut server = String::new();
    io::stdin().read_line(&mut server)?;
    let server_name = server.trim().if_empty("localhost");

    // 检查证书文件是否存在
    if !std::path::Path::new(&cert_file).exists() {
        println!("  ⚠️  警告: 证书文件不存在: {}", cert_file);
        println!("     请确保在启动服务器前放置正确的证书文件");
    }

    if !std::path::Path::new(&key_file).exists() {
        println!("  ⚠️  警告: 私钥文件不存在: {}", key_file);
        println!("     请确保在启动服务器前放置正确的私钥文件");
    }

    println!("  ✅ 已配置使用现有证书:");
    println!("     证书: {}", cert_file);
    println!("     私钥: {}", key_file);
    println!("     SNI: {}", server_name);

    Ok((
        true,
        server_name.to_string(),
        CertMode::UseExisting {
            cert_file: cert_file.to_string(),
            key_file: key_file.to_string(),
        },
    ))
}

/// 提示用户生成自签名证书
fn prompt_generate_cert() -> Result<(bool, String, CertMode)> {
    print!("  请输入服务器域名/SNI (默认: localhost): ");
    io::stdout().flush()?;
    let mut server = String::new();
    io::stdin().read_line(&mut server)?;
    let server_name = server.trim().if_empty("localhost");

    println!("  将自动生成自签名证书，SNI: {}", server_name);

    Ok((true, server_name.to_string(), CertMode::Generate))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cert_mode_variants() {
        // 测试证书模式枚举的基本功能
        let generate_mode = CertMode::Generate;
        let existing_mode = CertMode::UseExisting {
            cert_file: "test.crt".to_string(),
            key_file: "test.key".to_string(),
        };

        // 验证枚举变体可以正确创建
        match generate_mode {
            CertMode::Generate => assert!(true),
            _ => panic!("Expected Generate mode"),
        }

        match existing_mode {
            CertMode::UseExisting {
                cert_file,
                key_file,
            } => {
                assert_eq!(cert_file, "test.crt");
                assert_eq!(key_file, "test.key");
            }
            _ => panic!("Expected UseExisting mode"),
        }
    }

    #[test]
    fn test_if_empty_trait() {
        // 测试 IfEmpty trait 的功能
        assert_eq!("".if_empty("default"), "default");
        assert_eq!("value".if_empty("default"), "value");
        assert_eq!("  ".trim().if_empty("default"), "default");
    }
}
