use crate::patterns::event::types::Event;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Trait for event storage
#[async_trait]
pub trait EventStore: Send + Sync + 'static {
    /// Store an event
    async fn store_event(&self, event: &Event) -> Result<(), String>;

    /// Load an event by ID
    async fn load_event(&self, event_id: &str) -> Result<Event, String>;

    /// Load all events
    async fn load_all_events(&self) -> Result<Vec<Event>, String>;

    /// Load events by type
    async fn load_events_by_types(&self, event_types: &[String]) -> Result<Vec<Event>, String>;

    /// Load events by source
    async fn load_events_by_source(&self, source: &str) -> Result<Vec<Event>, String>;

    /// Load events by correlation ID
    async fn load_events_by_correlation_id(
        &self,
        correlation_id: &str,
    ) -> Result<Vec<Event>, String>;

    /// Delete an event
    async fn delete_event(&self, event_id: &str) -> Result<(), String>;
}

/// In-memory event store implementation
pub struct InMemoryEventStore {
    /// Stored events
    events: RwLock<HashMap<String, Event>>,
}

impl InMemoryEventStore {
    /// Create a new in-memory event store
    pub fn new() -> Self {
        InMemoryEventStore {
            events: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn store_event(&self, event: &Event) -> Result<(), String> {
        let mut events = self.events.write().await;
        events.insert(event.id.clone(), event.clone());
        Ok(())
    }

    async fn load_event(&self, event_id: &str) -> Result<Event, String> {
        let events = self.events.read().await;
        events
            .get(event_id)
            .cloned()
            .ok_or_else(|| format!("Event not found: {}", event_id))
    }

    async fn load_all_events(&self) -> Result<Vec<Event>, String> {
        let events = self.events.read().await;
        Ok(events.values().cloned().collect())
    }

    async fn load_events_by_types(&self, event_types: &[String]) -> Result<Vec<Event>, String> {
        let events = self.events.read().await;
        let filtered: Vec<Event> = events
            .values()
            .filter(|e| event_types.contains(&e.event_type))
            .cloned()
            .collect();

        Ok(filtered)
    }

    async fn load_events_by_source(&self, source: &str) -> Result<Vec<Event>, String> {
        let events = self.events.read().await;
        let filtered: Vec<Event> = events
            .values()
            .filter(|e| e.source == source)
            .cloned()
            .collect();

        Ok(filtered)
    }

    async fn load_events_by_correlation_id(
        &self,
        correlation_id: &str,
    ) -> Result<Vec<Event>, String> {
        let events = self.events.read().await;
        let filtered: Vec<Event> = events
            .values()
            .filter(|e| e.correlation_id.as_deref() == Some(correlation_id))
            .cloned()
            .collect();

        Ok(filtered)
    }

    async fn delete_event(&self, event_id: &str) -> Result<(), String> {
        let mut events = self.events.write().await;
        events.remove(event_id);
        Ok(())
    }
}

/// Create a more sophisticated event store implementation
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_store() {
        let store = InMemoryEventStore::new();

        // Create and store an event
        let event = Event::new("test_event", serde_json::json!({"data": "test"}))
            .with_source("test_source")
            .with_correlation_id("corr-123");

        store.store_event(&event).await.unwrap();

        // Load by ID
        let loaded = store.load_event(&event.id).await.unwrap();
        assert_eq!(loaded.id, event.id);

        // Load by type
        let events = store
            .load_events_by_types(&["test_event".to_string()])
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);

        // Load by source
        let events = store.load_events_by_source("test_source").await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);

        // Load by correlation ID
        let events = store
            .load_events_by_correlation_id("corr-123")
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);
    }
}
