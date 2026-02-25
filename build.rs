use std::fs;
use std::path::Path;

#[derive(Debug, serde::Deserialize)]
struct VersionInfo {
    #[serde(default)]
    #[allow(dead_code)]
    version: Option<String>,
    #[serde(default = "default_file_version")]
    #[allow(dead_code)]
    file_version: String,
    #[allow(dead_code)]
    product_name: String,
    #[serde(default = "default_file_description")]
    #[allow(dead_code)]
    file_description: String,
    #[serde(default)]
    #[allow(dead_code)]
    company_name: String,
    #[serde(default)]
    #[allow(dead_code)]
    legal_copyright: String,
    #[serde(default = "default_original_filename")]
    #[allow(dead_code)]
    original_filename: String,
    #[serde(default = "default_internal_name")]
    #[allow(dead_code)]
    internal_name: String,
    #[serde(default = "default_comments")]
    #[allow(dead_code)]
    comments: String,
    #[serde(default = "default_product_version")]
    #[allow(dead_code)]
    product_version: String,
    #[serde(default)]
    #[allow(dead_code)]
    private_build: String,
    #[serde(default)]
    #[allow(dead_code)]
    special_build: String,
    /// 作者信息 (可选字段)
    #[serde(default)]
    author: String,
}

fn default_file_version() -> String { "1.0.0.0".to_string() }
fn default_file_description() -> String { "High-performance VLESS protocol server".to_string() }
fn default_original_filename() -> String { "vless.exe".to_string() }
fn default_internal_name() -> String { "vless".to_string() }
fn default_comments() -> String { "High-performance VLESS protocol implementation in Rust".to_string() }
fn default_product_version() -> String { "1.0.0.0".to_string() }

fn parse_version_string(version: &str) -> u64 {
    let parts: Vec<&str> = version.split('.').collect();
    let major = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(1) as u64;
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0) as u64;
    let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0) as u64;
    let build = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0) as u64;

    // Windows 版本号格式: major.minor.patch.build
    // 转换为单个 u64: (major << 48) | (minor << 32) | (patch << 16) | build
    (major << 48) | (minor << 32) | (patch << 16) | build
}

fn main() {
    // 只在 Windows 上嵌入资源
    #[cfg(target_os = "windows")]
    {
        let version_path = Path::new("assets").join("version.json");

        // 读取版本配置
        let version_info: VersionInfo = match fs::File::open(&version_path) {
            Ok(file) => match serde_json::from_reader(file) {
                Ok(info) => info,
                Err(e) => {
                    eprintln!("Warning: Failed to parse version.json: {}", e);
                    eprintln!("Using default version information");
                    VersionInfo {
                        version: None,
                        file_version: "1.0.0.0".to_string(),
                        product_name: "VLESS Rust Server".to_string(),
                        file_description: "High-performance VLESS protocol server".to_string(),
                        company_name: "VLESS-Rust Project".to_string(),
                        legal_copyright: "Copyright (C) 2025".to_string(),
                        original_filename: "vless.exe".to_string(),
                        internal_name: "vless".to_string(),
                        comments: "High-performance VLESS protocol implementation in Rust".to_string(),
                        product_version: "1.0.0.0".to_string(),
                        private_build: String::new(),
                        special_build: String::new(),
                        author: String::new(),
                    }
                }
            },
            Err(_e) => {
                eprintln!("Warning: Failed to open version.json, using default version information");
                VersionInfo {
                    version: None,
                    file_version: "1.0.0.0".to_string(),
                    product_name: "VLESS Rust Server".to_string(),
                    file_description: "High-performance VLESS protocol server".to_string(),
                    company_name: "VLESS-Rust Project".to_string(),
                    legal_copyright: "Copyright (C) 2025".to_string(),
                    original_filename: "vless.exe".to_string(),
                    internal_name: "vless".to_string(),
                    comments: "High-performance VLESS protocol implementation in Rust".to_string(),
                    product_version: "1.0.0.0".to_string(),
                    private_build: String::new(),
                    special_build: String::new(),
                    author: String::new(),
                }
            }
        };

        // 使用 version 字段（如果有）或 file_version 作为版本号
        let version_str = version_info.version.as_ref()
            .map(|s| s.as_str())
            .unwrap_or(&version_info.file_version);
        let version = parse_version_string(version_str);

        let mut res = winres::WindowsResource::new();

        // 设置图标
        res.set_icon("assets/icon.ico");

        // 设置版本信息
        res.set_version_info(winres::VersionInfo::PRODUCTVERSION, version);
        res.set_version_info(winres::VersionInfo::FILEVERSION, version);

        // 设置字符串信息
        res.set("ProductName", &version_info.product_name);
        res.set("FileDescription", &version_info.file_description);
        res.set("CompanyName", &version_info.company_name);
        res.set("LegalCopyright", &version_info.legal_copyright);
        res.set("OriginalFilename", &version_info.original_filename);
        res.set("InternalName", &version_info.internal_name);
        res.set("Comments", &version_info.comments);
        res.set("ProductVersion", version_str);
        res.set("FileVersion", version_str);

        if !version_info.private_build.is_empty() {
            res.set("PrivateBuild", &version_info.private_build);
        }

        if !version_info.special_build.is_empty() {
            res.set("SpecialBuild", &version_info.special_build);
        }

        // 编译资源
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resources: {}", e);
            eprintln!("Continuing without embedded resources...");
        }

        println!("cargo:rerun-if-changed=assets/version.json");
        println!("cargo:rerun-if-changed=assets/icon.ico");
    }

    // 非Windows平台也需要声明依赖跟踪
    #[cfg(not(target_os = "windows"))]
    {
        println!("cargo:rerun-if-changed=assets/version.json");
    }

    // ============================================
    // 生成编译时版本信息常量 (所有平台)
    // ============================================
    generate_version_info();
}

/// 生成 src/version_info.rs 文件，包含编译时嵌入的版本信息
fn generate_version_info() {
    let version_path = Path::new("assets").join("version.json");

    // 默认版本信息
    let default_info = VersionInfo {
        version: None,
        file_version: "1.0.0.0".to_string(),
        product_name: "VLESS Rust Server".to_string(),
        file_description: "High-performance VLESS protocol server".to_string(),
        company_name: "VLESS-Rust Project".to_string(),
        legal_copyright: "Copyright (C) 2025".to_string(),
        original_filename: "vless.exe".to_string(),
        internal_name: "vless".to_string(),
        comments: "High-performance VLESS protocol implementation in Rust".to_string(),
        product_version: "1.0.0.0".to_string(),
        private_build: String::new(),
        special_build: String::new(),
        author: String::new(),
    };

    // 读取版本信息
    let version_info: VersionInfo = match fs::File::open(&version_path) {
        Ok(file) => match serde_json::from_reader(file) {
            Ok(info) => info,
            Err(e) => {
                println!("cargo:warning=Failed to parse version.json: {}", e);
                default_info
            }
        },
        Err(e) => {
            println!("cargo:warning=Failed to open version.json: {}", e);
            default_info
        }
    };

    // 获取编译信息（使用 std::env::var 在运行时读取）
    let rust_version = std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string());
    let build_profile = if cfg!(debug_assertions) { "Debug" } else { "Release" };
    let build_target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    // 获取当前日期作为构建日期（使用标准库）
    use std::time::SystemTime;
    let build_date = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or_else(|_| "unknown".to_string(), |d| {
            let secs = d.as_secs();
            let days_since_epoch = secs / 86400;
            // 简单计算日期（从 1970-01-01 开始）
            let year = 1970 + days_since_epoch / 365;
            let day_of_year = (days_since_epoch % 365) as u32;
            let month = day_of_year / 30 + 1;
            let day = day_of_year % 30 + 1;
            format!("{:04}-{:02}-{:02}", year, month, day)
        });

    // 获取版本号（从 version 字段或回退到 file_version）
    let version_str = version_info.version.as_ref()
        .map(|s| s.as_str())
        .unwrap_or(&version_info.file_version);

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
        version = version_str,
        product_name = version_info.product_name,
        // 优先使用 author 字段，如果为空则使用 company_name
        author = if version_info.author.is_empty() {
            &version_info.company_name
        } else {
            &version_info.author
        },
        file_description = version_info.file_description,
        legal_copyright = version_info.legal_copyright,
        company_name = version_info.company_name,
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

    println!("cargo:rerun-if-changed=assets/version.json");
    println!("cargo:rerun-if-changed=src/version_info.rs");
}
