pub mod sse;

pub use sse::{NetworkEvent, NetworkEventSender, SseError};

use crate::types::{
    agent::AgentState,
    plugin::{PluginResponse, PluginState},
    traits::LanguageMessage,
    Metadata,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Events related to agent lifecycle and operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// Agent has been spawned
    Spawned {
        /// Agent ID
        agent_id: Uuid,
        /// Initial prompt or configuration
        prompt: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Agent produced partial output
    PartialOutput {
        /// Agent ID
        agent_id: Uuid,
        /// Partial output content
        output: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Agent completed its task
    Completed {
        /// Agent ID
        agent_id: Uuid,
        /// Final result
        result: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Agent encountered an error
    Error {
        /// Agent ID
        agent_id: Uuid,
        /// Error message
        error: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Agent state changed
    StateChanged {
        /// Agent ID
        agent_id: Uuid,
        /// New state
        state: AgentState,
        /// Event metadata
        metadata: Metadata,
    },
    /// Agent received a message
    MessageReceived {
        /// Agent ID
        agent_id: Uuid,
        /// The message
        message: LanguageMessage,
        /// Event metadata
        metadata: Metadata,
    },
    /// Agent sent a message
    MessageSent {
        /// Agent ID
        agent_id: Uuid,
        /// The message
        message: LanguageMessage,
        /// Event metadata
        metadata: Metadata,
    },
}

/// Events related to plugin lifecycle and operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    /// Plugin has been loaded
    Load {
        /// Plugin ID
        plugin_id: Uuid,
        /// Plugin manifest
        manifest: crate::plugin_manager::manifest::PluginManifest,
        /// Optional manifest file path
        manifest_path: Option<String>,
        /// Event metadata
        metadata: Metadata,
    },
    /// Plugin has been invoked
    Invoked {
        /// Plugin ID
        plugin_id: Uuid,
        /// Input for the plugin
        input: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Plugin execution result
    Result {
        /// Plugin ID
        plugin_id: Uuid,
        /// Execution result
        result: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Plugin encountered an error
    Error {
        /// Plugin ID
        plugin_id: Uuid,
        /// Error message
        error: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Plugin state changed
    StateChanged {
        /// Plugin ID
        plugin_id: Uuid,
        /// New state
        state: PluginState,
        /// Event metadata
        metadata: Metadata,
    },
    /// List available plugins
    List,
    /// Plugin response
    Response(PluginResponse),
}

/// Events related to task lifecycle and operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEvent {
    /// Task has been submitted
    Submitted {
        /// Task ID
        task_id: Uuid,
        /// Task payload
        payload: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Task has been completed
    Completed {
        /// Task ID
        task_id: Uuid,
        /// Task result
        result: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Task encountered an error
    Error {
        /// Task ID
        task_id: Uuid,
        /// Error message
        error: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Task state changed
    StateChanged {
        /// Task ID
        task_id: Uuid,
        /// New state
        state: String,
        /// Event metadata
        metadata: Metadata,
    },
    /// Task assigned to agent
    Assigned {
        /// Task ID
        task_id: Uuid,
        /// Agent ID
        agent_id: Uuid,
        /// Event metadata
        metadata: Metadata,
    },
}

/// System-wide events that can occur in the microkernel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    /// Agent-related event
    Agent(AgentEvent),
    /// Plugin-related event
    Plugin(PluginEvent),
    /// Task-related event
    Task(TaskEvent),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Metadata;

    #[test]
    fn test_agent_event_serialization() {
        let event = AgentEvent::Spawned {
            agent_id: Uuid::new_v4(),
            prompt: "test prompt".to_string(),
            metadata: Metadata::new(None),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: AgentEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            AgentEvent::Spawned { prompt, .. } => {
                assert_eq!(prompt, "test prompt");
            }
            _ => panic!("Wrong event type after deserialization"),
        }
    }

    #[test]
    fn test_plugin_event_serialization() {
        let event = PluginEvent::Invoked {
            plugin_id: Uuid::new_v4(),
            input: "test input".to_string(),
            metadata: Metadata::new(None),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: PluginEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            PluginEvent::Invoked { input, .. } => {
                assert_eq!(input, "test input");
            }
            _ => panic!("Wrong event type after deserialization"),
        }
    }

    #[test]
    fn test_task_event_serialization() {
        let event = TaskEvent::Submitted {
            task_id: Uuid::new_v4(),
            payload: "test payload".to_string(),
            metadata: Metadata::new(None),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: TaskEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            TaskEvent::Submitted { payload, .. } => {
                assert_eq!(payload, "test payload");
            }
            _ => panic!("Wrong event type after deserialization"),
        }
    }

    #[test]
    fn test_system_event_serialization() {
        let event = SystemEvent::Agent(AgentEvent::Spawned {
            agent_id: Uuid::new_v4(),
            prompt: "test prompt".to_string(),
            metadata: Metadata::new(None),
        });

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: SystemEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            SystemEvent::Agent(AgentEvent::Spawned { prompt, .. }) => {
                assert_eq!(prompt, "test prompt");
            }
            _ => panic!("Wrong event type after deserialization"),
        }
    }
}
