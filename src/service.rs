use std::path::{Path, PathBuf};
use std::process::Command;

/// 服务名称
const SERVICE_NAME: &str = "vless-rust-serve";

/// 初始化系统类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitSystem {
    Systemd,
    OpenRC,
    None,
}

/// 检查是否为 Linux 系统
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// 检测可用的初始化系统
pub fn detect_init_system() -> InitSystem {
    // 优先检测 systemd
    if Path::new("/run/systemd/system").exists() {
        return InitSystem::Systemd;
    }
    // 检测 OpenRC
    if Path::new("/run/openrc").exists() || Path::new("/sbin/openrc").exists() {
        return InitSystem::OpenRC;
    }
    InitSystem::None
}

/// 检查 systemd 是否可用
pub fn is_systemd_available() -> bool {
    detect_init_system() == InitSystem::Systemd
}

/// 检查 OpenRC 是否可用
pub fn is_openrc_available() -> bool {
    detect_init_system() == InitSystem::OpenRC
}

/// 统一的服务安装入口
pub fn install_service() -> Result<(), String> {
    match detect_init_system() {
        InitSystem::Systemd => install_systemd_service(),
        InitSystem::OpenRC => install_openrc_service(),
        InitSystem::None => Err("No supported init system found (systemd or OpenRC required)".to_string()),
    }
}

/// 统一的服务卸载入口
pub fn uninstall_service() -> Result<(), String> {
    match detect_init_system() {
        InitSystem::Systemd => uninstall_systemd_service(),
        InitSystem::OpenRC => uninstall_openrc_service(),
        InitSystem::None => Err("No supported init system found (systemd or OpenRC required)".to_string()),
    }
}

/// 获取 systemd service 文件路径
pub fn get_systemd_service_file_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or("Failed to get home directory: user home not found")?;
    Ok(home.join(".config/systemd/user").join(format!("{}.service", SERVICE_NAME)))
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

    let exe_path_str = exe_path.to_str()
        .ok_or("Executable path contains non-UTF-8 characters")?
        .to_string();

    // 获取可执行文件所在目录作为工作目录
    let work_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();

    // 配置文件路径（与可执行文件同目录）
    let config_path = work_dir.join("config.json");

    // 如果配置文件不存在，警告用户
    if !config_path.exists() {
        println!();
        println!("==========================================");
        println!("Warning: Config file not found!");
        println!("==========================================");
        println!("Expected config path: {}", config_path.display());
        println!();
        println!("The service may fail to start without a valid config file.");
        println!("Please run the server normally first to create config.json:");
        println!("  {} --no-tui", exe_path_str);
        println!();
        println!("Continuing with service installation...");
        println!();
    }

    // 获取 service 文件路径
    let service_file = get_systemd_service_file_path()?;

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

    // 等待服务启动
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 检查服务状态
    let status_output = Command::new("systemctl")
        .args(["--user", "is-active", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to check service status: {}", e))?;

    let is_active = status_output.status.success();

    println!();
    println!("==========================================");
    if is_active {
        println!("Service '{}' installed and started successfully!", SERVICE_NAME);
    } else {
        println!("Service '{}' installed, but may not be running", SERVICE_NAME);
        println!("Check status with: systemctl --user status {}", SERVICE_NAME);
    }
    println!("==========================================");
    println!();
    println!("Service Status: {}", if is_active { "active (running)" } else { "inactive" });
    println!("Service file:   {}", service_file.display());
    println!("Config path:    {}", config_path.display());
    println!("Executable:     {}", exe_path_str);
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

    // 停止服务
    let output = Command::new("systemctl")
        .args(["--user", "stop", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to stop service: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Warning: Failed to stop service: {}", stderr);
    }

    // 禁用服务
    let output = Command::new("systemctl")
        .args(["--user", "disable", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to disable service: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Warning: Failed to disable service: {}", stderr);
    }

    // 删除 service 文件
    let service_file = get_systemd_service_file_path()?;
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

// ==================== OpenRC 服务管理 ====================

/// 获取 OpenRC service 文件路径
pub fn get_openrc_service_file_path() -> PathBuf {
    PathBuf::from("/etc/init.d").join(SERVICE_NAME)
}

/// 安装并启动 OpenRC 服务 (需要 root 权限)
pub fn install_openrc_service() -> Result<(), String> {
    // 检查 Linux 系统
    if !is_linux() {
        return Err("This feature is only supported on Linux".to_string());
    }

    // 检查 OpenRC 可用性
    if !is_openrc_available() {
        return Err("OpenRC is not available on this system".to_string());
    }

    // 检查是否为 root 用户
    if !is_root() {
        return Err("OpenRC service installation requires root privileges. Please run with sudo.".to_string());
    }

    // 获取当前可执行文件路径
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?;

    let exe_path_str = exe_path.to_str()
        .ok_or("Executable path contains non-UTF-8 characters")?
        .to_string();

    // 获取可执行文件所在目录作为工作目录
    let work_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();

    // 配置文件路径（与可执行文件同目录）
    let config_path = work_dir.join("config.json");

    // 如果配置文件不存在，警告用户
    if !config_path.exists() {
        println!();
        println!("==========================================");
        println!("Warning: Config file not found!");
        println!("==========================================");
        println!("Expected config path: {}", config_path.display());
        println!();
        println!("The service may fail to start without a valid config file.");
        println!("Please run the server normally first to create config.json:");
        println!("  {} --no-tui", exe_path_str);
        println!();
        println!("Continuing with service installation...");
        println!();
    }

    // 获取 service 文件路径
    let service_file = get_openrc_service_file_path();

    // 构建 OpenRC init 脚本内容
    let script_content = format!(
        r#"#!/sbin/openrc-run

name="vless-rust-serve"
description="VLESS Rust Server"

command="{exe_path}"
command_args="--no-tui {config_path}"
command_background="yes"
pidfile="/run/${{RC_SVCNAME}}.pid"

directory="{work_dir}"

# 日志文件路径
output_log="/var/log/${{RC_SVCNAME}}.log"
error_log="/var/log/${{RC_SVCNAME}}.err"

depend() {{
    need net
    after firewall
}}

start_pre() {{
    # 确保 /run 目录存在
    checkpath --directory --owner root:root --mode 0755 /run
    # 创建日志文件并设置权限
    checkpath --file --owner root:root --mode 0644 "$output_log" "$error_log"
}}

start() {{
    ebegin "Starting $name"
    start-stop-daemon --start \
        --background \
        --make-pidfile \
        --pidfile "$pidfile" \
        --exec "$command" \
        -- $command_args >> "$output_log" 2>> "$error_log"
    eend $?
}}
"#,
        exe_path = exe_path_str,
        config_path = config_path.display(),
        work_dir = work_dir.display()
    );

    // 写入 service 文件
    std::fs::write(&service_file, script_content)
        .map_err(|e| format!("Failed to write service file: {}", e))?;

    // 设置可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&service_file, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    println!("Created OpenRC service file: {}", service_file.display());

    // 添加服务到默认运行级别
    let output = Command::new("rc-update")
        .args(["add", SERVICE_NAME, "default"])
        .output()
        .map_err(|e| format!("Failed to add service to default runlevel: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to add service: {}", stderr));
    }

    // 启动服务
    let output = Command::new("rc-service")
        .args([SERVICE_NAME, "start"])
        .output()
        .map_err(|e| format!("Failed to start service: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to start service: {}", stderr));
    }

    // 等待服务启动
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 检查服务状态
    let status_output = Command::new("rc-service")
        .args([SERVICE_NAME, "status"])
        .output()
        .map_err(|e| format!("Failed to check service status: {}", e))?;

    let is_active = status_output.status.success();

    println!();
    println!("==========================================");
    if is_active {
        println!("Service '{}' installed and started successfully!", SERVICE_NAME);
    } else {
        println!("Service '{}' installed, but may not be running", SERVICE_NAME);
        println!("Check status with: rc-service {} status", SERVICE_NAME);
    }
    println!("==========================================");
    println!();
    println!("Service Status: {}", if is_active { "active (running)" } else { "inactive" });
    println!("Service file:   {}", service_file.display());
    println!("Config path:    {}", config_path.display());
    println!("Executable:     {}", exe_path_str);
    println!();
    println!("Commands:");
    println!("  View logs:   tail -f /var/log/{}.log", SERVICE_NAME);
    println!("  Stop:        rc-service {} stop", SERVICE_NAME);
    println!("  Restart:     rc-service {} restart", SERVICE_NAME);
    println!("  Status:      rc-service {} status", SERVICE_NAME);
    println!();

    Ok(())
}

/// 卸载 OpenRC 服务
pub fn uninstall_openrc_service() -> Result<(), String> {
    // 检查 Linux 系统
    if !is_linux() {
        return Err("This feature is only supported on Linux".to_string());
    }

    // 检查是否为 root 用户
    if !is_root() {
        return Err("OpenRC service uninstallation requires root privileges. Please run with sudo.".to_string());
    }

    // 停止服务
    let output = Command::new("rc-service")
        .args([SERVICE_NAME, "stop"])
        .output();

    if let Ok(output) = output {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Failed to stop service: {}", stderr);
        }
    }

    // 从运行级别移除服务
    let output = Command::new("rc-update")
        .args(["del", SERVICE_NAME])
        .output();

    if let Ok(output) = output {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Failed to remove from runlevel: {}", stderr);
        }
    }

    // 删除 service 文件
    let service_file = get_openrc_service_file_path();
    if service_file.exists() {
        std::fs::remove_file(&service_file)
            .map_err(|e| format!("Failed to remove service file: {}", e))?;
        println!("Removed service file: {}", service_file.display());
    }

    println!("Service '{}' has been stopped and removed.", SERVICE_NAME);

    Ok(())
}

/// 检查当前用户是否为 root
fn is_root() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::getuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}
