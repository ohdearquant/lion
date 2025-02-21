use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Represents a single log entry with optional agent and plugin identifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    /// When the log entry was created
    pub timestamp: DateTime<Utc>,
    /// Optional ID of the agent that generated this log
    pub agent_id: Option<Uuid>,
    /// Optional ID of the plugin that generated this log
    pub plugin_id: Option<Uuid>,
    /// Optional correlation ID to track related operations
    pub correlation_id: Option<Uuid>,
    /// The actual log message
    pub message: String,
}

/// Thread-safe ring buffer for storing log entries with a maximum capacity
pub struct LogBuffer {
    /// Internal storage using VecDeque as a ring buffer
    buffer: RwLock<VecDeque<LogLine>>,
    /// Maximum number of log lines to store
    capacity: usize,
}

impl LogBuffer {
    /// Creates a new LogBuffer with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    /// Adds a new log line to the buffer, potentially removing the oldest entry if at capacity
    pub async fn push(&self, log: LogLine) {
        let mut buffer = self.buffer.write().await;
        if buffer.len() >= self.capacity {
            buffer.pop_front(); // Remove oldest entry if we're at capacity
        }
        buffer.push_back(log);
    }

    /// Searches the buffer for log lines matching the given criteria
    pub async fn search(
        &self,
        agent_id: Option<Uuid>,
        plugin_id: Option<Uuid>,
        correlation_id: Option<Uuid>,
        text: Option<&str>,
    ) -> Vec<LogLine> {
        let buffer = self.buffer.read().await;
        buffer
            .iter()
            .filter(|line| {
                // Check if line matches all provided filters
                let agent_match = agent_id.map_or(true, |id| line.agent_id == Some(id));
                let plugin_match = plugin_id.map_or(true, |id| line.plugin_id == Some(id));
                let correlation_match = correlation_id.map_or(true, |id| line.correlation_id == Some(id));
                let text_match = text.map_or(true, |t| line.message.contains(t));

                agent_match && plugin_match && correlation_match && text_match
            })
            .cloned()
            .collect()
    }

    /// Returns all log lines in the buffer
    pub async fn get_all(&self) -> Vec<LogLine> {
        let buffer = self.buffer.read().await;
        buffer.iter().cloned().collect()
    }

    /// Returns the current number of log lines in the buffer
    pub async fn len(&self) -> usize {
        let buffer = self.buffer.read().await;
        buffer.len()
    }

    /// Returns true if the buffer is empty
    pub async fn is_empty(&self) -> bool {
        let buffer = self.buffer.read().await;
        buffer.is_empty()
    }

    /// Clears all log lines from the buffer
    pub async fn clear(&self) {
        let mut buffer = self.buffer.write().await;
        buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_log_buffer_basic_operations() {
        let buffer = LogBuffer::new(2);
        assert!(buffer.is_empty().await);

        let log1 = LogLine {
            timestamp: Utc::now(),
            agent_id: Some(Uuid::new_v4()),
            plugin_id: None,
            correlation_id: None,
            message: "Test log 1".to_string(),
        };

        buffer.push(log1.clone()).await;
        assert_eq!(buffer.len().await, 1);

        let logs = buffer.get_all().await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "Test log 1");
    }

    #[tokio::test]
    async fn test_log_buffer_capacity() {
        let buffer = LogBuffer::new(2);
        
        let log1 = LogLine {
            timestamp: Utc::now(),
            agent_id: None,
            plugin_id: None,
            correlation_id: None,
            message: "First log".to_string(),
        };

        let log2 = LogLine {
            timestamp: Utc::now(),
            agent_id: None,
            plugin_id: None,
            correlation_id: None,
            message: "Second log".to_string(),
        };

        let log3 = LogLine {
            timestamp: Utc::now(),
            agent_id: None,
            plugin_id: None,
            correlation_id: None,
            message: "Third log".to_string(),
        };

        buffer.push(log1).await;
        buffer.push(log2).await;
        buffer.push(log3).await;

        let logs = buffer.get_all().await;
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].message, "Second log");
        assert_eq!(logs[1].message, "Third log");
    }

    #[tokio::test]
    async fn test_log_buffer_search() {
        let buffer = LogBuffer::new(10);
        let agent_id = Uuid::new_v4();
        let plugin_id = Uuid::new_v4();

        let log1 = LogLine {
            timestamp: Utc::now(),
            agent_id: Some(agent_id),
            plugin_id: Some(plugin_id),
            correlation_id: None,
            message: "Test message".to_string(),
        };

        let log2 = LogLine {
            timestamp: Utc::now(),
            agent_id: Some(agent_id),
            plugin_id: None,
            correlation_id: None,
            message: "Another message".to_string(),
        };

        buffer.push(log1).await;
        buffer.push(log2).await;

        // Search by agent ID
        let results = buffer.search(Some(agent_id), None, None, None).await;
        assert_eq!(results.len(), 2);

        // Search by plugin ID
        let results = buffer.search(None, Some(plugin_id), None, None).await;
        assert_eq!(results.len(), 1);

        // Search by text
        let results = buffer.search(None, None, None, Some("Another")).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].message, "Another message");

        // Search with multiple criteria
        let results = buffer.search(Some(agent_id), Some(plugin_id), None, Some("Test")).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].message, "Test message");
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let buffer = Arc::new(LogBuffer::new(1000));
        let buffer_clone = buffer.clone();

        // Spawn multiple tasks to add logs concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let buffer = buffer.clone();
            handles.push(tokio::spawn(async move {
                for j in 0..100 {
                    let log = LogLine {
                        timestamp: Utc::now(),
                        agent_id: None,
                        plugin_id: None,
                        correlation_id: None,
                        message: format!("Task {} log {}", i, j),
                    };
                    buffer.push(log).await;
                }
            }));
        }

        // Spawn tasks to search while adding
        let mut search_handles = vec![];
        for i in 0..5 {
            let buffer = buffer.clone();
            search_handles.push(tokio::spawn(async move {
                for _ in 0..10 {
                    let results = buffer.search(None, None, None, Some(&format!("Task {}", i))).await;
                    sleep(Duration::from_millis(10)).await;
                }
            }));
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        for handle in search_handles {
            handle.await.unwrap();
        }

        // Verify final state
        let total_logs = buffer_clone.len().await;
        assert!(total_logs <= 1000, "Buffer exceeded capacity");
        
        // Verify no duplicates
        let all_logs = buffer_clone.get_all().await;
        let mut messages: Vec<_> = all_logs.iter().map(|log| &log.message).collect();
        messages.sort();
        messages.dedup();
        assert_eq!(messages.len(), all_logs.len(), "Found duplicate log entries");
    }

    #[tokio::test]
    async fn test_correlation_id_tracking() {
        let buffer = LogBuffer::new(10);
        let correlation_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();

        // Add logs with correlation ID
        for i in 0..3 {
            let log = LogLine {
                timestamp: Utc::now(),
                agent_id: Some(agent_id),
                plugin_id: None,
                correlation_id: Some(correlation_id),
                message: format!("Correlated log {}", i),
            };
            buffer.push(log).await;
        }

        // Add some unrelated logs
        buffer.push(LogLine {
            timestamp: Utc::now(),
            agent_id: Some(agent_id),
            plugin_id: None,
            correlation_id: None,
            message: "Unrelated log".to_string(),
        }).await;

        // Search by correlation ID
        let results = buffer.search(None, None, Some(correlation_id), None).await;
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|log| log.correlation_id == Some(correlation_id)));

        // Search by correlation ID and agent ID
        let results = buffer.search(Some(agent_id), None, Some(correlation_id), None).await;
        assert_eq!(results.len(), 3);

        // Search by non-existent correlation ID
        let results = buffer.search(None, None, Some(Uuid::new_v4()), None).await;
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_clear_and_capacity_edge_cases() {
        let buffer = LogBuffer::new(3);

        // Test clear on empty buffer
        buffer.clear().await;
        assert!(buffer.is_empty().await);

        // Fill buffer exactly to capacity
        for i in 0..3 {
            buffer.push(LogLine {
                timestamp: Utc::now(),
                agent_id: None,
                plugin_id: None,
                correlation_id: None,
                message: format!("Log {}", i),
            }).await;
        }
        assert_eq!(buffer.len().await, 3);

        // Clear and verify
        buffer.clear().await;
        assert!(buffer.is_empty().await);

        // Test capacity maintenance after clear
        for i in 0..5 {
            buffer.push(LogLine {
                timestamp: Utc::now(),
                agent_id: None,
                plugin_id: None,
                correlation_id: None,
                message: format!("New log {}", i),
            }).await;
        }
        assert_eq!(buffer.len().await, 3);
        let logs = buffer.get_all().await;
        assert_eq!(logs[0].message, "New log 2");
        assert_eq!(logs[2].message, "New log 4");
    }

    #[tokio::test]
    async fn test_search_performance_under_load() {
        let buffer = LogBuffer::new(10000);
        let search_id = Uuid::new_v4();

        // Add a large number of logs with one searchable entry near the end
        for i in 0..9998 {
            buffer.push(LogLine {
                timestamp: Utc::now(),
                agent_id: None,
                plugin_id: None,
                correlation_id: None,
                message: format!("Filler log {}", i),
            }).await;
        }

        // Add one searchable log
        buffer.push(LogLine {
            timestamp: Utc::now(),
            agent_id: Some(search_id),
            plugin_id: None,
            correlation_id: None,
            message: "Target log".to_string(),
        }).await;

        // Add one more filler log
        buffer.push(LogLine {
            timestamp: Utc::now(),
            agent_id: None,
            plugin_id: None,
            correlation_id: None,
            message: "Final filler".to_string(),
        }).await;

        // Time the search operation
        let start = std::time::Instant::now();
        let results = buffer.search(Some(search_id), None, None, None).await;
        let duration = start.elapsed();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].message, "Target log");
        
        // Ensure search completes in reasonable time (adjust threshold as needed)
        assert!(duration.as_millis() < 1000, "Search took too long: {:?}", duration);
    }
}
