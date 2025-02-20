use super::EventMetadata;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

/// Create new event metadata with optional correlation ID
pub fn create_metadata(correlation_id: Option<Uuid>) -> EventMetadata {
    EventMetadata {
        event_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        correlation_id,
        context: json!({}),
    }
}

/// Create new event metadata with context
pub fn create_metadata_with_context(
    correlation_id: Option<Uuid>,
    context: serde_json::Value,
) -> EventMetadata {
    EventMetadata {
        event_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        correlation_id,
        context,
    }
}

/// Create new event metadata that correlates with another event
pub fn create_correlated_metadata(other: &EventMetadata) -> EventMetadata {
    EventMetadata {
        event_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        correlation_id: other.correlation_id,
        context: other.context.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_metadata() {
        let correlation_id = Some(Uuid::new_v4());
        let metadata = create_metadata(correlation_id);

        assert_ne!(metadata.event_id, Uuid::nil());
        assert_eq!(metadata.correlation_id, correlation_id);
        assert_eq!(metadata.context, json!({}));
    }

    #[test]
    fn test_create_metadata_with_context() {
        let correlation_id = Some(Uuid::new_v4());
        let context = json!({
            "key": "value",
            "number": 42
        });
        let metadata = create_metadata_with_context(correlation_id, context.clone());

        assert_ne!(metadata.event_id, Uuid::nil());
        assert_eq!(metadata.correlation_id, correlation_id);
        assert_eq!(metadata.context, context);
    }

    #[test]
    fn test_create_correlated_metadata() {
        let original = create_metadata_with_context(
            Some(Uuid::new_v4()),
            json!({
                "key": "value"
            }),
        );
        let correlated = create_correlated_metadata(&original);

        assert_ne!(correlated.event_id, original.event_id);
        assert_eq!(correlated.correlation_id, original.correlation_id);
        assert_eq!(correlated.context, original.context);
    }
}