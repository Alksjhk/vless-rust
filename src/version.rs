// 编译时生成的版本信息（使用 include! 直接包含文件内容）
include!("version_info.rs");

use crate::config::ProtocolType;

/// 服务器状态信息
#[derive(Clone)]
pub struct ServerStatusInfo {
    pub listen_addr: String,
    pub protocol: ProtocolType,
    pub user_count: usize,
    pub buffer_size: usize,
    pub ws_path: Option<String>,
    #[allow(dead_code)]
    pub tcp_nodelay: bool,
    #[allow(dead_code)]
    pub buffer_pool_size: usize,
    #[allow(dead_code)]
    pub tcp_recv_buffer: usize,
    #[allow(dead_code)]
    pub tcp_send_buffer: usize,
}

/// 格式化缓冲区大小为人类可读格式
pub fn format_buffer_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;
    const GB: usize = 1024 * MB;

    if bytes >= GB {
        format!("{}GB", bytes / GB)
    } else if bytes >= MB {
        format!("{}MB", bytes / MB)
    } else if bytes >= KB {
        format!("{}KB", bytes / KB)
    } else {
        format!("{}B", bytes)
    }
}

/// 获取协议类型显示字符串
fn protocol_string(protocol: ProtocolType) -> &'static str {
    match protocol {
        ProtocolType::Tcp => "TCP",
        ProtocolType::WebSocket => "WebSocket",
    }
}

/// 打印服务器状态风格横幅（模板7）
///
/// # 参数
/// * `status_info` - 服务器状态信息
///
/// # 示例输出
/// ```
/// ╭──────────────────────────────────────────────╮
/// │  █ VLESS Rust Server v1.6.6                  │
/// │  █ by HuY                                    │
/// │  █ High-performance VLESS protocol server    │
/// │                                              │
/// │  [█] Listening on 0.0.0.0:8443              │
/// │  [█] Protocol: TCP                           │
/// │  [█] Users: 2                                │
/// │  [█] Buffer: 128KB                           │
/// ╰──────────────────────────────────────────────╯
/// ```
pub fn print_banner_with_status(status_info: &ServerStatusInfo) {
    let product_line = format!("█ {} v{}", VERSION_INFO.product_name, VERSION_INFO.version);
    let author_line = format!("█ by {}", VERSION_INFO.author);
    let desc_line = format!("█ {}", VERSION_INFO.file_description);
    let copyright_line = format!("█ {}", VERSION_INFO.legal_copyright);

    let protocol_str = protocol_string(status_info.protocol);
    let buffer_str = format_buffer_size(status_info.buffer_size);

    // 构建状态行
    let mut status_lines = Vec::new();

    // 监听地址行
    status_lines.push(format!("[█] Listening on {}", status_info.listen_addr));

    // 协议类型行
    status_lines.push(format!("[█] Protocol: {}", protocol_str));

    // WebSocket 路径行（如果适用，显示在 Protocol 下方）
    if let Some(ref ws_path) = status_info.ws_path {
        status_lines.push(format!("[█] WS Path: {}", ws_path));
    }

    // 用户数量行
    status_lines.push(format!("[█] Users: {}", status_info.user_count));

    // 缓冲区大小行
    status_lines.push(format!("[█] Buffer: {}", buffer_str));

    // 确定框的宽度（根据最长行）
    let mut max_width = product_line.len();
    max_width = max_width.max(author_line.len());
    max_width = max_width.max(desc_line.len());
    max_width = max_width.max(copyright_line.len());
    for line in &status_lines {
        max_width = max_width.max(line.len() + 2); // +2 for "│ " prefix
    }
    max_width = max_width.max(46); // 最小宽度

    // 打印横幅
    println!("╭─{}─╮", "─".repeat(max_width - 2));
    println!("│  {:width$}  │", "", width = max_width - 4);

    // 产品信息
    println!("│  {}  │", pad_right(&product_line, max_width - 4));
    println!("│  {}  │", pad_right(&author_line, max_width - 4));
    println!("│  {}  │", pad_right(&desc_line, max_width - 4));
    println!("│  {}  │", pad_right(&copyright_line, max_width - 4));

    // 空行分隔
    println!("│  {:width$}  │", "", width = max_width - 4);

    // 状态信息
    for line in &status_lines {
        println!("│  {}  │", pad_right(line, max_width - 4));
    }

    println!("│  {:width$}  │", "", width = max_width - 4);
    println!("╰─{}─╯", "─".repeat(max_width - 2));
    println!();
}

/// 右填充字符串到指定宽度
fn pad_right(s: &str, width: usize) -> String {
    format!("{:<width$}", s, width = width)
}
