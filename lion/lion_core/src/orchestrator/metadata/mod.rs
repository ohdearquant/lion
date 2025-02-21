use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

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
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            correlation_id,
            context: json!({}),
        }
    }

    /// Create new event metadata with context
    pub fn with_context(correlation_id: Option<Uuid>, context: Value) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            correlation_id,
            context,
        }
    }

    /// Create new event metadata that correlates with this event
    pub fn correlated(&self) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            correlation_id: self.correlation_id,
            context: self.context.clone(),
        }
    }
}

// For backward compatibility, expose the functions as free-standing APIs
/// Create new event metadata with optional correlation ID
#[deprecated(since = "0.1.0", note = "use EventMetadata::new instead")]
pub fn create_metadata(correlation_id: Option<Uuid>) -> EventMetadata {
    EventMetadata::new(correlation_id)
}

/// Create new event metadata with context
#[deprecated(since = "0.1.0", note = "use EventMetadata::with_context instead")]
pub fn create_metadata_with_context(correlation_id: Option<Uuid>, context: Value) -> EventMetadata {
    EventMetadata::with_context(correlation_id, context)
}

/// Create new event metadata that correlates with another event
#[deprecated(since = "0.1.0", note = "use EventMetadata::correlated instead")]
pub fn create_correlated_metadata(other: &EventMetadata) -> EventMetadata {
    other.correlated()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // Test deprecated functions
    #[test]
    fn test_deprecated_functions() {
        let correlation_id = Some(Uuid::new_v4());
        let context = json!({ "key": "value" });

        // Test create_metadata
        let metadata1 = create_metadata(correlation_id);
        let metadata2 = EventMetadata::new(correlation_id);
        assert_eq!(metadata1.correlation_id, metadata2.correlation_id);
        assert_eq!(metadata1.context, metadata2.context);

        // Test create_metadata_with_context
        let metadata3 = create_metadata_with_context(correlation_id, context.clone());
        let metadata4 = EventMetadata::with_context(correlation_id, context.clone());
        assert_eq!(metadata3.correlation_id, metadata4.correlation_id);
        assert_eq!(metadata3.context, metadata4.context);

        // Test create_correlated_metadata
        let metadata5 = create_correlated_metadata(&metadata1);
        let metadata6 = metadata1.correlated();
        assert_eq!(metadata5.correlation_id, metadata6.correlation_id);
        assert_eq!(metadata5.context, metadata6.context);
    }
}
