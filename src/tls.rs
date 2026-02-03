//! TLS 模块
//!
//! 提供 TLS 配置加载、证书生成和握手处理功能

use anyhow::{Context, Result};
use rustls::pki_types::CertificateDer;
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsStream;
use crate::config::TlsConfig as ConfigTlsConfig;

/// 确保 TLS 证书文件存在
///
/// 如果证书文件不存在，则自动生成自签名证书
pub fn ensure_cert_exists(config: &ConfigTlsConfig) -> Result<()> {
    let cert_path = Path::new(&config.cert_file);
    let key_path = Path::new(&config.key_file);

    // 如果证书已存在，直接返回
    if cert_path.exists() && key_path.exists() {
        return Ok(());
    }

    // 生成自签名证书
    generate_self_signed_cert(cert_path, key_path, &config.server_name)
}

/// 加载 TLS 配置
///
/// 从证书文件和私钥文件加载 TLS 配置
pub async fn load_tls_config(config: &ConfigTlsConfig) -> Result<Arc<ServerConfig>> {
    // 读取证书文件
    let cert_file = File::open(&config.cert_file)
        .with_context(|| format!("无法打开证书文件: {}", config.cert_file))?;
    let mut cert_reader = BufReader::new(cert_file);
    let cert_chain: Vec<CertificateDer> = certs(&mut cert_reader)
        .collect::<Result<_, _>>()
        .with_context(|| format!("解析证书文件失败: {}", config.cert_file))?;

    if cert_chain.is_empty() {
        anyhow::bail!("证书文件为空: {}", config.cert_file);
    }

    // 读取私钥文件
    let key_file = File::open(&config.key_file)
        .with_context(|| format!("无法打开私钥文件: {}", config.key_file))?;
    let mut key_reader = BufReader::new(key_file);
    let key = private_key(&mut key_reader)
        .with_context(|| format!("解析私钥文件失败: {}", config.key_file))?
        .context("私钥文件为空")?;

    // 创建 TLS 配置
    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| anyhow::anyhow!("创建 TLS 配置失败: {}", e))?;

    // 设置 ALPN 协议（可选）
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(Arc::new(server_config))
}

/// 接受 TLS 连接
///
/// 对 TCP 流执行 TLS 握手
pub async fn accept_tls(
    stream: TcpStream,
    config: Arc<ServerConfig>,
) -> Result<TlsStream<TcpStream>> {
    // 完成 TLS 握手
    let acceptor = tokio_rustls::TlsAcceptor::from(config);
    let tls_stream = acceptor.accept(stream).await?;

    Ok(TlsStream::Server(tls_stream))
}

/// 生成自签名证书并保存到文件
///
/// # 参数
/// * `cert_path` - 证书保存路径
/// * `key_path` - 私钥保存路径
/// * `server_name` - 服务器名称（用于证书的 SAN）
///
/// # 返回
/// 成功时返回 Ok(())，失败时返回错误信息
///
/// # 证书特性
/// - 有效期：10年（3650天）
/// - 包含多个 SAN（服务器名称、localhost、回环地址）
pub fn generate_self_signed_cert(
    cert_path: &Path,
    key_path: &Path,
    server_name: &str,
) -> Result<()> {
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
    use time::Duration;

    // 创建证书目录（如果不存在）
    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建证书目录失败: {}", parent.display()))?;
    }

    // 创建证书参数
    let mut cert_params = CertificateParams::default();
    cert_params.not_before = time::OffsetDateTime::now_utc();
    cert_params.not_after = time::OffsetDateTime::now_utc()
        .checked_add(Duration::days(365 * 10))
        .context("计算证书到期时间失败")?;

    // 设置证书主题
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::OrganizationName, "VLESS Rust Server");
    distinguished_name.push(DnType::CommonName, server_name);
    cert_params.distinguished_name = distinguished_name;

    // 设置 SAN（Subject Alternative Names）
    // 使用 try_from 将字符串转换为 Ia5String
    cert_params.subject_alt_names = vec![
        SanType::DnsName(server_name.try_into()?),
        SanType::DnsName("localhost".try_into()?),
        SanType::DnsName("*.local".try_into()?),
        SanType::IpAddress("127.0.0.1".parse()?),
        SanType::IpAddress("::1".parse()?),
    ];

    // 生成密钥对
    let key_pair = KeyPair::generate()
        .context("生成密钥对失败")?;

    // 生成证书
    let cert = cert_params.self_signed(&key_pair)
        .context("生成自签名证书失败")?;

    // 序列化为 PEM 格式
    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    // 写入证书文件
    fs::write(cert_path, cert_pem)
        .with_context(|| format!("写入证书文件失败: {}", cert_path.display()))?;

    // 写入私钥文件
    fs::write(key_path, key_pem)
        .with_context(|| format!("写入私钥文件失败: {}", key_path.display()))?;

    println!("✅ TLS 证书已生成:");
    println!("   证书文件: {}", cert_path.display());
    println!("   私钥文件: {}", key_path.display());
    println!("   服务器名称: {}", server_name);
    println!("   有效期: 10年 (3650天)");

    Ok(())
}
