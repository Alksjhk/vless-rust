use std::path::{Path, PathBuf};
use std::process::Command;

/// 服务名称
const SERVICE_NAME: &str = "vless-rust-serve";

/// 检查是否为 Linux 系统
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// 检查 systemd 是否可用
pub fn is_systemd_available() -> bool {
    // 检查 /run/systemd/system 目录是否存在
    Path::new("/run/systemd/system").exists()
}

/// 获取 systemd service 文件路径
pub fn get_service_file_path() -> PathBuf {
    let home = dirs::home_dir().expect("Failed to get home directory");
    home.join(".config/systemd/user").join(format!("{}.service", SERVICE_NAME))
}

/// 安装并启动 systemd 服务
pub fn install_systemd_service() -> Result<(), String> {
    // 检查 Linux 系统
    if !is_linux() {
        return Err("This feature is only supported on Linux".to_string());
    }

    // 检查 systemd 可用性
    if !is_systemd_available() {
        return Err("systemd is not available on this system".to_string());
    }

    // 获取当前可执行文件路径
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?;

    let exe_path_str = exe_path.to_string_lossy().to_string();

    // 获取可执行文件所在目录作为工作目录
    let work_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();

    // 配置文件路径（与可执行文件同目录）
    let config_path = work_dir.join("config.json");

    // 如果配置文件不存在，提示用户
    if !config_path.exists() {
        println!("Config file not found, please run the server normally first to create config.json");
        println!("Expected config path: {}", config_path.display());
    }

    // 获取 service 文件路径
    let service_file = get_service_file_path();

    // 确保 systemd user 目录存在
    if let Some(parent) = service_file.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create systemd directory: {}", e))?;
        }
    }

    // 构建 service 文件内容
    let service_content = format!(
        r#"[Unit]
Description=VLESS Rust Server
After=network.target

[Service]
Type=simple
WorkingDirectory={work_dir}
ExecStart={exe_path} --no-tui {config_path}
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
"#,
        work_dir = work_dir.display(),
        exe_path = exe_path_str,
        config_path = config_path.display()
    );

    // 写入 service 文件
    std::fs::write(&service_file, service_content)
        .map_err(|e| format!("Failed to write service file: {}", e))?;

    println!("Created systemd service file: {}", service_file.display());

    // 重新加载 systemd 守护进程
    let output = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output()
        .map_err(|e| format!("Failed to run systemctl daemon-reload: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("daemon-reload failed: {}", stderr));
    }

    // 启用并启动服务
    let output = Command::new("systemctl")
        .args(["--user", "enable", "--now", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to enable service: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to start service: {}", stderr));
    }

    println!();
    println!("==========================================");
    println!("Service '{}' installed and started successfully!", SERVICE_NAME);
    println!("==========================================");
    println!();
    println!("Service file: {}", service_file.display());
    println!("Config path:  {}", config_path.display());
    println!("Executable:   {}", exe_path_str);
    println!();
    println!("Commands:");
    println!("  View logs:   journalctl --user -u {} -f", SERVICE_NAME);
    println!("  Stop:        systemctl --user stop {}", SERVICE_NAME);
    println!("  Restart:     systemctl --user restart {}", SERVICE_NAME);
    println!("  Status:      systemctl --user status {}", SERVICE_NAME);
    println!();

    Ok(())
}

/// 卸载 systemd 服务
pub fn uninstall_systemd_service() -> Result<(), String> {
    // 检查 Linux 系统
    if !is_linux() {
        return Err("This feature is only supported on Linux".to_string());
    }

    // 停止并禁用服务
    let _output = Command::new("systemctl")
        .args(["--user", "stop", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to stop service: {}", e))?;

    let _ = Command::new("systemctl")
        .args(["--user", "disable", SERVICE_NAME])
        .output();

    // 删除 service 文件
    let service_file = get_service_file_path();
    if service_file.exists() {
        std::fs::remove_file(&service_file)
            .map_err(|e| format!("Failed to remove service file: {}", e))?;
        println!("Removed service file: {}", service_file.display());
    }

    // 重新加载 systemd
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    println!("Service '{}' has been stopped and removed.", SERVICE_NAME);

    Ok(())
}
