use lion_ui_tauri::logging::{add_log, LogBuffer, LogEntry, LogLevel};
use std::sync::Arc;

#[tokio::test]
async fn test_log_without_window() {
    // Create a log buffer
    let buffer = LogBuffer::new(100);

    // Add a log without a window (should not emit an event but still add to buffer)
    add_log(
        LogLevel::Warning,
        "test",
        "No window message",
        &buffer,
        None,
    )
    .await;

    // Verify the log was added to the buffer
    let logs = buffer.get_all_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].level, LogLevel::Warning);
    assert_eq!(logs[0].message, "No window message");
}

#[tokio::test]
async fn test_multiple_logs() {
    // Create a log buffer
    let buffer = LogBuffer::new(100);

    // Add multiple logs
    for i in 0..5 {
        let level = match i % 4 {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warning,
            _ => LogLevel::Error,
        };

        add_log(level, "test_multi", format!("Message {}", i), &buffer, None).await;
    }

    // Verify all logs were added
    let logs = buffer.get_all_logs().await;
    assert_eq!(logs.len(), 5);

    // Verify the log levels
    assert_eq!(logs[0].level, LogLevel::Debug);
    assert_eq!(logs[1].level, LogLevel::Info);
    assert_eq!(logs[2].level, LogLevel::Warning);
    assert_eq!(logs[3].level, LogLevel::Error);
    assert_eq!(logs[4].level, LogLevel::Debug);
}

#[tokio::test]
async fn test_get_recent_logs_function() {
    // Create a log buffer
    let buffer = LogBuffer::new(100);

    // Add some logs
    for i in 0..10 {
        let entry = LogEntry::new(
            LogLevel::Info,
            "test_command",
            format!("Command test message {}", i),
        );
        buffer.add_log(entry).await;
    }

    // Get recent logs directly from the buffer
    let logs = buffer.get_recent_logs(5).await;

    // Verify the result
    assert_eq!(logs.len(), 5);

    // Verify we got the most recent logs
    assert_eq!(logs[0].message, "Command test message 5");
    assert_eq!(logs[4].message, "Command test message 9");
}
