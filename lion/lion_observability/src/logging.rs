//! Structured logging for the observability system
//!
//! This module provides structured logging capabilities with
//! context propagation and capability-based access control.

use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::capability::{LogLevel, ObservabilityCapability, ObservabilityCapabilityChecker};
use crate::config::LoggingConfig;
use crate::context::Context;
use crate::error::ObservabilityError;
use crate::Result;
use serde::{Deserialize, Serialize};
use tracing::Level;

/// A structured log event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    /// Timestamp in ISO 8601 / RFC 3339 format
    pub timestamp: String,

    /// Log level
    pub level: LogLevel,

    /// Message
    pub message: String,

    /// Optional plugin ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,

    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Optional trace ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// Optional span ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,

    /// Additional attributes
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub attributes: std::collections::HashMap<String, serde_json::Value>,
}

impl LogEvent {
    /// Create a new log event
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        // Get the current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp =
            chrono::DateTime::<chrono::Utc>::from(std::time::UNIX_EPOCH + now).to_rfc3339();

        // Create the event
        let mut event = Self {
            timestamp,
            level,
            message: message.into(),
            plugin_id: None,
            request_id: None,
            trace_id: None,
            span_id: None,
            attributes: std::collections::HashMap::new(),
        };

        // Add context information if available
        if let Some(ctx) = Context::current() {
            event.plugin_id = ctx.plugin_id.clone();
            event.request_id = ctx.request_id.clone();

            if let Some(span_ctx) = &ctx.span_context {
                event.trace_id = Some(span_ctx.trace_id.clone());
                event.span_id = Some(span_ctx.span_id.clone());
            }
        }

        event
    }

    /// Add an attribute to the log event
    pub fn with_attribute<K: Into<String>, V: Serialize>(
        mut self,
        key: K,
        value: V,
    ) -> Result<Self> {
        let json_value = serde_json::to_value(value)?;
        self.attributes.insert(key.into(), json_value);
        Ok(self)
    }

    /// Add multiple attributes to the log event
    pub fn with_attributes<K: Into<String>, V: Serialize>(
        mut self,
        attributes: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self> {
        for (key, value) in attributes {
            let json_value = serde_json::to_value(value)?;
            self.attributes.insert(key.into(), json_value);
        }
        Ok(self)
    }

    /// Convert the log event to JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Convert the log event to a formatted string
    pub fn to_string(&self) -> String {
        let level_str = match self.level {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };

        let mut result = format!("[{}] {} - {}", self.timestamp, level_str, self.message);

        if let Some(plugin_id) = &self.plugin_id {
            result.push_str(&format!(" [plugin:{}]", plugin_id));
        }

        if let Some(request_id) = &self.request_id {
            result.push_str(&format!(" [request:{}]", request_id));
        }

        if let (Some(trace_id), Some(span_id)) = (&self.trace_id, &self.span_id) {
            result.push_str(&format!(" [trace:{},span:{}]", trace_id, span_id));
        }

        if !self.attributes.is_empty() {
            result.push_str(" {");
            let attrs: Vec<String> = self
                .attributes
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();
            result.push_str(&attrs.join(", "));
            result.push('}');
        }

        result
    }
}

impl fmt::Display for LogEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Base logger trait for outputting log events (object-safe)
pub trait LoggerBase: Send + Sync {
    /// Log an event
    fn log(&self, event: LogEvent) -> Result<()>;

    /// Shutdown the logger
    fn shutdown(&self) -> Result<()>;

    /// Get the name of the logger
    fn name(&self) -> &str;
}

/// Logger trait with convenience methods
pub trait Logger: LoggerBase {
    /// Log a message at the specified level with a string slice
    fn log_message(&self, level: LogLevel, message: &str) -> Result<()> {
        self.log(LogEvent::new(level, message.to_string()))
    }

    /// Log at trace level with a string slice
    fn trace(&self, message: &str) -> Result<()> {
        self.log_message(LogLevel::Trace, message)
    }

    /// Log at debug level with a string slice
    fn debug(&self, message: &str) -> Result<()> {
        self.log_message(LogLevel::Debug, message)
    }

    /// Log at info level with a string slice
    fn info(&self, message: &str) -> Result<()> {
        self.log_message(LogLevel::Info, message)
    }

    /// Log at warn level with a string slice
    fn warn(&self, message: &str) -> Result<()> {
        self.log_message(LogLevel::Warn, message)
    }

    /// Log at error level with a string slice
    fn error(&self, message: &str) -> Result<()> {
        self.log_message(LogLevel::Error, message)
    }
}

// Blanket implementation for all types that implement LoggerBase
impl<T: LoggerBase> Logger for T {}

/// Extension trait for Logger with generic methods
pub trait LoggerExt: Logger {
    /// Log a message at the specified level with any type that can be converted to a string
    fn log_message_generic<S: AsRef<str>>(&self, level: LogLevel, message: S) -> Result<()> {
        self.log_message(level, message.as_ref())
    }

    /// Log at trace level with any type that can be converted to a string
    fn trace_generic<S: AsRef<str>>(&self, message: S) -> Result<()> {
        self.trace(message.as_ref())
    }

    /// Log at debug level with any type that can be converted to a string
    fn debug_generic<S: AsRef<str>>(&self, message: S) -> Result<()> {
        self.debug(message.as_ref())
    }

    /// Log at info level with any type that can be converted to a string
    fn info_generic<S: AsRef<str>>(&self, message: S) -> Result<()> {
        self.info(message.as_ref())
    }

    /// Log at warn level with any type that can be converted to a string
    fn warn_generic<S: AsRef<str>>(&self, message: S) -> Result<()> {
        self.warn(message.as_ref())
    }

    /// Log at error level with any type that can be converted to a string
    fn error_generic<S: AsRef<str>>(&self, message: S) -> Result<()> {
        self.error(message.as_ref())
    }
}

// Blanket implementation for all types that implement Logger
impl<T: Logger> LoggerExt for T {}

/// Create a logger based on the configuration
pub fn create_logger(config: &LoggingConfig) -> Result<Box<dyn LoggerBase>> {
    if !config.enabled {
        let logger: Box<dyn LoggerBase> = Box::new(NoopLogger::new());
        return Ok(logger);
    }

    // For simplicity, we're using a simplified logger implementation
    let logger = SimpleLogger::new(config);
    let boxed_logger: Box<dyn LoggerBase> = Box::new(logger);
    Ok(boxed_logger)
}

/// Logger implementation that discards all logs
#[derive(Debug, Clone)]
pub struct NoopLogger {
    name: String,
}

impl NoopLogger {
    /// Create a new noop logger
    pub fn new() -> Self {
        Self {
            name: "noop_logger".to_string(),
        }
    }
}

impl LoggerBase for NoopLogger {
    fn log(&self, _event: LogEvent) -> Result<()> {
        // Discard the log
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        // Nothing to shut down
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A simple logger implementation that writes to stdout
#[derive(Debug, Clone)]
pub struct SimpleLogger {
    name: String,
    config: LoggingConfig,
}

impl SimpleLogger {
    /// Create a new simple logger
    pub fn new(config: &LoggingConfig) -> Self {
        Self {
            name: "simple_logger".to_string(),
            config: config.clone(),
        }
    }

    /// Convert LogLevel to tracing::Level
    fn to_tracing_level(level: LogLevel) -> Level {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl LoggerBase for SimpleLogger {
    fn log(&self, event: LogEvent) -> Result<()> {
        // For simplicity, just print to stdout
        if self.config.log_to_stdout {
            println!("{}", event);
        }

        // If structured logging is enabled, also print JSON
        if self.config.structured {
            if let Ok(json) = event.to_json() {
                println!("{}", json);
            }
        }

        // If file logging is enabled, we would write to file here
        // but for simplicity, we're skipping that

        // Use tracing if available
        let _level = Self::to_tracing_level(event.level);
        // Use a string format instead of passing the level directly
        let message = event.message.clone();
        match event.level {
            LogLevel::Trace => tracing::event!(Level::TRACE, "{}", message),
            LogLevel::Debug => tracing::event!(Level::DEBUG, "{}", message),
            LogLevel::Info => tracing::event!(Level::INFO, "{}", message),
            LogLevel::Warn => tracing::event!(Level::WARN, "{}", message),
            LogLevel::Error => tracing::event!(Level::ERROR, "{}", message),
        }

        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        // No special shutdown needed
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Logger implementation that enforces capability checks
pub struct CapabilityLogger {
    name: String,
    inner: Box<dyn LoggerBase>,
    checker: Arc<dyn ObservabilityCapabilityChecker>,
}

impl CapabilityLogger {
    /// Create a new capability logger
    pub fn new(
        inner: impl LoggerBase + 'static,
        checker: Arc<dyn ObservabilityCapabilityChecker>,
    ) -> Self {
        Self {
            name: format!("capability_logger({})", inner.name()),
            inner: Box::new(inner),
            checker,
        }
    }

    /// Check if the current plugin has the required capability
    fn check_capability(&self, level: LogLevel) -> Result<bool> {
        let plugin_id = Context::current()
            .and_then(|ctx| ctx.plugin_id)
            .unwrap_or_else(|| "unknown".to_string());

        self.checker
            .check_capability(&plugin_id, ObservabilityCapability::Log(level))
    }
}

impl LoggerBase for CapabilityLogger {
    fn log(&self, event: LogEvent) -> Result<()> {
        // Check if the plugin has the required capability
        if !self.check_capability(event.level)? {
            return Err(ObservabilityError::CapabilityError(format!(
                "Plugin {:?} does not have the capability to log at {:?} level",
                event.plugin_id, event.level
            )));
        }

        // Forward to the inner logger
        self.inner.log(event)
    }

    fn shutdown(&self) -> Result<()> {
        self.inner.shutdown()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Logger implementation that buffers logs
pub struct BufferedLogger {
    name: String,
    inner: Box<dyn LoggerBase>,
    buffer: dashmap::DashMap<String, Vec<LogEvent>>,
    max_buffer_size: usize,
}

impl BufferedLogger {
    /// Create a new buffered logger
    pub fn new(inner: impl LoggerBase + 'static, max_buffer_size: usize) -> Self {
        Self {
            name: format!("buffered_logger({})", inner.name()),
            inner: Box::new(inner),
            buffer: dashmap::DashMap::new(),
            max_buffer_size,
        }
    }

    /// Flush the buffer for a specific plugin
    pub fn flush_plugin(&self, plugin_id: &str) -> Result<()> {
        if let Some((_, events)) = self.buffer.remove(plugin_id) {
            for event in events {
                self.inner.log(event)?;
            }
        }
        Ok(())
    }

    /// Flush all buffers
    pub fn flush_all(&self) -> Result<()> {
        for entry in self.buffer.iter() {
            let plugin_id = entry.key().clone();
            self.flush_plugin(&plugin_id)?;
        }
        Ok(())
    }
}

impl LoggerBase for BufferedLogger {
    fn log(&self, event: LogEvent) -> Result<()> {
        // Try to log directly
        match self.inner.log(event.clone()) {
            Ok(_) => Ok(()),
            Err(ObservabilityError::CapabilityError(_)) => {
                // Buffer the event if the capability check failed
                let plugin_id = event
                    .plugin_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                let mut entry = self.buffer.entry(plugin_id).or_insert_with(Vec::new);

                // Check buffer size
                if entry.len() < self.max_buffer_size {
                    entry.push(event);
                    Ok(())
                } else {
                    Err(ObservabilityError::LoggingError(
                        "Buffer full, log event dropped".to_string(),
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    fn shutdown(&self) -> Result<()> {
        // Flush all buffers
        self.flush_all()?;
        self.inner.shutdown()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{AllowAllCapabilityChecker, DenyAllCapabilityChecker};

    #[test]
    fn test_log_event_creation() {
        let event = LogEvent::new(LogLevel::Info, "Test message");
        assert_eq!(event.level, LogLevel::Info);
        assert_eq!(event.message, "Test message");
    }

    #[test]
    fn test_log_event_with_attributes() {
        let event = LogEvent::new(LogLevel::Debug, "Test message")
            .with_attribute("key1", "value1")
            .unwrap()
            .with_attribute("key2", 42)
            .unwrap();

        assert_eq!(event.attributes.len(), 2);
        assert_eq!(
            event.attributes.get("key1").unwrap(),
            &serde_json::Value::String("value1".to_string())
        );
        assert_eq!(
            event.attributes.get("key2").unwrap(),
            &serde_json::Value::Number(42.into())
        );
    }

    #[test]
    fn test_noop_logger() {
        let logger = NoopLogger::new();
        assert!(logger.info("Test message").is_ok());
        assert!(logger.error("Error message").is_ok());
    }

    #[test]
    fn test_capability_logger_allow() {
        let inner = NoopLogger::new();
        let checker = Arc::new(AllowAllCapabilityChecker);
        let logger = CapabilityLogger::new(inner, checker);

        assert!(logger.info("Test message").is_ok());
        assert!(logger.error("Error message").is_ok());
    }

    #[test]
    fn test_capability_logger_deny() {
        let inner = NoopLogger::new();
        let checker = Arc::new(DenyAllCapabilityChecker);
        let logger = CapabilityLogger::new(inner, checker);

        assert!(logger.info("Test message").is_err());
        assert!(logger.error("Error message").is_err());
    }

    #[test]
    fn test_buffered_logger() {
        let inner = NoopLogger::new();
        let checker = Arc::new(DenyAllCapabilityChecker);
        let inner_cap = CapabilityLogger::new(inner, checker);
        let logger = BufferedLogger::new(inner_cap, 10);

        // These logs will be buffered due to capability denial
        assert!(logger.info("Test message").is_ok());
        assert!(logger.error("Error message").is_ok());

        // Check buffer state (implementation detail)
        assert_eq!(logger.buffer.len(), 1);
        let unknown_buffer = logger.buffer.get("unknown").unwrap();
        assert_eq!(unknown_buffer.len(), 2);
    }
}
