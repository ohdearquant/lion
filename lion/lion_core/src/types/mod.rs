pub mod agent;
pub mod element;
pub mod plugin;
pub mod traits;

// Re-export commonly used types
pub use agent::*;
pub use element::*;
pub use plugin::*;
pub use traits::*;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Core error type for the lion system
#[derive(Error, Debug)]
pub enum Error {
    #[error("Agent error: {0}")]
    Agent(String),
    
    #[error("Plugin error: {0}")]
    Plugin(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Orchestration error: {0}")]
    Orchestration(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Operation timeout: {0}")]
    Timeout(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for lion operations
pub type Result<T> = std::result::Result<T, Error>;

/// Represents the state of any participant in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantState {
    /// Initial state when first created
    Uninitialized,
    /// Currently being initialized
    Initializing,
    /// Ready for operation
    Ready,
    /// Currently processing a task
    Running,
    /// Temporarily paused
    Paused,
    /// Permanently disabled
    Disabled,
    /// In error state
    Error,
    /// Processing a language-based task
    ProcessingLanguage,
}

/// Common metadata attached to messages and events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Unique identifier for this metadata
    pub id: Uuid,
    /// When this metadata was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional correlation ID for tracking related events
    pub correlation_id: Option<Uuid>,
    /// Additional context as JSON
    pub context: serde_json::Value,
}

impl Metadata {
    /// Create new metadata with optional correlation ID
    pub fn new(correlation_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            correlation_id,
            context: serde_json::json!({}),
        }
    }

    /// Add context data to the metadata
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = context;
        self
    }
}

/// Trait for types that can be validated
pub trait Validatable {
    /// Validate the object, returning Ok(()) if valid or an error if not
    fn validate(&self) -> Result<()>;
}

/// Trait for types that can be serialized to/from JSON
pub trait JsonSerializable: serde::Serialize + serde::de::DeserializeOwned {
    /// Convert the object to a JSON string
    fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| Error::Internal(e.to_string()))
    }

    /// Parse an object from a JSON string
    fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| Error::Internal(e.to_string()))
    }
}

/// Implement JsonSerializable for any type that implements Serialize and DeserializeOwned
impl<T: serde::Serialize + serde::de::DeserializeOwned> JsonSerializable for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let correlation_id = Some(Uuid::new_v4());
        let metadata = Metadata::new(correlation_id);
        
        assert_eq!(metadata.correlation_id, correlation_id);
        assert!(metadata.context.as_object().unwrap().is_empty());

        let context = serde_json::json!({
            "key": "value"
        });
        let metadata = metadata.with_context(context.clone());
        assert_eq!(metadata.context, context);
    }

    #[test]
    fn test_participant_state_transitions() {
        let state = ParticipantState::Uninitialized;
        assert_ne!(state, ParticipantState::Ready);
        
        let state = ParticipantState::Ready;
        assert_ne!(state, ParticipantState::Error);
    }
}