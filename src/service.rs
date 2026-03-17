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

/// 安装上下文，封装共用安装逻辑
struct InstallContext {
    pub exe_path: PathBuf,
    pub exe_path_str: String,
    pub work_dir: PathBuf,
    pub config_path: PathBuf,
}

impl InstallContext {
    fn new() -> Result<Self, String> {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;

        let exe_path_str = exe_path.to_str()
            .ok_or("Executable path contains non-UTF-8 characters")?
            .to_string();

        let work_dir = exe_path
            .parent()
            .ok_or("Failed to get executable directory")?
            .to_path_buf();

        let config_path = work_dir.join("config.json");

        Ok(Self { exe_path, exe_path_str, work_dir, config_path })
    }

    fn prepare_config(&self) -> Result<(), String> {
        check_config_path_conflict(&self.exe_path, &self.config_path)?;
        check_config_file_writable(&self.config_path)?;

        if !self.config_path.exists() {
            println!("\n==========================================");
            println!("Config file not found!");
            println!("==========================================");
            println!("Expected config path: {}", self.config_path.display());
            println!("\nStarting configuration wizard to create config file...\n");

            let config = ConfigWizard::run()
                .map_err(|e| format!("Configuration wizard failed: {}", e))?;

            let json = config.to_json()
                .map_err(|e| format!("Failed to serialize config: {}", e))?;

            atomic_write::atomic_write_file_with_perms(&self.config_path, &json, 0o600)
                .map_err(|e| format!("Failed to write config file: {}", e))?;

            println!("\n✓ Config file created: {}\n", self.config_path.display());
        }

        Ok(())
    }

    fn backup_file(&self, file: &Path) -> Option<PathBuf> {
        if !file.exists() { return None; }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let backup_path = file.with_extension(format!("service.backup.{}", timestamp));

        match std::fs::copy(file, &backup_path) {
            Ok(_) => {
                println!("Backup created: {}", backup_path.display());
                Some(backup_path)
            }
            Err(e) => {
                eprintln!("Warning: Failed to backup: {}", e);
                None
            }
        }
    }

    fn restore_backup(&self, backup: Option<&PathBuf>, original: &Path) {
        if let Some(backup_path) = backup {
            if backup_path.exists() {
                if let Err(e) = std::fs::copy(backup_path, original) {
                    eprintln!("Error restoring backup: {}", e);
                }
            }
        }
    }
}

/// 检查是否为 Linux 系统
pub fn is_linux() -> bool { cfg!(target_os = "linux") }

/// 检测可用的初始化系统
pub fn detect_init_system() -> InitSystem {
    if Path::new("/run/systemd/system").exists() {
        return InitSystem::Systemd;
    }
    if Path::new("/run/openrc").exists() || Path::new("/sbin/openrc").exists() {
        return InitSystem::OpenRC;
    }
    InitSystem::None
}

pub fn is_systemd_available() -> bool { detect_init_system() == InitSystem::Systemd }
pub fn is_openrc_available() -> bool { detect_init_system() == InitSystem::OpenRC }

/// 统一的服务安装入口
pub fn install_service() -> Result<(), String> {
    match detect_init_system() {
        InitSystem::Systemd => install_systemd_service(),
        InitSystem::OpenRC => install_openrc_service(),
        InitSystem::None => Err("No supported init system found".to_string()),
    }
}

/// 统一的服务卸载入口
pub fn uninstall_service() -> Result<(), String> {
    match detect_init_system() {
        InitSystem::Systemd => uninstall_systemd_service(),
        InitSystem::OpenRC => uninstall_openrc_service(),
        InitSystem::None => Err("No supported init system found".to_string()),
    }
}

fn check_config_file_writable(config_path: &Path) -> Result<(), String> {
    if config_path.exists() {
        if !atomic_write::is_file_writable(config_path) {
            return Err(format!(
                "Config file '{}' is busy or locked. Stop the running service first.",
                config_path.display()
            ));
        }
    } else if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            return Err(format!("Parent directory does not exist: {}", parent.display()));
        }
    }
    Ok(())
}

pub fn get_systemd_service_file_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or("Failed to get home directory")?;
    Ok(home.join(".config/systemd/user").join(format!("{}.service", SERVICE_NAME)))
}

fn print_summary(name: &str, service_file: &Path, config_path: &Path, exe: &str, active: bool, init: InitSystem) {
    println!("\n==========================================");
    println!("Service '{}' {}", name, if active { "installed and started!" } else { "installed." });
    println!("==========================================");
    println!("Service file: {}", service_file.display());
    println!("Config:       {}", config_path.display());
    println!("Executable:   {}", exe);
    println!("\nCommands:");
    match init {
        InitSystem::Systemd => {
            println!("  Stop:    systemctl --user stop {}", name);
            println!("  Restart: systemctl --user restart {}", name);
            println!("  Status:  systemctl --user status {}", name);
        }
        InitSystem::OpenRC => {
            println!("  Stop:    rc-service {} stop", name);
            println!("  Restart: rc-service {} restart", name);
            println!("  Status:  rc-service {} status", name);
        }
        _ => {}
    }
    println!();
}

// ==================== Systemd 服务管理 ====================

pub fn install_systemd_service() -> Result<(), String> {
    if !is_linux() { return Err("Linux only".to_string()); }
    if !is_systemd_available() { return Err("systemd not available".to_string()); }

    let ctx = InstallContext::new()?;

    if is_systemd_service_running() {
        return Err(format!("Service '{}' running. Stop first: systemctl --user stop {}", SERVICE_NAME, SERVICE_NAME));
    }

    ctx.prepare_config()?;

    let service_file = get_systemd_service_file_path()?;
    if let Some(parent) = service_file.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    let backup = ctx.backup_file(&service_file);

    let content = format!(
        r#"[Unit]
Description=VLESS Rust Server
After=network.target

[Service]
Type=simple
WorkingDirectory={work_dir}
ExecStart={exe} {config} --no-tui
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
"#,
        work_dir = ctx.work_dir.display(),
        exe = ctx.exe_path_str,
        config = ctx.config_path.display()
    );

    if let Err(e) = atomic_write::atomic_write_file(&service_file, &content) {
        ctx.restore_backup(backup.as_ref(), &service_file);
        return Err(format!("Failed to write service file: {}", e));
    }

    if let Some(ref b) = backup {
        println!("Backup at: {}", b.display());
    }

    Command::new("systemctl").args(["--user", "daemon-reload"]).output()
        .map_err(|e| format!("daemon-reload failed: {}", e))?;

    let _ = Command::new("loginctl").args(["enable-linger"]).output();

    let output = Command::new("systemctl")
        .args(["--user", "enable", "--now", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to enable: {}", e))?;

    if !output.status.success() {
        return Err(format!("Failed to start: {}", String::from_utf8_lossy(&output.stderr)));
    }

    std::thread::sleep(std::time::Duration::from_secs(1));

    let is_active = Command::new("systemctl")
        .args(["--user", "is-active", SERVICE_NAME])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    print_summary(SERVICE_NAME, &service_file, &ctx.config_path, &ctx.exe_path_str, is_active, InitSystem::Systemd);
    Ok(())
}

pub fn uninstall_systemd_service() -> Result<(), String> {
    if !is_linux() { return Err("Linux only".to_string()); }

    let _ = Command::new("systemctl").args(["--user", "stop", SERVICE_NAME]).output();
    let _ = Command::new("systemctl").args(["--user", "disable", SERVICE_NAME]).output();

    let service_file = get_systemd_service_file_path()?;
    if service_file.exists() {
        let _ = std::fs::remove_file(&service_file);
    }

    let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).output();
    println!("Service '{}' stopped and removed.", SERVICE_NAME);
    Ok(())
}

fn is_systemd_service_running() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", SERVICE_NAME])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ==================== OpenRC 服务管理 ====================

pub fn get_openrc_service_file_path() -> PathBuf {
    PathBuf::from("/etc/init.d").join(SERVICE_NAME)
}

pub fn install_openrc_service() -> Result<(), String> {
    if !is_linux() { return Err("Linux only".to_string()); }
    if !is_openrc_available() { return Err("OpenRC not available".to_string()); }
    if !is_root() { return Err("OpenRC requires root. Run with sudo.".to_string()); }

    let ctx = InstallContext::new()?;

    if is_openrc_service_running() {
        return Err(format!("Service running. Stop first: rc-service {} stop", SERVICE_NAME));
    }

    ctx.prepare_config()?;

    let service_file = get_openrc_service_file_path();
    if ctx.exe_path == service_file {
        return Err("Executable path conflicts with service path. Move to /usr/local/bin/".to_string());
    }

    let backup = ctx.backup_file(&service_file);

    let content = format!(
        r#"#!/sbin/openrc-run

name="{name}"
description="VLESS Rust Server"

command="{exe}"
command_args="{config} --no-tui"
command_background="yes"
pidfile="/run/${{RC_SVCNAME}}.pid"
directory="{work_dir}"

output_log="/var/log/${{RC_SVCNAME}}.log"
error_log="/var/log/${{RC_SVCNAME}}.err"

depend() {{
    need net
    after firewall
    after network-online
    want network-online
}}

start_pre() {{
    checkpath --directory --owner root:root --mode 0755 /run
    checkpath --directory --owner root:root --mode 0755 /var/log
    checkpath --file --owner root:root --mode 0644 "$output_log" "$error_log"
}}
"#,
        name = SERVICE_NAME,
        exe = ctx.exe_path_str,
        config = ctx.config_path.display(),
        work_dir = ctx.work_dir.display()
    );

    if let Err(e) = atomic_write::atomic_write_file(&service_file, &content) {
        ctx.restore_backup(backup.as_ref(), &service_file);
        return Err(format!("Failed to write service file: {}", e));
    }

    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&service_file, std::fs::Permissions::from_mode(0o755)) {
            ctx.restore_backup(backup.as_ref(), &service_file);
            return Err(format!("Failed to set permissions: {}", e));
        }
    }

    if let Some(ref b) = backup {
        println!("Backup at: {}", b.display());
    }

    let output = Command::new("rc-update").args(["add", SERVICE_NAME, "default"]).output()
        .map_err(|e| format!("Failed to add service: {}", e))?;
    if !output.status.success() {
        return Err(format!("rc-update failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let output = Command::new("rc-service").args([SERVICE_NAME, "start"]).output()
        .map_err(|e| format!("Failed to start: {}", e))?;
    if !output.status.success() {
        return Err(format!("Start failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    std::thread::sleep(std::time::Duration::from_secs(1));

    let is_active = Command::new("rc-service")
        .args([SERVICE_NAME, "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    print_summary(SERVICE_NAME, &service_file, &ctx.config_path, &ctx.exe_path_str, is_active, InitSystem::OpenRC);
    Ok(())
}

pub fn uninstall_openrc_service() -> Result<(), String> {
    if !is_linux() { return Err("Linux only".to_string()); }
    if !is_root() { return Err("OpenRC requires root. Run with sudo.".to_string()); }

    let _ = Command::new("rc-service").args([SERVICE_NAME, "stop"]).output();
    let _ = Command::new("rc-update").args(["del", SERVICE_NAME]).output();

    let service_file = get_openrc_service_file_path();
    if service_file.exists() {
        let _ = std::fs::remove_file(&service_file);
    }

    println!("Service '{}' stopped and removed.", SERVICE_NAME);
    Ok(())
}

fn is_openrc_service_running() -> bool {
    Command::new("rc-service")
        .args([SERVICE_NAME, "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_root() -> bool {
    #[cfg(unix)] {
        unsafe { libc::getuid() == 0 }
    }
    #[cfg(not(unix))] {
        false
    }
}

// ==================== 配置路径检查 ====================

fn check_config_path_conflict(exe_path: &Path, config_path: &Path) -> Result<(), String> {
    if exe_path == config_path {
        return Err("Config file path cannot be the same as executable path".to_string());
    }

    let exe_canon = exe_path.canonicalize().ok();
    let config_canon = config_path.canonicalize().ok();

    if let (Some(e), Some(c)) = (&exe_canon, &config_canon) {
        if e == c {
            return Err("Config file resolves to the same file as executable (possibly via symlink)".to_string());
        }
    }

    if config_path.exists() {
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(config_path) {
                if meta.permissions().mode() & 0o111 != 0 {
                    return Err(format!(
                        "Config path '{}' already exists and is an executable. Refusing to overwrite.",
                        config_path.display()
                    ));
                }
            }
        }
        if !config_path.is_file() {
            return Err(format!("Config path exists but is not a file: {}", config_path.display()));
        }
    }

    Ok(())
}
