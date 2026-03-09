mod protocol;
mod server;
mod config;
mod http;
mod wizard;
mod ws;
mod version;
mod tui;
mod public_ip;
mod vless_link;
mod socket;
mod tcp;
mod api;
mod service;

use anyhow::Result;
use config::Config;
use server::{ServerConfig, VlessServer};
use std::env;
use tokio::signal;
use tracing::{info, error};
use std::sync::mpsc;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::util::SubscriberInitExt;

// 使用 mimalloc 作为全局内存分配器，提升内存分配性能
// musl 目标不使用 mimalloc，因为与静态链接存在兼容性问题（__memcpy_chk、__memset_chk）
#[cfg(not(target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    // 读取命令行参数
    let args: Vec<String> = env::args().collect();

    // 检查 --init 参数（安装系统服务）
    if args.iter().any(|a| a == "--init") {
        match service::install_service() {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    // 检查 --remove 参数（卸载系统服务）
    if args.iter().any(|a| a == "--remove") {
        match service::uninstall_service() {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    // 读取配置文件路径（重用上面的 args 变量）
    let config_path = args.iter()
        .find(|p| !p.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "config.json".to_string());

    // 检查是否禁用 TUI（可通过 --no-tui 参数或环境变量）
    let use_tui = !args.iter().any(|a| a == "--no-tui")
        && env::var("DISABLE_TUI").is_err();

    // 加载配置（不输出日志）
    let (config, config_messages) = match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            let config = Config::from_json(&content)?;
            (config, vec![format!("Loading config from {}", config_path)])
        }
        Err(_) => {
            let mut messages = vec![
                format!("Config file not found at {}", config_path),
                "Starting configuration wizard...".to_string()
            ];
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
                    let mut perms_set = false;
                    if let Err(e) = std::fs::set_permissions(&config_path, perms) {
                        eprintln!("Failed to set config file permissions: {}", e);
                    } else {
                        perms_set = true;
                    }
                    if perms_set {
                        messages.push(format!("Config file permissions set to 600 (rw-------)"));
                    }
                }
            }

            messages.push(format!("Config saved to {}", config_path));
            (config, messages)
        }
    };

    // 获取公网 IP（用于生成 VLESS 链接）
    let public_ip = match public_ip::fetch_public_ip_with_timeout(5).await {
        Some(ip) => {
            eprintln!("Public IP detected: {} (from {})", ip.ip, ip.source);
            Some(ip.ip)
        }
        None => {
            eprintln!("Warning: Failed to detect public IP, VLESS links will use listen address");
            None
        }
    };

    // 打印服务器状态横幅（模板7）
    let listen_addr = format!("{}:{}", config.server.listen, config.server.port);
    let ws_path = if config.server.protocol == config::ProtocolType::WebSocket {
        Some(config.server.ws_path.clone())
    } else {
        None
    };

    let server_status = version::ServerStatusInfo {
        listen_addr,
        protocol: config.server.protocol,
        user_count: config.users.len(),
        buffer_size: config.performance.buffer_size,
        ws_path,
        tcp_nodelay: config.performance.tcp_nodelay,
        buffer_pool_size: config.performance.buffer_pool_size,
        tcp_recv_buffer: config.performance.tcp_recv_buffer,
        tcp_send_buffer: config.performance.tcp_send_buffer,
        public_ip: public_ip.clone(),
    };

    if use_tui {
        // 创建日志通信通道
        let (log_tx, log_rx) = mpsc::channel();

        // 初始化日志到 TUI
        init_tui_logging(log_tx);

        // 创建停止标志
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        // 在后台线程运行服务器
        let config_clone = config.clone();
        let shutdown_flag_clone = shutdown_flag.clone();
        let public_ip_clone = public_ip.clone();
        let server_handle = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime - this is a fatal error");
            let _ = rt.block_on(async {
                run_server_with_flag(config_clone, shutdown_flag_clone, public_ip_clone).await
            });
        });

        // 在主线程运行 TUI
        let result = run_tui_with_channel(log_rx, &server_status);

        // 通知服务器停止
        shutdown_flag.store(true, Ordering::SeqCst);

        // 等待服务器线程结束
        let _ = server_handle.join();

        result.map_err(|e| anyhow::anyhow!("{}", e))
    } else {
        // 传统模式：直接打印横幅
        version::print_banner_with_status(&server_status);

        // 初始化日志（在横幅之后）
        tracing_subscriber::fmt::init();

        // 输出配置加载信息
        for msg in config_messages {
            info!("{}", msg);
        }

        info!("Server configuration loaded:");
        info!("  Listen: {}:{}", config.server.listen, config.server.port);
        info!("  Protocol: {:?}", config.server.protocol);
        if config.server.protocol == config::ProtocolType::WebSocket {
            info!("  WS Path: {}", config.server.ws_path);
        }
        info!("  Users: {}", config.users.len());

        run_server(config, public_ip).await.map_err(|e| e.into())
    }
}

/// 运行服务器
async fn run_server(config: Config, public_ip: Option<String>) -> Result<()> {
    run_server_with_flag(config, Arc::new(AtomicBool::new(false)), public_ip).await
}

/// 运行服务器（带停止标志）
async fn run_server_with_flag(config: Config, shutdown_flag: Arc<AtomicBool>, public_ip: Option<String>) -> Result<()> {
    // 创建服务器配置
    let bind_addr = config.bind_addr()?;
    let port = config.server.port;

    // 添加用户及邮箱信息
    let mut server_config = ServerConfig::new(
        bind_addr,
        config.server.protocol,
        config.server.ws_path,
        public_ip,
        port,
    );

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

    // 创建优雅关闭信号处理器
    let shutdown = async {
        // 等待 SIGINT (Ctrl+C) 或 SIGTERM
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigint = signal(SignalKind::interrupt()).unwrap();
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sigint.recv() => {
                    info!("Received SIGINT, initiating graceful shutdown...");
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown...");
                }
            }
        }
        #[cfg(not(unix))]
        {
            // Windows: 只监听 Ctrl+C
            let _ = signal::ctrl_c().await;
            info!("Received Ctrl+C, initiating graceful shutdown...");
        }
    };

    // 定期检查停止标志
    let flag_check = async {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if shutdown_flag.load(Ordering::SeqCst) {
                info!("Shutdown requested...");
                break;
            }
        }
    };

    // 运行服务器直到收到关闭信号
    tokio::select! {
        result = server.run() => {
            if let Err(e) = result {
                error!("Server error: {}", e);
                return Err(e);
            }
        }
        _ = shutdown => {
            info!("Shutting down server...");
        }
        _ = flag_check => {
            info!("Shutting down server...");
        }
    }

    info!("Server stopped");
    Ok(())
}

/// 初始化 TUI 日志系统
fn init_tui_logging(log_tx: mpsc::Sender<tui::LogEntry>) {
    use tracing_subscriber::layer::SubscriberExt;

    let layer = tui::TuiLayer::new(log_tx);

    tracing_subscriber::registry()
        .with(layer)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::sink))
        .init();
}

/// 运行 TUI 并接收日志
fn run_tui_with_channel(
    log_rx: mpsc::Receiver<tui::LogEntry>,
    status_info: &version::ServerStatusInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    use ratatui::{
        backend::CrosstermBackend,
        crossterm::{
            event::{self, Event, KeyCode, KeyEventKind},
            execute,
            terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        },
        layout::{Alignment, Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::Line,
        widgets::{Block, Borders, Paragraph, Wrap},
        Terminal,
    };
    use std::time::Duration;

    const MAX_LOG_ENTRIES: usize = 1000;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 构建头部行
    let header_lines = build_header_lines(status_info);
    let header_height = header_lines.len() as u16;

    let mut log_entries: Vec<tui::LogEntry> = Vec::new();
    let mut scroll_offset: usize = 0;
    let mut auto_scroll = true; // 自动滚动标志

    let result = loop {
        // 接收新日志
        while let Ok(entry) = log_rx.try_recv() {
            log_entries.push(entry);
            if log_entries.len() > MAX_LOG_ENTRIES {
                log_entries.remove(0);
            }
            // 如果启用自动滚动，调整偏移量以显示最新日志
            if auto_scroll {
                // 设置为 len() - log_height 以确保显示最新日志（在绘制时会自动限制范围）
                scroll_offset = log_entries.len().saturating_sub(1);
            }
        }

        // 处理事件
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                break Ok(());
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                auto_scroll = false; // 用户手动滚动时禁用自动滚动
                                // 计算最大滚动位置
                                let log_height = terminal.size()?.height.saturating_sub(header_height + 2) as usize;
                                let max_scroll = log_entries.len().saturating_sub(log_height);
                                scroll_offset = scroll_offset.saturating_add(1).min(max_scroll);
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                auto_scroll = false;
                                scroll_offset = scroll_offset.saturating_sub(1);
                            }
                            KeyCode::PageDown => {
                                auto_scroll = false;
                                let log_height = terminal.size()?.height.saturating_sub(header_height + 2) as usize;
                                let max_scroll = log_entries.len().saturating_sub(log_height);
                                scroll_offset = scroll_offset.saturating_add(10).min(max_scroll);
                            }
                            KeyCode::PageUp => {
                                auto_scroll = false;
                                scroll_offset = scroll_offset.saturating_sub(10);
                            }
                            KeyCode::Home => {
                                auto_scroll = false;
                                scroll_offset = 0;
                            }
                            KeyCode::End => {
                                auto_scroll = true; // End键重新启用自动滚动
                                // 设置为 len() - log_height 以确保显示最新日志
                                scroll_offset = log_entries.len().saturating_sub(1);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        // 绘制界面
        terminal.draw(|f| {
            let size = f.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(header_height), Constraint::Min(0)])
                .split(size);

            // 绘制头部
            let header_text: Vec<Line> = header_lines.iter()
                .map(|line| Line::from(line.as_str()))
                .collect();

            let header = Paragraph::new(header_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan))
                        .title(" Server Status ")
                        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                )
                .alignment(Alignment::Left);

            f.render_widget(header, chunks[0]);

            // 计算日志区域高度
            let log_height = chunks[1].height.saturating_sub(2) as usize; // 减去边框

            // 绘制日志区域
            let log_lines: Vec<Line> = log_entries.iter().map(|entry| {
                let level_color = match entry.level {
                    tracing::Level::ERROR => Color::Red,
                    tracing::Level::WARN => Color::Yellow,
                    tracing::Level::INFO => Color::Green,
                    tracing::Level::DEBUG => Color::Cyan,
                    tracing::Level::TRACE => Color::Gray,
                };

                let level_str = match entry.level {
                    tracing::Level::ERROR => "ERROR",
                    tracing::Level::WARN => "WARN ",
                    tracing::Level::INFO => "INFO ",
                    tracing::Level::DEBUG => "DEBUG",
                    tracing::Level::TRACE => "TRACE",
                };

                Line::from(vec![
                    ratatui::text::Span::styled(
                        format!("[{}] ", entry.timestamp),
                        Style::default().fg(Color::DarkGray),
                    ),
                    ratatui::text::Span::styled(
                        level_str,
                        Style::default().fg(level_color).add_modifier(Modifier::BOLD),
                    ),
                    ratatui::text::Span::raw(" "),
                    ratatui::text::Span::styled(
                        entry.message.clone(),
                        Style::default().fg(Color::White),
                    ),
                ])
            }).collect();

            // 计算实际滚动偏移：确保最新日志显示在底部
            // 当 scroll_offset >= log_lines.len() 时，显示最新日志（底部对齐效果）
            let actual_scroll = if log_lines.len() > log_height {
                scroll_offset.min(log_lines.len() - log_height)
            } else {
                0
            };

            let log_paragraph = Paragraph::new(log_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue))
                        .title(" Logs (Press q to quit, use arrow keys to scroll) ")
                        .title_style(Style::default().fg(Color::Blue))
                )
                .wrap(Wrap { trim: false })
                .scroll((actual_scroll as u16, 0));

            f.render_widget(log_paragraph, chunks[1]);
        })?;
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    result
}

/// 构建头部显示行（纯文本，无边框）
fn build_header_lines(status_info: &version::ServerStatusInfo) -> Vec<String> {
    use crate::version::VERSION_INFO;
    use crate::config::ProtocolType;

    let product_line = format!("█ {} v{}", VERSION_INFO.product_name, VERSION_INFO.version);
    let author_line = format!("█ by {}", VERSION_INFO.author);
    let desc_line = format!("█ {}", VERSION_INFO.file_description);
    let copyright_line = format!("█ {}", VERSION_INFO.legal_copyright);

    let protocol_str = match status_info.protocol {
        ProtocolType::Tcp => "TCP",
        ProtocolType::WebSocket => "WebSocket",
    };

    // 使用公共函数格式化缓冲区大小
    let buffer_str = version::format_buffer_size(status_info.buffer_size);

    let mut lines = Vec::new();
    lines.push(product_line);
    lines.push(author_line);
    lines.push(desc_line);
    lines.push(copyright_line);
    lines.push(String::new()); // 空行分隔
    
    // 公网 IP 行（如果可用）
    if let Some(ref public_ip) = status_info.public_ip {
        lines.push(format!("[█] Public IP: {}", public_ip));
    }
    
    lines.push(format!("[█] Listening on {}", status_info.listen_addr));
    lines.push(format!("[█] Protocol: {}", protocol_str));

    // WebSocket 路径（如果适用，显示在 Protocol 下方）
    if let Some(ref ws_path) = status_info.ws_path {
        lines.push(format!("[█] WS Path: {}", ws_path));
    }

    lines.push(format!("[█] Users: {}", status_info.user_count));
    lines.push(format!("[█] Buffer: {}", buffer_str));

    lines
}
