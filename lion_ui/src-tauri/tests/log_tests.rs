use chrono::Utc;
use lion_ui_tauri::logging::{LogBuffer, LogEntry, LogLevel};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_log_buffer_creation() {
    // Create a log buffer with a maximum size
    let max_logs = 100;
    let buffer = LogBuffer::new(max_logs);

    // Verify the buffer is initially empty
    let logs = buffer.get_all_logs().await;
    assert!(logs.is_empty());
}

#[tokio::test]
async fn test_log_entry_creation() {
    // Create a log entry
    let level = LogLevel::Info;
    let source = "test_source";
    let message = "Test message";

    let entry = LogEntry::new(level, source, message);

    // Verify the entry has the correct values
    assert_eq!(entry.level, LogLevel::Info);
    assert_eq!(entry.source, "test_source");
    assert_eq!(entry.message, "Test message");
    assert!(!entry.id.is_empty());

    // Timestamp should be close to now
    let now = Utc::now();
    let diff = now.signed_duration_since(entry.timestamp);
    assert!(diff.num_seconds() < 5); // Should be created within 5 seconds
}

#[tokio::test]
async fn test_add_log_entry() {
    // Create a log buffer
    let buffer = LogBuffer::new(10);

    // Create and add a log entry
    let entry = LogEntry::new(LogLevel::Debug, "test", "Debug message");
    buffer.add_log(entry.clone()).await;

    // Verify the entry was added
    let logs = buffer.get_all_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].id, entry.id);
    assert_eq!(logs[0].level, LogLevel::Debug);
    assert_eq!(logs[0].message, "Debug message");
}

#[tokio::test]
async fn test_max_logs_limit() {
    // Create a log buffer with a small max size
    let max_logs = 3;
    let buffer = LogBuffer::new(max_logs);

    // Add more entries than the max size
    for i in 0..5 {
        let entry = LogEntry::new(LogLevel::Info, "test", format!("Message {}", i));
        buffer.add_log(entry).await;
    }

    // Verify only the most recent entries are kept
    let logs = buffer.get_all_logs().await;
    assert_eq!(logs.len(), max_logs);

    // The oldest entries should be removed first (FIFO)
    assert_eq!(logs[0].message, "Message 2");
    assert_eq!(logs[1].message, "Message 3");
    assert_eq!(logs[2].message, "Message 4");
}

#[tokio::test]
async fn test_get_recent_logs() {
    // Create a log buffer
    let buffer = LogBuffer::new(10);

    // Add several log entries
    for i in 0..5 {
        let entry = LogEntry::new(LogLevel::Info, "test", format!("Message {}", i));
        buffer.add_log(entry).await;
    }

    // Get a limited number of recent logs
    let recent_logs = buffer.get_recent_logs(3).await;

    // Verify we get the correct number and order
    assert_eq!(recent_logs.len(), 3);
    assert_eq!(recent_logs[0].message, "Message 2");
    assert_eq!(recent_logs[1].message, "Message 3");
    assert_eq!(recent_logs[2].message, "Message 4");
}

#[tokio::test]
async fn test_clear_logs() {
    // Create a log buffer
    let buffer = LogBuffer::new(10);

    // Add some log entries
    for i in 0..3 {
        let entry = LogEntry::new(LogLevel::Info, "test", format!("Message {}", i));
        buffer.add_log(entry).await;
    }

    // Verify logs were added
    assert_eq!(buffer.get_all_logs().await.len(), 3);

    // Clear the logs
    buffer.clear_logs().await;

    // Verify logs were cleared
    assert_eq!(buffer.get_all_logs().await.len(), 0);
}

#[tokio::test]
async fn test_set_max_logs() {
    // Create a log buffer with initial max size
    let buffer = LogBuffer::new(10);

    // Add some log entries
    for i in 0..8 {
        let entry = LogEntry::new(LogLevel::Info, "test", format!("Message {}", i));
        buffer.add_log(entry).await;
    }

    // Verify all logs were added
    assert_eq!(buffer.get_all_logs().await.len(), 8);

    // Reduce the max logs size
    buffer.set_max_logs(5).await;

    // Verify oldest logs were removed
    let logs = buffer.get_all_logs().await;
    assert_eq!(logs.len(), 5);
    assert_eq!(logs[0].message, "Message 3");
    assert_eq!(logs[4].message, "Message 7");
}

#[tokio::test]
async fn test_log_levels() {
    // Test all log levels
    let debug = LogEntry::new(LogLevel::Debug, "test", "Debug message");
    let info = LogEntry::new(LogLevel::Info, "test", "Info message");
    let warning = LogEntry::new(LogLevel::Warning, "test", "Warning message");
    let error = LogEntry::new(LogLevel::Error, "test", "Error message");

    assert_eq!(debug.level, LogLevel::Debug);
    assert_eq!(info.level, LogLevel::Info);
    assert_eq!(warning.level, LogLevel::Warning);
    assert_eq!(error.level, LogLevel::Error);

    // Test string representation
    assert_eq!(format!("{}", LogLevel::Debug), "Debug");
    assert_eq!(format!("{}", LogLevel::Info), "Info");
    assert_eq!(format!("{}", LogLevel::Warning), "Warning");
    assert_eq!(format!("{}", LogLevel::Error), "Error");
}
