use crate::patterns::event::retry::RetryManager;
use crate::patterns::event::store::EventStore;
use crate::patterns::event::subscription::{EventSubscription, SubscriptionManager};
use crate::patterns::event::types::{
    DeliverySemantic, Event, EventAck, EventBrokerConfig, EventError, EventStatus,
};

use log;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;

/// Event broker for managing event distribution
pub struct EventBroker {
    /// Configuration (Arc-wrapped for non-blocking cloning)
    config: Arc<RwLock<EventBrokerConfig>>,

    /// Subscriptions by event type
    subscriptions: RwLock<HashMap<String, Vec<EventSubscription>>>,

    /// In-flight events
    in_flight: RwLock<HashMap<String, Event>>,

    /// Processed event IDs (for deduplication)
    processed_events: RwLock<HashSet<String>>,

    /// Event store (for persistence and replay)
    event_store: Option<Arc<dyn EventStore>>,

    /// Retry manager
    retry_manager: Arc<RetryManager>,
}

impl EventBroker {
    /// Create a new event broker
    pub fn new(config: EventBrokerConfig) -> Self {
        let retry_manager = Arc::new(RetryManager::new(
            config.max_retries,
            Duration::from_millis(config.retry_delay_ms.unwrap_or(1000)),
        ));

        EventBroker {
            config: Arc::new(RwLock::new(config)),
            subscriptions: RwLock::new(HashMap::new()),
            in_flight: RwLock::new(HashMap::new()),
            processed_events: RwLock::new(HashSet::new()),
            event_store: None,
            retry_manager,
        }
    }

    /// Set the event store
    pub fn with_event_store(mut self, store: Arc<dyn EventStore>) -> Self {
        self.event_store = Some(store);
        self
    }

    /// Get the retry manager
    pub fn retry_manager(&self) -> Arc<RetryManager> {
        self.retry_manager.clone()
    }

    /// Get the size of the retry queue
    pub async fn get_retry_queue_size(&self) -> usize {
        // Delegate to the retry manager
        self.retry_manager.size().await
    }

    /// Subscribe to events
    pub async fn subscribe(
        &self,
        event_type: &str,
        subscriber_id: &str,
        capability: Option<lion_core::CapabilityId>,
    ) -> Result<(mpsc::Receiver<Event>, mpsc::Sender<EventAck>), EventError> {
        let config = self.config.read().await;
        let buffer_size = config.channel_buffer_size;

        // Create a new subscription and channels
        let (subscription, event_rx, ack_tx) = <Self as SubscriptionManager>::create_subscription(
            event_type,
            subscriber_id,
            capability,
            buffer_size,
        );

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
                    let broker_clone = self.clone();

                    // Spawn a task to handle acknowledgment
                    tokio::spawn(async move {
                        broker_clone.process_acknowledgment(event_id).await;
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
    async fn process_acknowledgment(&self, event_id: String) {
        let ack_timeout = {
            let config = self.config.read().await;
            config.ack_timeout
        };

        // Wait for acknowledgment or timeout
        let ack_result = timeout(ack_timeout, self.wait_for_acknowledgment(&event_id)).await;

        match ack_result {
            Ok(Ok(ack)) if ack.event_id == event_id => {
                // Process acknowledgment
                match ack.status {
                    EventStatus::Acknowledged => {
                        // Success, event delivered
                        self.mark_event_processed(&event_id).await;
                        log::debug!("Event {} acknowledged and marked as processed", event_id);
                        self.remove_in_flight(&event_id).await;
                    }
                    EventStatus::Failed => {
                        // Handle failure, maybe retry
                        let event_opt = self.remove_in_flight(&event_id).await;

                        if let Some(event) = event_opt {
                            // Add to retry queue if under max retries
                            let max_retries = {
                                let config = self.config.read().await;
                                config.max_retries
                            };

                            if event.retry_count < max_retries {
                                if let Err(e) = self.retry_manager.enqueue(event).await {
                                    log::error!("Failed to enqueue event for retry: {}", e);
                                }
                            } else {
                                // Too many retries, give up
                                log::error!(
                                    "Event {} failed after {} retries",
                                    event_id,
                                    max_retries
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
            Ok(Ok(_)) => {
                // Received an ack for a different event, ignore and continue
                log::debug!("Received acknowledgment for different event, ignoring");
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

                if let Some(event) = event_opt {
                    // Add to retry queue if under max retries
                    let max_retries = {
                        let config = self.config.read().await;
                        config.max_retries
                    };

                    if event.retry_count < max_retries {
                        if let Err(e) = self.retry_manager.enqueue(event).await {
                            log::error!("Failed to enqueue event for retry after timeout: {}", e);
                        }
                    } else {
                        // Too many retries, give up
                        log::error!("Event {} timed out after {} retries", event_id, max_retries);
                    }
                }
            }
        }
    }

    /// Wait for acknowledgment of an event
    async fn wait_for_acknowledgment(&self, event_id: &str) -> Result<EventAck, EventError> {
        // Try multiple times to find an acknowledgment before giving up
        let max_attempts = 50; // Increase attempts to give more time for acknowledgment
        for _ in 0..max_attempts {
            // Get a reference to the subscriptions
            let subs_map = self.subscriptions.read().await;

            // Check all subscriptions for acknowledgments
            for subscriptions in subs_map.values() {
                // We need to clone the subscriptions to avoid borrowing issues
                let cloned_subs = subscriptions.clone();

                for mut subscription in cloned_subs {
                    // Try to receive from each subscription's channel
                    if let Ok(ack) = subscription.ack_receiver.try_recv() {
                        if ack.event_id == event_id {
                            return Ok(ack);
                        }
                    }
                }
            }

            // Release the read lock and wait before trying again
            drop(subs_map);
            tokio::time::sleep(Duration::from_millis(20)).await; // Shorter sleep for more frequent checks
        }

        // If we reach here, no acknowledgment was found after all attempts
        Err(EventError::Other("No acknowledgment found".to_string()))
    }

    /// Remove an event from in-flight
    async fn remove_in_flight(&self, event_id: &str) -> Option<Event> {
        let mut in_flight = self.in_flight.write().await;
        in_flight.remove(event_id)
    }

    /// Mark an event as processed
    async fn mark_event_processed(&self, event_id: &str) {
        // First check if we need to track processed events
        let track_events = {
            let config = self.config.read().await;
            config.track_processed_events
        };

        // If tracking is enabled, add to processed events set
        if track_events {
            let mut processed = self.processed_events.write().await;
            processed.insert(event_id.to_string());
            log::debug!("Event {} marked as processed", event_id);
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

    /// Process events in the retry queue
    pub async fn process_retry_queue(&self) -> Result<usize, EventError> {
        let mut processed = 0;

        // Only process up to a reasonable batch size per call
        let max_batch = 10;

        for _ in 0..max_batch {
            // Check if queue is empty
            if self.retry_manager.is_empty().await {
                break;
            }

            // Get next event
            if let Some(event) = self.retry_manager.dequeue().await {
                match self.publish(event).await {
                    Ok(_) => {
                        processed += 1;
                    }
                    Err(e) => {
                        log::error!("Error processing retry event: {:?}", e);
                    }
                }
            }
        }

        Ok(processed)
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
                // We'll publish these directly without spawning tasks for better control
                if self.publish(event).await.is_ok() {
                    published += 1;
                }
            }

            Ok(published)
        } else {
            Err(EventError::Other("No event store configured".to_string()))
        }
    }
}

impl Clone for EventBroker {
    fn clone(&self) -> Self {
        // Create new RwLocks with cloned data
        let subscriptions = {
            let guard = futures::executor::block_on(self.subscriptions.read());
            RwLock::new(guard.clone())
        };

        let in_flight = {
            let guard = futures::executor::block_on(self.in_flight.read());
            RwLock::new(guard.clone())
        };

        let processed_events = {
            let guard = futures::executor::block_on(self.processed_events.read());
            RwLock::new(guard.clone())
        };

        EventBroker {
            config: Arc::clone(&self.config),
            subscriptions,
            in_flight,
            processed_events,
            event_store: self.event_store.clone(),
            retry_manager: self.retry_manager.clone(),
        }
    }
}

impl SubscriptionManager for EventBroker {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_event_broker_publish_subscribe() {
        let config = EventBrokerConfig {
            delivery_semantic: DeliverySemantic::AtLeastOnce,
            track_processed_events: true,
            retry_delay_ms: Some(100),
            ack_timeout: Duration::from_millis(500), // Increased timeout
            ..Default::default()
        };

        println!("Creating broker");

        let broker = EventBroker::new(config);

        // Subscribe to events
        let (mut event_rx, ack_tx) = broker
            .subscribe("test_event", "test_subscriber", None)
            .await
            .unwrap();

        println!("Subscription created");

        // Create and publish an event
        let event = Event::new("test_event", serde_json::json!({"data": "test"}));
        let event_id = event.id.clone();

        // Make sure event explicitly requires acknowledgment
        let mut event = event;
        event.requires_ack = true;
        println!("Publishing event: {}", event_id);
        let status = broker.publish(event).await.unwrap();
        assert_eq!(status, EventStatus::Sent);

        // Receive event
        let received = event_rx.recv().await.unwrap();
        assert_eq!(received.id, event_id);

        // Send acknowledgment and ensure it's received
        let ack = EventAck::success(&event_id, "test_subscriber");
        println!("Sending acknowledgment for event: {}", event_id);
        ack_tx.send(ack).await.unwrap();

        // Poll with a longer timeout to ensure acknowledgment is processed
        for _ in 0..100 {
            // Increase polling attempts
            if broker.is_event_processed(&event_id).await {
                println!("Event processed successfully");
                return; // Test passes
            }
            tokio::time::sleep(Duration::from_millis(20)).await; // Shorter sleep for more frequent checks
        }
        panic!("Event was not processed after waiting");
    }
    #[tokio::test]
    async fn test_event_broker_retry_queue() {
        // Create a broker with retry capability
        let config = EventBrokerConfig {
            delivery_semantic: DeliverySemantic::AtLeastOnce,
            max_retries: 3,
            ack_timeout: Duration::from_millis(100),
            retry_delay_ms: Some(100),
            ..Default::default()
        };

        let broker = EventBroker::new(config);

        // Test the retry queue
        let event = Event::new("test_event", serde_json::json!({"data": "test"}));

        // Enqueue an event for retry
        broker.retry_manager.enqueue(event.clone()).await.unwrap();

        // Process retry queue
        broker.process_retry_queue().await.unwrap();

        // Verify the retry manager queue is empty
        assert!(broker.retry_manager.is_empty().await);
    }
}
