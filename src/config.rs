use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use uuid::Uuid;
use anyhow::Result;

/// 服务器配置文件格式
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerSettings,
    pub users: Vec<UserConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerSettings {
    pub listen: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserConfig {
    pub uuid: String,
    pub email: Option<String>,
}

impl Config {
    /// 从JSON字符串加载配置
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// 转换为JSON字符串
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 获取绑定地址
    pub fn bind_addr(&self) -> Result<SocketAddr> {
        let addr_str = format!("{}:{}", self.server.listen, self.server.port);
        Ok(addr_str.parse()?)
    }

    /// 获取用户UUID集合
    pub fn user_uuids(&self) -> Result<HashSet<Uuid>> {
        let mut uuids = HashSet::new();
        for user in &self.users {
            let uuid = Uuid::parse_str(&user.uuid)?;
            uuids.insert(uuid);
        }
        Ok(uuids)
    }

    /// 创建默认配置
    pub fn default() -> Self {
        Self {
            server: ServerSettings {
                listen: "0.0.0.0".to_string(),
                port: 443,
            },
            users: vec![
                UserConfig {
                    uuid: Uuid::new_v4().to_string(),
                    email: Some("user@example.com".to_string()),
                }
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = config.to_json().unwrap();
        let parsed = Config::from_json(&json).unwrap();
        
        assert_eq!(config.server.listen, parsed.server.listen);
        assert_eq!(config.server.port, parsed.server.port);
        assert_eq!(config.users.len(), parsed.users.len());
    }
}