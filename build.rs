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
            legal_copyright: format!("Copyright (C) {}", current_year()),
            comments: "High-performance VLESS protocol implementation in Rust".to_string(),
        }
    }
}

/// 使用精确的格里高利历计算当前年份（处理闰年）
fn current_year() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 从 Unix 时间戳精确计算年份（考虑闰年）
    let mut days = (secs / 86400) as i64;
    let mut year = 1970i32;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    year as u32
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn main() {
    let metadata = read_version_metadata();

    #[cfg(target_os = "windows")]
    embed_windows_resources(&metadata);

    generate_version_info(&metadata);
}

fn read_version_metadata() -> VersionMetadata {
    let content = match fs::read_to_string("Cargo.toml") {
        Ok(c) => c,
        Err(e) => {
            println!("cargo:warning=Failed to read Cargo.toml: {}", e);
            return VersionMetadata::default();
        }
    };

    let mut metadata = VersionMetadata::default();
    let mut in_winres = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[package.metadata.winres]" {
            in_winres = true;
            continue;
        }
        if line.starts_with('[') {
            in_winres = false;
        }

        if in_winres {
            if let Some((key, value)) = parse_toml_kv(line) {
                match key.as_str() {
                    "ProductName" => metadata.product_name = value,
                    "CompanyName" => metadata.company_name = value,
                    "FileDescription" => metadata.file_description = value,
                    "LegalCopyright" => metadata.legal_copyright = value,
                    "Comments" => metadata.comments = value,
                    _ => {}
                }
            }
        }
    }

    if metadata.author.is_empty() {
        metadata.author = metadata.company_name.clone();
    }

    metadata
}

fn parse_toml_kv(line: &str) -> Option<(String, String)> {
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut parts = line.splitn(2, '=');
    let key = parts.next()?.trim().to_string();
    let val = parts.next()?.trim().trim_matches('"').to_string();
    Some((key, val))
}

/// 将 "1.7.9" 解析为 (major, minor, patch)
fn parse_version(version: &str) -> (u64, u64, u64) {
    let mut parts = version.split('.');
    let major = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

#[cfg(target_os = "windows")]
fn embed_windows_resources(metadata: &VersionMetadata) {
    let (major, minor, patch) = parse_version(&metadata.version);

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let rc_path = Path::new(&out_dir).join("vless_res.rc");

    let icon_path = std::env::current_dir()
        .unwrap()
        .join("assets")
        .join("icon.ico");
    let icon_path_str = icon_path.to_string_lossy().replace('\\', "\\\\");

    let rc_content = format!(
        r#"#include <windows.h>

IDI_ICON1 ICON "{icon}"

VS_VERSION_INFO VERSIONINFO
FILEVERSION     {major},{minor},{patch},0
PRODUCTVERSION  {major},{minor},{patch},0
FILEFLAGSMASK   VS_FFI_FILEFLAGSMASK
FILEFLAGS       0
FILEOS          VOS_NT_WINDOWS32
FILETYPE        VFT_APP
FILESUBTYPE     VFT2_UNKNOWN
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "080404b0"
        BEGIN
            VALUE "FileDescription",  "{file_desc}\0"
            VALUE "FileVersion",      "{version}\0"
            VALUE "ProductName",      "{product}\0"
            VALUE "ProductVersion",   "{version}\0"
            VALUE "CompanyName",      "{company}\0"
            VALUE "LegalCopyright",   "{copyright}\0"
            VALUE "OriginalFilename", "vless.exe\0"
            VALUE "Comments",         "{comments}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0804, 0x04b0
    END
END
"#,
        icon = icon_path_str,
        major = major,
        minor = minor,
        patch = patch,
        version = metadata.version,
        product = metadata.product_name,
        file_desc = metadata.file_description,
        company = metadata.company_name,
        copyright = metadata.legal_copyright,
        comments = metadata.comments,
    );

    fs::write(&rc_path, rc_content).expect("Failed to write vless_res.rc");

    embed_resource::compile(&rc_path, embed_resource::NONE);

    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=assets/icon.ico");
}

fn generate_version_info(metadata: &VersionMetadata) {
    let build_target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let build_profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let build_date = get_build_date();

    let dest_path = Path::new("src").join("version_info.rs");

    let code = format!(
        r#"// 编译时嵌入的版本信息，由 build.rs 自动生成，请勿手动修改
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BuildInfoConst {{
    pub rust_version: &'static str,
    pub build_profile: &'static str,
    pub build_target: &'static str,
    pub build_date: &'static str,
}}

pub static VERSION_INFO: VersionInfoConst = VersionInfoConst {{
    version: "{version}",
    product_name: "{product_name}",
    author: "{author}",
    file_description: "{file_description}",
    legal_copyright: "{legal_copyright}",
    company_name: "{company_name}",
}};

#[allow(dead_code)]
pub static BUILD_INFO: BuildInfoConst = BuildInfoConst {{
    rust_version: "unknown",
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
        build_profile = build_profile,
        build_target = build_target,
        build_date = build_date,
    );

    fs::write(&dest_path, code).expect("Failed to write version_info.rs");

    println!("cargo:rerun-if-changed=Cargo.toml");
}

/// 使用精确的格里高利历计算构建日期
fn get_build_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => return "unknown".to_string(),
    };

    let mut days = secs / 86400;
    let mut year = 1970i64;

    loop {
        let days_in_year = if is_leap_year(year as i32) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    // 月份天数（考虑闰年）
    let month_days: [i64; 12] = if is_leap_year(year as i32) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1i64;
    for &d in &month_days {
        if days < d {
            break;
        }
        days -= d;
        month += 1;
    }
    let day = days + 1;

    format!("{:04}-{:02}-{:02}", year, month, day)
}
