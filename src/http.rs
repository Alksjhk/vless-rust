//! HTTP 请求检测和响应构建模块
//!
//! 用于区分 HTTP 请求和 VLESS 协议请求，并构建 HTTP 响应

use std::collections::HashMap;

/// 检测数据是否为 HTTP 请求（支持 HTTP/1.x 和 HTTP/2）
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
    matches!(
        prefix,
        b"GET " | b"POST" | b"HEAD" | b"PUT " | b"DELE" | b"OPTI" | b"PATC" | b"CONN" | b"TRAC"
    ) || (data.len() >= 4 && &data[..4] == b"POST")
}

/// HTTP 查询参数
#[derive(Debug, Clone)]
pub struct HttpQuery {
    /// 请求路径
    pub path: String,
    /// 查询参数
    pub params: HashMap<String, String>,
}

/// 解析 HTTP 请求
///
/// 从 HTTP 请求数据中提取路径和查询参数
///
/// # Arguments
/// * `data` - HTTP 请求数据
///
/// # Returns
/// * `Option<HttpQuery>` - 解析结果
pub fn parse_http_request(data: &[u8]) -> Option<HttpQuery> {
    // 将数据转换为字符串
    let request_str = std::str::from_utf8(data).ok()?;

    // 解析请求行: "GET /path?param=value HTTP/1.1"
    let first_line = request_str.lines().next()?;
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 {
        return None;
    }

    let uri = parts[1];

    // 分离路径和查询参数
    let (path, query_string) = if let Some(pos) = uri.find('?') {
        (&uri[..pos], &uri[pos + 1..])
    } else {
        (uri, "")
    };

    // 解析查询参数
    let mut params = HashMap::new();
    if !query_string.is_empty() {
        for pair in query_string.split('&') {
            if let Some(pos) = pair.find('=') {
                let key = urlencoding::decode(&pair[..pos]).ok()?;
                let value = urlencoding::decode(&pair[pos + 1..]).ok()?;
                params.insert(key.to_string(), value.to_string());
            }
        }
    }

    Some(HttpQuery {
        path: path.to_string(),
        params,
    })
}

/// 构建 HTTP 响应
fn build_response(status: u16, status_text: &str, content_type: &str, body: &str) -> Vec<u8> {
    let content_length = body.len();
    format!(
        "HTTP/1.1 {} {}\r\n\
Content-Type: {}\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\
X-Content-Type-Options: nosniff\r\n\
X-Frame-Options: DENY\r\n\
X-XSS-Protection: 1; mode=block\r\n\
Referrer-Policy: no-referrer\r\n\
Content-Security-Policy: default-src 'none'; style-src 'self' 'unsafe-inline' 'unsafe-hashes'; script-src 'none'\r\n\
\r\n\
{}",
        status, status_text, content_type, content_length, body
    )
    .into_bytes()
}

/// 构建 JSON 响应
pub fn build_json_response(json: &str) -> Vec<u8> {
    build_response(200, "OK", "application/json; charset=utf-8", json)
}

/// 构建 HTML 响应
#[allow(dead_code)]
pub fn build_html_response(html: &str) -> Vec<u8> {
    build_response(200, "OK", "text/html; charset=utf-8", html)
}

/// 构建 404 响应
pub fn build_404_response() -> Vec<u8> {
    let body = r#"{"success":false,"error":"Not Found"}"#;
    build_response(404, "Not Found", "application/json; charset=utf-8", body)
}

/// 构建 400 响应
pub fn build_400_response(error: &str) -> Vec<u8> {
    let body = format!(r#"{{"success":false,"error":"{}"}}"#, error);
    build_response(400, "Bad Request", "application/json; charset=utf-8", &body)
}

/// 从 HTTP 请求数据中提取请求路径
///
/// # Arguments
/// * `data` - HTTP 请求数据
///
/// # Returns
/// * `Option<String>` - 请求路径（包含安全检查）
pub fn extract_http_path(data: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    for line in text.lines() {
        if line.starts_with("GET ") || line.starts_with("POST ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let path = parts[1].to_string();
                // 安全检查：防止路径遍历攻击
                let decoded_path = urlencoding::decode(&path)
                    .map(|s| s.into_owned())
                    .unwrap_or_else(|_| path.clone());
                if decoded_path.contains("..") || decoded_path.contains('\\') {
                    return None;
                }
                return Some(path);
            }
        }
    }
    None
}

/// 从 HTTP 头中提取指定头的值
///
/// # Arguments
/// * `headers` - HTTP 头数据
/// * `header_name` - 头名称
///
/// # Returns
/// * `Option<String>` - 头的值
pub fn extract_header_value(headers: &[u8], header_name: &str) -> Option<String> {
    let text = String::from_utf8_lossy(headers);
    let target_lower = header_name.to_lowercase();
    for line in text.lines() {
        let lower_line = line.to_lowercase();
        if let Some(pos) = lower_line.find(':') {
            let name = &lower_line[..pos].trim();
            if *name == target_lower {
                // 防御性边界检查
                let value_start = (pos + 1).min(line.len());
                return Some(line[value_start..].trim().to_string());
            }
        }
    }
    None
}

/// 验证 HTTP 请求头的基本安全性
///
/// 检查 Content-Length 是否过大
pub fn validate_http_headers(headers: &[u8]) -> Option<&'static str> {
    if let Some(content_length) = extract_header_value(headers, "Content-Length") {
        let length: usize = content_length.parse().unwrap_or(0);
        if length > 1024 * 1024 {
            return Some("Content-Length too large");
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_request_root() {
        let data = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let query = parse_http_request(data).unwrap();
        assert_eq!(query.path, "/");
        assert!(query.params.is_empty());
    }

    #[test]
    fn test_parse_http_request_with_params() {
        let data = b"GET /?email=user@example.com HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let query = parse_http_request(data).unwrap();
        assert_eq!(query.path, "/");
        assert_eq!(query.params.get("email").unwrap(), "user@example.com");
    }

    #[test]
    fn test_parse_http_request_with_multiple_params() {
        let data = b"GET /test?foo=bar&baz=qux HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let query = parse_http_request(data).unwrap();
        assert_eq!(query.path, "/test");
        assert_eq!(query.params.get("foo").unwrap(), "bar");
        assert_eq!(query.params.get("baz").unwrap(), "qux");
    }
}
