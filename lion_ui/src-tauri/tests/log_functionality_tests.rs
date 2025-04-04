use lion_ui_tauri::logging::{add_log, LogBuffer, LogEntry, LogLevel};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_log_buffer_add_log() {
    // Create a log buffer with a maximum of 5 logs
    let log_buffer = LogBuffer::new(5);

    // Add some logs
    log_buffer
        .add_log(LogEntry::new(LogLevel::Info, "Test", "Test message 1"))
        .await;
    log_buffer
        .add_log(LogEntry::new(LogLevel::Debug, "Test", "Test message 2"))
        .await;
    log_buffer
        .add_log(LogEntry::new(LogLevel::Warning, "Test", "Test message 3"))
        .await;

    // Get recent logs
    let logs = log_buffer.get_recent_logs(10).await;

    // Verify the logs
    assert_eq!(logs.len(), 3);
    assert_eq!(logs[0].level, LogLevel::Info);
    assert_eq!(logs[0].message, "Test message 1");
    assert_eq!(logs[1].level, LogLevel::Debug);
    assert_eq!(logs[1].message, "Test message 2");
    assert_eq!(logs[2].level, LogLevel::Warning);
    assert_eq!(logs[2].message, "Test message 3");
}

#[tokio::test]
async fn test_log_buffer_max_logs() {
    // Create a log buffer with a maximum of 3 logs
    let log_buffer = LogBuffer::new(3);

    // Add more logs than the maximum
    log_buffer
        .add_log(LogEntry::new(LogLevel::Info, "Test", "Test message 1"))
        .await;
    log_buffer
        .add_log(LogEntry::new(LogLevel::Debug, "Test", "Test message 2"))
        .await;
    log_buffer
        .add_log(LogEntry::new(LogLevel::Warning, "Test", "Test message 3"))
        .await;
    log_buffer
        .add_log(LogEntry::new(LogLevel::Error, "Test", "Test message 4"))
        .await;
    log_buffer
        .add_log(LogEntry::new(LogLevel::Info, "Test", "Test message 5"))
        .await;

    // Get recent logs
    let logs = log_buffer.get_recent_logs(10).await;

    // Verify the logs (should only have the 3 most recent)
    assert_eq!(logs.len(), 3);
    assert_eq!(logs[0].level, LogLevel::Warning);
    assert_eq!(logs[0].message, "Test message 3");
    assert_eq!(logs[1].level, LogLevel::Error);
    assert_eq!(logs[1].message, "Test message 4");
    assert_eq!(logs[2].level, LogLevel::Info);
    assert_eq!(logs[2].message, "Test message 5");
}

#[tokio::test]
async fn test_log_buffer_get_recent_logs_limit() {
    // Create a log buffer with a maximum of 5 logs
    let log_buffer = LogBuffer::new(5);

    // Add some logs
    log_buffer
        .add_log(LogEntry::new(LogLevel::Info, "Test", "Test message 1"))
        .await;
    log_buffer
        .add_log(LogEntry::new(LogLevel::Debug, "Test", "Test message 2"))
        .await;
    log_buffer
        .add_log(LogEntry::new(LogLevel::Warning, "Test", "Test message 3"))
        .await;
    log_buffer
        .add_log(LogEntry::new(LogLevel::Error, "Test", "Test message 4"))
        .await;

    // Get recent logs with a limit of 2
    let logs = log_buffer.get_recent_logs(2).await;

    // Verify the logs (should only have the 2 most recent)
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].level, LogLevel::Warning);
    assert_eq!(logs[0].message, "Test message 3");
    assert_eq!(logs[1].level, LogLevel::Error);
    assert_eq!(logs[1].message, "Test message 4");
}
