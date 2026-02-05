use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::Mutex;

use crate::config::MonitoringConfig;
use crate::connection_pool::GlobalConnectionPools;
use crate::xtls::get_vision_stats;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorData {
    pub upload_speed: String,
    pub download_speed: String,
    pub total_traffic: String,
    pub uptime: String,
    pub memory_usage: String,
    pub total_memory: String,
    pub active_connections: usize,
    pub max_connections: usize,
    pub users: Vec<UserMonitorData>,
    pub connection_pool: Option<ConnectionPoolMonitorData>,
    pub xtls_vision: Option<XtlsVisionMonitorData>,
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
pub struct ConnectionPoolMonitorData {
    pub total_created: usize,
    pub total_reused: usize,
    pub total_closed: usize,
    pub current_active: usize,
    pub current_idle: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XtlsVisionMonitorData {
    pub active_connections: usize,
    pub total_detections: u64,
    pub splice_switches: u64,
    pub splice_bytes: u64,
    pub encrypted_bytes: u64,
    pub splice_ratio: f64,
    pub performance_gain: f64,
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
}

pub struct Stats {
    total_upload_bytes: u64,   // 客户端上传的总字节数
    total_download_bytes: u64, // 客户端下载的总字节数
    active_connections: usize,
    start_time: Instant,
    speed_history: Vec<SpeedSnapshot>,
    system: System,
    config_path: String,
    current_pid: u32,
    last_upload_snapshot: Option<SpeedSnapshot>,
    last_download_snapshot: Option<SpeedSnapshot>,
    config: MonitoringConfig,
    user_stats: std::collections::HashMap<String, UserStats>,
    connection_pools: Option<Arc<GlobalConnectionPools>>,
}

impl Stats {
    pub fn new(config_path: String, monitoring_config: MonitoringConfig) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let current_pid = std::process::id();
        let now = Instant::now();

        Self {
            total_upload_bytes: 0,
            total_download_bytes: 0,
            active_connections: 0,
            start_time: now,
            speed_history: Vec::new(),
            system,
            config_path,
            current_pid,
            last_upload_snapshot: Some(SpeedSnapshot {
                upload_bytes: 0,
                download_bytes: 0,
                timestamp: now,
                upload_speed: 0.0,
                download_speed: 0.0,
            }),
            last_download_snapshot: Some(SpeedSnapshot {
                upload_bytes: 0,
                download_bytes: 0,
                timestamp: now,
                upload_speed: 0.0,
                download_speed: 0.0,
            }),
            config: monitoring_config,
            user_stats: HashMap::new(),
            connection_pools: None,
        }
    }

    /// 设置连接池引用用于监控
    pub fn set_connection_pools(&mut self, pools: Arc<GlobalConnectionPools>) {
        self.connection_pools = Some(pools);
    }

    pub fn add_upload_bytes(&mut self, bytes: u64) {
        self.total_upload_bytes += bytes;
    }

    pub fn add_download_bytes(&mut self, bytes: u64) {
        self.total_download_bytes += bytes;
    }

    pub fn add_user_upload_bytes(&mut self, uuid: &str, bytes: u64, email: Option<String>) {
        let user_stats = self
            .user_stats
            .entry(uuid.to_string())
            .or_insert_with(|| UserStats {
                uuid: uuid.to_string(),
                email: email.clone(),
                total_upload_bytes: 0,
                total_download_bytes: 0,
                active_connections: 0,
            });
        user_stats.total_upload_bytes += bytes;
        if email.is_some() && user_stats.email.is_none() {
            user_stats.email = email;
        }
    }

    pub fn add_user_download_bytes(&mut self, uuid: &str, bytes: u64, email: Option<String>) {
        let user_stats = self
            .user_stats
            .entry(uuid.to_string())
            .or_insert_with(|| UserStats {
                uuid: uuid.to_string(),
                email: email.clone(),
                total_upload_bytes: 0,
                total_download_bytes: 0,
                active_connections: 0,
            });
        user_stats.total_download_bytes += bytes;
        if email.is_some() && user_stats.email.is_none() {
            user_stats.email = email;
        }
    }

    pub fn increment_user_connection(&mut self, uuid: &str, email: Option<String>) {
        let user_stats = self
            .user_stats
            .entry(uuid.to_string())
            .or_insert_with(|| UserStats {
                uuid: uuid.to_string(),
                email: email.clone(),
                total_upload_bytes: 0,
                total_download_bytes: 0,
                active_connections: 0,
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

    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn get_memory_usage(&mut self) -> u64 {
        self.system.refresh_processes();
        if let Some(process) = self
            .system
            .process(sysinfo::Pid::from_u32(self.current_pid))
        {
            process.memory()
        } else {
            0
        }
    }

    pub fn get_total_memory(&mut self) -> u64 {
        self.system.refresh_memory();
        self.system.total_memory()
    }

    pub fn calculate_speeds(&mut self) -> (f64, f64) {
        self.system.refresh_memory();
        let now = Instant::now();

        let (upload_speed, download_speed) =
            if let Some(last_snapshot) = self.last_upload_snapshot.take() {
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
                    let upload_diff = self
                        .total_upload_bytes
                        .saturating_sub(last_snapshot.upload_bytes);
                    let download_diff = self
                        .total_download_bytes
                        .saturating_sub(last_snapshot.download_bytes);

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
            self.speed_history.retain(|s| {
                now.duration_since(s.timestamp)
                    < Duration::from_secs(self.config.speed_history_duration)
            });
        }

        (upload_speed, download_speed)
    }

    pub fn get_speed_history_response(&self) -> SpeedHistoryResponse {
        let history: Vec<SpeedHistoryItem> = self
            .speed_history
            .iter()
            .map(|snapshot| SpeedHistoryItem {
                timestamp: snapshot
                    .timestamp
                    .duration_since(self.start_time)
                    .as_secs()
                    .to_string(),
                upload_speed: format_speed(snapshot.upload_speed),
                download_speed: format_speed(snapshot.download_speed),
            })
            .collect();

        SpeedHistoryResponse {
            history,
            duration_seconds: self.config.speed_history_duration,
        }
    }

    pub fn get_all_user_stats(&self) -> Vec<UserMonitorData> {
        self.user_stats
            .values()
            .map(|user| {
                let total_traffic = user.total_upload_bytes + user.total_download_bytes;
                UserMonitorData {
                    uuid: user.uuid.clone(),
                    email: user.email.clone(),
                    upload_speed: "0 B/s".to_string(),
                    download_speed: "0 B/s".to_string(),
                    total_traffic: format_bytes(total_traffic),
                    active_connections: user.active_connections,
                }
            })
            .collect()
    }

    pub fn get_monitor_data(&mut self) -> MonitorData {
        let (upload_speed, download_speed) = self.calculate_speeds();

        let total_bytes = self.total_upload_bytes + self.total_download_bytes;

        let users: Vec<UserMonitorData> = self
            .user_stats
            .values()
            .map(|user| {
                let total_traffic = user.total_upload_bytes + user.total_download_bytes;
                UserMonitorData {
                    uuid: user.uuid.clone(),
                    email: user.email.clone(),
                    upload_speed: "0 B/s".to_string(),
                    download_speed: "0 B/s".to_string(),
                    total_traffic: format_bytes(total_traffic),
                    active_connections: user.active_connections,
                }
            })
            .collect();

        // 获取连接池统计信息
        let connection_pool = if let Some(pools) = &self.connection_pools {
            // 使用 tokio::task::block_in_place 在同步上下文中调用异步方法
            let pool_stats = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(pools.get_stats())
            });
            
            let hit_rate = if pool_stats.cache_hits + pool_stats.cache_misses > 0 {
                (pool_stats.cache_hits as f64) / ((pool_stats.cache_hits + pool_stats.cache_misses) as f64) * 100.0
            } else {
                0.0
            };

            Some(ConnectionPoolMonitorData {
                total_created: pool_stats.total_created,
                total_reused: pool_stats.total_reused,
                total_closed: pool_stats.total_closed,
                current_active: pool_stats.current_active,
                current_idle: pool_stats.current_idle,
                cache_hits: pool_stats.cache_hits,
                cache_misses: pool_stats.cache_misses,
                hit_rate,
            })
        } else {
            None
        };

        // 获取XTLS Vision统计信息
        let xtls_vision = {
            let vision_stats = get_vision_stats();
            let active_connections = vision_stats.active_connections.load(std::sync::atomic::Ordering::Relaxed);
            let total_detections = vision_stats.detections.load(std::sync::atomic::Ordering::Relaxed);
            let splice_switches = vision_stats.splice_switches.load(std::sync::atomic::Ordering::Relaxed);
            let splice_bytes = vision_stats.splice_bytes.load(std::sync::atomic::Ordering::Relaxed);
            let encrypted_bytes = vision_stats.encrypted_bytes.load(std::sync::atomic::Ordering::Relaxed);
            
            let total_bytes = splice_bytes + encrypted_bytes;
            let splice_ratio = if total_bytes > 0 {
                (splice_bytes as f64) / (total_bytes as f64) * 100.0
            } else {
                0.0
            };
            
            // 估算性能提升：Splice模式比加密模式快约2-3倍
            let performance_gain = if total_bytes > 0 {
                let splice_gain = (splice_bytes as f64) * 2.0; // 2倍性能提升
                splice_gain / (total_bytes as f64) * 100.0
            } else {
                0.0
            };

            if active_connections > 0 || total_detections > 0 {
                Some(XtlsVisionMonitorData {
                    active_connections,
                    total_detections,
                    splice_switches,
                    splice_bytes,
                    encrypted_bytes,
                    splice_ratio,
                    performance_gain,
                })
            } else {
                None
            }
        };

        MonitorData {
            upload_speed: format_speed(upload_speed),
            download_speed: format_speed(download_speed),
            total_traffic: format_bytes(total_bytes),
            uptime: format_duration(self.get_uptime()),
            memory_usage: format_bytes(self.get_memory_usage()),
            total_memory: format_bytes(self.get_total_memory()),
            active_connections: self.active_connections,
            max_connections: self.config.vless_max_connections,
            users,
            connection_pool,
            xtls_vision,
        }
    }

    pub fn load_from_config(&mut self) -> anyhow::Result<()> {
        if let Ok(content) = std::fs::read_to_string(&self.config_path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                // 加载总流量统计
                if let Some(monitor) = config.get("monitor") {
                    if let Some(sent) = monitor.get("total_upload_bytes").and_then(|v| v.as_u64()) {
                        self.total_upload_bytes = sent;
                    } else if let Some(sent) =
                        monitor.get("total_bytes_sent").and_then(|v| v.as_u64())
                    {
                        // 兼容旧字段名
                        self.total_upload_bytes = sent;
                    }
                    if let Some(received) =
                        monitor.get("total_download_bytes").and_then(|v| v.as_u64())
                    {
                        self.total_download_bytes = received;
                    } else if let Some(received) =
                        monitor.get("total_bytes_received").and_then(|v| v.as_u64())
                    {
                        // 兼容旧字段名
                        self.total_download_bytes = received;
                    }

                    // 加载用户流量统计
                    if let Some(users) = monitor.get("users").and_then(|v| v.as_object()) {
                        for (uuid_str, user_data) in users {
                            if let Some(user_obj) = user_data.as_object() {
                                let upload = user_obj
                                    .get("total_upload_bytes")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let download = user_obj
                                    .get("total_download_bytes")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let email = user_obj
                                    .get("email")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                let user_stats = self
                                    .user_stats
                                    .entry(uuid_str.clone())
                                    .or_insert_with(|| UserStats {
                                        uuid: uuid_str.clone(),
                                        email: email.clone(),
                                        total_upload_bytes: 0,
                                        total_download_bytes: 0,
                                        active_connections: 0,
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

        let users_data: serde_json::Map<String, serde_json::Value> = self
            .user_stats
            .iter()
            .map(|(uuid, stats)| {
                (
                    uuid.clone(),
                    serde_json::json!({
                        "total_upload_bytes": stats.total_upload_bytes,
                        "total_download_bytes": stats.total_download_bytes,
                        "email": stats.email,
                    }),
                )
            })
            .collect();

        let monitor = serde_json::json!({
            "total_upload_bytes": self.total_upload_bytes,
            "total_download_bytes": self.total_download_bytes,
            "last_update": Utc::now().to_rfc3339(),
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
