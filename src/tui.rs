//! Terminal User Interface (TUI) module
//!
//! Provides TUI support for fixed header and scrollable logs.

use std::sync::mpsc::{Sender};
use tracing::Level;

/// Log entry structure
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: Level,
    pub message: String,
}

/// Custom tracing Layer that sends log entries to a channel
pub struct TuiLayer {
    sender: Sender<LogEntry>,
}

impl TuiLayer {
    pub fn new(sender: Sender<LogEntry>) -> Self {
        Self { sender }
    }
}

impl<S> tracing_subscriber::Layer<S> for TuiLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Extract metadata
        let metadata = event.metadata();
        let level = *metadata.level();
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();

        // Try to get the message from the event
        let mut message = String::new();
        let mut found_message = false;

        event.record(&mut MessageVisitor(&mut message, &mut found_message));

        // If no message field was found, use a default
        if !found_message {
            message = format!("event at {}", metadata.name());
        }

        let entry = LogEntry {
            timestamp,
            level,
            message,
        };

        // 使用 send 并忽略错误（通道满时丢弃日志，避免阻塞日志系统）
        // 注意：在 tracing Layer 中不能阻塞，否则会影响性能
        if self.sender.send(entry).is_err() {
            // 通道已关闭或已满，丢弃日志
        }
    }
}

struct MessageVisitor<'a>(&'a mut String, &'a mut bool);

impl tracing::field::Visit for MessageVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" && !*self.1 {
            *self.0 = value.to_string();
            *self.1 = true;
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" && !*self.1 {
            *self.0 = format!("{:?}", value);
            *self.1 = true;
        }
    }
}
