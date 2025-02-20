use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use std::collections::HashMap;
use crate::types::traits::{LanguageMessage, LanguageMessageType, ParticipantState};

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

/// Types of storage elements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    /// Regular data element
    Data,
    /// Language message
    Message,
    /// Conversation transcript
    Conversation,
    /// Agent state
    AgentState,
    /// Plugin state
    PluginState,
}

/// Metadata associated with storage elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Type of the element
    pub element_type: ElementType,
    /// Parent element ID (for versioning/history)
    pub parent_id: Option<ElementId>,
    /// Agent ID if this element is associated with an agent
    pub agent_id: Option<Uuid>,
    /// Conversation ID if this element is part of a conversation
    pub conversation_id: Option<Uuid>,
    /// Additional metadata as key-value pairs
    pub attributes: HashMap<String, String>,
}

impl Default for ElementMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            version: 1,
            tags: Vec::new(),
            element_type: ElementType::Data,
            parent_id: None,
            agent_id: None,
            conversation_id: None,
            attributes: HashMap::new(),
        }
    }
}

/// The actual data stored in an element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementData {
    pub content: serde_json::Value,
    #[serde(default)]
    pub content_type: String,
    /// Sequence number for ordered content (like conversation messages)
    pub sequence: Option<u64>,
    /// Whether this is a partial update
    pub is_partial: bool,
}

impl ElementData {
    pub fn new(content: serde_json::Value) -> Self {
        Self {
            content,
            content_type: "application/json".to_string(),
            sequence: None,
            is_partial: false,
        }
    }

    pub fn with_type(content: serde_json::Value, content_type: impl Into<String>) -> Self {
        Self {
            content,
            content_type: content_type.into(),
            sequence: None,
            is_partial: false,
        }
    }

    pub fn with_sequence(content: serde_json::Value, sequence: u64) -> Self {
        Self {
            content,
            content_type: "application/json".to_string(),
            sequence: Some(sequence),
            is_partial: false,
        }
    }

    pub fn as_partial(content: serde_json::Value, sequence: u64) -> Self {
        Self {
            content,
            content_type: "application/json".to_string(),
            sequence: Some(sequence),
            is_partial: true,
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

    /// Create a new message element
    pub fn new_message(message: &LanguageMessage) -> Self {
        let mut metadata = ElementMetadata::default();
        metadata.element_type = ElementType::Message;
        metadata.agent_id = Some(message.sender_id);
        metadata.conversation_id = Some(message.id);
        metadata.attributes.insert("message_type".to_string(), format!("{:?}", message.message_type));

        Self {
            id: ElementId(message.id),
            data: ElementData::new(serde_json::to_value(message).unwrap()),
            metadata,
        }
    }

    /// Create a new conversation element
    pub fn new_conversation(
        conversation_id: Uuid,
        participants: Vec<Uuid>,
        metadata_attrs: HashMap<String, String>,
    ) -> Self {
        let mut metadata = ElementMetadata::default();
        metadata.element_type = ElementType::Conversation;
        metadata.conversation_id = Some(conversation_id);
        metadata.attributes = metadata_attrs;

        Self {
            id: ElementId(conversation_id),
            data: ElementData::new(serde_json::json!({
                "participants": participants,
                "start_time": Utc::now(),
                "messages": []
            })),
            metadata,
        }
    }

    /// Create a new agent state element
    pub fn new_agent_state(
        agent_id: Uuid,
        state: ParticipantState,
        context: serde_json::Value,
    ) -> Self {
        let mut metadata = ElementMetadata::default();
        metadata.element_type = ElementType::AgentState;
        metadata.agent_id = Some(agent_id);
        metadata.attributes.insert("state".to_string(), format!("{:?}", state));

        Self {
            id: ElementId(Uuid::new_v4()),
            data: ElementData::new(context),
            metadata,
        }
    }

    /// Get the element type
    pub fn element_type(&self) -> &ElementType {
        &self.metadata.element_type
    }

    /// Check if this element is a message
    pub fn is_message(&self) -> bool {
        self.metadata.element_type == ElementType::Message
    }

    /// Check if this element is a conversation
    pub fn is_conversation(&self) -> bool {
        self.metadata.element_type == ElementType::Conversation
    }

    /// Check if this element is an agent state
    pub fn is_agent_state(&self) -> bool {
        self.metadata.element_type == ElementType::AgentState
    }

    /// Get the conversation ID if this element is part of a conversation
    pub fn conversation_id(&self) -> Option<Uuid> {
        self.metadata.conversation_id
    }

    /// Get the agent ID if this element is associated with an agent
    pub fn agent_id(&self) -> Option<Uuid> {
        self.metadata.agent_id
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
    fn test_conversation_element() {
        let conversation_id = Uuid::new_v4();
        let participants = vec![Uuid::new_v4(), Uuid::new_v4()];
        let mut attrs = HashMap::new();
        attrs.insert("topic".to_string(), "test conversation".to_string());

        let element = Element::new_conversation(conversation_id, participants.clone(), attrs);
        assert!(element.is_conversation());
        assert_eq!(element.conversation_id(), Some(conversation_id));

        if let serde_json::Value::Object(obj) = element.data.content {
            let stored_participants: Vec<Uuid> = serde_json::from_value(obj["participants"].clone()).unwrap();
            assert_eq!(stored_participants, participants);
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_agent_state_element() {
        let agent_id = Uuid::new_v4();
        let state = ParticipantState::Ready;
        let context = json!({
            "memory": "test memory",
            "variables": {
                "x": 42
            }
        });

        let element = Element::new_agent_state(agent_id, state, context.clone());
        assert!(element.is_agent_state());
        assert_eq!(element.agent_id(), Some(agent_id));
        assert_eq!(element.data.content, context);
        assert_eq!(
            element.metadata.attributes.get("state").unwrap(),
            "Ready"
        );
    }

    #[test]
    fn test_partial_element() {
        let data = ElementData::as_partial(json!({ "chunk": "partial data" }), 1);
        assert!(data.is_partial);
        assert_eq!(data.sequence, Some(1));
    }
}
