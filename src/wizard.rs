use std::io::{self, Write};
use anyhow::Result;
use crate::config::{Config, UserConfig, ServerSettings, ProtocolType};
use crate::utils::generate_vless_url;
use uuid::Uuid;

/// 交互式配置向导
pub struct ConfigWizard;

impl ConfigWizard {
    /// 启动配置向导
    pub fn run() -> Result<Config> {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║         VLESS Server - 首次配置向导                        ║");
        println!("║                                                            ║");
        println!("║  欢迎使用 VLESS 服务器！                                   ║");
        println!("║  这个向导将帮助您完成基本配置。                            ║");
        println!("╚════════════════════════════════════════════════════════════╝\n");

        // 配置服务器监听地址
        let listen = Self::prompt_listen_address()?;

        // 配置端口
        let port = Self::prompt_port()?;

        // 配置协议类型
        let (protocol, ws_path) = Self::prompt_protocol()?;

        // 配置用户
        let users = Self::prompt_users()?;

        println!("\n✓ 配置完成！正在生成配置文件...\n");

        // 创建配置
        let config = Config {
            server: ServerSettings {
                listen,
                port,
                protocol,
                ws_path,
            },
            users,
            performance: Default::default(),
        };

        // 显示生成的 VLESS URL
        Self::display_vless_urls(&config);

        Ok(config)
    }

    /// 提示输入监听地址
    fn prompt_listen_address() -> Result<String> {
        println!("【服务器监听地址】");
        println!("  监听地址决定了服务器接受连接的网络接口。");
        println!("  • 0.0.0.0  - 监听所有网络接口（推荐）");
        println!("  • 127.0.0.1 - 仅本地访问");
        println!("  • 特定IP   - 仅指定网卡");

        loop {
            print!("  请输入监听地址 [默认: 0.0.0.0]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                return Ok("0.0.0.0".to_string());
            }

            // 验证IP地址格式
            if input.parse::<std::net::IpAddr>().is_ok() {
                return Ok(input.to_string());
            }

            println!("  ⚠ 无效的IP地址格式，请重新输入");
        }
    }

    /// 提示输入端口
    fn prompt_port() -> Result<u16> {
        println!("\n【服务器监听端口】");
        println!("  监听端口用于接受 VLESS 连接和 HTTP 监控请求。");
        println!("  常用端口：443 (HTTPS)、8443 (备用HTTPS)");

        loop {
            print!("  请输入端口 [1-65535，默认: 443]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                return Ok(443);
            }

            match input.parse::<u16>() {
                Ok(port) if port > 0 => return Ok(port),
                _ => println!("  ⚠ 无效的端口号，请输入 1-65535 之间的数字"),
            }
        }
    }

    /// 提示选择协议类型
    fn prompt_protocol() -> Result<(ProtocolType, String)> {
        println!("\n【协议类型】");
        println!("  选择服务器接受的连接协议类型。");
        println!("  • TCP - 直接 TCP 连接（推荐，需要端口转发）");
        println!("  • WebSocket - WS 协议（可绑过墙，需要 Web 服务器配合）");

        let protocol = loop {
            print!("  请选择协议类型 [1]TCP / [2]WebSocket [默认: 1]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() || input == "1" {
                break ProtocolType::Tcp;
            } else if input == "2" {
                break ProtocolType::WebSocket;
            } else {
                println!("  ⚠ 无效选择，请输入 1 或 2");
            }
        };

        let ws_path = if protocol == ProtocolType::WebSocket {
            Self::prompt_ws_path()?
        } else {
            "/".to_string()
        };

        Ok((protocol, ws_path))
    }

    /// 提示输入 WebSocket 路径
    fn prompt_ws_path() -> Result<String> {
        println!("\n【WebSocket 路径】");
        println!("  WebSocket 路径用于客户端识别请求。");
        println!("  常用路径：/, /vless, /ws");

        loop {
            print!("  请输入 WebSocket 路径 [默认: /]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                return Ok("/".to_string());
            }

            // 路径必须以 / 开头
            if !input.starts_with('/') {
                println!("  ⚠ 路径必须以 / 开头");
                continue;
            }

            return Ok(input.to_string());
        }
    }

    /// 提示配置用户
    fn prompt_users() -> Result<Vec<UserConfig>> {
        println!("\n【用户配置】");
        println!("  VLESS 协议使用 UUID 作为用户认证凭据。");
        println!("  每个用户需要唯一的 UUID 和可选的邮箱地址。\n");

        let mut users = Vec::new();

        loop {
            let user = Self::prompt_user(&users)?;
            users.push(user);

            println!("\n当前用户数: {}", users.len());

            if !users.is_empty() {
                print!("是否继续添加用户？[y/N]: ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim().to_lowercase();

                if input != "y" && input != "yes" {
                    break;
                }
            }

            println!();
        }

        Ok(users)
    }

    /// 提示配置单个用户
    fn prompt_user(existing_users: &[UserConfig]) -> Result<UserConfig> {
        loop {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("添加用户 #{}", existing_users.len() + 1);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

            // UUID 配置
            println!("【用户 UUID】");
            println!("  UUID 是用户的唯一认证凭据，必须保密。");
            println!("  • 自动生成 - 系统随机生成安全的 UUID（推荐）");
            println!("  • 手动输入 - 使用自定义 UUID（8-4-4-4-12 格式）");

            let uuid = loop {
                print!("  选择 [A]自动生成 / [M]手动输入 [默认: A]: ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim().to_lowercase();

                if input.is_empty() || input == "a" || input == "auto" {
                    let new_uuid = Uuid::new_v4();
                    println!("  ✓ 已生成 UUID: {}", new_uuid);
                    break new_uuid.to_string();
                } else if input == "m" || input == "manual" {
                    print!("  请输入 UUID: ");
                    io::stdout().flush()?;

                    let mut uuid_input = String::new();
                    io::stdin().read_line(&mut uuid_input)?;
                    let uuid_input = uuid_input.trim();

                    match Uuid::parse_str(uuid_input) {
                        Ok(uuid) => break uuid.to_string(),
                        Err(_) => {
                            println!("  ⚠ 无效的 UUID 格式，示例: 550e8400-e29b-41d4-a716-446655440000");
                        }
                    }
                } else {
                    println!("  ⚠ 无效选择，请输入 A 或 M");
                }
            };

            // 邮箱配置
            let default_email = format!("user{}@a.com", existing_users.len() + 1);
            println!("\n【用户邮箱】");
            println!("  邮箱地址用于标识用户，方便管理。");
            println!("  可以在客户端显示，帮助识别连接。");

            print!("  请输入邮箱地址 [默认: {}]: ", default_email);
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            let email = if input.is_empty() {
                default_email.clone()
            } else {
                // 验证邮箱格式（基本格式检查）
                if !Self::is_valid_email_format(input) {
                    println!("  ⚠ 邮箱格式不正确，但仍然接受");
                }
                input.to_string()
            };

            // 验证 UUID 唯一性
            if let Some(existing) = existing_users.iter().find(|u| u.uuid == uuid) {
                println!("  ✗ 错误：UUID 与现有用户重复: {}", existing.email.as_deref().unwrap_or("未命名"));
                println!("  请重新输入不同的 UUID\n");
                continue; // 重新开始循环，而不是递归
            }

            println!("\n✓ 用户配置完成");
            println!("  UUID: {}", uuid);
            println!("  Email: {}", email);

            return Ok(UserConfig { uuid, email: Some(email) });
        }
    }

    /// 验证邮箱格式（基本检查）
    ///
    /// 检查规则：
    /// - 必须包含 @ 符号
    /// - @ 后面必须包含 .
    /// - @ 和 . 不能在开头或结尾
    /// - @ 和 . 之间必须有字符
    fn is_valid_email_format(email: &str) -> bool {
        let at_pos = match email.find('@') {
            Some(pos) => pos,
            None => return false,
        };

        // @ 不能在开头或结尾
        if at_pos == 0 || at_pos == email.len() - 1 {
            return false;
        }

        // @ 后面必须有 .
        let dot_pos = match email[at_pos + 1..].find('.') {
            Some(pos) => at_pos + 1 + pos,
            None => return false,
        };

        // . 不能在 @ 后面紧接着，也不能在最后
        if dot_pos == at_pos + 1 || dot_pos == email.len() - 1 {
            return false;
        }

        true
    }

    /// 显示生成的 VLESS URL
    ///
    /// 为每个用户生成 VLESS URL，方便客户端导入配置
    fn display_vless_urls(config: &Config) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║              VLESS 客户端配置 URL                          ║");
        println!("╚════════════════════════════════════════════════════════════╝\n");

        println!("提示：请将以下 URL 导入到 VLESS 客户端中");
        println!("注意：需要将 {} 替换为实际的服务器地址\n", config.server.listen);

        for (idx, user) in config.users.iter().enumerate() {
            let uuid = match Uuid::parse_str(&user.uuid) {
                Ok(u) => u,
                Err(_) => {
                    println!("  ⚠ 用户 #{}: UUID 格式无效，跳过生成 URL", idx + 1);
                    continue;
                }
            };

            let ws_path = if config.server.protocol == ProtocolType::WebSocket {
                Some(config.server.ws_path.as_str())
            } else {
                None
            };

            let url = generate_vless_url(
                &config.server.listen,
                config.server.port,
                &uuid,
                user.email.as_deref(),
                ws_path,
            );

            println!("【用户 #{} - {}】", idx + 1, user.email.as_deref().unwrap_or("未命名"));
            println!("{}\n", url);
        }
    }
}
