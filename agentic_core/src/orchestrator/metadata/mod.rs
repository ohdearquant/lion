use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

mod helpers;
pub use helpers::*;

/// Metadata associated with system events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique identifier for this event
    pub event_id: Uuid,

    /// When the event occurred
    pub timestamp: DateTime<Utc>,

    /// Optional correlation ID to link related events
    pub correlation_id: Option<Uuid>,

    /// Additional context for the event
    pub context: Value,
}

impl EventMetadata {
    /// Create new event metadata with optional correlation ID
    pub fn new(correlation_id: Option<Uuid>) -> Self {
        create_metadata(correlation_id)
    }

    /// Create new event metadata with context
    pub fn with_context(correlation_id: Option<Uuid>, context: Value) -> Self {
        create_metadata_with_context(correlation_id, context)
    }

    /// Create new event metadata that correlates with this event
    pub fn correlated(&self) -> Self {
        create_correlated_metadata(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_metadata() {
        let correlation_id = Some(Uuid::new_v4());
        let metadata = EventMetadata::new(correlation_id);

        assert_ne!(metadata.event_id, Uuid::nil());
        assert_eq!(metadata.correlation_id, correlation_id);
        assert_eq!(metadata.context, json!({}));
    }

    #[test]
    fn test_event_metadata_with_context() {
        let correlation_id = Some(Uuid::new_v4());
        let context = json!({
            "key": "value",
            "number": 42
        });
        let metadata = EventMetadata::with_context(correlation_id, context.clone());

        assert_ne!(metadata.event_id, Uuid::nil());
        assert_eq!(metadata.correlation_id, correlation_id);
        assert_eq!(metadata.context, context);
    }

    #[test]
    fn test_correlated_metadata() {
        let original = EventMetadata::with_context(
            Some(Uuid::new_v4()),
            json!({
                "key": "value"
            }),
        );
        let correlated = original.correlated();

        assert_ne!(correlated.event_id, original.event_id);
        assert_eq!(correlated.correlation_id, original.correlation_id);
        assert_eq!(correlated.context, original.context);
    }
}