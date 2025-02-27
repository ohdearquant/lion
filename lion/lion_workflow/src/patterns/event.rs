use lion_core::CapabilityId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use uuid::Uuid;

/// Error types for event-driven workflows
#[derive(Error, Debug)]
pub enum EventError {
    #[error("Event timeout: {0}")]
    Timeout(String),

    #[error("Event delivery failed: {0}")]
    DeliveryFailed(String),

    #[error("Event already processed: {0}")]
    AlreadyProcessed(String),

    #[error("Event not found: {0}")]
    NotFound(String),

    #[error("Event handler error: {0}")]
    HandlerError(String),

    #[error("Capability error: {0}")]
    CapabilityError(String),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Event delivery semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliverySemantic {
    /// At most once delivery (may lose events)
    AtMostOnce,

    /// At least once delivery (may duplicate events)
    AtLeastOnce,

    /// Exactly once delivery (no loss, no duplication)
    ExactlyOnce,
}

impl Default for DeliverySemantic {
    fn default() -> Self {
        DeliverySemantic::AtLeastOnce
    }
}

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for EventPriority {
    fn default() -> Self {
        EventPriority::Normal
    }
}

/// Workflow event status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventStatus {
    /// Event created but not yet delivered
    Created,

    /// Event sent but not yet acknowledged
    Sent,

    /// Event delivered and acknowledged
    Acknowledged,

    /// Event delivery failed
    Failed,

    /// Event was rejected by consumer
    Rejected,

    /// Event was processed (idempotent check)
    Processed,
}

/// Workflow event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID
    pub id: String,

    /// Event type
    pub event_type: String,

    /// Event payload
    pub payload: serde_json::Value,

    /// Event source
    pub source: String,

    /// Event creation time
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Event expiration time (if any)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Event priority
    #[serde(default)]
    pub priority: EventPriority,

    /// Event correlation ID (for tracking related events)
    pub correlation_id: Option<String>,

    /// Event causation ID (event that caused this one)
    pub causation_id: Option<String>,

    /// Retry count (for retried events)
    #[serde(default)]
    pub retry_count: u32,

    /// Required capability to receive this event
    pub required_capability: Option<CapabilityId>,

    /// Custom metadata
    pub metadata: serde_json::Value,

    /// Whether this event requires acknowledgment
    #[serde(default)]
    pub requires_ack: bool,
}

impl Event {
    /// Create a new event
    pub fn new(event_type: &str, payload: serde_json::Value) -> Self {
        Event {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: event_type.to_string(),
            payload,
            source: "system".to_string(),
            created_at: chrono::Utc::now(),
            expires_at: None,
            priority: EventPriority::Normal,
            correlation_id: None,
            causation_id: None,
            retry_count: 0,
            required_capability: None,
            metadata: serde_json::Value::Null,
            requires_ack: true,
        }
    }

    /// Set the event source
    pub fn with_source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }

    /// Set the event expiration
    pub fn with_expiration(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set the event expiration in seconds from now
    pub fn expires_in_seconds(mut self, seconds: i64) -> Self {
        self.expires_at = Some(chrono::Utc::now() + chrono::Duration::seconds(seconds));
        self
    }

    /// Set the event priority
    pub fn with_priority(mut self, priority: EventPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the event correlation ID
    pub fn with_correlation_id(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }

    /// Set the event causation ID
    pub fn with_causation_id(mut self, causation_id: &str) -> Self {
        self.causation_id = Some(causation_id.to_string());
        self
    }

    /// Set the required capability
    pub fn with_capability(mut self, capability_id: CapabilityId) -> Self {
        self.required_capability = Some(capability_id);
        self
    }

    /// Set custom metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if the event has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Create a new event in response to this one
    pub fn create_response(&self, event_type: &str, payload: serde_json::Value) -> Self {
        Event {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: event_type.to_string(),
            payload,
            source: self.source.clone(),
            created_at: chrono::Utc::now(),
            expires_at: None,
            priority: self.priority,
            correlation_id: self.correlation_id.clone(),
            causation_id: Some(self.id.clone()),
            retry_count: 0,
            required_capability: None,
            metadata: serde_json::Value::Null,
            requires_ack: true,
        }
    }

    /// Increment the retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Event acknowledgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventAck {
    /// ID of the acknowledged event
    pub event_id: String,

    /// Acknowledgment status
    pub status: EventStatus,

    /// Acknowledgment time
    pub time: chrono::DateTime<chrono::Utc>,

    /// Error message (if any)
    pub error: Option<String>,

    /// Consumer ID
    pub consumer: String,
}

impl EventAck {
    /// Create a new successful acknowledgment
    pub fn success(event_id: &str, consumer: &str) -> Self {
        EventAck {
            event_id: event_id.to_string(),
            status: EventStatus::Acknowledged,
            time: chrono::Utc::now(),
            error: None,
            consumer: consumer.to_string(),
        }
    }

    /// Create a new failure acknowledgment
    pub fn failure(event_id: &str, consumer: &str, error: &str) -> Self {
        EventAck {
            event_id: event_id.to_string(),
            status: EventStatus::Failed,
            time: chrono::Utc::now(),
            error: Some(error.to_string()),
            consumer: consumer.to_string(),
        }
    }

    /// Create a new rejection acknowledgment
    pub fn rejection(event_id: &str, consumer: &str, reason: &str) -> Self {
        EventAck {
            event_id: event_id.to_string(),
            status: EventStatus::Rejected,
            time: chrono::Utc::now(),
            error: Some(reason.to_string()),
            consumer: consumer.to_string(),
        }
    }
}

/// Event broker configuration
#[derive(Debug, Clone)]
pub struct EventBrokerConfig {
    /// Delivery semantic
    pub delivery_semantic: DeliverySemantic,

    /// Acknowledgment timeout
    pub ack_timeout: Duration,

    /// Maximum retries
    pub max_retries: u32,

    /// Default event expiration
    pub default_expiration: Option<Duration>,

    /// Channel buffer size
    pub channel_buffer_size: usize,

    /// Enable backpressure
    pub enable_backpressure: bool,

    /// Maximum in-flight events
    pub max_in_flight: usize,

    /// Whether to track processed events (for deduplication)
    pub track_processed_events: bool,

    /// How long to keep processed event IDs (for deduplication)
    pub processed_event_ttl: Duration,
}

impl Default for EventBrokerConfig {
    fn default() -> Self {
        EventBrokerConfig {
            delivery_semantic: DeliverySemantic::AtLeastOnce,
            ack_timeout: Duration::from_secs(30),
            max_retries: 3,
            default_expiration: Some(Duration::from_secs(3600)),
            channel_buffer_size: 1000,
            enable_backpressure: true,
            max_in_flight: 100,
            track_processed_events: true,
            processed_event_ttl: Duration::from_secs(3600),
        }
    }
}

/// Event subscription
#[derive(Debug)]
pub struct EventSubscription {
    /// Subscription ID
    pub id: String,

    /// Event type pattern
    pub event_type: String,

    /// Subscriber ID
    pub subscriber_id: String,

    /// Subscription creation time
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Required capability to receive events
    pub required_capability: Option<CapabilityId>,

    /// Event sender channel
    pub sender: mpsc::Sender<Event>,

    /// Event acknowledgment receiver
    pub ack_receiver: mpsc::Receiver<EventAck>,
}

// Manual implementation of Clone for EventSubscription
// Note: We can't clone the ack_receiver, so we'll create a new channel
impl Clone for EventSubscription {
    fn clone(&self) -> Self {
        let (ack_tx, ack_rx) = mpsc::channel(100); // Use a reasonable buffer size

        Self {
            id: self.id.clone(),
            event_type: self.event_type.clone(),
            subscriber_id: self.subscriber_id.clone(),
            created_at: self.created_at,
            required_capability: self.required_capability.clone(),
            sender: self.sender.clone(),
            ack_receiver: ack_rx,
        }
    }
}

// Serializable version of EventSubscription for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableSubscription {
    /// Subscription ID
    pub id: String,

    /// Event type pattern
    pub event_type: String,

    /// Subscriber ID
    pub subscriber_id: String,

    /// Subscription creation time
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Required capability to receive events
    pub required_capability: Option<CapabilityId>,
}

impl From<&EventSubscription> for SerializableSubscription {
    fn from(sub: &EventSubscription) -> Self {
        SerializableSubscription {
            id: sub.id.clone(),
            event_type: sub.event_type.clone(),
            subscriber_id: sub.subscriber_id.clone(),
            created_at: sub.created_at,
            required_capability: sub.required_capability.clone(),
        }
    }
}

/// Event broker for managing event distribution
pub struct EventBroker {
    /// Configuration
    config: RwLock<EventBrokerConfig>,

    /// Subscriptions by event type
    subscriptions: RwLock<HashMap<String, Vec<EventSubscription>>>,

    /// In-flight events
    in_flight: RwLock<HashMap<String, Event>>,

    /// Processed event IDs (for deduplication)
    processed_events: RwLock<HashSet<String>>,

    /// Event store (for persistence and replay)
    event_store: Option<Arc<dyn EventStore>>,
}

impl EventBroker {
    /// Create a new event broker
    pub fn new(config: EventBrokerConfig) -> Self {
        EventBroker {
            config: RwLock::new(config),
            subscriptions: RwLock::new(HashMap::new()),
            in_flight: RwLock::new(HashMap::new()),
            processed_events: RwLock::new(HashSet::new()),
            event_store: None,
        }
    }

    /// Set the event store
    pub fn with_event_store(mut self, store: Arc<dyn EventStore>) -> Self {
        self.event_store = Some(store);
        self
    }

    /// Subscribe to events
    pub async fn subscribe(
        &self,
        event_type: &str,
        subscriber_id: &str,
        capability: Option<CapabilityId>,
    ) -> Result<(mpsc::Receiver<Event>, mpsc::Sender<EventAck>), EventError> {
        let config = self.config.read().await;

        // Create channels
        let (event_tx, event_rx) = mpsc::channel(config.channel_buffer_size);
        let (ack_tx, ack_rx) = mpsc::channel(config.channel_buffer_size);

        // Create subscription
        let subscription = EventSubscription {
            id: format!("sub-{}", Uuid::new_v4()),
            event_type: event_type.to_string(),
            subscriber_id: subscriber_id.to_string(),
            created_at: chrono::Utc::now(),
            required_capability: capability,
            sender: event_tx,
            ack_receiver: ack_rx,
        };

        // Add to subscriptions
        let mut subs = self.subscriptions.write().await;

        subs.entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(subscription);

        Ok((event_rx, ack_tx))
    }

    /// Publish an event
    pub async fn publish(&self, event: Event) -> Result<EventStatus, EventError> {
        // Check if event already processed (for exactly-once)
        let config = self.config.read().await;

        if config.delivery_semantic == DeliverySemantic::ExactlyOnce && event.retry_count == 0 {
            let processed = self.processed_events.read().await;
            if processed.contains(&event.id) {
                return Err(EventError::AlreadyProcessed(event.id.clone()));
            }
        }

        // Check if event expired
        if event.is_expired() {
            return Err(EventError::Other(format!("Event {} expired", event.id)));
        }

        // Store event if persistent
        if let Some(store) = &self.event_store {
            store
                .store_event(&event)
                .await
                .map_err(|e| EventError::Other(format!("Failed to store event: {}", e)))?;
        }

        // Find subscribers
        let subs = self.subscriptions.read().await;
        let subscribers = subs.get(&event.event_type);

        if let Some(subscribers) = subscribers {
            if subscribers.is_empty() {
                // No subscribers, event considered acknowledged if at-most-once
                if config.delivery_semantic == DeliverySemantic::AtMostOnce {
                    return Ok(EventStatus::Acknowledged);
                } else {
                    // Otherwise, keep in store for future subscribers
                    return Ok(EventStatus::Created);
                }
            }

            // Add to in-flight
            if config.delivery_semantic != DeliverySemantic::AtMostOnce {
                let mut in_flight = self.in_flight.write().await;
                in_flight.insert(event.id.clone(), event.clone());
            }

            // Send to subscribers
            let mut sent = false;
            for subscription in subscribers {
                // Check capability if required
                if let Some(req_cap) = &event.required_capability {
                    if let Some(sub_cap) = &subscription.required_capability {
                        if req_cap != sub_cap {
                            // Skip this subscriber, capability mismatch
                            continue;
                        }
                    } else {
                        // Skip this subscriber, no capability
                        continue;
                    }
                }

                // Send event
                if subscription.sender.try_send(event.clone()).is_ok() {
                    sent = true;

                    // If at-most-once, one subscriber is enough
                    if config.delivery_semantic == DeliverySemantic::AtMostOnce {
                        break;
                    }
                }
            }

            if sent {
                // Start ack handler if needed
                if config.delivery_semantic != DeliverySemantic::AtMostOnce && event.requires_ack {
                    let event_id = event.id.clone();
                    let config_clone = config.clone();
                    let broker_clone = self.clone();

                    // Spawn a task to handle acknowledgment
                    tokio::spawn(async move {
                        broker_clone
                            .process_acknowledgment(event_id, config_clone)
                            .await;
                    });
                }

                Ok(EventStatus::Sent)
            } else {
                // No subscribers could receive the event
                Err(EventError::DeliveryFailed(format!(
                    "No subscribers could receive event {}",
                    event.id
                )))
            }
        } else {
            // No subscribers for this event type
            Ok(EventStatus::Created)
        }
    }

    /// Process acknowledgment for an event
    async fn process_acknowledgment(&self, event_id: String, config: EventBrokerConfig) {
        // Wait for acknowledgment or timeout
        let ack_result = timeout(config.ack_timeout, self.wait_for_acknowledgment(&event_id)).await;

        match ack_result {
            Ok(Ok(ack)) => {
                // Process acknowledgment
                match ack.status {
                    EventStatus::Acknowledged => {
                        // Success, event delivered
                        self.mark_event_processed(&event_id).await;
                        self.remove_in_flight(&event_id).await;
                    }
                    EventStatus::Failed => {
                        // Handle failure, maybe retry
                        let event_opt = self.remove_in_flight(&event_id).await;

                        if let Some(mut event) = event_opt {
                            // Increment retry count
                            event.increment_retry();

                            if event.retry_count < config.max_retries {
                                // Retry publishing directly
                                let broker_clone = self.clone();
                                let event_clone = event.clone();

                                // Spawn a task to retry
                                tokio::spawn(async move {
                                    let _ = broker_clone.publish(event_clone).await;
                                });
                            } else {
                                // Too many retries, give up
                                log::error!(
                                    "Event {} failed after {} retries",
                                    event_id,
                                    config.max_retries
                                );
                            }
                        }
                    }
                    EventStatus::Rejected => {
                        // Event rejected, don't retry
                        self.remove_in_flight(&event_id).await;
                        log::warn!(
                            "Event {} rejected by consumer {}: {:?}",
                            event_id,
                            ack.consumer,
                            ack.error
                        );
                    }
                    _ => {
                        // Other statuses not expected in ack
                        self.remove_in_flight(&event_id).await;
                    }
                }
            }
            Ok(Err(e)) => {
                // Error waiting for ack
                log::error!(
                    "Error waiting for acknowledgment of event {}: {:?}",
                    event_id,
                    e
                );

                // Remove from in-flight
                self.remove_in_flight(&event_id).await;
            }
            Err(_) => {
                // Timeout waiting for ack
                log::warn!("Timeout waiting for acknowledgment of event {}", event_id);

                // Handle timeout, maybe retry
                let event_opt = self.remove_in_flight(&event_id).await;

                if let Some(mut event) = event_opt {
                    // Increment retry count
                    event.increment_retry();

                    if event.retry_count < config.max_retries {
                        // Retry publishing directly
                        let broker_clone = self.clone();
                        let event_clone = event.clone();

                        // Spawn a task to retry
                        tokio::spawn(async move {
                            let _ = broker_clone.publish(event_clone).await;
                        });
                    } else {
                        // Too many retries, give up
                        log::error!(
                            "Event {} timed out after {} retries",
                            event_id,
                            config.max_retries
                        );
                    }
                }
            }
        }
    }

    /// Wait for acknowledgment of an event
    async fn wait_for_acknowledgment(&self, event_id: &str) -> Result<EventAck, EventError> {
        // We need to get a mutable reference to the subscriptions
        let subs_map = self.subscriptions.read().await;

        // Check all subscriptions for acknowledgments
        for subscriptions in subs_map.values() {
            // We need to clone the subscriptions to avoid borrowing issues
            let cloned_subs = subscriptions.clone();

            for mut subscription in cloned_subs {
                // Now we can try to receive from each subscription's channel
                // Since we have a cloned subscription with its own receiver
                if let Ok(ack) = subscription.ack_receiver.try_recv() {
                    if ack.event_id == *event_id {
                        return Ok(ack);
                    }
                }
            }
        }

        // No immediate ack, wait for next check
        tokio::time::sleep(Duration::from_millis(100)).await;

        Err(EventError::Other("No acknowledgment found".to_string()))
    }

    /// Remove an event from in-flight
    async fn remove_in_flight(&self, event_id: &str) -> Option<Event> {
        let mut in_flight = self.in_flight.write().await;
        in_flight.remove(event_id)
    }

    /// Mark an event as processed
    async fn mark_event_processed(&self, event_id: &str) {
        let config = self.config.read().await;

        if config.track_processed_events {
            let mut processed = self.processed_events.write().await;
            processed.insert(event_id.to_string());

            // TODO: Schedule cleanup of old processed events
        }
    }

    /// Check if an event has been processed
    pub async fn is_event_processed(&self, event_id: &str) -> bool {
        let processed = self.processed_events.read().await;
        processed.contains(event_id)
    }

    /// Get the count of in-flight events
    pub async fn get_in_flight_count(&self) -> usize {
        let in_flight = self.in_flight.read().await;
        in_flight.len()
    }

    /// Get the count of subscriptions
    pub async fn get_subscription_count(&self) -> usize {
        let subs = self.subscriptions.read().await;
        subs.values().map(|v| v.len()).sum()
    }

    /// Cleanup expired events
    pub async fn cleanup_expired_events(&self) -> usize {
        let mut in_flight = self.in_flight.write().await;
        let now = chrono::Utc::now();

        let expired: Vec<String> = in_flight
            .iter()
            .filter(|(_, event)| {
                if let Some(expires_at) = event.expires_at {
                    expires_at < now
                } else {
                    false
                }
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired {
            in_flight.remove(id);
        }

        expired.len()
    }

    /// Replay events from storage
    pub async fn replay_events(
        &self,
        event_types: Option<Vec<String>>,
    ) -> Result<usize, EventError> {
        if let Some(store) = &self.event_store {
            // Load events from store
            let events = match event_types {
                Some(types) => store.load_events_by_types(&types).await.map_err(|e| {
                    EventError::Other(format!("Failed to load events by types: {}", e))
                })?,
                None => store
                    .load_all_events()
                    .await
                    .map_err(|e| EventError::Other(format!("Failed to load all events: {}", e)))?,
            };

            let mut published = 0;

            // Republish events
            for event in events {
                let broker_clone = self.clone();
                let event_clone = event.clone();

                // Spawn a task to publish each event
                tokio::spawn(async move {
                    if broker_clone.publish(event_clone).await.is_ok() {
                        // We can't increment published here since it's in a separate task
                        // This is a limitation of this approach
                    }
                });

                published += 1;
            }

            Ok(published)
        } else {
            Err(EventError::Other("No event store configured".to_string()))
        }
    }
}

impl Clone for EventBroker {
    fn clone(&self) -> Self {
        EventBroker {
            config: RwLock::new(
                // We need to acquire a read lock and clone the inner data
                // This is a blocking operation, but it's only used during cloning
                (*self.config.blocking_read()).clone(),
            ),
            subscriptions: RwLock::new(
                // Create a new HashMap with cloned values
                {
                    let subs = self.subscriptions.blocking_read();
                    let mut new_subs = HashMap::new();
                    for (key, value) in subs.iter() {
                        new_subs.insert(key.clone(), value.clone());
                    }
                    new_subs
                },
            ),
            in_flight: RwLock::new(
                // Clone the inner HashMap
                (*self.in_flight.blocking_read()).clone(),
            ),
            processed_events: RwLock::new(
                // Clone the inner HashSet
                (*self.processed_events.blocking_read()).clone(),
            ),
            event_store: self.event_store.clone(),
        }
    }
}

/// Trait for event storage
#[async_trait::async_trait]
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

#[async_trait::async_trait]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_creation() {
        let event = Event::new("test_event", serde_json::json!({"data": "test"}))
            .with_source("test_source")
            .with_priority(EventPriority::High)
            .with_correlation_id("corr-123")
            .expires_in_seconds(60);

        assert_eq!(event.event_type, "test_event");
        assert_eq!(event.source, "test_source");
        assert_eq!(event.priority, EventPriority::High);
        assert_eq!(event.correlation_id, Some("corr-123".to_string()));
        assert!(event.expires_at.is_some());
        assert!(!event.is_expired());
    }

    #[tokio::test]
    async fn test_event_broker_publish_subscribe() {
        let config = EventBrokerConfig {
            delivery_semantic: DeliverySemantic::AtLeastOnce,
            ..Default::default()
        };

        let broker = EventBroker::new(config);

        // Subscribe to events
        let (mut event_rx, ack_tx) = broker
            .subscribe("test_event", "test_subscriber", None)
            .await
            .unwrap();

        // Create and publish an event
        let event = Event::new("test_event", serde_json::json!({"data": "test"}));
        let event_id = event.id.clone();

        let status = broker.publish(event).await.unwrap();
        assert_eq!(status, EventStatus::Sent);

        // Receive event
        let received = event_rx.recv().await.unwrap();
        assert_eq!(received.id, event_id);

        // Send acknowledgment
        let ack = EventAck::success(&event_id, "test_subscriber");
        ack_tx.send(ack).await.unwrap();

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check if event was marked as processed
        assert!(broker.is_event_processed(&event_id).await);
    }

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

    #[tokio::test]
    async fn test_exactly_once_delivery() {
        let config = EventBrokerConfig {
            delivery_semantic: DeliverySemantic::ExactlyOnce,
            ..Default::default()
        };

        let broker = EventBroker::new(config);

        // Subscribe to events
        let (mut event_rx, ack_tx) = broker
            .subscribe("test_event", "test_subscriber", None)
            .await
            .unwrap();

        // Create and publish an event
        let event = Event::new("test_event", serde_json::json!({"data": "test"}));
        let event_id = event.id.clone();

        let status = broker.publish(event.clone()).await.unwrap();
        assert_eq!(status, EventStatus::Sent);

        // Receive event
        let received = event_rx.recv().await.unwrap();
        assert_eq!(received.id, event_id);

        // Send acknowledgment
        let ack = EventAck::success(&event_id, "test_subscriber");
        ack_tx.send(ack).await.unwrap();

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Try to publish the same event again
        let result = broker.publish(event).await;

        // Should fail because event has already been processed
        assert!(result.is_err());
        match result {
            Err(EventError::AlreadyProcessed(_)) => (),
            _ => panic!("Expected AlreadyProcessed error"),
        }
    }
}
