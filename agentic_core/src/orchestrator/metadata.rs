use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Represents system-level events that flow through the orchestrator.
/// Each event carries metadata for tracking and correlation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique identifier for this event
    pub event_id: Uuid,
    /// When this event was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional correlation ID to track related events
    pub correlation_id: Option<Uuid>,
    /// Additional context as key-value pairs
    pub context: Value,
}

impl EventMetadata {
    pub fn new(correlation_id: Option<Uuid>) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            correlation_id,
            context: serde_json::json!({}),
        }
    }

    pub fn with_context(correlation_id: Option<Uuid>, context: Value) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            correlation_id,
            context,
        }
    }
}
