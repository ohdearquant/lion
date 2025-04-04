use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::Mutex;
use uuid::Uuid;

// LogLevel enum for categorizing log entries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "Debug"),
            LogLevel::Info => write!(f, "Info"),
            LogLevel::Warning => write!(f, "Warning"),
            LogLevel::Error => write!(f, "Error"),
        }
    }
}

// Struct to represent a log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, source: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level,
            source: source.into(),
            message: message.into(),
        }
    }
}

// LogBuffer to store and manage log entries
#[derive(Default)]
pub struct LogBuffer {
    // VecDeque allows efficient additions/removals at both ends
    pub logs: Arc<Mutex<VecDeque<LogEntry>>>,
    // Maximum number of logs to keep in the buffer
    pub max_logs: Arc<Mutex<usize>>,
}

impl LogBuffer {
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(max_logs))),
            max_logs: Arc::new(Mutex::new(max_logs)),
        }
    }

    // Add a new log entry to the buffer
    pub async fn add_log(&self, entry: LogEntry) {
        let mut logs = self.logs.lock().await;
        let max_logs = *self.max_logs.lock().await;

        // Add the new log entry
        logs.push_back(entry);

        // Remove oldest entries if we exceed the maximum
        while logs.len() > max_logs {
            logs.pop_front();
        }
    }

    // Get recent logs, up to a specified limit
    pub async fn get_recent_logs(&self, limit: usize) -> Vec<LogEntry> {
        let logs = self.logs.lock().await;
        let max_logs = std::cmp::min(limit, logs.len());

        // Take the most recent logs
        logs.iter()
            .rev()
            .take(max_logs)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    // Get all logs
    pub async fn get_all_logs(&self) -> Vec<LogEntry> {
        let logs = self.logs.lock().await;
        logs.iter().cloned().collect()
    }

    // Clear all logs
    pub async fn clear_logs(&self) {
        let mut logs = self.logs.lock().await;
        logs.clear();
    }

    // Set the maximum number of logs to keep
    pub async fn set_max_logs(&self, max_logs: usize) {
        let mut max_logs_lock = self.max_logs.lock().await;
        *max_logs_lock = max_logs;

        // Remove excess logs if needed
        let mut logs = self.logs.lock().await;
        while logs.len() > max_logs {
            logs.pop_front();
        }
    }
}

// Helper function for adding logs from different parts of the application
pub async fn add_log(
    level: LogLevel,
    source: impl Into<String>,
    message: impl Into<String>,
    log_buffer: &LogBuffer,
    window: Option<&tauri::WebviewWindow>,
) {
    let entry = LogEntry::new(level, source, message);

    // If a window is provided, emit an event with the log entry
    if let Some(window) = window {
        let _ = window.emit_to(window.label(), "new_log_entry", entry.clone());
    }

    // Add the log to the buffer
    log_buffer.add_log(entry).await;
}
