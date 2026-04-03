//! 原子文件写入模块测试

use tempfile::TempDir;
use vless_rust::atomic_write::{atomic_write_file, atomic_write_file_with_perms, is_file_writable};

#[test]
fn test_atomic_write_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    atomic_write_file(&file_path, "Hello, World!").unwrap();
    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        "Hello, World!"
    );

    atomic_write_file(&file_path, "New content").unwrap();
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "New content");
}

#[test]
fn test_is_file_writable_new_file() {
    let temp_dir = TempDir::new().unwrap();
    let new_file = temp_dir.path().join("new.txt");

    assert!(is_file_writable(&new_file));
}

#[test]
fn test_is_file_writable_existing_file() {
    let temp_dir = TempDir::new().unwrap();
    let existing_file = temp_dir.path().join("existing.txt");

    std::fs::write(&existing_file, "test").unwrap();
    assert!(is_file_writable(&existing_file));
}

#[test]
fn test_atomic_write_creates_parent() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("subdir/nested/test.txt");

    // 父目录不存在时应该失败（因为 atomic_write_file 不会创建父目录）
    let result = atomic_write_file(&file_path, "content");
    assert!(result.is_err());
}

#[test]
fn test_atomic_write_overwrites() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    atomic_write_file(&file_path, "first").unwrap();
    atomic_write_file(&file_path, "second").unwrap();
    atomic_write_file(&file_path, "third").unwrap();

    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "third");
}

#[test]
fn test_atomic_write_empty_content() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.txt");

    atomic_write_file(&file_path, "").unwrap();
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "");
}

#[test]
fn test_atomic_write_unicode() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("unicode.txt");

    let unicode_content = "你好，世界！Hello 世界 🌍";
    atomic_write_file(&file_path, unicode_content).unwrap();

    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        unicode_content
    );
}

#[test]
fn test_atomic_write_with_perms() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("perms.txt");

    // Unix 权限 0o600 = rw-------
    atomic_write_file_with_perms(&file_path, "content", 0o600).unwrap();

    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "content");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&file_path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
