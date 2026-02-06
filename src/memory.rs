//! 跨平台内存信息获取模块
//!
//! 替代 sysinfo 库，提供进程内存和系统总内存获取功能

#[cfg(target_os = "linux")]
use std::fs;

/// 获取当前进程的内存使用量（字节）
///
/// 平台支持：
/// - Linux: 读取 /proc/self/status 的 VmRSS 字段
/// - Windows: 使用 GetProcessMemoryInfo API
pub fn get_process_memory() -> u64 {
    #[cfg(target_os = "linux")]
    {
        get_process_memory_linux()
    }

    #[cfg(target_os = "windows")]
    {
        get_process_memory_windows()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        // 其他平台返回 0
        0
    }
}

/// 获取系统总内存（字节）
///
/// 平台支持：
/// - Linux: 读取 /proc/meminfo 的 MemTotal 字段
/// - Windows: 使用 GlobalMemoryStatusEx API
pub fn get_total_memory() -> u64 {
    #[cfg(target_os = "linux")]
    {
        get_total_memory_linux()
    }

    #[cfg(target_os = "windows")]
    {
        get_total_memory_windows()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        // 其他平台返回 0
        0
    }
}

// Linux 平台实现
#[cfg(target_os = "linux")]
fn get_process_memory_linux() -> u64 {
    match fs::read_to_string("/proc/self/status") {
        Ok(content) => {
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    // VmRSS:	  12345 kB
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // 转换为字节
                        }
                    }
                }
            }
            tracing::warn!("Failed to parse VmRSS from /proc/self/status");
            0
        }
        Err(e) => {
            tracing::error!("Failed to read /proc/self/status: {}", e);
            0
        }
    }
}

#[cfg(target_os = "linux")]
fn get_total_memory_linux() -> u64 {
    match fs::read_to_string("/proc/meminfo") {
        Ok(content) => {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    // MemTotal:       16384000 kB
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // 转换为字节
                        }
                    }
                }
            }
            tracing::warn!("Failed to parse MemTotal from /proc/meminfo");
            0
        }
        Err(e) => {
            tracing::error!("Failed to read /proc/meminfo: {}", e);
            0
        }
    }
}

// Windows 平台实现
#[cfg(target_os = "windows")]
fn get_process_memory_windows() -> u64 {
    use windows::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo,
        PROCESS_MEMORY_COUNTERS,
    };
    use windows::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        let handle = GetCurrentProcess();
        let mut info = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };

        if GetProcessMemoryInfo(handle, &mut info, info.cb).is_ok() {
            return info.WorkingSetSize as u64;
        }
    }
    0
}

#[cfg(target_os = "windows")]
fn get_total_memory_windows() -> u64 {
    use windows::Win32::System::SystemInformation::{
        GlobalMemoryStatusEx,
        MEMORYSTATUSEX,
    };

    unsafe {
        let mut status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };

        if GlobalMemoryStatusEx(&mut status).is_ok() {
            return status.ullTotalPhys;
        }
    }
    0
}

