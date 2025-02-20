use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashSet;

/// Information about an agent in the language network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique identifier for the agent
    pub id: Uuid,
    /// Current status of the agent
    pub status: AgentStatus,
    /// Optional error message if the agent failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Agent capabilities and permissions
    #[serde(default)]
    pub capabilities: AgentCapabilities,
    /// Current conversation context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ConversationContext>,
}

/// Status of an agent in the language network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// Agent is initializing
    Initializing,
    /// Agent is running
    Running,
    /// Agent is waiting for input from another agent
    Waiting,
    /// Agent is processing a language message
    Processing,
    /// Agent has completed successfully
    Completed,
    /// Agent has failed
    Failed,
    /// Agent is paused
    Paused,
}

/// Agent capabilities and permissions in the language network
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Whether the agent can initiate conversations
    pub can_initiate: bool,
    /// Whether the agent can call plugins
    pub can_use_plugins: bool,
    /// Allowed plugin domains (if empty, all domains are allowed)
    pub allowed_plugins: HashSet<String>,
    /// Maximum concurrent conversations
    pub max_concurrent_conversations: usize,
    /// Network access permissions
    pub network_permissions: NetworkPermissions,
}

/// Network access permissions for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPermissions {
    /// Whether network access is allowed
    pub enabled: bool,
    /// Allowed domains (if empty and enabled=true, all domains are allowed)
    pub allowed_domains: HashSet<String>,
    /// Maximum requests per minute
    pub rate_limit: usize,
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

/// Context for an agent's conversation in the language network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    /// ID of the current conversation
    pub conversation_id: Uuid,
    /// IDs of other agents in the conversation
    pub participant_ids: HashSet<Uuid>,
    /// Current conversation state
    pub state: ConversationState,
    /// Timestamp of last activity
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// State of a conversation in the language network
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationState {
    /// Conversation is active
    Active,
    /// Waiting for response from specific agent
    WaitingFor(Uuid),
    /// Conversation is completed
    Completed,
    /// Conversation failed
    Failed(String),
}

impl AgentInfo {
    /// Create a new agent info
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            status: AgentStatus::Initializing,
            error: None,
            capabilities: AgentCapabilities::default(),
            context: None,
        }
    }

    /// Set the agent status
    pub fn with_status(mut self, status: AgentStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self.status = AgentStatus::Failed;
        self
    }

    /// Set agent capabilities
    pub fn with_capabilities(mut self, capabilities: AgentCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Set conversation context
    pub fn with_context(mut self, context: ConversationContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Check if the agent can use a specific plugin
    pub fn can_use_plugin(&self, plugin_id: &str) -> bool {
        self.capabilities.can_use_plugins && (
            self.capabilities.allowed_plugins.is_empty() ||
            self.capabilities.allowed_plugins.contains(plugin_id)
        )
    }

    /// Check if the agent can access a specific domain
    pub fn can_access_domain(&self, domain: &str) -> bool {
        self.capabilities.network_permissions.enabled && (
            self.capabilities.network_permissions.allowed_domains.is_empty() ||
            self.capabilities.network_permissions.allowed_domains.contains(domain)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_agent_info() {
        let id = Uuid::new_v4();
        let info = AgentInfo::new(id);
        assert_eq!(info.id, id);
        assert_eq!(info.status, AgentStatus::Initializing);
        assert!(info.error.is_none());

        let info = info.with_status(AgentStatus::Running);
        assert_eq!(info.status, AgentStatus::Running);
        assert!(info.error.is_none());

        let info = info.with_error("test error");
        assert_eq!(info.status, AgentStatus::Failed);
        assert_eq!(info.error, Some("test error".to_string()));
    }

    #[test]
    fn test_agent_capabilities() {
        let id = Uuid::new_v4();
        let mut capabilities = AgentCapabilities::default();
        capabilities.can_use_plugins = true;
        capabilities.allowed_plugins.insert("test_plugin".to_string());
        
        let info = AgentInfo::new(id).with_capabilities(capabilities);
        assert!(info.can_use_plugin("test_plugin"));
        assert!(!info.can_use_plugin("other_plugin"));
    }

    #[test]
    fn test_conversation_context() {
        let id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let other_agent_id = Uuid::new_v4();
        
        let mut participants = HashSet::new();
        participants.insert(other_agent_id);
        
        let context = ConversationContext {
            conversation_id,
            participant_ids: participants,
            state: ConversationState::Active,
            last_activity: Utc::now(),
        };
        
        let info = AgentInfo::new(id).with_context(context);
        assert!(info.context.is_some());
        assert_eq!(info.context.unwrap().conversation_id, conversation_id);
    }
}