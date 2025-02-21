use crate::types::agent::{AgentCapabilities, AgentInfo, AgentState};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Events related to agent operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// Start a new agent
    Start {
        /// Agent ID
        agent_id: Uuid,
        /// Initial prompt
        prompt: String,
    },
    /// Partial output from an agent
    PartialOutput {
        /// Agent ID
        agent_id: Uuid,
        /// Output content
        output: String,
    },
    /// Agent completed successfully
    Done {
        /// Agent ID
        agent_id: Uuid,
        /// Final output
        final_output: String,
    },
    /// Agent encountered an error
    Error {
        /// Agent ID
        agent_id: Uuid,
        /// Error message
        error: String,
    },
}

impl AgentEvent {
    /// Create a new agent start event
    pub fn start(agent_id: Uuid, prompt: impl Into<String>) -> Self {
        Self::Start {
            agent_id,
            prompt: prompt.into(),
        }
    }

    /// Create a new agent partial output event
    pub fn partial_output(agent_id: Uuid, output: impl Into<String>) -> Self {
        Self::PartialOutput {
            agent_id,
            output: output.into(),
        }
    }

    /// Create a new agent completion event
    pub fn done(agent_id: Uuid, final_output: impl Into<String>) -> Self {
        Self::Done {
            agent_id,
            final_output: final_output.into(),
        }
    }

    /// Create a new agent error event
    pub fn error(agent_id: Uuid, error: impl Into<String>) -> Self {
        Self::Error {
            agent_id,
            error: error.into(),
        }
    }

    /// Get the agent ID from the event
    pub fn agent_id(&self) -> Uuid {
        match self {
            Self::Start { agent_id, .. }
            | Self::PartialOutput { agent_id, .. }
            | Self::Done { agent_id, .. }
            | Self::Error { agent_id, .. } => *agent_id,
        }
    }

    /// Get the agent info from the event
    pub fn agent_info(&self) -> AgentInfo {
        let id = self.agent_id();
        match self {
            Self::Start { .. } => AgentInfo::new(
                id,
                "agent",
                "Agent instance",
                "1.0.0",
                AgentCapabilities::default(),
            )
            .with_status(AgentState::Initializing),
            Self::PartialOutput { .. } => AgentInfo::new(
                id,
                "agent",
                "Agent instance",
                "1.0.0",
                AgentCapabilities::default(),
            )
            .with_status(AgentState::Running),
            Self::Done { .. } => AgentInfo::new(
                id,
                "agent",
                "Agent instance",
                "1.0.0",
                AgentCapabilities::default(),
            )
            .with_status(AgentState::Ready),
            Self::Error { error, .. } => AgentInfo::new(
                id,
                "agent",
                "Agent instance",
                "1.0.0",
                AgentCapabilities::default(),
            )
            .with_error(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_events() {
        let agent_id = Uuid::new_v4();

        // Test start event
        let event = AgentEvent::start(agent_id, "test prompt");
        match event {
            AgentEvent::Start {
                agent_id: aid,
                prompt,
            } => {
                assert_eq!(aid, agent_id);
                assert_eq!(prompt, "test prompt");
            }
            _ => panic!("Expected Start event"),
        }

        // Test partial output event
        let event = AgentEvent::partial_output(agent_id, "test output");
        match event {
            AgentEvent::PartialOutput {
                agent_id: aid,
                output,
            } => {
                assert_eq!(aid, agent_id);
                assert_eq!(output, "test output");
            }
            _ => panic!("Expected PartialOutput event"),
        }

        // Test done event
        let event = AgentEvent::done(agent_id, "test output");
        match event {
            AgentEvent::Done {
                agent_id: aid,
                final_output,
            } => {
                assert_eq!(aid, agent_id);
                assert_eq!(final_output, "test output");
            }
            _ => panic!("Expected Done event"),
        }

        // Test error event
        let event = AgentEvent::error(agent_id, "test error");
        match event {
            AgentEvent::Error {
                agent_id: aid,
                error,
            } => {
                assert_eq!(aid, agent_id);
                assert_eq!(error, "test error");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_agent_info() {
        let agent_id = Uuid::new_v4();

        // Test start event info
        let event = AgentEvent::start(agent_id, "test prompt");
        let info = event.agent_info();
        assert_eq!(info.id, agent_id);
        assert_eq!(info.status.state, AgentState::Initializing);
        assert_eq!(info.status.metadata, serde_json::json!({}));

        // Test partial output event info
        let event = AgentEvent::partial_output(agent_id, "test output");
        let info = event.agent_info();
        assert_eq!(info.id, agent_id);
        assert_eq!(info.status.state, AgentState::Running);
        assert_eq!(info.status.metadata, serde_json::json!({}));

        // Test done event info
        let event = AgentEvent::done(agent_id, "test output");
        let info = event.agent_info();
        assert_eq!(info.id, agent_id);
        assert_eq!(info.status.state, AgentState::Ready);
        assert_eq!(info.status.metadata, serde_json::json!({}));

        // Test error event info
        let event = AgentEvent::error(agent_id, "test error");
        let info = event.agent_info();
        assert_eq!(info.id, agent_id);
        assert_eq!(info.status.state, AgentState::Error);
        assert_eq!(
            info.status.metadata,
            serde_json::json!({"error": "test error"})
        );
    }

    #[test]
    fn test_serialization() {
        let agent_id = Uuid::new_v4();
        let event = AgentEvent::start(agent_id, "test prompt");

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: AgentEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            AgentEvent::Start {
                agent_id: aid,
                prompt,
            } => {
                assert_eq!(aid, agent_id);
                assert_eq!(prompt, "test prompt");
            }
            _ => panic!("Expected Start event after deserialization"),
        }
    }
}
