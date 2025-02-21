use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use super::traits::{Describable, Validatable, Versionable};
use super::{Error, ParticipantState, Result};

/// Represents the state of an agent in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is registered but not initialized
    Uninitialized,
    /// Agent is being initialized
    Initializing,
    /// Agent is ready for tasks
    Ready,
    /// Agent is currently processing
    Running,
    /// Agent has encountered an error
    Error,
    /// Agent is disabled
    Disabled,
    /// Agent is processing a language task
    ProcessingLanguage,
    /// Agent is paused
    Paused,
}

impl From<AgentState> for ParticipantState {
    fn from(state: AgentState) -> Self {
        match state {
            AgentState::Uninitialized => ParticipantState::Uninitialized,
            AgentState::Initializing => ParticipantState::Initializing,
            AgentState::Ready => ParticipantState::Ready,
            AgentState::Running => ParticipantState::Running,
            AgentState::Error => ParticipantState::Error,
            AgentState::Disabled => ParticipantState::Disabled,
            AgentState::ProcessingLanguage => ParticipantState::ProcessingLanguage,
            AgentState::Paused => ParticipantState::Paused,
        }
    }
}

/// Agent capabilities in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Whether this agent can process language
    pub language_processor: bool,
    /// Whether this agent can generate content
    pub can_generate: bool,
    /// Whether this agent can modify content
    pub can_modify: bool,
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Supported task types
    pub supported_tasks: HashSet<String>,
    /// Required plugins
    pub required_plugins: HashSet<Uuid>,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            language_processor: true,
            can_generate: true,
            can_modify: true,
            max_concurrent_tasks: 1,
            supported_tasks: HashSet::new(),
            required_plugins: HashSet::new(),
        }
    }
}

/// Resource usage metrics for an agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Number of tasks completed
    pub tasks_completed: usize,
    /// Number of tasks failed
    pub tasks_failed: usize,
    /// Average task completion time in milliseconds
    pub avg_completion_time_ms: f64,
    /// Total processing time in milliseconds
    pub total_processing_time_ms: u64,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// Number of messages processed
    pub messages_processed: usize,
}

/// Status information for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Agent ID
    pub agent_id: Uuid,
    /// Current state
    pub state: AgentState,
    /// Number of active tasks
    pub active_tasks: usize,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// Additional status information
    pub metadata: serde_json::Value,
}

/// Public information about an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent ID
    pub id: Uuid,
    /// Agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// Agent version
    pub version: String,
    /// Agent capabilities
    pub capabilities: AgentCapabilities,
    /// Current status
    pub status: AgentStatus,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl AgentInfo {
    /// Create new agent info
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        description: impl Into<String>,
        version: impl Into<String>,
        capabilities: AgentCapabilities,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            description: description.into(),
            version: version.into(),
            capabilities,
            status: AgentStatus {
                agent_id: id,
                state: AgentState::Uninitialized,
                active_tasks: 0,
                last_activity: chrono::Utc::now(),
                memory_usage: 0,
                metadata: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
        }
    }

    /// Set the agent's status
    pub fn with_status(mut self, state: AgentState) -> Self {
        self.status.state = state;
        self.status.last_activity = chrono::Utc::now();
        self
    }

    /// Set an error state with message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.status.state = AgentState::Error;
        self.status.last_activity = chrono::Utc::now();
        self.status.metadata = serde_json::json!({
            "error": error.into()
        });
        self
    }
}

impl Describable for AgentInfo {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }
}

impl Versionable for AgentInfo {
    fn version(&self) -> &str {
        &self.version
    }

    fn is_compatible_with(&self, requirement: &str) -> bool {
        // TODO: Implement proper version compatibility check
        self.version == requirement
    }
}

impl Validatable for AgentInfo {
    fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(Error::InvalidState("Agent name cannot be empty".into()));
        }
        if self.version.is_empty() {
            return Err(Error::InvalidState("Agent version cannot be empty".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_conversion() {
        assert_eq!(
            ParticipantState::from(AgentState::Ready),
            ParticipantState::Ready
        );
        assert_eq!(
            ParticipantState::from(AgentState::Error),
            ParticipantState::Error
        );
    }

    #[test]
    fn test_agent_info_validation() {
        let capabilities = AgentCapabilities::default();
        
        let valid_info = AgentInfo::new(
            Uuid::new_v4(),
            "test-agent",
            "A test agent",
            "1.0.0",
            capabilities.clone(),
        );
        assert!(valid_info.validate().is_ok());

        let invalid_info = AgentInfo::new(
            Uuid::new_v4(),
            "",
            "A test agent",
            "1.0.0",
            capabilities,
        );
        assert!(invalid_info.validate().is_err());
    }
}