//! 跨平台内存信息获取模块
//!
//! 替代 sysinfo 库，提供进程内存和系统总内存获取功能

/// 获取当前进程的内存使用量（字节）
///
/// 平台支持：
/// - Linux: 读取 /proc/self/status 的 VmRSS 字段
/// - Windows: 使用 GetProcessMemoryInfo API
/// - macOS: 使用 task_info API
pub fn get_process_memory() -> u64 {
    #[cfg(target_os = "linux")]
    {
        get_process_memory_linux()
    }

    #[cfg(target_os = "windows")]
    {
        get_process_memory_windows()
    }

    #[cfg(target_os = "macos")]
    {
        get_process_memory_macos()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
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
/// - macOS: 使用 host_statistics API
pub fn get_total_memory() -> u64 {
    #[cfg(target_os = "linux")]
    {
        get_total_memory_linux()
    }

    #[cfg(target_os = "windows")]
    {
        get_total_memory_windows()
    }

    #[cfg(target_os = "macos")]
    {
        get_total_memory_macos()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        // 其他平台返回 0
        0
    }
}

// Linux 平台实现
#[cfg(target_os = "linux")]
fn get_process_memory_linux() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/self/status") {
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                // VmRSS:	  12345 kB
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return kb * 1024; // 转换为字节
                    }
                }
            }
        }
    }
    0
}

#[cfg(target_os = "linux")]
fn get_total_memory_linux() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                // MemTotal:       16384000 kB
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return kb * 1024; // 转换为字节
                    }
                }
            }
        }
    }
    0
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
        let mut info = PROCESS_MEMORY_COUNTERS::default();
        info.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

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
        let mut status = MEMORYSTATUSEX::default();
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

        if GlobalMemoryStatusEx(&mut status).is_ok() {
            return status.ullTotalPhys;
        }
    }
    0
}

// macOS 平台实现
#[cfg(target_os = "macos")]
fn get_process_memory_macos() -> u64 {
    use libc::{c_int, task_info, task_basic_info, mach_task_self, task_t, mach_port_t};

    unsafe {
        let task: task_t = mach_task_self();
        let mut info: task_basic_info = std::mem::zeroed();
        let mut count = mach_task_self() as u32;

        let result = task_info(
            task,
            4, // TASK_BASIC_INFO
            &mut info as *mut _ as *mut _,
            &mut count as *mut _ as *mut _,
        );

        if result == 0 {
            return info.resident_size as u64;
        }
    }
    0
}

#[cfg(target_os = "macos")]
fn get_total_memory_macos() -> u64 {
    use libc::{c_int, host_statistics, host_info, vm_statistics, mach_host_self, host_t};

    unsafe {
        let host: host_t = mach_host_self();
        let mut info: vm_statistics = std::mem::zeroed();
        let mut count = std::mem::size_of::<vm_statistics>() as c_int;

        let result = host_statistics(
            host,
            2, // HOST_VM_INFO
            &mut info as *mut _ as *mut _,
            &mut count as *mut _ as *mut _,
        );

        if result == 0 {
            let page_size = 4096u64; // macOS 默认页面大小
            return (info.active_count as u64
                + info.inactive_count as u64
                + info.wire_count as u64
                + info.free_count as u64)
                * page_size;
        }
    }
    0
}
