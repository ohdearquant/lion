// Import from lion-ui-tauri crate (the Tauri backend)
use lion_ui_tauri::logging::{add_log, LogBuffer, LogEntry, LogLevel};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_log_buffer_with_logs() {
    let buffer = LogBuffer::new(10);

    // Add a test log
    buffer
        .add_log(LogEntry::new(LogLevel::Info, "test", "Test log message"))
        .await;

    // Verify log was added
    let logs = buffer.get_recent_logs(5).await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].level, LogLevel::Info);
    assert_eq!(logs[0].message, "Test log message");
}

#[tokio::test]
async fn test_log_levels_display() {
    assert_eq!(format!("{}", LogLevel::Debug), "Debug");
    assert_eq!(format!("{}", LogLevel::Info), "Info");
    assert_eq!(format!("{}", LogLevel::Warning), "Warning");
    assert_eq!(format!("{}", LogLevel::Error), "Error");
}
