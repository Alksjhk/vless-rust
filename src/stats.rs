use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use chrono::Utc;
use sysinfo::System;

use crate::config::MonitoringConfig;

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
    bytes_sent: u64,
    bytes_received: u64,
    timestamp: Instant,
    upload_speed: f64,
    download_speed: f64,
}

pub struct Stats {
    total_bytes_sent: u64,
    total_bytes_received: u64,
    active_connections: usize,
    start_time: Instant,
    speed_history: Vec<SpeedSnapshot>,
    system: System,
    config_path: String,
    current_pid: u32,
    last_sent_snapshot: Option<SpeedSnapshot>,
    last_received_snapshot: Option<SpeedSnapshot>,
    config: MonitoringConfig,
}

impl Stats {
    pub fn new(config_path: String, monitoring_config: MonitoringConfig) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let current_pid = std::process::id();
        let now = Instant::now();

        Self {
            total_bytes_sent: 0,
            total_bytes_received: 0,
            active_connections: 0,
            start_time: now,
            speed_history: Vec::new(),
            system,
            config_path,
            current_pid,
            last_sent_snapshot: Some(SpeedSnapshot {
                bytes_sent: 0,
                bytes_received: 0,
                timestamp: now,
                upload_speed: 0.0,
                download_speed: 0.0,
            }),
            last_received_snapshot: Some(SpeedSnapshot {
                bytes_sent: 0,
                bytes_received: 0,
                timestamp: now,
                upload_speed: 0.0,
                download_speed: 0.0,
            }),
            config: monitoring_config,
        }
    }

    pub fn add_sent_bytes(&mut self, bytes: u64) {
        self.total_bytes_sent += bytes;
    }

    pub fn add_received_bytes(&mut self, bytes: u64) {
        self.total_bytes_received += bytes;
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
        if let Some(process) = self.system.process(sysinfo::Pid::from_u32(self.current_pid)) {
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

        let (upload_speed, download_speed) = if let Some(last_snapshot) = self.last_sent_snapshot.take() {
            let duration_secs = now.duration_since(last_snapshot.timestamp).as_secs_f64();

            if duration_secs < 0.1 {
                self.last_sent_snapshot = Some(last_snapshot);
                self.last_received_snapshot = Some(SpeedSnapshot {
                    bytes_sent: self.total_bytes_sent,
                    bytes_received: self.total_bytes_received,
                    timestamp: now,
                    upload_speed: 0.0,
                    download_speed: 0.0,
                });
                (0.0, 0.0)
            } else {
                let sent_diff = self.total_bytes_sent.saturating_sub(last_snapshot.bytes_sent);
                let received_diff = self.total_bytes_received.saturating_sub(last_snapshot.bytes_received);

                let upload_speed = (sent_diff as f64) / duration_secs;
                let download_speed = (received_diff as f64) / duration_secs;

                self.last_sent_snapshot = Some(SpeedSnapshot {
                    bytes_sent: self.total_bytes_sent,
                    bytes_received: self.total_bytes_received,
                    timestamp: now,
                    upload_speed,
                    download_speed,
                });
                self.last_received_snapshot = Some(SpeedSnapshot {
                    bytes_sent: self.total_bytes_sent,
                    bytes_received: self.total_bytes_received,
                    timestamp: now,
                    upload_speed,
                    download_speed,
                });
                (upload_speed, download_speed)
            }
        } else {
            let snapshot = SpeedSnapshot {
                bytes_sent: self.total_bytes_sent,
                bytes_received: self.total_bytes_received,
                timestamp: now,
                upload_speed: 0.0,
                download_speed: 0.0,
            };
            self.last_sent_snapshot = Some(snapshot.clone());
            self.last_received_snapshot = Some(snapshot);
            (0.0, 0.0)
        };

        if let Some(last_snapshot) = &self.last_sent_snapshot {
            self.speed_history.push(last_snapshot.clone());
            self.speed_history.retain(|s| now.duration_since(s.timestamp) < Duration::from_secs(self.config.speed_history_duration));
        }

        (upload_speed, download_speed)
    }

    pub fn get_speed_history_response(&self) -> SpeedHistoryResponse {
        let history: Vec<SpeedHistoryItem> = self.speed_history
            .iter()
            .map(|snapshot| SpeedHistoryItem {
                timestamp: snapshot.timestamp.duration_since(self.start_time).as_secs().to_string(),
                upload_speed: format_speed(snapshot.upload_speed),
                download_speed: format_speed(snapshot.download_speed),
            })
            .collect();

        SpeedHistoryResponse {
            history,
            duration_seconds: self.config.speed_history_duration,
        }
    }

    pub fn get_monitor_data(&mut self) -> MonitorData {
        let (upload_speed, download_speed) = self.calculate_speeds();

        let total_bytes = self.total_bytes_sent + self.total_bytes_received;

        MonitorData {
            upload_speed: format_speed(upload_speed),
            download_speed: format_speed(download_speed),
            total_traffic: format_bytes(total_bytes),
            uptime: format_duration(self.get_uptime()),
            memory_usage: format_bytes(self.get_memory_usage()),
            total_memory: format_bytes(self.get_total_memory()),
            active_connections: self.active_connections,
            max_connections: self.config.vless_max_connections,
        }
    }

    pub fn load_from_config(&mut self) -> anyhow::Result<()> {
        if let Ok(content) = std::fs::read_to_string(&self.config_path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(monitor) = config.get("monitor") {
                    if let Some(sent) = monitor.get("total_bytes_sent").and_then(|v| v.as_u64()) {
                        self.total_bytes_sent = sent;
                    }
                    if let Some(received) = monitor.get("total_bytes_received").and_then(|v| v.as_u64()) {
                        self.total_bytes_received = received;
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
        
        let monitor = serde_json::json!({
            "total_bytes_sent": self.total_bytes_sent,
            "total_bytes_received": self.total_bytes_received,
            "last_update": Utc::now().to_rfc3339()
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
