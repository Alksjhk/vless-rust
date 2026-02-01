use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use uuid::Uuid;
use anyhow::{Result, anyhow};

/// VLESS协议版本
pub const VLESS_VERSION_BETA: u8 = 0;  // 测试版本
pub const VLESS_VERSION_RELEASE: u8 = 1;  // 正式版本

/// VLESS命令类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    Tcp = 1,
    Udp = 2,
    Mux = 3,
}

impl TryFrom<u8> for Command {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Command::Tcp),
            2 => Ok(Command::Udp),
            3 => Ok(Command::Mux),
            _ => Err(anyhow!("Invalid command: {}", value)),
        }
    }
}

/// 地址类型
#[derive(Debug, Clone, PartialEq)]
pub enum AddressType {
    Ipv4 = 1,
    Domain = 2,
    Ipv6 = 3,
}

impl TryFrom<u8> for AddressType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(AddressType::Ipv4),
            2 => Ok(AddressType::Domain),
            3 => Ok(AddressType::Ipv6),
            _ => Err(anyhow!("Invalid address type: {}", value)),
        }
    }
}

/// 目标地址
#[derive(Debug, Clone, PartialEq)]
pub enum Address {
    Ipv4(Ipv4Addr),
    Domain(String),
    Ipv6(Ipv6Addr),
}

impl Address {
    pub fn decode(buf: &mut Bytes) -> Result<Self> {
        if buf.is_empty() {
            return Err(anyhow!("Empty buffer"));
        }

        let addr_type = AddressType::try_from(buf.get_u8())?;
        
        match addr_type {
            AddressType::Ipv4 => {
                if buf.len() < 4 {
                    return Err(anyhow!("Invalid IPv4 address length"));
                }
                let mut octets = [0u8; 4];
                buf.copy_to_slice(&mut octets);
                Ok(Address::Ipv4(Ipv4Addr::from(octets)))
            }
            AddressType::Domain => {
                if buf.is_empty() {
                    return Err(anyhow!("Empty domain length"));
                }
                let len = buf.get_u8() as usize;
                if buf.len() < len {
                    return Err(anyhow!("Invalid domain length"));
                }
                let domain = String::from_utf8(buf.split_to(len).to_vec())?;
                Ok(Address::Domain(domain))
            }
            AddressType::Ipv6 => {
                if buf.len() < 16 {
                    return Err(anyhow!("Invalid IPv6 address length"));
                }
                let mut octets = [0u8; 16];
                buf.copy_to_slice(&mut octets);
                Ok(Address::Ipv6(Ipv6Addr::from(octets)))
            }
        }
    }

    pub fn to_socket_addr(&self, port: u16) -> Result<SocketAddr> {
        match self {
            Address::Ipv4(addr) => Ok(SocketAddr::new((*addr).into(), port)),
            Address::Ipv6(addr) => Ok(SocketAddr::new((*addr).into(), port)),
            Address::Domain(_) => Err(anyhow!("Cannot convert domain to socket address directly")),
        }
    }
}

/// VLESS请求
#[derive(Debug, Clone)]
pub struct VlessRequest {
    pub version: u8,
    pub uuid: Uuid,
    #[allow(dead_code)]
    pub addons_length: u8,
    #[allow(dead_code)]
    pub addons: Vec<u8>,
    pub command: Command,
    pub port: u16,
    pub address: Address,
}

impl VlessRequest {
    pub fn decode(mut buf: Bytes) -> Result<(Self, Bytes)> {
        if buf.len() < 18 {
            return Err(anyhow!("Buffer too short for VLESS request"));
        }

        // 协议版本 - 支持版本0（测试版）和版本1（正式版）
        let version = buf.get_u8();
        if version != VLESS_VERSION_BETA && version != VLESS_VERSION_RELEASE {
            return Err(anyhow!("Unsupported VLESS version: {}", version));
        }

        // UUID (16字节)
        let mut uuid_bytes = [0u8; 16];
        buf.copy_to_slice(&mut uuid_bytes);
        let uuid = Uuid::from_bytes(uuid_bytes);

        // Addons长度
        let addons_length = buf.get_u8();
        
        // Addons数据
        let mut addons = vec![0u8; addons_length as usize];
        if addons_length > 0 {
            if buf.len() < addons_length as usize {
                return Err(anyhow!("Invalid addons length"));
            }
            buf.copy_to_slice(&mut addons);
        }

        // 命令
        let command = Command::try_from(buf.get_u8())?;

        // 端口
        let port = buf.get_u16();

        // 地址
        let address = Address::decode(&mut buf)?;

        let request = VlessRequest {
            version,
            uuid,
            addons_length,
            addons,
            command,
            port,
            address,
        };

        Ok((request, buf))
    }
}

/// VLESS响应
#[derive(Debug, Clone)]
pub struct VlessResponse {
    pub version: u8,
    pub addons_length: u8,
    pub addons: Vec<u8>,
}

impl VlessResponse {
    pub fn new_with_version(version: u8) -> Self {
        Self {
            version,  // 使用客户端相同的版本号
            addons_length: 0,
            addons: Vec::new(),
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(2 + self.addons.len());
        buf.put_u8(self.version);
        buf.put_u8(self.addons_length);
        if !self.addons.is_empty() {
            buf.put_slice(&self.addons);
        }
        buf.freeze()
    }
}