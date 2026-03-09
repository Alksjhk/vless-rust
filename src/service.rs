use std::path::{Path, PathBuf};
use std::process::Command;

use crate::wizard::ConfigWizard;
use crate::atomic_write;

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

/// 检查配置文件路径是否可写入
fn check_config_file_writable(config_path: &Path) -> Result<(), String> {
    if config_path.exists() {
        // 配置文件已存在，检查是否可写入
        if !atomic_write::is_file_writable(config_path) {
            return Err(format!(
                "Config file '{}' is busy or locked. Please stop the service first:\n\
                 systemctl --user stop {}\n\
                 or\n\
                 rc-service {} stop",
                config_path.display(),
                SERVICE_NAME,
                SERVICE_NAME
            ));
        }
    } else {
        // 配置文件不存在，检查父目录是否可写入
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                return Err(format!(
                    "Parent directory does not exist: {}",
                    parent.display()
                ));
            }
        }
    }
    Ok(())
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

    // 检查配置文件路径冲突
    check_config_path_conflict(&exe_path, &config_path)?;

    // 检查配置文件是否可写入
    check_config_file_writable(&config_path)?;

    // 检查服务是否正在运行
    if is_systemd_service_running() {
        return Err(format!(
            "Service '{}' is currently running. Please stop the service before installing:\n\
             systemctl --user stop {}",
            SERVICE_NAME, SERVICE_NAME
        ));
    }

    // 如果配置文件不存在，先运行配置向导
    if !config_path.exists() {
        println!();
        println!("==========================================");
        println!("Config file not found!");
        println!("==========================================");
        println!("Expected config path: {}", config_path.display());
        println!();
        println!("Starting configuration wizard to create config file...");
        println!();

        // 运行配置向导
        let config = ConfigWizard::run()
            .map_err(|e| format!("Configuration wizard failed: {}", e))?;

        // 保存配置文件（使用原子写入）
        let json = config.to_json()
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        atomic_write::atomic_write_file_with_perms(&config_path, &json, 0o600)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        println!();
        println!("✓ Config file created: {}", config_path.display());
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

    // 备份现有的 service 文件（如果存在）
    // 使用时间戳生成唯一的备份文件名，避免覆盖之前的备份
    let backup_file = if service_file.exists() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_path = service_file.with_extension(format!("service.backup.{}", timestamp));
        if let Err(e) = std::fs::copy(&service_file, &backup_path) {
            eprintln!("Warning: Failed to backup existing service file: {}", e);
            None
        } else {
            println!("Backup created: {}", backup_path.display());
            Some(backup_path)
        }
    } else {
        None
    };

    // 构建 service 文件内容
    let service_content = format!(
        r#"[Unit]
Description=VLESS Rust Server
After=network.target

[Service]
Type=simple
WorkingDirectory={work_dir}
ExecStart={exe_path} {config_path} --no-tui
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

    // 使用原子写入 service 文件
    if let Err(e) = atomic_write::atomic_write_file(&service_file, &service_content) {
        // 写入失败，尝试恢复备份
        if let Some(ref backup_path) = backup_file {
            if backup_path.exists() {
                if let Err(restore_err) = std::fs::copy(backup_path, &service_file) {
                    eprintln!("Error: Failed to restore backup: {}", restore_err);
                    eprintln!("Backup file preserved at: {}", backup_path.display());
                } else {
                    eprintln!("Restored backup service file after write failure");
                }
            }
        }
        // 注意：不删除配置文件，因为配置文件本身是完整有效的
        // 用户可以手动修复 service 文件或重新运行安装命令
        return Err(format!("Failed to write service file: {}. Config file preserved at: {}", e, config_path.display()));
    }

    // 写入成功，保留备份文件（不删除），用户可以手动清理
    if let Some(ref backup_path) = backup_file {
        println!("Backup preserved at: {} (can be deleted manually if service works correctly)", backup_path.display());
    }

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

    // 启用 lingering（让用户服务在系统启动时自动运行，而不需要用户登录）
    // 这是实现开机自启动的关键步骤
    let linger_output = Command::new("loginctl")
        .args(["enable-linger"])
        .output();

    if let Ok(output) = &linger_output {
        if output.status.success() {
            println!("Enabled lingering for user services (auto-start on boot)");
        }
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

    // 检查配置文件路径冲突
    check_config_path_conflict(&exe_path, &config_path)?;

    // 检查配置文件是否可写入
    check_config_file_writable(&config_path)?;

    // 检查服务是否正在运行
    if is_openrc_service_running() {
        return Err(format!(
            "Service '{}' is currently running. Please stop the service before installing:\n\
             rc-service {} stop",
            SERVICE_NAME, SERVICE_NAME
        ));
    }

    // 如果配置文件不存在，先运行配置向导
    if !config_path.exists() {
        println!();
        println!("==========================================");
        println!("Config file not found!");
        println!("==========================================");
        println!("Expected config path: {}", config_path.display());
        println!();
        println!("Starting configuration wizard to create config file...");
        println!();

        // 运行配置向导
        let config = ConfigWizard::run()
            .map_err(|e| format!("Configuration wizard failed: {}", e))?;

        // 保存配置文件（使用原子写入）
        let json = config.to_json()
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        atomic_write::atomic_write_file_with_perms(&config_path, &json, 0o600)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        println!();
        println!("✓ Config file created: {}", config_path.display());
        println!();
    }

    // 获取 service 文件路径
    let service_file = get_openrc_service_file_path();

    // 检查 service 文件路径是否与当前可执行文件冲突
    // 如果用户直接将二进制文件放在 /etc/init.d/ 目录下运行，会导致覆盖正在运行的程序
    if exe_path == service_file {
        return Err(format!(
            "Cannot install service: executable is located at the service file path.\n\
             Please move the executable to a different location (e.g., /usr/local/bin/{}):\n\
             mv {} /usr/local/bin/{}\n\
             Then run: /usr/local/bin/{}/{} --init",
            SERVICE_NAME, exe_path_str, SERVICE_NAME, SERVICE_NAME, SERVICE_NAME
        ));
    }

    // 备份现有的 service 文件（如果存在）
    // 使用时间戳生成唯一的备份文件名，避免覆盖之前的备份
    let backup_file = if service_file.exists() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_path = service_file.with_extension(format!("backup.{}", timestamp));
        if let Err(e) = std::fs::copy(&service_file, &backup_path) {
            eprintln!("Warning: Failed to backup existing service file: {}", e);
            None
        } else {
            println!("Backup created: {}", backup_path.display());
            Some(backup_path)
        }
    } else {
        None
    };

    // 构建 OpenRC init 脚本内容
    let script_content = format!(
        r#"#!/sbin/openrc-run

name="vless-rust-serve"
description="VLESS Rust Server"

command="{exe_path}"
command_args="{config_path} --no-tui"
command_background="yes"
pidfile="/run/${{RC_SVCNAME}}.pid"

directory="{work_dir}"

# 日志文件路径
output_log="/var/log/${{RC_SVCNAME}}.log"
error_log="/var/log/${{RC_SVCNAME}}.err"

depend() {{
    need net
    after firewall
    after network-online
    want network-online
}}

start_pre() {{
    # 确保 /run 目录存在
    checkpath --directory --owner root:root --mode 0755 /run
    # 确保日志目录存在
    checkpath --directory --owner root:root --mode 0755 /var/log
    # 创建日志文件并设置权限
    checkpath --file --owner root:root --mode 0644 "$output_log" "$error_log"
}}
"#,
        exe_path = exe_path_str,
        config_path = config_path.display(),
        work_dir = work_dir.display()
    );

    // 使用原子写入 service 文件
    if let Err(e) = atomic_write::atomic_write_file(&service_file, &script_content) {
        // 写入失败，尝试恢复备份
        if let Some(ref backup_path) = backup_file {
            if backup_path.exists() {
                if let Err(restore_err) = std::fs::copy(backup_path, &service_file) {
                    eprintln!("Error: Failed to restore backup: {}", restore_err);
                    eprintln!("Backup file preserved at: {}", backup_path.display());
                } else {
                    eprintln!("Restored backup service file after write failure");
                }
            }
        }
        // 注意：不删除配置文件，因为配置文件本身是完整有效的
        // 用户可以手动修复 service 文件或重新运行安装命令
        return Err(format!("Failed to write service file: {}. Config file preserved at: {}", e, config_path.display()));
    }

    // 设置可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&service_file, std::fs::Permissions::from_mode(0o755)) {
            // 权限设置失败，恢复备份
            if let Some(ref backup_path) = backup_file {
                if backup_path.exists() {
                    if let Err(restore_err) = std::fs::copy(backup_path, &service_file) {
                        eprintln!("Error: Failed to restore backup after permission error: {}", restore_err);
                        eprintln!("Backup file preserved at: {}", backup_path.display());
                    } else {
                        eprintln!("Restored backup after permission error");
                    }
                }
            }
            // 注意：不删除配置文件，因为配置文件本身是完整有效的
            return Err(format!("Failed to set permissions: {}. Config file preserved at: {}", e, config_path.display()));
        }
    }

    // 写入成功，保留备份文件（不删除），用户可以手动清理
    if let Some(ref backup_path) = backup_file {
        println!("Backup preserved at: {} (can be deleted manually if service works correctly)", backup_path.display());
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
    println!("  Disable:     rc-update del {} default", SERVICE_NAME);
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

/// 检查 systemd 服务是否正在运行
fn is_systemd_service_running() -> bool {
    let output = Command::new("systemctl")
        .args(["--user", "is-active", SERVICE_NAME])
        .output();

    if let Ok(output) = output {
        output.status.success()
    } else {
        false
    }
}

/// 检查 OpenRC 服务是否正在运行
fn is_openrc_service_running() -> bool {
    let output = Command::new("rc-service")
        .args([SERVICE_NAME, "status"])
        .output();

    if let Ok(output) = output {
        output.status.success()
    } else {
        false
    }
}

/// 检查配置文件路径是否与可执行文件冲突
fn check_config_path_conflict(exe_path: &Path, config_path: &Path) -> Result<(), String> {
    // 规范化路径进行比较（解析符号链接和相对路径）
    let exe_canonical = exe_path.canonicalize().ok();
    let config_canonical = config_path.canonicalize().ok();

    // 检查配置文件路径是否与可执行文件相同
    if exe_path == config_path {
        return Err(format!(
            "Config file path conflicts with executable path: {}\n\
             The config file cannot be the same as the executable.\n\
             Please move the executable to a different location.",
            config_path.display()
        ));
    }

    // 检查规范化的路径是否相同（处理符号链接）
    if let (Some(exe_canon), Some(config_canon)) = (&exe_canonical, &config_canonical) {
        if exe_canon == config_canon {
            return Err(format!(
                "Config file path resolves to the same file as the executable (possibly via symlink):\n\
                 Executable: {}\n\
                 Config path: {}\n\
                 Please specify a different config file location.",
                exe_canon.display(),
                config_canon.display()
            ));
        }
    }

    // 检查配置文件路径是否已经存在且是可执行文件
    // 这可以防止覆盖任何已存在的二进制文件
    if config_path.exists() {
        // 尝试检查文件是否是可执行文件
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(config_path) {
                let mode = metadata.permissions().mode();
                // 检查是否有执行权限（用户、组或其他）
                if mode & 0o111 != 0 {
                    return Err(format!(
                        "Config file path '{}' already exists and appears to be an executable.\n\
                         Refusing to overwrite an executable file.",
                        config_path.display()
                    ));
                }
            }
        }

        // 跨平台检查：如果文件不是普通文件（可能是目录、设备等）
        if !config_path.is_file() {
            return Err(format!(
                "Config file path '{}' exists but is not a regular file.\n\
                 Cannot create config file at this location.",
                config_path.display()
            ));
        }
    }

    Ok(())
}
