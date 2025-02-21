use crate::events::{AppState, LogFilter, LogLine};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::test]
async fn test_log_buffer_add_and_search() {
    // Create app state with empty log buffer
    let state = Arc::new(AppState::new(tokio::sync::mpsc::channel(100).0, 1000));

    let agent_id = Uuid::new_v4();
    let plugin_id = Uuid::new_v4();

    // Add test logs
    {
        let mut buffer = state.log_buffer.write().await;
        buffer.push(LogLine {
            timestamp: chrono::Utc::now(),
            agent_id: Some(agent_id),
            plugin_id: None,
            correlation_id: None,
            message: "Agent started processing".to_string(),
        });
        buffer.push(LogLine {
            timestamp: chrono::Utc::now(),
            agent_id: None,
            plugin_id: Some(plugin_id),
            correlation_id: None,
            message: "Plugin invoked with input".to_string(),
        });
        buffer.push(LogLine {
            timestamp: chrono::Utc::now(),
            agent_id: Some(agent_id),
            plugin_id: Some(plugin_id),
            correlation_id: None,
            message: "Agent using plugin".to_string(),
        });
    }

    // Test search by agent ID
    let filter = LogFilter {
        agent: Some(agent_id.to_string()),
        plugin: None,
        text: None,
        limit: 1000,
    };
    let results = state.search_logs(&filter).await;
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|log| log.agent_id == Some(agent_id)));

    // Test search by plugin ID
    let filter = LogFilter {
        agent: None,
        plugin: Some(plugin_id.to_string()),
        text: None,
        limit: 1000,
    };
    let results = state.search_logs(&filter).await;
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|log| log.plugin_id == Some(plugin_id)));

    // Test search by text
    let filter = LogFilter {
        agent: None,
        plugin: None,
        text: Some("plugin".to_string()),
        limit: 1000,
    };
    let results = state.search_logs(&filter).await;
    assert_eq!(results.len(), 2);
    assert!(results
        .iter()
        .all(|log| log.message.to_lowercase().contains("plugin")));

    // Test combined search
    let filter = LogFilter {
        agent: Some(agent_id.to_string()),
        plugin: Some(plugin_id.to_string()),
        text: None,
        limit: 1000,
    };
    let results = state.search_logs(&filter).await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].message, "Agent using plugin");
}

#[tokio::test]
async fn test_log_buffer_size_limit() {
    let state = Arc::new(AppState::new(tokio::sync::mpsc::channel(100).0, 1000));

    let max_logs = 1000; // Should match MAX_LOG_BUFFER in events.rs

    // Add more than max_logs entries
    {
        let mut buffer = state.log_buffer.write().await;
        for i in 0..max_logs + 100 {
            buffer.push(LogLine {
                timestamp: chrono::Utc::now(),
                agent_id: None,
                plugin_id: None,
                correlation_id: None,
                message: format!("Log entry {}", i),
            });
        }
    }

    // Verify buffer is trimmed to max size
    let buffer = state.log_buffer.read().await;
    assert_eq!(buffer.len(), max_logs);

    // Verify we kept the most recent logs
    assert!(buffer[max_logs - 1]
        .message
        .contains(&format!("Log entry {}", max_logs + 99)));
}

#[tokio::test]
async fn test_empty_search_returns_all_logs() {
    let state = Arc::new(AppState::new(tokio::sync::mpsc::channel(100).0, 1000));

    // Add some test logs
    {
        let mut buffer = state.log_buffer.write().await;
        for i in 0..5 {
            buffer.push(LogLine {
                timestamp: chrono::Utc::now(),
                agent_id: None,
                plugin_id: None,
                correlation_id: None,
                message: format!("Log entry {}", i),
            });
        }
    }

    // Search with empty query params
    let filter = LogFilter {
        agent: None,
        plugin: None,
        text: None,
        limit: 1000,
    };
    let results = state.search_logs(&filter).await;

    // Should return all logs
    assert_eq!(results.len(), 5);
}

#[tokio::test]
async fn test_concurrent_log_additions() {
    let state = Arc::new(AppState::new(tokio::sync::mpsc::channel(100).0, 1000));
    let state_clone = Arc::clone(&state);

    // Spawn multiple tasks to add logs concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let state = Arc::clone(&state_clone);
        handles.push(tokio::spawn(async move {
            for j in 0..100 {
                state
                    .add_log(LogLine {
                        timestamp: chrono::Utc::now(),
                        agent_id: None,
                        plugin_id: None,
                        correlation_id: None,
                        message: format!("Task {} log {}", i, j),
                    })
                    .await;
            }
        }));
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify total logs and no duplicates
    let buffer = state.log_buffer.read().await;
    let mut messages = buffer.iter().map(|log| &log.message).collect::<Vec<_>>();
    messages.sort();
    messages.dedup();
    assert_eq!(messages.len(), buffer.len(), "Found duplicate log entries");
}

#[tokio::test]
async fn test_concurrent_search_during_additions() {
    let state = Arc::new(AppState::new(tokio::sync::mpsc::channel(100).0, 1000));
    let state_clone = Arc::clone(&state);

    // Spawn task to continuously add logs
    let add_handle = tokio::spawn(async move {
        for i in 0..500 {
            state_clone
                .add_log(LogLine {
                    timestamp: chrono::Utc::now(),
                    agent_id: None,
                    plugin_id: None,
                    correlation_id: None,
                    message: format!("Continuous log {}", i),
                })
                .await;
            sleep(Duration::from_millis(1)).await;
        }
    });

    // Perform searches while logs are being added
    let mut search_handles = vec![];
    for i in 0..5 {
        let state = Arc::clone(&state);
        search_handles.push(tokio::spawn(async move {
            for _ in 0..10 {
                let filter = LogFilter {
                    agent: None,
                    plugin: None,
                    text: Some(format!("log {}", i * 50)),
                    limit: 1000,
                };
                let results = state.search_logs(&filter).await;
                assert!(results.len() <= 1000, "Search returned too many results");
                sleep(Duration::from_millis(10)).await;
            }
        }));
    }

    // Wait for all tasks to complete
    add_handle.await.unwrap();
    for handle in search_handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_broadcast_channel_integration() {
    let (tx, mut rx1) = broadcast::channel(100);
    let mut rx2 = tx.subscribe();

    let state = Arc::new(AppState::new_with_logs(
        tokio::sync::mpsc::channel(100).0,
        tx,
    ));

    // Add a log entry
    state
        .add_log(LogLine {
            timestamp: chrono::Utc::now(),
            agent_id: None,
            plugin_id: None,
            correlation_id: None,
            message: "Test broadcast".to_string(),
        })
        .await;

    // Verify both receivers get the message
    let msg1 = rx1.try_recv().unwrap();
    let msg2 = rx2.try_recv().unwrap();
    assert!(msg1.contains("Test broadcast"));
    assert!(msg2.contains("Test broadcast"));
}

#[tokio::test]
async fn test_invalid_uuid_handling() {
    let state = Arc::new(AppState::new(tokio::sync::mpsc::channel(100).0, 1000));

    // Add some logs
    state
        .add_log(LogLine {
            timestamp: chrono::Utc::now(),
            agent_id: Some(Uuid::new_v4()),
            plugin_id: None,
            correlation_id: None,
            message: "Test log".to_string(),
        })
        .await;

    // Test with invalid UUID format
    let filter = LogFilter {
        agent: Some("invalid-uuid".to_string()),
        plugin: None,
        text: None,
        limit: 1000,
    };
    let results = state.search_logs(&filter).await;
    assert_eq!(results.len(), 0, "Invalid UUID should return no results");
}

#[tokio::test]
async fn test_case_sensitive_text_search() {
    let state = Arc::new(AppState::new(tokio::sync::mpsc::channel(100).0, 1000));

    // Add logs with mixed case
    state
        .add_log(LogLine {
            timestamp: chrono::Utc::now(),
            agent_id: None,
            plugin_id: None,
            correlation_id: None,
            message: "TEST message".to_string(),
        })
        .await;

    state
        .add_log(LogLine {
            timestamp: chrono::Utc::now(),
            agent_id: None,
            plugin_id: None,
            correlation_id: None,
            message: "test message".to_string(),
        })
        .await;

    // Search with different cases
    let filter = LogFilter {
        agent: None,
        plugin: None,
        text: Some("TEST".to_string()),
        limit: 1000,
    };
    let results = state.search_logs(&filter).await;
    assert_eq!(
        results.len(),
        1,
        "Case-sensitive search should match exact case only"
    );

    let filter = LogFilter {
        agent: None,
        plugin: None,
        text: Some("test".to_string()),
        limit: 1000,
    };
    let results = state.search_logs(&filter).await;
    assert_eq!(
        results.len(),
        1,
        "Case-sensitive search should match exact case only"
    );
}
