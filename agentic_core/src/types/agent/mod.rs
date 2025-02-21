use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Information about an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique identifier for the agent
    pub id: Uuid,
    /// Current status of the agent
    pub status: AgentStatus,
    /// Optional error message if the agent failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Status of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// Agent is initializing
    Initializing,
    /// Agent is running
    Running,
    /// Agent has completed successfully
    Completed,
    /// Agent has failed
    Failed,
}

impl AgentInfo {
    /// Create a new agent info
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            status: AgentStatus::Initializing,
            error: None,
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
