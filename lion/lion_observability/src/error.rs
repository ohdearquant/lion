//! Error types for the observability system

use std::fmt;
use thiserror::Error;

/// Error type for observability operations
#[derive(Error, Debug)]
pub enum ObservabilityError {
    /// Failed to initialize observability component
    #[error("Failed to initialize observability component: {0}")]
    InitializationError(String),

    /// Failed to shutdown observability component
    #[error("Failed to shutdown observability component: {0}")]
    ShutdownError(String),

    /// Failed to write log
    #[error("Failed to write log: {0}")]
    LoggingError(String),

    /// Failed to create or record span
    #[error("Failed to create or record span: {0}")]
    TracingError(String),

    /// Failed to record metric
    #[error("Failed to record metric: {0}")]
    MetricError(String),

    /// Missing capability for observability operation
    #[error("Missing capability for observability operation: {0}")]
    CapabilityError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigurationError(String),

    /// Invalid plugin ID
    #[error("Invalid plugin ID: {0}")]
    InvalidPluginId(String),

    /// I/O error during observability operation
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Tracing subscriber error
    #[error("Tracing subscriber error: {0}")]
    TracingSubscriberError(String),

    /// OpenTelemetry error
    #[error("OpenTelemetry error: {0}")]
    OpenTelemetryError(String),

    /// Metrics error
    #[error("Metrics error: {0}")]
    MetricsError(String),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<serde_json::Error> for ObservabilityError {
    fn from(err: serde_json::Error) -> Self {
        ObservabilityError::SerializationError(err.to_string())
    }
}

impl From<&str> for ObservabilityError {
    fn from(err: &str) -> Self {
        ObservabilityError::Unknown(err.to_string())
    }
}

impl From<String> for ObservabilityError {
    fn from(err: String) -> Self {
        ObservabilityError::Unknown(err)
    }
}

impl From<fmt::Error> for ObservabilityError {
    fn from(err: fmt::Error) -> Self {
        ObservabilityError::SerializationError(err.to_string())
    }
}

#[cfg(feature = "capability-integration")]
impl From<lion_capability::error::CapabilityError> for ObservabilityError {
    fn from(err: lion_capability::error::CapabilityError) -> Self {
        ObservabilityError::CapabilityError(err.to_string())
    }
}
