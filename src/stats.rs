use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::config::MonitoringConfig;
use crate::time::UtcTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorData {
    pub timestamp: String,         // 当前 Unix 时间戳（秒）
    pub upload_speed: String,
    pub download_speed: String,
    pub total_traffic: String,
    pub uptime: String,
    pub memory_usage: String,
    pub total_memory: String,
    pub active_connections: usize,
    pub max_connections: usize,
    pub rejected_connections: u64, // 拒绝的连接总数
    pub public_ip: String,         // 服务器公网IP
    pub users: Vec<UserMonitorData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMonitorData {
    pub uuid: String,
    pub email: Option<String>,
    pub upload_speed: String,
    pub download_speed: String,
    pub total_traffic: String,
    pub active_connections: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedHistoryItem {
    pub timestamp: String,
    pub upload_speed: String,
    pub download_speed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedHistoryResponse {
    pub history: Vec<SpeedHistoryItem>,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone)]
struct SpeedSnapshot {
    upload_bytes: u64,
    download_bytes: u64,
    timestamp: Instant,
    upload_speed: f64,
    download_speed: f64,
}

#[derive(Debug, Clone)]
struct UserStats {
    uuid: String,
    email: Option<String>,
    total_upload_bytes: u64,
    total_download_bytes: u64,
    active_connections: usize,
    last_upload_snapshot: Option<SpeedSnapshot>,
    last_download_snapshot: Option<SpeedSnapshot>,
    current_upload_speed: f64,
    current_download_speed: f64,
}

pub struct Stats {
    total_upload_bytes: u64,      // 客户端上传的总字节数
    total_download_bytes: u64,    // 客户端下载的总字节数
    active_connections: usize,
    start_time: Instant,
    start_unix_time: i64,         // 服务器启动时的 Unix 时间戳（秒）
    speed_history: Vec<SpeedSnapshot>,
    config_path: String,
    last_upload_snapshot: Option<SpeedSnapshot>,
    last_download_snapshot: Option<SpeedSnapshot>,
    config: MonitoringConfig,
    user_stats: std::collections::HashMap<String, UserStats>,
    public_ip: String,            // 服务器公网IP
    rejected_connections: Arc<AtomicU64>, // 拒绝的连接数（原子操作）
}

impl Stats {
    pub fn new(config_path: String, monitoring_config: MonitoringConfig, public_ip: String) -> Self {
        let now = Instant::now();
        let start_unix_time = UtcTime::now().timestamp();

        let initial_snapshot = SpeedSnapshot {
            upload_bytes: 0,
            download_bytes: 0,
            timestamp: now,
            upload_speed: 0.0,
            download_speed: 0.0,
        };

        Self {
            total_upload_bytes: 0,
            total_download_bytes: 0,
            active_connections: 0,
            start_time: now,
            start_unix_time,
            speed_history: vec![initial_snapshot.clone()],
            config_path,
            last_upload_snapshot: Some(initial_snapshot.clone()),
            last_download_snapshot: Some(initial_snapshot),
            config: monitoring_config,
            user_stats: HashMap::new(),
            public_ip,
            rejected_connections: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn add_upload_bytes(&mut self, bytes: u64) {
        self.total_upload_bytes += bytes;
    }

    pub fn add_download_bytes(&mut self, bytes: u64) {
        self.total_download_bytes += bytes;
    }

    pub fn add_user_upload_bytes(&mut self, uuid: &str, bytes: u64, email: Option<String>) {
        let user_stats = self.user_stats.entry(uuid.to_string()).or_insert_with(|| UserStats {
            uuid: uuid.to_string(),
            email: email.clone(),
            total_upload_bytes: 0,
            total_download_bytes: 0,
            active_connections: 0,
            last_upload_snapshot: None,
            last_download_snapshot: None,
            current_upload_speed: 0.0,
            current_download_speed: 0.0,
        });
        user_stats.total_upload_bytes += bytes;
        if email.is_some() && user_stats.email.is_none() {
            user_stats.email = email;
        }
    }

    pub fn add_user_download_bytes(&mut self, uuid: &str, bytes: u64, email: Option<String>) {
        let user_stats = self.user_stats.entry(uuid.to_string()).or_insert_with(|| UserStats {
            uuid: uuid.to_string(),
            email: email.clone(),
            total_upload_bytes: 0,
            total_download_bytes: 0,
            active_connections: 0,
            last_upload_snapshot: None,
            last_download_snapshot: None,
            current_upload_speed: 0.0,
            current_download_speed: 0.0,
        });
        user_stats.total_download_bytes += bytes;
        if email.is_some() && user_stats.email.is_none() {
            user_stats.email = email;
        }
    }

    pub fn increment_user_connection(&mut self, uuid: &str, email: Option<String>) {
        let user_stats = self.user_stats.entry(uuid.to_string()).or_insert_with(|| UserStats {
            uuid: uuid.to_string(),
            email: email.clone(),
            total_upload_bytes: 0,
            total_download_bytes: 0,
            active_connections: 0,
            last_upload_snapshot: None,
            last_download_snapshot: None,
            current_upload_speed: 0.0,
            current_download_speed: 0.0,
        });
        user_stats.active_connections += 1;
        if email.is_some() && user_stats.email.is_none() {
            user_stats.email = email;
        }
    }

    pub fn decrement_user_connection(&mut self, uuid: &str) {
        if let Some(user_stats) = self.user_stats.get_mut(uuid) {
            if user_stats.active_connections > 0 {
                user_stats.active_connections -= 1;
            }
        }
    }

    pub fn increment_connections(&mut self) {
        self.active_connections += 1;
    }

    pub fn decrement_connections(&mut self) {
        if self.active_connections > 0 {
            self.active_connections -= 1;
        }
    }

    pub fn get_active_connections(&self) -> usize {
        self.active_connections
    }

    pub fn increment_rejected_connections(&self) {
        self.rejected_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_rejected_connections(&self) -> u64 {
        self.rejected_connections.load(Ordering::Relaxed)
    }

    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn get_memory_usage(&mut self) -> u64 {
        crate::memory::get_process_memory()
    }

    pub fn get_total_memory(&mut self) -> u64 {
        crate::memory::get_total_memory()
    }

    pub fn calculate_speeds(&mut self) -> (f64, f64) {
        let now = Instant::now();

        let (upload_speed, download_speed) = if let Some(last_snapshot) = self.last_upload_snapshot.take() {
            let duration_secs = now.duration_since(last_snapshot.timestamp).as_secs_f64();

            if duration_secs < 0.1 {
                self.last_upload_snapshot = Some(last_snapshot);
                self.last_download_snapshot = Some(SpeedSnapshot {
                    upload_bytes: self.total_upload_bytes,
                    download_bytes: self.total_download_bytes,
                    timestamp: now,
                    upload_speed: 0.0,
                    download_speed: 0.0,
                });
                (0.0, 0.0)
            } else {
                let upload_diff = self.total_upload_bytes.saturating_sub(last_snapshot.upload_bytes);
                let download_diff = self.total_download_bytes.saturating_sub(last_snapshot.download_bytes);

                let upload_speed = (upload_diff as f64) / duration_secs;
                let download_speed = (download_diff as f64) / duration_secs;

                self.last_upload_snapshot = Some(SpeedSnapshot {
                    upload_bytes: self.total_upload_bytes,
                    download_bytes: self.total_download_bytes,
                    timestamp: now,
                    upload_speed,
                    download_speed,
                });
                self.last_download_snapshot = Some(SpeedSnapshot {
                    upload_bytes: self.total_upload_bytes,
                    download_bytes: self.total_download_bytes,
                    timestamp: now,
                    upload_speed,
                    download_speed,
                });
                (upload_speed, download_speed)
            }
        } else {
            let snapshot = SpeedSnapshot {
                upload_bytes: self.total_upload_bytes,
                download_bytes: self.total_download_bytes,
                timestamp: now,
                upload_speed: 0.0,
                download_speed: 0.0,
            };
            self.last_upload_snapshot = Some(snapshot.clone());
            self.last_download_snapshot = Some(snapshot);
            (0.0, 0.0)
        };

        if let Some(last_snapshot) = &self.last_upload_snapshot {
            self.speed_history.push(last_snapshot.clone());
            self.speed_history.retain(|s| now.duration_since(s.timestamp) < Duration::from_secs(self.config.speed_history_duration));
        }

        // 计算所有用户的速度
        for user_stats in self.user_stats.values_mut() {
            let (user_upload_speed, user_download_speed) = Self::calculate_user_speed_internal(user_stats, now);
            user_stats.current_upload_speed = user_upload_speed;
            user_stats.current_download_speed = user_download_speed;
        }

        (upload_speed, download_speed)
    }

    fn calculate_user_speed_internal(user_stats: &mut UserStats, now: Instant) -> (f64, f64) {
        let upload_speed = if let Some(last_snapshot) = user_stats.last_upload_snapshot.take() {
            let duration_secs = now.duration_since(last_snapshot.timestamp).as_secs_f64();

            if duration_secs < 0.1 {
                user_stats.last_upload_snapshot = Some(last_snapshot);
                0.0
            } else {
                let upload_diff = user_stats.total_upload_bytes.saturating_sub(last_snapshot.upload_bytes);
                let speed = (upload_diff as f64) / duration_secs;

                user_stats.last_upload_snapshot = Some(SpeedSnapshot {
                    upload_bytes: user_stats.total_upload_bytes,
                    download_bytes: user_stats.total_download_bytes,
                    timestamp: now,
                    upload_speed: speed,
                    download_speed: 0.0,
                });
                speed
            }
        } else {
            let snapshot = SpeedSnapshot {
                upload_bytes: user_stats.total_upload_bytes,
                download_bytes: user_stats.total_download_bytes,
                timestamp: now,
                upload_speed: 0.0,
                download_speed: 0.0,
            };
            user_stats.last_upload_snapshot = Some(snapshot);
            0.0
        };

        let download_speed = if let Some(last_snapshot) = user_stats.last_download_snapshot.take() {
            let duration_secs = now.duration_since(last_snapshot.timestamp).as_secs_f64();

            if duration_secs < 0.1 {
                user_stats.last_download_snapshot = Some(last_snapshot);
                0.0
            } else {
                let download_diff = user_stats.total_download_bytes.saturating_sub(last_snapshot.download_bytes);
                let speed = (download_diff as f64) / duration_secs;

                user_stats.last_download_snapshot = Some(SpeedSnapshot {
                    upload_bytes: user_stats.total_upload_bytes,
                    download_bytes: user_stats.total_download_bytes,
                    timestamp: now,
                    upload_speed: 0.0,
                    download_speed: speed,
                });
                speed
            }
        } else {
            let snapshot = SpeedSnapshot {
                upload_bytes: user_stats.total_upload_bytes,
                download_bytes: user_stats.total_download_bytes,
                timestamp: now,
                upload_speed: 0.0,
                download_speed: 0.0,
            };
            user_stats.last_download_snapshot = Some(snapshot);
            0.0
        };

        (upload_speed, download_speed)
    }

    pub fn get_speed_history_response(&self) -> SpeedHistoryResponse {
        let history: Vec<SpeedHistoryItem> = self.speed_history
            .iter()
            .map(|snapshot| {
                // 计算绝对 Unix 时间戳（秒）
                let unix_timestamp = self.start_unix_time + snapshot.timestamp.duration_since(self.start_time).as_secs() as i64;

                SpeedHistoryItem {
                    timestamp: unix_timestamp.to_string(),
                    upload_speed: format_speed(snapshot.upload_speed),
                    download_speed: format_speed(snapshot.download_speed),
                }
            })
            .collect();

        SpeedHistoryResponse {
            history,
            duration_seconds: self.config.speed_history_duration,
        }
    }

    pub fn get_all_user_stats(&self) -> Vec<UserMonitorData> {
        self.user_stats.values().map(|user| {
            let total_traffic = user.total_upload_bytes + user.total_download_bytes;
            UserMonitorData {
                uuid: user.uuid.clone(),
                email: user.email.clone(),
                upload_speed: format_speed(user.current_upload_speed),
                download_speed: format_speed(user.current_download_speed),
                total_traffic: format_bytes(total_traffic),
                active_connections: user.active_connections,
            }
        }).collect()
    }

    pub fn get_monitor_data(&mut self) -> MonitorData {
        let (upload_speed, download_speed) = self.calculate_speeds();

        let total_bytes = self.total_upload_bytes + self.total_download_bytes;

        // 计算当前 Unix 时间戳
        let now_unix = self.start_unix_time + self.start_time.elapsed().as_secs() as i64;

        let users: Vec<UserMonitorData> = self.user_stats.values().map(|user| {
            let total_traffic = user.total_upload_bytes + user.total_download_bytes;
            UserMonitorData {
                uuid: user.uuid.clone(),
                email: user.email.clone(),
                upload_speed: format_speed(user.current_upload_speed),
                download_speed: format_speed(user.current_download_speed),
                total_traffic: format_bytes(total_traffic),
                active_connections: user.active_connections,
            }
        }).collect();

        MonitorData {
            timestamp: now_unix.to_string(),
            upload_speed: format_speed(upload_speed),
            download_speed: format_speed(download_speed),
            total_traffic: format_bytes(total_bytes),
            uptime: format_duration(self.get_uptime()),
            memory_usage: format_bytes(self.get_memory_usage()),
            total_memory: format_bytes(self.get_total_memory()),
            active_connections: self.active_connections,
            max_connections: self.config.vless_max_connections,
            rejected_connections: self.get_rejected_connections(),
            public_ip: self.public_ip.clone(),
            users,
        }
    }

    pub fn load_from_config(&mut self) -> anyhow::Result<()> {
        if let Ok(content) = std::fs::read_to_string(&self.config_path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                // 加载总流量统计
                if let Some(monitor) = config.get("monitor") {
                    if let Some(sent) = monitor.get("total_upload_bytes").and_then(|v| v.as_u64()) {
                        self.total_upload_bytes = sent;
                    } else if let Some(sent) = monitor.get("total_bytes_sent").and_then(|v| v.as_u64()) {
                        // 兼容旧字段名
                        self.total_upload_bytes = sent;
                    }
                    if let Some(received) = monitor.get("total_download_bytes").and_then(|v| v.as_u64()) {
                        self.total_download_bytes = received;
                    } else if let Some(received) = monitor.get("total_bytes_received").and_then(|v| v.as_u64()) {
                        // 兼容旧字段名
                        self.total_download_bytes = received;
                    }

                    // 加载用户流量统计
                    if let Some(users) = monitor.get("users").and_then(|v| v.as_object()) {
                        for (uuid_str, user_data) in users {
                            if let Some(user_obj) = user_data.as_object() {
                                let upload = user_obj.get("total_upload_bytes")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let download = user_obj.get("total_download_bytes")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let email = user_obj.get("email")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                let user_stats = self.user_stats.entry(uuid_str.clone()).or_insert_with(|| UserStats {
                                    uuid: uuid_str.clone(),
                                    email: email.clone(),
                                    total_upload_bytes: 0,
                                    total_download_bytes: 0,
                                    active_connections: 0,
                                    last_upload_snapshot: None,
                                    last_download_snapshot: None,
                                    current_upload_speed: 0.0,
                                    current_download_speed: 0.0,
                                });
                                user_stats.total_upload_bytes = upload;
                                user_stats.total_download_bytes = download;
                                if email.is_some() && user_stats.email.is_none() {
                                    user_stats.email = email;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn save_to_config(&self) -> anyhow::Result<()> {
        let mut config = if let Ok(content) = std::fs::read_to_string(&self.config_path) {
            serde_json::from_str::<serde_json::Value>(&content)?
        } else {
            serde_json::json!({})
        };

        let users_data: serde_json::Map<String, serde_json::Value> = self.user_stats.iter().map(|(uuid, stats)| {
            (
                uuid.clone(),
                serde_json::json!({
                    "total_upload_bytes": stats.total_upload_bytes,
                    "total_download_bytes": stats.total_download_bytes,
                    "email": stats.email,
                })
            )
        }).collect();

        let monitor = serde_json::json!({
            "total_upload_bytes": self.total_upload_bytes,
            "total_download_bytes": self.total_download_bytes,
            "last_update": crate::time::utc_now_rfc3339(),
            "users": serde_json::Value::from(users_data)
        });

        config["monitor"] = monitor;

        std::fs::write(&self.config_path, serde_json::to_string_pretty(&config)?)?;
        Ok(())
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

fn format_speed(bytes_per_sec: f64) -> String {
    format_bytes(bytes_per_sec as u64) + "/s"
}

fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    
    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

pub type SharedStats = Arc<Mutex<Stats>>;

pub async fn start_stats_persistence(stats: SharedStats, _config_path: String) {
    let mut interval = tokio::time::interval(Duration::from_secs(600));
    
    loop {
        interval.tick().await;
        let stats_guard = stats.lock().await;
        if let Err(e) = stats_guard.save_to_config() {
            eprintln!("Failed to save stats to config: {}", e);
        }
    }
}
