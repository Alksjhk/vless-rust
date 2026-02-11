/// HTTP请求检测模块
/// 用于区分HTTP请求和VLESS协议请求
///
/// 检测数据是否为HTTP请求（支持 HTTP/1.x 和 HTTP/2）
pub fn is_http_request(data: &[u8]) -> bool {
    if data.len() < 3 {
        return false;
    }

    // HTTP/2 PRI 方法 (用于 HTTP/2 连接前言)
    if data.len() >= 3 && &data[..3] == b"PRI" {
        return true;
    }

    if data.len() < 4 {
        return false;
    }

    // HTTP/1.x 方法
    let prefix = &data[..4];
    matches!(prefix, b"GET " | b"POST" | b"HEAD" | b"PUT " | b"DELE" | b"OPTI" | b"PATC" | b"CONN" | b"TRAC")
}
