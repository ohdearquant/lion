//! Structured logging for the observability system
//!
//! This module provides structured logging capabilities with
//! context propagation and capability-based access control.

use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{Level, Subscriber};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, FmtSubscriber, Registry};

use crate::capability::{LogLevel, ObservabilityCapability, ObservabilityCapabilityChecker};
use crate::config::LoggingConfig;
use crate::context::Context;
use crate::error::ObservabilityError;
use crate::Result;

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

/// Logger trait for outputting log events
pub trait Logger: Send + Sync {
    /// Log an event
    fn log(&self, event: LogEvent) -> Result<()>;

    /// Log a message at the specified level
    fn log_message(&self, level: LogLevel, message: impl AsRef<str>) -> Result<()> {
        self.log(LogEvent::new(level, message.as_ref().to_string()))
    }

    /// Log at trace level
    fn trace(&self, message: impl AsRef<str>) -> Result<()> {
        self.log_message(LogLevel::Trace, message)
    }

    /// Log at debug level
    fn debug(&self, message: impl AsRef<str>) -> Result<()> {
        self.log_message(LogLevel::Debug, message)
    }

    /// Log at info level
    fn info(&self, message: impl AsRef<str>) -> Result<()> {
        self.log_message(LogLevel::Info, message)
    }

    /// Log at warn level
    fn warn(&self, message: impl AsRef<str>) -> Result<()> {
        self.log_message(LogLevel::Warn, message)
    }

    /// Log at error level
    fn error(&self, message: impl AsRef<str>) -> Result<()> {
        self.log_message(LogLevel::Error, message)
    }

    /// Shutdown the logger
    fn shutdown(&self) -> Result<()>;

    /// Get the name of the logger
    fn name(&self) -> &str;
}

/// Create a logger based on the configuration
pub fn create_logger(config: &LoggingConfig) -> Result<impl Logger> {
    if !config.enabled {
        return Ok(NoopLogger::new());
    }

    let mut logger = TracingLogger::new(config)?;

    // Add file logging if configured
    if let Some(path) = &config.file_path {
        logger = logger.with_file(path, config.max_file_size, config.max_files)?;
    }

    // Add stdout logging if configured
    if config.log_to_stdout {
        logger = logger.with_stdout();
    }

    // Add stderr logging if configured
    if config.log_to_stderr {
        logger = logger.with_stderr();
    }

    Ok(logger)
}

/// Logger implementation that uses tracing
pub struct TracingLogger {
    name: String,
    initialized: AtomicBool,
    config: LoggingConfig,
}

impl TracingLogger {
    /// Create a new tracing logger
    pub fn new(config: &LoggingConfig) -> Result<Self> {
        Ok(Self {
            name: "tracing_logger".to_string(),
            initialized: AtomicBool::new(false),
            config: config.clone(),
        })
    }

    /// Add file logging
    pub fn with_file(
        self,
        path: impl AsRef<Path>,
        max_size: usize,
        max_files: usize,
    ) -> Result<Self> {
        // We're using initialization on first use, so just save the config
        let mut config = self.config.clone();
        config.file_path = Some(path.as_ref().to_path_buf());
        config.max_file_size = max_size;
        config.max_files = max_files;

        Ok(Self {
            name: self.name,
            initialized: AtomicBool::new(false),
            config,
        })
    }

    /// Add stdout logging
    pub fn with_stdout(self) -> Self {
        let mut config = self.config.clone();
        config.log_to_stdout = true;

        Self {
            name: self.name,
            initialized: AtomicBool::new(false),
            config,
        }
    }

    /// Add stderr logging
    pub fn with_stderr(self) -> Self {
        let mut config = self.config.clone();
        config.log_to_stderr = true;

        Self {
            name: self.name,
            initialized: AtomicBool::new(false),
            config,
        }
    }

    /// Initialize the tracing subscriber
    fn initialize(&self) -> Result<()> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Parse the log level
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&self.config.level));

        // Create the subscriber
        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_ansi(true)
            .with_writer(move || {
                // This creates a writer that writes to all configured outputs
                let mut writers: Vec<Box<dyn MakeWriter + Send + Sync>> = Vec::new();

                if self.config.log_to_stdout {
                    writers.push(Box::new(std::io::stdout));
                }

                if self.config.log_to_stderr {
                    writers.push(Box::new(std::io::stderr));
                }

                if let Some(path) = &self.config.file_path {
                    // Simple file writer - in production we'd use a rolling file writer
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .unwrap_or_else(|_| {
                            // Fallback to stdout if we can't open the file
                            eprintln!(
                                "Failed to open log file {}, falling back to stdout",
                                path.display()
                            );
                            std::fs::File::create("/dev/null").unwrap()
                        });

                    writers.push(Box::new(move || {
                        Box::new(std::io::BufWriter::new(file.try_clone().unwrap()))
                    }));
                }

                // If no writers specified, use stdout
                if writers.is_empty() {
                    writers.push(Box::new(std::io::stdout));
                }

                // Create a multi-writer that writes to all outputs
                let writers = writers;
                struct MultiWriter(Vec<Box<dyn std::io::Write + Send>>);

                impl std::io::Write for MultiWriter {
                    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                        for writer in &mut self.0 {
                            let _ = writer.write(buf);
                        }
                        Ok(buf.len())
                    }

                    fn flush(&mut self) -> std::io::Result<()> {
                        for writer in &mut self.0 {
                            let _ = writer.flush();
                        }
                        Ok(())
                    }
                }

                let mut multi_writer = MultiWriter(Vec::new());
                for writer_fn in &writers {
                    multi_writer.0.push(writer_fn.make_writer());
                }

                Box::new(multi_writer) as Box<dyn std::io::Write + Send>
            })
            .finish();

        // Set up JSON formatting if configured
        let subscriber = if self.config.structured {
            let format = tracing_subscriber::fmt::format()
                .json()
                .with_current_span(true)
                .with_span_list(true);
            let layer = tracing_subscriber::fmt::layer().event_format(format);
            Registry::default().with(layer).with(subscriber)
        } else {
            subscriber
        };

        // Initialize the subscriber
        subscriber.init();
        self.initialized.store(true, Ordering::SeqCst);

        Ok(())
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

impl Logger for TracingLogger {
    fn log(&self, event: LogEvent) -> Result<()> {
        // Initialize on first use
        if !self.initialized.load(Ordering::SeqCst) {
            self.initialize()?;
        }

        // Get the tracing level
        let level = Self::to_tracing_level(event.level);

        // Create the span attributes
        let mut attributes = event.attributes.clone();
        if let Some(plugin_id) = &event.plugin_id {
            attributes.insert(
                "plugin_id".to_string(),
                serde_json::Value::String(plugin_id.clone()),
            );
        }
        if let Some(request_id) = &event.request_id {
            attributes.insert(
                "request_id".to_string(),
                serde_json::Value::String(request_id.clone()),
            );
        }
        if let Some(trace_id) = &event.trace_id {
            attributes.insert(
                "trace_id".to_string(),
                serde_json::Value::String(trace_id.clone()),
            );
        }
        if let Some(span_id) = &event.span_id {
            attributes.insert(
                "span_id".to_string(),
                serde_json::Value::String(span_id.clone()),
            );
        }

        // Log using tracing
        tracing::event!(
            level,
            message = %event.message,
            timestamp = %event.timestamp,
            attributes = ?attributes,
        );

        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        // No special shutdown needed for tracing
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
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

impl Logger for NoopLogger {
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

/// Logger implementation that enforces capability checks
pub struct CapabilityLogger {
    name: String,
    inner: Box<dyn Logger>,
    checker: Arc<dyn ObservabilityCapabilityChecker>,
}

impl CapabilityLogger {
    /// Create a new capability logger
    pub fn new(
        inner: impl Logger + 'static,
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

impl Logger for CapabilityLogger {
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
    inner: Box<dyn Logger>,
    buffer: dashmap::DashMap<String, Vec<LogEvent>>,
    max_buffer_size: usize,
}

impl BufferedLogger {
    /// Create a new buffered logger
    pub fn new(inner: impl Logger + 'static, max_buffer_size: usize) -> Self {
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

impl Logger for BufferedLogger {
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
