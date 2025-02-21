use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Represents system-level events that flow through the orchestrator.
/// Each event carries metadata for tracking and correlation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique identifier for this event
    pub event_id: Uuid,
    /// When this event was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional correlation ID to track related events
    pub correlation_id: Option<Uuid>,
    /// Additional context as key-value pairs
    pub context: serde_json::Value,
}

/// System events that can be processed by the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    /// A new task has been submitted for processing
    TaskSubmitted {
        task_id: Uuid,
        payload: String,
        metadata: EventMetadata,
    },
    /// A task has been completed
    TaskCompleted {
        task_id: Uuid,
        result: String,
        metadata: EventMetadata,
    },
    /// An error occurred during task processing
    TaskError {
        task_id: Uuid,
        error: String,
        metadata: EventMetadata,
    },
    /// A plugin is being invoked
    PluginInvoked {
        plugin_id: Uuid,
        input: String,
        metadata: EventMetadata,
    },
    /// A plugin has produced a result
    PluginResult {
        plugin_id: Uuid,
        output: String,
        metadata: EventMetadata,
    },
    /// A plugin invocation resulted in an error
    PluginError {
        plugin_id: Uuid,
        error: String,
        metadata: EventMetadata,
    },
    /// A new agent has been spawned
    AgentSpawned {
        agent_id: Uuid,
        prompt: String,
        metadata: EventMetadata,
    },
    /// An agent has produced partial output
    AgentPartialOutput {
        agent_id: Uuid,
        chunk: String,
        metadata: EventMetadata,
    },
    /// An agent has completed its task
    AgentCompleted {
        agent_id: Uuid,
        result: String,
        metadata: EventMetadata,
    },
    /// An agent encountered an error
    AgentError {
        agent_id: Uuid,
        error: String,
        metadata: EventMetadata,
    },
}

impl SystemEvent {
    /// Create a new TaskSubmitted event
    pub fn new_task(payload: String, correlation_id: Option<Uuid>) -> Self {
        SystemEvent::TaskSubmitted {
            task_id: Uuid::new_v4(),
            payload,
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id,
                context: json!({}),
            },
        }
    }

    /// Create a new PluginInvoked event
    pub fn new_plugin_invocation(
        plugin_id: Uuid,
        input: String,
        correlation_id: Option<Uuid>,
    ) -> Self {
        SystemEvent::PluginInvoked {
            plugin_id,
            input,
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id,
                context: json!({}),
            },
        }
    }

    /// Create a new AgentSpawned event
    pub fn new_agent(prompt: String, correlation_id: Option<Uuid>) -> Self {
        SystemEvent::AgentSpawned {
            agent_id: Uuid::new_v4(),
            prompt,
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id,
                context: json!({}),
            },
        }
    }

    /// Get the event's metadata
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            SystemEvent::TaskSubmitted { metadata, .. } => metadata,
            SystemEvent::TaskCompleted { metadata, .. } => metadata,
            SystemEvent::TaskError { metadata, .. } => metadata,
            SystemEvent::PluginInvoked { metadata, .. } => metadata,
            SystemEvent::PluginResult { metadata, .. } => metadata,
            SystemEvent::PluginError { metadata, .. } => metadata,
            SystemEvent::AgentSpawned { metadata, .. } => metadata,
            SystemEvent::AgentPartialOutput { metadata, .. } => metadata,
            SystemEvent::AgentCompleted { metadata, .. } => metadata,
            SystemEvent::AgentError { metadata, .. } => metadata,
        }
    }
}
