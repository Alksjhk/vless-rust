use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;

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

/// 原始监控数据（不含格式化字符串，用于在锁内快速复制）
#[derive(Debug, Clone)]
pub struct MonitorDataRaw {
    pub timestamp: i64,
    pub upload_speed: f64,
    pub download_speed: f64,
    pub total_bytes: u64,
    pub uptime_seconds: u64,
    pub memory_usage_bytes: u64,
    pub total_memory_bytes: u64,
    pub active_connections: usize,
    pub max_connections: usize,
    pub rejected_connections: u64,
    pub public_ip: String,
    pub users: Vec<UserMonitorDataRaw>,
}

/// 用户原始数据
#[derive(Debug, Clone)]
pub struct UserMonitorDataRaw {
    pub uuid: String,
    pub email: Option<String>,
    pub total_upload_bytes: u64,
    pub total_download_bytes: u64,
    pub current_upload_speed: f64,
    pub current_download_speed: f64,
    pub active_connections: usize,
}

impl MonitorDataRaw {
    /// 格式化原始数据为 MonitorData（在锁外调用）
    pub fn format(&self) -> MonitorData {
        MonitorData {
            timestamp: self.timestamp.to_string(),
            upload_speed: format_speed(self.upload_speed),
            download_speed: format_speed(self.download_speed),
            total_traffic: format_bytes(self.total_bytes),
            uptime: format_duration(Duration::from_secs(self.uptime_seconds)),
            memory_usage: format_bytes(self.memory_usage_bytes),
            total_memory: format_bytes(self.total_memory_bytes),
            active_connections: self.active_connections,
            max_connections: self.max_connections,
            rejected_connections: self.rejected_connections,
            public_ip: self.public_ip.clone(),
            users: self.users.iter().map(|u| UserMonitorData {
                uuid: u.uuid.clone(),
                email: u.email.clone(),
                upload_speed: format_speed(u.current_upload_speed),
                download_speed: format_speed(u.current_download_speed),
                total_traffic: format_bytes(u.total_upload_bytes + u.total_download_bytes),
                active_connections: u.active_connections,
            }).collect(),
        }
    }
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
    last_active: Instant,  // 最后活跃时间，用于增量速度计算
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
    _last_download_snapshot: Option<SpeedSnapshot>,  // 保留用于对称性，暂未使用
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
            _last_download_snapshot: Some(initial_snapshot),
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

    pub fn add_user_upload_bytes(&mut self, uuid: &str, bytes: u64, email: Option<&str>) {
        let now = Instant::now();
        let email_string = email.map(|e| e.to_string());
        let user_stats = self.user_stats.entry(uuid.to_string()).or_insert_with(|| UserStats {
            uuid: uuid.to_string(),
            email: email_string.clone(),
            total_upload_bytes: 0,
            total_download_bytes: 0,
            active_connections: 0,
            last_upload_snapshot: None,
            last_download_snapshot: None,
            current_upload_speed: 0.0,
            current_download_speed: 0.0,
            last_active: now,
        });
        user_stats.total_upload_bytes += bytes;
        user_stats.last_active = now;  // 更新活跃时间
        if email.is_some() && user_stats.email.is_none() {
            user_stats.email = email_string;
        }
    }

    pub fn add_user_download_bytes(&mut self, uuid: &str, bytes: u64, email: Option<&str>) {
        let now = Instant::now();
        let email_string = email.map(|e| e.to_string());
        let user_stats = self.user_stats.entry(uuid.to_string()).or_insert_with(|| UserStats {
            uuid: uuid.to_string(),
            email: email_string.clone(),
            total_upload_bytes: 0,
            total_download_bytes: 0,
            active_connections: 0,
            last_upload_snapshot: None,
            last_download_snapshot: None,
            current_upload_speed: 0.0,
            current_download_speed: 0.0,
            last_active: now,
        });
        user_stats.total_download_bytes += bytes;
        user_stats.last_active = now;  // 更新活跃时间
        if email.is_some() && user_stats.email.is_none() {
            user_stats.email = email_string;
        }
    }

    pub fn increment_user_connection(&mut self, uuid: &str, email: Option<&str>) {
        let now = Instant::now();
        let email_string = email.map(|e| e.to_string());
        let user_stats = self.user_stats.entry(uuid.to_string()).or_insert_with(|| UserStats {
            uuid: uuid.to_string(),
            email: email_string.clone(),
            total_upload_bytes: 0,
            total_download_bytes: 0,
            active_connections: 0,
            last_upload_snapshot: None,
            last_download_snapshot: None,
            current_upload_speed: 0.0,
            current_download_speed: 0.0,
            last_active: now,
        });
        user_stats.active_connections += 1;
        user_stats.last_active = now;  // 更新活跃时间
        if email.is_some() && user_stats.email.is_none() {
            user_stats.email = email_string;
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

    pub fn get_memory_usage(&self) -> u64 {
        crate::memory::get_process_memory()
    }

    pub fn get_total_memory(&self) -> u64 {
        crate::memory::get_total_memory()
    }

    /// 计算所有用户和全局的速度（保留用于未来功能）
    #[allow(dead_code)]
    pub fn calculate_speeds(&mut self) -> (f64, f64) {
        let (upload_speed, download_speed) = self.calculate_speeds_read_only();
        self.update_speed_snapshots(upload_speed, download_speed);

        // 计算活跃用户的速度
        let now = Instant::now();
        let inactive_threshold = Duration::from_secs(self.config.inactive_user_timeout);

        for user_stats in self.user_stats.values_mut() {
            // 检查用户是否活跃，跳过不活跃用户
            if now.duration_since(user_stats.last_active) > inactive_threshold {
                // 不活跃用户，速度保持为 0（如果长时间无流量）
                // 只有当超过2倍阈值时才重置速度为0，避免短暂波动
                if now.duration_since(user_stats.last_active) > inactive_threshold * 2 {
                    user_stats.current_upload_speed = 0.0;
                    user_stats.current_download_speed = 0.0;
                }
                continue;
            }

            // 活跃用户才计算速度
            let (user_upload_speed, user_download_speed) = Self::calculate_user_speed_internal(user_stats, now);
            user_stats.current_upload_speed = user_upload_speed;
            user_stats.current_download_speed = user_download_speed;
        }

        (upload_speed, download_speed)
    }

    /// 只读计算速度（不更新快照），用于 get_monitor_data_raw
    fn calculate_speeds_read_only(&self) -> (f64, f64) {
        let now = Instant::now();

        let (upload_speed, download_speed) = if let Some(last_snapshot) = &self.last_upload_snapshot {
            let duration_secs = now.duration_since(last_snapshot.timestamp).as_secs_f64();

            if duration_secs < 0.1 {
                (0.0, 0.0)
            } else {
                let upload_diff = self.total_upload_bytes.saturating_sub(last_snapshot.upload_bytes);
                let download_diff = self.total_download_bytes.saturating_sub(last_snapshot.download_bytes);

                let upload_speed = (upload_diff as f64) / duration_secs;
                let download_speed = (download_diff as f64) / duration_secs;

                (upload_speed, download_speed)
            }
        } else {
            (0.0, 0.0)
        };

        (upload_speed, download_speed)
    }

    /// 更新速度快照（保留用于未来功能）
    #[allow(dead_code)]
    fn update_speed_snapshots(&mut self, upload_speed: f64, download_speed: f64) {
        let now = Instant::now();
        let snapshot = SpeedSnapshot {
            upload_bytes: self.total_upload_bytes,
            download_bytes: self.total_download_bytes,
            timestamp: now,
            upload_speed,
            download_speed,
        };

        self.last_upload_snapshot = Some(snapshot.clone());
        self._last_download_snapshot = Some(snapshot);

        if let Some(last_snapshot) = &self.last_upload_snapshot {
            self.speed_history.push(last_snapshot.clone());
            self.speed_history.retain(|s| now.duration_since(s.timestamp) < Duration::from_secs(self.config.speed_history_duration));
        }
    }

    /// 计算单个用户的速度（内部方法，保留用于未来功能）
    #[allow(dead_code)]
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

    pub fn get_monitor_data_raw(&self) -> MonitorDataRaw {
        let (upload_speed, download_speed) = self.calculate_speeds_read_only();
        let total_bytes = self.total_upload_bytes + self.total_download_bytes;
        let now_unix = self.start_unix_time + self.start_time.elapsed().as_secs() as i64;

        let users: Vec<UserMonitorDataRaw> = self.user_stats.values().map(|user| {
            UserMonitorDataRaw {
                uuid: user.uuid.clone(),
                email: user.email.clone(),
                total_upload_bytes: user.total_upload_bytes,
                total_download_bytes: user.total_download_bytes,
                current_upload_speed: user.current_upload_speed,
                current_download_speed: user.current_download_speed,
                active_connections: user.active_connections,
            }
        }).collect();

        MonitorDataRaw {
            timestamp: now_unix,
            upload_speed,
            download_speed,
            total_bytes,
            uptime_seconds: self.get_uptime().as_secs(),
            memory_usage_bytes: self.get_memory_usage(),
            total_memory_bytes: self.get_total_memory(),
            active_connections: self.active_connections,
            max_connections: self.config.vless_max_connections,
            rejected_connections: self.get_rejected_connections(),
            public_ip: self.public_ip.clone(),
            users,
        }
    }

    /// 获取监控数据（格式化版本，保留用于API兼容性）
    #[allow(dead_code)]
    pub fn get_monitor_data(&mut self) -> MonitorData {
        let raw = self.get_monitor_data_raw();
        raw.format()
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
                                    last_active: Instant::now(),
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

pub type SharedStats = Arc<RwLock<Stats>>;

pub async fn start_stats_persistence(stats: SharedStats, _config_path: String) {
    let mut interval = tokio::time::interval(Duration::from_secs(600));

    loop {
        interval.tick().await;
        let stats_guard = stats.read().await;
        if let Err(e) = stats_guard.save_to_config() {
            eprintln!("Failed to save stats to config: {}", e);
        }
    }
}
