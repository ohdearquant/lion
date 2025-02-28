use crate::patterns::event::types::{Event, EventError};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;

/// Retry queue manager for failed events
pub struct RetryManager {
    /// Queue for failed events
    retry_queue: RwLock<VecDeque<Event>>,

    /// Maximum number of retries allowed
    max_retries: u32,

    /// Delay between retries
    retry_delay: Duration,
}

impl RetryManager {
    /// Create a new retry manager
    pub fn new(max_retries: u32, retry_delay: Duration) -> Self {
        RetryManager {
            retry_queue: RwLock::new(VecDeque::new()),
            max_retries,
            retry_delay,
        }
    }

    /// Add an event to the retry queue
    pub async fn enqueue(&self, mut event: Event) -> Result<(), EventError> {
        // Increment retry count
        event.increment_retry();

        // Check if we've exceeded max retries
        if event.retry_count > self.max_retries {
            return Err(EventError::Other(format!(
                "Event {} exceeded maximum retry count",
                event.id
            )));
        }

        // Add to queue
        let mut queue = self.retry_queue.write().await;
        queue.push_back(event);

        Ok(())
    }

    /// Get the next event from the retry queue
    pub async fn dequeue(&self) -> Option<Event> {
        let mut queue = self.retry_queue.write().await;
        queue.pop_front()
    }

    /// Check if the retry queue is empty
    pub async fn is_empty(&self) -> bool {
        let queue = self.retry_queue.read().await;
        queue.is_empty()
    }

    /// Get the current size of the retry queue
    pub async fn size(&self) -> usize {
        let queue = self.retry_queue.read().await;
        queue.len()
    }

    /// Get all events in the retry queue (for diagnostics)
    pub async fn get_all_events(&self) -> Vec<Event> {
        let queue = self.retry_queue.read().await;
        queue.iter().cloned().collect()
    }

    /// Clear all events from the retry queue
    pub async fn clear(&self) {
        let mut queue = self.retry_queue.write().await;
        queue.clear();
    }

    /// Get the retry delay
    pub fn get_retry_delay(&self) -> Duration {
        self.retry_delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patterns::event::types::EventPriority;

    #[tokio::test]
    async fn test_retry_queue() {
        // Create a retry manager
        let retry_manager = RetryManager::new(3, Duration::from_millis(100));

        // Create an event
        let event = Event::new("test_event", serde_json::json!({"data": "test"}))
            .with_source("test_source")
            .with_priority(EventPriority::High);

        // Enqueue the event
        retry_manager.enqueue(event.clone()).await.unwrap();

        // Check queue size
        assert_eq!(retry_manager.size().await, 1);

        // Dequeue the event
        let dequeued = retry_manager.dequeue().await.unwrap();
        assert_eq!(dequeued.id, event.id);
        assert_eq!(dequeued.retry_count, 1); // Should be incremented

        // Queue should be empty now
        assert!(retry_manager.is_empty().await);

        // Test exceeding max retries
        let mut event = Event::new("test_event", serde_json::json!({"data": "test"}));
        event.retry_count = 3; // Already at max

        let result = retry_manager.enqueue(event).await;
        assert!(result.is_err());
    }
}
