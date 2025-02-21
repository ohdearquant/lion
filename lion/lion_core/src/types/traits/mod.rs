use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use super::{Error, ParticipantState, Result};

/// A message in the language network protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMessage {
    /// Unique identifier for this message
    pub id: Uuid,
    /// The actual content of the message
    pub content: String,
    /// ID of the agent that sent this message
    pub sender_id: Uuid,
    /// IDs of agents that should receive this message
    pub recipient_ids: HashSet<Uuid>,
    /// Type of message
    pub message_type: LanguageMessageType,
    /// Additional metadata as JSON
    pub metadata: serde_json::Value,
    /// When this message was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of language messages that can be exchanged
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageMessageType {
    /// Plain text message
    Text,
    /// A question requiring a response
    Question,
    /// A response to a question
    Answer,
    /// A command to be executed
    Command,
    /// Result of a command execution
    CommandResult,
    /// An error message
    Error,
    /// System-level message
    System,
    /// Custom message type with string identifier
    Custom(String),
}

/// Trait for types that can participate in the language network
pub trait LanguageParticipant {
    /// Get the unique ID of this participant
    fn id(&self) -> Uuid;
    
    /// Get the current state of this participant
    fn state(&self) -> ParticipantState;
    
    /// Process an incoming language message
    fn process_message(&mut self, message: LanguageMessage) -> Result<Option<LanguageMessage>>;
    
    /// Generate a new message
    fn generate_message(&self, content: String, recipients: HashSet<Uuid>) -> LanguageMessage {
        LanguageMessage {
            id: Uuid::new_v4(),
            content,
            sender_id: self.id(),
            recipient_ids: recipients,
            message_type: LanguageMessageType::Text,
            metadata: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Trait for types that can be identified uniquely
pub trait Identifiable {
    /// Get the unique ID of this entity
    fn id(&self) -> Uuid;
}

/// Trait for types that maintain state
pub trait Stateful {
    /// Get the current state
    fn state(&self) -> ParticipantState;
    
    /// Update the state
    fn set_state(&mut self, state: ParticipantState);
    
    /// Check if in a specific state
    fn is_in_state(&self, state: ParticipantState) -> bool {
        self.state() == state
    }
}

/// Trait for types that can be initialized
pub trait Initializable {
    /// Initialize the object
    fn initialize(&mut self) -> Result<()>;
    
    /// Check if initialized
    fn is_initialized(&self) -> bool;
}

/// Trait for types that can be enabled/disabled
pub trait Toggleable {
    /// Enable the object
    fn enable(&mut self) -> Result<()>;
    
    /// Disable the object
    fn disable(&mut self) -> Result<()>;
    
    /// Check if enabled
    fn is_enabled(&self) -> bool;
}

/// Trait for types that can process tasks
pub trait TaskProcessor {
    /// The type of task this processor handles
    type Task;
    /// The type of result this processor produces
    type Result;
    
    /// Process a task
    fn process(&mut self, task: Self::Task) -> Result<Self::Result>;
    
    /// Check if can process a specific task
    fn can_process(&self, task: &Self::Task) -> bool;
}

/// Trait for types that maintain metrics
pub trait MetricsProvider {
    /// Get current metrics as JSON
    fn metrics(&self) -> serde_json::Value;
    
    /// Reset metrics to initial state
    fn reset_metrics(&mut self);
}

/// Trait for types that can be validated
pub trait Validatable {
    /// Validate the object
    fn validate(&self) -> Result<()>;
    
    /// Check if valid
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Trait for types that have a version
pub trait Versionable {
    /// Get the version string
    fn version(&self) -> &str;
    
    /// Check if version is compatible with a requirement
    fn is_compatible_with(&self, requirement: &str) -> bool;
}

/// Trait for types that have a description
pub trait Describable {
    /// Get the name
    fn name(&self) -> &str;
    
    /// Get the description
    fn description(&self) -> &str;
    
    /// Get additional metadata
    fn metadata(&self) -> &serde_json::Value;
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