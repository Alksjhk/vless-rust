use crate::stats::SharedStats;
use crate::config::MonitoringConfig;
use anyhow::{Result, anyhow};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Asset;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    #[allow(dead_code)]
    raw_request: String,
}

pub fn is_http_request(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    let prefix = &data[..4];
    matches!(prefix, b"GET " | b"POST" | b"HEAD" | b"PUT " | b"DELE" | b"OPTI" | b"PATC" | b"CONN" | b"TRAC")
}

pub fn parse_http_request(data: &[u8]) -> Result<HttpRequest> {
    let request_str = std::str::from_utf8(data)
        .map_err(|_| anyhow!("Invalid UTF-8 in HTTP request"))?;

    let lines: Vec<&str> = request_str.lines().collect();
    if lines.is_empty() {
        return Err(anyhow!("Empty HTTP request"));
    }

    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid HTTP request line"));
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();

    let mut headers = Vec::new();
    for line in lines.iter().skip(1) {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    }

    let raw_request = request_str.to_string();

    Ok(HttpRequest {
        method,
        path,
        headers,
        raw_request,
    })
}

pub async fn handle_http_request(
    request: &HttpRequest,
    stats: SharedStats,
    monitoring_config: MonitoringConfig,
) -> Result<Vec<u8>> {
    match request.path.as_str() {
        "/" | "/index.html" => {
            serve_embedded_file("index.html", "text/html")
        }
        path if path.starts_with("/assets/") => {
            let relative_path = path.trim_start_matches('/');
            serve_embedded_file(relative_path, "")
        }
        "/vite.svg" => {
            serve_embedded_file("vite.svg", "image/svg+xml")
        }
        "/api/stats" => {
            let mut stats_guard = stats.lock().await;
            let monitor_data = stats_guard.get_monitor_data();
            let json = serde_json::to_string(&monitor_data)?;
            Ok(create_http_response_bytes(200, "application/json", json.as_bytes()))
        }
        "/api/speed-history" => {
            let stats_guard = stats.lock().await;
            let history = stats_guard.get_speed_history_response();
            let json = serde_json::to_string(&history)?;
            Ok(create_http_response_bytes(200, "application/json", json.as_bytes()))
        }
        "/api/config" => {
            let json = serde_json::to_string(&monitoring_config)?;
            Ok(create_http_response_bytes(200, "application/json", json.as_bytes()))
        }
        _ => {
            Ok(create_http_response(404, "text/plain", "Not Found"))
        }
    }
}

fn serve_embedded_file(path: &str, default_content_type: &str) -> Result<Vec<u8>> {
    match Asset::get(path) {
        Some(content) => {
            let content_type = if !default_content_type.is_empty() {
                default_content_type
            } else {
                guess_content_type(path)
            };

            let data = content.data.to_vec();
            Ok(create_http_response_bytes(200, content_type, &data))
        }
        None => {
            Ok(create_http_response(404, "text/plain", "File Not Found"))
        }
    }
}

fn guess_content_type(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "woff" | "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "eot" => "application/vnd.ms-fontobject",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

pub struct HttpResponseBuilder {
    status: u16,
    status_text: &'static str,
    content_type: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpResponseBuilder {
    fn new(status: u16, content_type: &str) -> Self {
        let status_text = match status {
            200 => "OK",
            404 => "Not Found",
            _ => "Unknown",
        };
        Self {
            status,
            status_text,
            content_type: content_type.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    fn security_headers(self) -> Self {
        self.header("X-Content-Type-Options", "nosniff")
            .header("X-Frame-Options", "SAMEORIGIN")
            .header("Content-Security-Policy",
                "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
                 style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:;")
            .header("Referrer-Policy", "strict-origin-when-cross-origin")
            .header("X-XSS-Protection", "1; mode=block")
    }

    fn body(mut self, body: &[u8]) -> Self {
        self.body = body.to_vec();
        self
    }

    fn build(self) -> Vec<u8> {
        let mut header = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            self.status, self.status_text, self.content_type, self.body.len()
        );
        for (name, value) in &self.headers {
            header.push_str(name);
            header.push_str(": ");
            header.push_str(value);
            header.push_str("\r\n");
        }
        header.push_str("\r\n");

        let mut response = header.into_bytes();
        response.extend_from_slice(&self.body);
        response
    }
}

fn create_http_response(status: u16, content_type: &str, body: &str) -> Vec<u8> {
    HttpResponseBuilder::new(status, content_type)
        .security_headers()
        .body(body.as_bytes())
        .build()
}

fn create_http_response_bytes(status: u16, content_type: &str, body: &[u8]) -> Vec<u8> {
    HttpResponseBuilder::new(status, content_type)
        .security_headers()
        .body(body)
        .build()
}
