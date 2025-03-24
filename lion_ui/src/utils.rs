use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use uuid::Uuid;

/// Custom error type for lion_ui operations
#[derive(Debug)]
pub enum LionUiError {
    /// Runtime error
    Runtime(String),

    /// Configuration error
    Config(String),

    /// Validation error
    Validation(String),

    /// Not found error
    NotFound(String),

    /// Plugin error
    Plugin(String),
}

impl fmt::Display for LionUiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(msg) => write!(f, "Runtime error: {}", msg),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Validation(msg) => write!(f, "Validation error: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::Plugin(msg) => write!(f, "Plugin error: {}", msg),
        }
    }
}

impl Error for LionUiError {}

/// Result type for lion_ui operations
pub type LionUiResult<T> = Result<T, LionUiError>;

/// Trace context for distributed tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// Trace ID
    pub trace_id: Uuid,

    /// Span ID
    pub span_id: Uuid,

    /// Parent span ID
    pub parent_span_id: Option<Uuid>,

    /// Trace start time
    pub start_time: DateTime<Utc>,
}

impl TraceContext {
    /// Create a new trace context
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            span_id: Uuid::new_v4(),
            parent_span_id: None,
            start_time: Utc::now(),
        }
    }

    /// Create a child span from this trace context
    pub fn create_child_span(&self) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: Uuid::new_v4(),
            parent_span_id: Some(self.span_id),
            start_time: Utc::now(),
        }
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Advanced pagination parameters for API endpoints
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-based)
    pub page: Option<usize>,

    /// Items per page
    pub per_page: Option<usize>,

    /// Continuation token for cursor-based pagination
    pub cursor: Option<String>,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    /// Overall status
    pub status: String,

    /// Version
    pub version: String,

    /// Uptime in seconds
    pub uptime: u64,

    /// Subsystem statuses
    pub subsystems: serde_json::Value,
}
