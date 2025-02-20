use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for storage elements
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ElementId(pub Uuid);

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ElementId {
    fn from(uuid: Uuid) -> Self {
        ElementId(uuid)
    }
}

impl From<ElementId> for Uuid {
    fn from(id: ElementId) -> Self {
        id.0
    }
}

/// Metadata associated with storage elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for ElementMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            version: 1,
            tags: Vec::new(),
        }
    }
}

/// The actual data stored in an element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementData {
    pub content: serde_json::Value,
    #[serde(default)]
    pub content_type: String,
}

impl ElementData {
    pub fn new(content: serde_json::Value) -> Self {
        Self {
            content,
            content_type: "application/json".to_string(),
        }
    }

    pub fn with_type(content: serde_json::Value, content_type: impl Into<String>) -> Self {
        Self {
            content,
            content_type: content_type.into(),
        }
    }
}

/// A complete storage element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub id: ElementId,
    pub data: ElementData,
    pub metadata: ElementMetadata,
}

impl Element {
    pub fn new(id: impl Into<ElementId>, data: ElementData) -> Self {
        Self {
            id: id.into(),
            data,
            metadata: ElementMetadata::default(),
        }
    }

    pub fn with_metadata(
        id: impl Into<ElementId>,
        data: ElementData,
        metadata: ElementMetadata,
    ) -> Self {
        Self {
            id: id.into(),
            data,
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
        let id = ElementId(Uuid::new_v4());
        let data = ElementData::new(json!({
            "name": "test",
            "value": 42
        }));

        let element = Element::new(id, data);
        assert_eq!(element.id, id);
        assert_eq!(element.data.content_type, "application/json");
        assert_eq!(element.metadata.version, 1);
    }

    #[test]
    fn test_element_serialization() {
        let id = ElementId(Uuid::new_v4());
        let data = ElementData::with_type(json!({ "test": true }), "application/test");
        let element = Element::new(id, data);

        let serialized = serde_json::to_string(&element).unwrap();
        let deserialized: Element = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, element.id);
        assert_eq!(deserialized.data.content_type, "application/test");
    }

    #[test]
    fn test_element_metadata() {
        let id = ElementId(Uuid::new_v4());
        let data = ElementData::new(json!({}));
        let mut metadata = ElementMetadata::default();
        metadata.tags = vec!["test".to_string()];
        metadata.version = 2;

        let element = Element::with_metadata(id, data, metadata);
        assert_eq!(element.metadata.version, 2);
        assert_eq!(element.metadata.tags, vec!["test"]);
    }

    #[test]
    fn test_element_id_display() {
        let uuid = Uuid::new_v4();
        let id = ElementId(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }
}
