//! VLESS-Rust 高性能代理服务器库
//!
//! 提供 VLESS 协议服务器核心功能

pub mod address;
pub mod api;
pub mod atomic_write;
pub mod config;
pub mod http;
pub mod protocol;
pub mod public_ip;
pub mod server;
pub mod socket;
pub mod tcp;
pub mod tui;
pub mod version;
pub mod vless_link;
pub mod wizard;
pub mod ws;

// service 模块仅在二进制目标中可用
// 注意：main.rs 中的模块声明会覆盖这里
