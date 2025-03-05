use axum::{
    extract::{Query, State},
    response::Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use crate::state::AppState;

/// Log severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// A structured log entry with metadata for advanced filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp when the log was created
    pub timestamp: DateTime<Utc>,
    
    /// Severity level of the log
    pub level: LogLevel,
    
    /// The message content
    pub message: String,
    
    /// Source of the log (agent, plugin, system, etc.)
    pub source: String,
    
    /// Optional agent ID if the log is from an agent
    pub agent_id: Option<Uuid>,
    
    /// Optional plugin ID if the log is from a plugin
    pub plugin_id: Option<Uuid>,
    
    /// Optional correlation ID for tracking related logs
    pub correlation_id: Option<Uuid>,
    
    /// Additional metadata as key-value pairs
    pub metadata: serde_json::Value,
}

impl LogEntry {
    /// Create a new log entry with minimal fields
    pub fn new(level: LogLevel, message: String, source: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            message,
            source: source.into(),
            agent_id: None,
            plugin_id: None,
            correlation_id: None,
            metadata: serde_json::Value::Null,
        }
    }
    
    /// Add an agent ID to this log entry
    pub fn with_agent_id(mut self, agent_id: Uuid) -> Self {
        self.agent_id = Some(agent_id);
        self
    }
    
    /// Add a plugin ID to this log entry
    pub fn with_plugin_id(mut self, plugin_id: Uuid) -> Self {
        self.plugin_id = Some(plugin_id);
        self
    }
    
    /// Add a correlation ID to this log entry
    pub fn with_correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
    
    /// Add metadata to this log entry
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Filter parameters for the log search endpoint
#[derive(Debug, Deserialize)]
pub struct LogFilter {
    /// Filter by log level (minimum level to include)
    pub level: Option<LogLevel>,
    
    /// Text search in the message field
    pub text: Option<String>,
    
    /// Filter by source
    pub source: Option<String>,
    
    /// Filter by agent ID
    pub agent_id: Option<Uuid>,
    
    /// Filter by plugin ID
    pub plugin_id: Option<Uuid>,
    
    /// Filter by correlation ID
    pub correlation_id: Option<Uuid>,
    
    /// Maximum number of logs to return
    pub limit: Option<usize>,
    
    /// Starting from (for pagination)
    pub offset: Option<usize>,
}

/// Handler for the advanced log search endpoint
pub async fn search_logs_handler(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<LogFilter>,
) -> Json<Vec<LogEntry>> {
    // Get a read lock on the log buffer
    let buffer = state.log_buffer.read().await;
    
    // Apply filters
    let filtered_logs = buffer.iter()
        // Filter by minimum log level if specified
        .filter(|log| {
            if let Some(level) = filter.level {
                match (level, log.level) {
                    (LogLevel::Trace, _) => true,
                    (LogLevel::Debug, LogLevel::Debug | LogLevel::Info | LogLevel::Warn | LogLevel::Error) => true,
                    (LogLevel::Info, LogLevel::Info | LogLevel::Warn | LogLevel::Error) => true,
                    (LogLevel::Warn, LogLevel::Warn | LogLevel::Error) => true,
                    (LogLevel::Error, LogLevel::Error) => true,
                    _ => false,
                }
            } else {
                true
            }
        })
        // Filter by source if specified
        .filter(|log| {
            filter.source.as_ref()
                .map(|s| log.source.contains(s))
                .unwrap_or(true)
        })
        // Filter by text if specified
        .filter(|log| {
            filter.text.as_ref()
                .map(|text| log.message.contains(text))
                .unwrap_or(true)
        })
        // Filter by agent ID if specified
        .filter(|log| {
            filter.agent_id
                .map(|id| log.agent_id.map(|log_id| log_id == id).unwrap_or(false))
                .unwrap_or(true)
        })
        // Filter by plugin ID if specified
        .filter(|log| {
            filter.plugin_id
                .map(|id| log.plugin_id.map(|log_id| log_id == id).unwrap_or(false))
                .unwrap_or(true)
        })
        // Filter by correlation ID if specified
        .filter(|log| {
            filter.correlation_id
                .map(|id| log.correlation_id.map(|log_id| log_id == id).unwrap_or(false))
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();
    
    // Apply pagination
    let offset = filter.offset.unwrap_or(0);
    let limit = filter.limit.unwrap_or(100);
    
    let paginated_logs = filtered_logs.into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    
    Json(paginated_logs)
}
