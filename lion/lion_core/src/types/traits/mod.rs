use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use std::collections::HashSet;

/// A trait for entities that can be uniquely identified
pub trait Identifiable {
    fn id(&self) -> Uuid;
}

/// A trait for entities that have timestamps
pub trait Timestamped {
    fn created_at(&self) -> DateTime<Utc>;
    fn updated_at(&self) -> Option<DateTime<Utc>>;
}

/// A trait for entities that can be serialized/deserialized
pub trait DataFormat: Serialize + for<'de> Deserialize<'de> {}

/// A trait for entities that can be stored and retrieved
#[async_trait]
pub trait Storable: Identifiable + DataFormat {
    type Error;
    
    async fn save(&self) -> Result<(), Self::Error>;
    async fn load(id: Uuid) -> Result<Self, Self::Error> where Self: Sized;
    async fn delete(id: Uuid) -> Result<(), Self::Error>;
}

/// A trait for entities that can be validated
pub trait Validatable {
    type Error;
    
    fn validate(&self) -> Result<(), Self::Error>;
}

/// A trait for entities that can be cloned with modifications
pub trait Modifiable: Clone {
    fn with_id(self, id: Uuid) -> Self;
    fn with_timestamp(self, timestamp: DateTime<Utc>) -> Self;
}

/// A trait for entities that can be converted to/from JSON
pub trait JsonFormat {
    fn to_json(&self) -> serde_json::Result<Value>;
    fn from_json(value: Value) -> serde_json::Result<Self> where Self: Sized;
}

/// A trait for entities that can be versioned
pub trait Versionable {
    fn version(&self) -> String;
    fn is_compatible_with(&self, other_version: &str) -> bool;
}

/// A trait for entities that can be described
pub trait Describable {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}

/// A trait for entities that can be enabled/disabled
pub trait Toggleable {
    fn is_enabled(&self) -> bool;
    fn enable(&mut self);
    fn disable(&mut self);
}

/// A trait for entities that can handle events
#[async_trait]
pub trait EventHandler {
    type Event;
    type Response;
    type Error;

    async fn handle(&self, event: Self::Event) -> Result<Self::Response, Self::Error>;
}

/// A trait for entities that can be initialized
#[async_trait]
pub trait Initializable {
    type Config;
    type Error;

    async fn initialize(config: Self::Config) -> Result<Self, Self::Error> where Self: Sized;
}

/// A message in the language network protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMessage {
    /// Unique identifier for the message
    pub id: Uuid,
    /// The content of the message
    pub content: String,
    /// The sender of the message
    pub sender_id: Uuid,
    /// The intended recipient(s) of the message
    pub recipient_ids: HashSet<Uuid>,
    /// The type of message
    pub message_type: LanguageMessageType,
    /// Additional metadata
    pub metadata: Value,
    /// Timestamp of the message
    pub timestamp: DateTime<Utc>,
}

/// Types of language messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LanguageMessageType {
    /// Regular text message
    Text,
    /// Function call request
    FunctionCall,
    /// Function call response
    FunctionResponse,
    /// Error message
    Error,
    /// System message
    System,
    /// Control message (e.g., start/stop)
    Control,
}

/// A trait for entities that can process language messages
#[async_trait]
pub trait LanguageProcessor {
    type Error;

    /// Process an incoming language message
    async fn process_message(&self, message: LanguageMessage) -> Result<LanguageMessage, Self::Error>;

    /// Check if this processor can handle a specific message type
    fn can_handle_message_type(&self, message_type: &LanguageMessageType) -> bool;

    /// Get the supported language models
    fn supported_models(&self) -> HashSet<String>;
}

/// A trait for entities that can participate in the network
#[async_trait]
pub trait NetworkParticipant {
    type Error;

    /// Check if the participant can access a specific domain
    fn can_access_domain(&self, domain: &str) -> bool;

    /// Get the network permissions
    fn network_permissions(&self) -> &NetworkPermissions;

    /// Send a message to another participant
    async fn send_message(&self, message: LanguageMessage) -> Result<(), Self::Error>;

    /// Receive a message from another participant
    async fn receive_message(&self, message: LanguageMessage) -> Result<(), Self::Error>;
}

/// Network permissions for a participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPermissions {
    /// Whether network access is enabled
    pub enabled: bool,
    /// Allowed domains
    pub allowed_domains: HashSet<String>,
    /// Rate limit (requests per minute)
    pub rate_limit: usize,
}

/// A trait for participants in the language network
#[async_trait]
pub trait Participant: 
    Identifiable + 
    Describable + 
    Versionable + 
    Validatable + 
    EventHandler + 
    LanguageProcessor + 
    NetworkParticipant 
{
    /// Get the participant's capabilities
    fn capabilities(&self) -> &ParticipantCapabilities;

    /// Check if the participant can interact with another participant
    fn can_interact_with(&self, other_id: Uuid) -> bool;

    /// Get the participant's current state
    fn state(&self) -> ParticipantState;
}

/// Capabilities of a participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantCapabilities {
    /// Whether the participant can initiate conversations
    pub can_initiate: bool,
    /// Whether the participant can use plugins
    pub can_use_plugins: bool,
    /// Maximum concurrent conversations
    pub max_concurrent_conversations: usize,
    /// Supported message types
    pub supported_message_types: HashSet<LanguageMessageType>,
}

/// State of a participant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantState {
    /// Participant is initializing
    Initializing,
    /// Participant is ready
    Ready,
    /// Participant is processing
    Processing,
    /// Participant is waiting
    Waiting,
    /// Participant has completed
    Completed,
    /// Participant has failed
    Failed,
    /// Participant is disabled
    Disabled,
}

impl Default for NetworkPermissions {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_domains: HashSet::new(),
            rate_limit: 60,
        }
    }
}

impl Default for ParticipantCapabilities {
    fn default() -> Self {
        Self {
            can_initiate: false,
            can_use_plugins: false,
            max_concurrent_conversations: 1,
            supported_message_types: [LanguageMessageType::Text].into_iter().collect(),
        }
    }
}