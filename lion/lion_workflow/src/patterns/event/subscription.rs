use crate::patterns::event::types::*;
use lion_core::CapabilityId;
use tokio::sync::mpsc;
use uuid::Uuid;

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
        let (_ack_tx, ack_rx) = mpsc::channel(100); // Use a reasonable buffer size

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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// Helper functions for subscriptions
pub trait SubscriptionManager {
    /// Create a new subscription
    fn create_subscription(
        event_type: &str,
        subscriber_id: &str,
        capability: Option<CapabilityId>,
        buffer_size: usize,
    ) -> (
        EventSubscription,
        mpsc::Receiver<Event>,
        mpsc::Sender<EventAck>,
    ) {
        // Create channels
        let (event_tx, event_rx) = mpsc::channel(buffer_size);
        let (ack_tx, ack_rx) = mpsc::channel(buffer_size);

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

        (subscription, event_rx, ack_tx)
    }
}
