use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementData {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub metadata: Value,
}

impl ElementData {
    pub fn new(metadata: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_element_creation() {
        let meta = json!({ "title": "Test Element" });
        let elem = ElementData::new(meta.clone());
        assert_eq!(elem.metadata, meta);
        assert_ne!(elem.id, Uuid::nil());
    }

    #[test]
    fn test_element_serialization() {
        let meta = json!({ "key": "value" });
        let elem = ElementData::new(meta);
        let serialized = serde_json::to_string(&elem).unwrap();
        let deserialized: ElementData = serde_json::from_str(&serialized).unwrap();
        assert_eq!(elem.id, deserialized.id);
        assert_eq!(elem.metadata, deserialized.metadata);
    }
}
