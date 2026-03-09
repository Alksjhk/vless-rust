use std::fs;
use std::path::Path;

/// 从 Cargo.toml 的 metadata 中读取版本元数据
#[derive(Debug, Clone)]
struct VersionMetadata {
    version: String,
    product_name: String,
    author: String,
    file_description: String,
    company_name: String,
    legal_copyright: String,
    comments: String,
}

impl Default for VersionMetadata {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            product_name: "VLESS Rust Server".to_string(),
            author: env!("CARGO_PKG_AUTHORS").to_string(),
            file_description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            company_name: "VLESS-Rust Project".to_string(),
            legal_copyright: format!("Copyright (C) {}", chrono_year()),
            comments: "High-performance VLESS protocol implementation in Rust".to_string(),
        }
    }
}

/// 获取当前年份
fn chrono_year() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days = duration.as_secs() / 86400;
    (1970 + days / 365) as u32
}

fn main() {
    // 从 Cargo.toml 读取版本元数据
    let metadata = read_version_metadata();

    // Windows 平台：嵌入资源
    #[cfg(target_os = "windows")]
    {
        embed_windows_resources(&metadata);
    }

    // 所有平台：生成 version_info.rs
    generate_version_info(&metadata);
}

/// 从 Cargo.toml 读取版本元数据
fn read_version_metadata() -> VersionMetadata {
    let cargo_toml_path = Path::new("Cargo.toml");

    let content = match fs::read_to_string(cargo_toml_path) {
        Ok(content) => content,
        Err(e) => {
            println!("cargo:warning=Failed to read Cargo.toml: {}", e);
            return VersionMetadata::default();
        }
    };

    // 解析 Cargo.toml
    let mut metadata = VersionMetadata::default();

    // 解析 [package.metadata.windows] 段
    let mut in_metadata_section = false;
    for line in content.lines() {
        let line = line.trim();

        if line == "[package.metadata.windows]" {
            in_metadata_section = true;
            continue;
        }

        // 遇到新段落则退出
        if line.starts_with('[') && !line.starts_with("[package.metadata") {
            in_metadata_section = false;
        }

        if in_metadata_section {
            if let Some((key, value)) = parse_toml_line(line) {
                match key.as_str() {
                    "product-name" => metadata.product_name = value,
                    "company-name" => metadata.company_name = value,
                    "file-description" => metadata.file_description = value,
                    "legal-copyright" => metadata.legal_copyright = value,
                    "comments" => metadata.comments = value,
                    _ => {}
                }
            }
        }
    }

    // author 从 CARGO_PKG_AUTHORS 获取（Cargo.toml 的 authors 字段）
    // 如果为空，使用 company_name
    if metadata.author.is_empty() {
        metadata.author = metadata.company_name.clone();
    }

    metadata
}

/// 解析 TOML 行 (key = "value")
fn parse_toml_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }

    let key = parts[0].trim().to_string();
    let value = parts[1].trim();

    // 移除引号
    let value = value.trim_matches('"').to_string();

    Some((key, value))
}

/// 将版本字符串转换为 Windows 版本号 (u64)
fn parse_version_string(version: &str) -> u64 {
    let parts: Vec<&str> = version.split('.').collect();
    let major = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(1) as u64;
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0) as u64;
    let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0) as u64;
    let build = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0) as u64;

    // Windows 版本号格式: major.minor.patch.build
    (major << 48) | (minor << 32) | (patch << 16) | build
}

/// Windows 平台：嵌入资源
#[cfg(target_os = "windows")]
fn embed_windows_resources(metadata: &VersionMetadata) {
    let version = parse_version_string(&metadata.version);

    let mut res = winres::WindowsResource::new();

    // 设置图标
    res.set_icon("assets/icon.ico");

    // 设置版本信息
    res.set_version_info(winres::VersionInfo::PRODUCTVERSION, version);
    res.set_version_info(winres::VersionInfo::FILEVERSION, version);

    // 设置字符串信息
    res.set("ProductName", &metadata.product_name);
    res.set("FileDescription", &metadata.file_description);
    res.set("CompanyName", &metadata.company_name);
    res.set("LegalCopyright", &metadata.legal_copyright);
    res.set("Comments", &metadata.comments);
    res.set("ProductVersion", &metadata.version);
    res.set("FileVersion", &metadata.version);
    res.set("Author", &metadata.author);

    // 编译资源
    if let Err(e) = res.compile() {
        eprintln!("Failed to compile Windows resources: {}", e);
        eprintln!("Continuing without embedded resources...");
    }

    // 声明依赖跟踪
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=assets/icon.ico");
}

/// 生成 src/version_info.rs 文件
fn generate_version_info(metadata: &VersionMetadata) {
    // 获取编译信息
    let rust_version = std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string());
    let build_profile = if cfg!(debug_assertions) { "Debug" } else { "Release" };
    let build_target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    // 获取构建日期
    let build_date = get_build_date();

    // 生成 Rust 代码
    let code = format!(
        r#"/// 编译时嵌入的版本信息
/// 此文件由 build.rs 自动生成，请勿手动修改

/// 版本信息常量结构体
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VersionInfoConst {{
    pub version: &'static str,
    pub product_name: &'static str,
    pub author: &'static str,
    pub file_description: &'static str,
    pub legal_copyright: &'static str,
    pub company_name: &'static str,
}}

/// 编译信息常量结构体
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BuildInfoConst {{
    pub rust_version: &'static str,
    pub build_profile: &'static str,
    pub build_target: &'static str,
    pub build_date: &'static str,
}}

/// 编译时嵌入的版本信息
pub static VERSION_INFO: VersionInfoConst = VersionInfoConst {{
    version: "{version}",
    product_name: "{product_name}",
    author: "{author}",
    file_description: "{file_description}",
    legal_copyright: "{legal_copyright}",
    company_name: "{company_name}",
}};

/// 编译时嵌入的编译信息
#[allow(dead_code)]
pub static BUILD_INFO: BuildInfoConst = BuildInfoConst {{
    rust_version: "{rust_version}",
    build_profile: "{build_profile}",
    build_target: "{build_target}",
    build_date: "{build_date}",
}};
"#,
        version = metadata.version,
        product_name = metadata.product_name,
        author = metadata.author,
        file_description = metadata.file_description,
        legal_copyright = metadata.legal_copyright,
        company_name = metadata.company_name,
        rust_version = rust_version,
        build_profile = build_profile,
        build_target = build_target,
        build_date = build_date,
    );

    // 写入文件
    let out_path = Path::new("src").join("version_info.rs");
    if let Err(e) = fs::write(&out_path, code) {
        eprintln!("Error: Failed to write version_info.rs: {}", e);
        std::process::exit(1);
    }

    // 声明依赖跟踪
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src/version_info.rs");
}

/// 获取构建日期
fn get_build_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let days_since_epoch = secs / 86400;
            // 简单计算日期（从 1970-01-01 开始）
            let year = 1970 + days_since_epoch / 365;
            let day_of_year = (days_since_epoch % 365) as u32;
            let month = day_of_year / 30 + 1;
            let day = day_of_year % 30 + 1;
            format!("{:04}-{:02}-{:02}", year, month, day)
        })
        .unwrap_or_else(|_| "unknown".to_string())
}