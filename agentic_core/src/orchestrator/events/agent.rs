use crate::orchestrator::metadata::{create_metadata, EventMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::SystemEvent;

/// Events related to agent operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// Agent spawned
    Spawned {
        /// Agent ID
        agent_id: Uuid,
        /// Agent prompt
        prompt: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    /// Agent partial output
    PartialOutput {
        /// Agent ID
        agent_id: Uuid,
        /// Partial output
        output: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    /// Agent completed
    Completed {
        /// Agent ID
        agent_id: Uuid,
        /// Final result
        result: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    /// Agent error
    Error {
        /// Agent ID
        agent_id: Uuid,
        /// Error message
        error: String,
        /// Event metadata
        metadata: EventMetadata,
    },
}

impl AgentEvent {
    /// Create a new agent spawn event
    pub fn spawn(agent_id: Uuid, prompt: impl Into<String>, correlation_id: Option<Uuid>) -> SystemEvent {
        SystemEvent::Agent(AgentEvent::Spawned {
            agent_id,
            prompt: prompt.into(),
            metadata: create_metadata(correlation_id),
        })
    }

    /// Create a new agent partial output event
    pub fn partial_output(
        agent_id: Uuid,
        output: impl Into<String>,
        correlation_id: Option<Uuid>,
    ) -> SystemEvent {
        SystemEvent::Agent(AgentEvent::PartialOutput {
            agent_id,
            output: output.into(),
            metadata: create_metadata(correlation_id),
        })
    }

    /// Create a new agent completion event
    pub fn complete(agent_id: Uuid, result: impl Into<String>, correlation_id: Option<Uuid>) -> SystemEvent {
        SystemEvent::Agent(AgentEvent::Completed {
            agent_id,
            result: result.into(),
            metadata: create_metadata(correlation_id),
        })
    }

    /// Create a new agent error event
    pub fn error(agent_id: Uuid, error: impl Into<String>, correlation_id: Option<Uuid>) -> SystemEvent {
        SystemEvent::Agent(AgentEvent::Error {
            agent_id,
            error: error.into(),
            metadata: create_metadata(correlation_id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_events() {
        let agent_id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());

        // Test spawn event
        match AgentEvent::spawn(agent_id, "test prompt", correlation_id) {
            SystemEvent::Agent(AgentEvent::Spawned {
                agent_id: aid,
                prompt,
                metadata,
            }) => {
                assert_eq!(aid, agent_id);
                assert_eq!(prompt, "test prompt");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Spawned event"),
        }

        // Test partial output event
        match AgentEvent::partial_output(agent_id, "test output", correlation_id) {
            SystemEvent::Agent(AgentEvent::PartialOutput {
                agent_id: aid,
                output,
                metadata,
            }) => {
                assert_eq!(aid, agent_id);
                assert_eq!(output, "test output");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected PartialOutput event"),
        }

        // Test complete event
        match AgentEvent::complete(agent_id, "test result", correlation_id) {
            SystemEvent::Agent(AgentEvent::Completed {
                agent_id: aid,
                result,
                metadata,
            }) => {
                assert_eq!(aid, agent_id);
                assert_eq!(result, "test result");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Completed event"),
        }

        // Test error event
        match AgentEvent::error(agent_id, "test error", correlation_id) {
            SystemEvent::Agent(AgentEvent::Error {
                agent_id: aid,
                error,
                metadata,
            }) => {
                assert_eq!(aid, agent_id);
                assert_eq!(error, "test error");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Error event"),
        }

        // Test serialization/deserialization
        let event = AgentEvent::Spawned {
            agent_id,
            prompt: "test prompt".to_string(),
            metadata: create_metadata(correlation_id),
        };
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: AgentEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            AgentEvent::Spawned {
                agent_id: aid,
                prompt,
                metadata,
            } => {
                assert_eq!(aid, agent_id);
                assert_eq!(prompt, "test prompt");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Spawned event after deserialization"),
        }
    }
}