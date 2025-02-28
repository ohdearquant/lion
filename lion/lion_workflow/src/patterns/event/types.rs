use lion_core::CapabilityId;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
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

    /// Delay between retries in milliseconds
    pub retry_delay_ms: Option<u64>,

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
            retry_delay_ms: Some(1000), // Default 1 second
            default_expiration: Some(Duration::from_secs(3600)),
            channel_buffer_size: 1000,
            enable_backpressure: true,
            max_in_flight: 100,
            track_processed_events: true,
            processed_event_ttl: Duration::from_secs(3600),
        }
    }
}
