//! Logging utilities.
//!
//! This module provides utilities for logging throughout the system.
//! It defines log levels and provides helpers for structured logging.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Log level.
///
/// This enum represents the different log levels in the system,
/// ordered by increasing severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    /// Verbose debug information.
    Trace,

    /// Debug information.
    Debug,

    /// Informational messages.
    Info,

    /// Warning messages.
    Warning,

    /// Error messages.
    Error,
}

impl LogLevel {
    /// Convert from a string.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to convert from. Case-insensitive.
    ///
    /// # Returns
    ///
    /// The corresponding log level, or `None` if the string is not a valid log level.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Some(Self::Trace),
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warning" | "warn" => Some(Self::Warning),
            "error" | "err" => Some(Self::Error),
            _ => None,
        }
    }

    /// Get the name of this log level.
    ///
    /// # Returns
    ///
    /// A string representation of this log level.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
        }
    }

    /// Get the numeric value of this log level.
    ///
    /// # Returns
    ///
    /// A numeric value representing the severity of this log level.
    /// Higher values indicate higher severity.
    pub fn as_number(&self) -> u8 {
        match self {
            Self::Trace => 0,
            Self::Debug => 1,
            Self::Info => 2,
            Self::Warning => 3,
            Self::Error => 4,
        }
    }

    /// Check if this log level is at least as severe as the given level.
    ///
    /// # Arguments
    ///
    /// * `level` - The level to compare against.
    ///
    /// # Returns
    ///
    /// `true` if this log level is at least as severe as the given level,
    /// `false` otherwise.
    pub fn is_at_least(&self, level: LogLevel) -> bool {
        self.as_number() >= level.as_number()
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A log record.
///
/// This structure represents a log record in the system, including
/// the level, message, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    /// The log level.
    pub level: LogLevel,

    /// The log message.
    pub message: String,

    /// The module path where the log was recorded.
    pub module_path: String,

    /// The file where the log was recorded.
    pub file: String,

    /// The line number where the log was recorded.
    pub line: u32,

    /// The timestamp when the log was recorded.
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Additional metadata.
    pub metadata: std::collections::HashMap<String, String>,
}

impl LogRecord {
    /// Create a new log record.
    ///
    /// # Arguments
    ///
    /// * `level` - The log level.
    /// * `message` - The log message.
    /// * `module_path` - The module path where the log was recorded.
    /// * `file` - The file where the log was recorded.
    /// * `line` - The line number where the log was recorded.
    ///
    /// # Returns
    ///
    /// A new log record with the given information and current timestamp.
    pub fn new(
        level: LogLevel,
        message: impl Into<String>,
        module_path: impl Into<String>,
        file: impl Into<String>,
        line: u32,
    ) -> Self {
        Self {
            level,
            message: message.into(),
            module_path: module_path.into(),
            file: file.into(),
            line,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata to this log record.
    ///
    /// # Arguments
    ///
    /// * `key` - The metadata key.
    /// * `value` - The metadata value.
    ///
    /// # Returns
    ///
    /// A reference to this log record for chaining.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Format this log record for display.
    ///
    /// # Returns
    ///
    /// A formatted string representation of this log record.
    pub fn format(&self) -> String {
        let mut result = format!(
            "{} [{}] - {} [{}:{}]",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.level,
            self.message,
            self.file,
            self.line
        );

        if !self.metadata.is_empty() {
            let metadata_str = self
                .metadata
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            result = format!("{} - {}", result, metadata_str);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error > LogLevel::Warning);
        assert!(LogLevel::Warning > LogLevel::Info);
        assert!(LogLevel::Info > LogLevel::Debug);
        assert!(LogLevel::Debug > LogLevel::Trace);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("trace"), Some(LogLevel::Trace));
        assert_eq!(LogLevel::from_str("TRACE"), Some(LogLevel::Trace));
        assert_eq!(LogLevel::from_str("debug"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("DEBUG"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("warning"), Some(LogLevel::Warning));
        assert_eq!(LogLevel::from_str("warn"), Some(LogLevel::Warning));
        assert_eq!(LogLevel::from_str("WARNING"), Some(LogLevel::Warning));
        assert_eq!(LogLevel::from_str("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("err"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("ERROR"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warning.as_str(), "WARNING");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_log_level_as_number() {
        assert_eq!(LogLevel::Trace.as_number(), 0);
        assert_eq!(LogLevel::Debug.as_number(), 1);
        assert_eq!(LogLevel::Info.as_number(), 2);
        assert_eq!(LogLevel::Warning.as_number(), 3);
        assert_eq!(LogLevel::Error.as_number(), 4);
    }

    #[test]
    fn test_log_level_is_at_least() {
        assert!(LogLevel::Error.is_at_least(LogLevel::Error));
        assert!(LogLevel::Error.is_at_least(LogLevel::Warning));
        assert!(LogLevel::Error.is_at_least(LogLevel::Info));
        assert!(LogLevel::Error.is_at_least(LogLevel::Debug));
        assert!(LogLevel::Error.is_at_least(LogLevel::Trace));

        assert!(!LogLevel::Trace.is_at_least(LogLevel::Debug));
        assert!(!LogLevel::Debug.is_at_least(LogLevel::Info));
        assert!(!LogLevel::Info.is_at_least(LogLevel::Warning));
        assert!(!LogLevel::Warning.is_at_least(LogLevel::Error));
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Trace.to_string(), "TRACE");
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Warning.to_string(), "WARNING");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn test_log_record() {
        let record = LogRecord::new(
            LogLevel::Info,
            "Test message",
            "test_module",
            "test_file.rs",
            42,
        );

        assert_eq!(record.level, LogLevel::Info);
        assert_eq!(record.message, "Test message");
        assert_eq!(record.module_path, "test_module");
        assert_eq!(record.file, "test_file.rs");
        assert_eq!(record.line, 42);

        // Add metadata
        let record = record
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");
        assert_eq!(record.metadata.get("key1").unwrap(), "value1");
        assert_eq!(record.metadata.get("key2").unwrap(), "value2");

        // Format
        let formatted = record.format();
        assert!(formatted.contains("[INFO]"));
        assert!(formatted.contains("Test message"));
        assert!(formatted.contains("test_file.rs:42"));
        assert!(formatted.contains("key1=value1"));
        assert!(formatted.contains("key2=value2"));
    }

    #[test]
    fn test_log_level_serialization() {
        let level = LogLevel::Info;
        let serialized = serde_json::to_string(&level).unwrap();
        let deserialized: LogLevel = serde_json::from_str(&serialized).unwrap();
        assert_eq!(level, deserialized);
    }

    #[test]
    fn test_log_record_serialization() {
        let record = LogRecord::new(
            LogLevel::Info,
            "Test message",
            "test_module",
            "test_file.rs",
            42,
        )
        .with_metadata("key1", "value1");

        let serialized = serde_json::to_string(&record).unwrap();
        let deserialized: LogRecord = serde_json::from_str(&serialized).unwrap();

        assert_eq!(record.level, deserialized.level);
        assert_eq!(record.message, deserialized.message);
        assert_eq!(record.module_path, deserialized.module_path);
        assert_eq!(record.file, deserialized.file);
        assert_eq!(record.line, deserialized.line);
        assert_eq!(
            record.metadata.get("key1"),
            deserialized.metadata.get("key1")
        );
    }
}
