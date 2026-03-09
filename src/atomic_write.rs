use std::path::Path;
use std::io::{self, Write};
use anyhow::Result;

/// 原子写入文件
///
/// 使用临时文件+重命名的方式确保写入操作的原子性：
/// 1. 写入到临时文件（.tmp 后缀）
/// 2. 重命名临时文件到目标文件
/// 3. 如果写入失败，原文件不受影响
///
/// # 优势
/// - 避免写入过程中断导致文件损坏
/// - 避免多进程同时写入导致数据混乱
/// - 重命名操作在大多数文件系统上是原子的
pub fn atomic_write_file(path: &Path, content: &str) -> Result<()> {
    // 使用追加 .tmp 后缀而不是替换扩展名，避免文件名冲突
    // 例如：config.json → config.json.tmp（而不是 config.tmp）
    let temp_path = format!("{}.tmp", path.display());
    let temp_path = Path::new(&temp_path);

    // 清理旧的临时文件（如果存在）
    // 这可以处理上次崩溃留下的临时文件
    if temp_path.exists() {
        // 检查临时文件是否可写入（确保没有被锁定）
        if is_file_writable(temp_path) {
            // 安全删除旧的临时文件
            if let Err(e) = std::fs::remove_file(temp_path) {
                // 删除失败，但继续尝试创建新文件
                // File::create 会覆盖旧文件
                eprintln!("Warning: Failed to remove old temp file: {}", e);
            }
        }
    }

    // 写入临时文件
    let mut file = std::fs::File::create(temp_path)
        .map_err(|e| format_file_error("create temporary file", temp_path, e))?;

    file.write_all(content.as_bytes())
        .map_err(|e| format_file_error("write to temporary file", temp_path, e))?;

    // 确保数据刷新到磁盘
    file.sync_data()
        .map_err(|e| format_file_error("sync temporary file", temp_path, e))?;

    // 原子重命名（在 Unix 和 Windows 上都是原子操作）
    std::fs::rename(temp_path, path)
        .map_err(|e| format_file_error("rename temporary file to target", path, e))?;

    Ok(())
}

/// 原子写入文件（带权限设置）
///
/// 在写入前设置临时文件权限，确保原子性
#[cfg(unix)]
pub fn atomic_write_file_with_perms(path: &Path, content: &str, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // 使用追加 .tmp 后缀而不是替换扩展名
    let temp_path = format!("{}.tmp", path.display());
    let temp_path = Path::new(&temp_path);

    // 创建临时文件并立即设置权限（在写入数据之前）
    let mut file = std::fs::File::create(temp_path)
        .map_err(|e| format_file_error("create temporary file", temp_path, e))?;

    // 立即设置权限（在写入数据之前），避免权限窗口期
    std::fs::set_permissions(temp_path, std::fs::Permissions::from_mode(mode))
        .map_err(|e| format_file_error("set temporary file permissions", temp_path, e))?;

    // 写入数据
    file.write_all(content.as_bytes())
        .map_err(|e| format_file_error("write to temporary file", temp_path, e))?;

    // 确保数据刷新到磁盘
    file.sync_data()
        .map_err(|e| format_file_error("sync temporary file", temp_path, e))?;

    // 原子重命名
    std::fs::rename(temp_path, path)
        .map_err(|e| format_file_error("rename temporary file to target", path, e))?;

    Ok(())
}

/// 原子写入文件（带权限设置）
///
/// Windows 版本不设置权限
#[cfg(not(unix))]
pub fn atomic_write_file_with_perms(path: &Path, content: &str, _mode: u32) -> Result<()> {
    atomic_write_file(path, content)
}

/// 格式化文件操作错误信息
///
/// 提供更友好的错误提示，区分不同类型的错误
fn format_file_error(operation: &str, path: &Path, error: io::Error) -> anyhow::Error {
    let error_type = match error.kind() {
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
            "file is busy or locked by another process"
        }
        io::ErrorKind::PermissionDenied => "permission denied",
        io::ErrorKind::NotFound => "parent directory does not exist",
        io::ErrorKind::StorageFull => "disk is full",
        _ => "IO error",
    };

    anyhow::anyhow!(
        "Failed to {} for '{}': {} ({})",
        operation,
        path.display(),
        error_type,
        error
    )
}

/// 检查文件是否可写入
///
/// 尝试创建一个测试文件来检查目标路径是否可写入
/// 如果文件已存在，不会覆盖它
///
/// 注意：临时文件后缀与 atomic_write_file() 保持一致（.tmp）
pub fn is_file_writable(path: &Path) -> bool {
    if path.exists() {
        // 文件已存在，尝试打开进行追加
        std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(path)
            .is_ok()
    } else {
        // 文件不存在，尝试创建临时文件
        // 使用 .tmp 后缀与 atomic_write_file 保持一致
        let test_path = path.with_extension("tmp");
        let result = std::fs::File::create(&test_path);
        if result.is_ok() {
            // 清理测试文件
            let _ = std::fs::remove_file(&test_path);
        }
        result.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // 第一次写入
        atomic_write_file(&file_path, "Hello, World!").unwrap();
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "Hello, World!");

        // 覆盖写入
        atomic_write_file(&file_path, "New content").unwrap();
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "New content");
    }

    #[test]
    fn test_is_file_writable() {
        let temp_dir = TempDir::new().unwrap();

        // 不存在的文件
        let new_file = temp_dir.path().join("new.txt");
        assert!(is_file_writable(&new_file));

        // 已存在的文件
        let existing_file = temp_dir.path().join("existing.txt");
        std::fs::write(&existing_file, "test").unwrap();
        assert!(is_file_writable(&existing_file));
    }
}