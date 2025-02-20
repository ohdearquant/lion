use crate::orchestrator::metadata::EventMetadata;
use crate::plugin_manager::PluginManifest;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEvent {
    /// A new task has been submitted for processing
    Submitted {
        task_id: Uuid,
        payload: String,
        metadata: EventMetadata,
    },
    /// A task has been completed
    Completed {
        task_id: Uuid,
        result: String,
        metadata: EventMetadata,
    },
    /// An error occurred during task processing
    Error {
        task_id: Uuid,
        error: String,
        metadata: EventMetadata,
    },
}

/// Plugin-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    /// A plugin is being loaded
    Load {
        plugin_id: Uuid,
        manifest: PluginManifest,
        manifest_path: Option<String>,
        metadata: EventMetadata,
    },
    /// A plugin is being invoked
    Invoked {
        plugin_id: Uuid,
        input: String,
        metadata: EventMetadata,
    },
    /// A plugin has produced a result
    Result {
        plugin_id: Uuid,
        result: String,
        metadata: EventMetadata,
    },
    /// A plugin invocation resulted in an error
    Error {
        plugin_id: Uuid,
        error: String,
        metadata: EventMetadata,
    },
    /// List all loaded plugins
    List,
}

/// Agent-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// A new agent has been spawned
    Spawned {
        agent_id: Uuid,
        prompt: String,
        metadata: EventMetadata,
    },
    /// An agent has produced partial output
    PartialOutput {
        agent_id: Uuid,
        chunk: String,
        metadata: EventMetadata,
    },
    /// An agent has completed its task
    Completed {
        agent_id: Uuid,
        result: String,
        metadata: EventMetadata,
    },
    /// An agent encountered an error
    Error {
        agent_id: Uuid,
        error: String,
        metadata: EventMetadata,
    },
}

/// System events that can be processed by the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    Task(TaskEvent),
    Plugin(PluginEvent),
    Agent(AgentEvent),
}

impl SystemEvent {
    /// Create a new TaskSubmitted event
    pub fn new_task(payload: String, correlation_id: Option<Uuid>) -> Self {
        SystemEvent::Task(TaskEvent::Submitted {
            task_id: Uuid::new_v4(),
            payload,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    /// Create a new PluginInvoked event
    pub fn new_plugin_invocation(
        plugin_id: Uuid,
        input: String,
        correlation_id: Option<Uuid>,
    ) -> Self {
        SystemEvent::Plugin(PluginEvent::Invoked {
            plugin_id,
            input,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    /// Create a new AgentSpawned event
    pub fn new_agent(prompt: String, correlation_id: Option<Uuid>) -> Self {
        SystemEvent::Agent(AgentEvent::Spawned {
            agent_id: Uuid::new_v4(),
            prompt,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    /// Get the event's metadata
    pub fn metadata(&self) -> Option<&EventMetadata> {
        match self {
            SystemEvent::Task(task_event) => match task_event {
                TaskEvent::Submitted { metadata, .. } => Some(metadata),
                TaskEvent::Completed { metadata, .. } => Some(metadata),
                TaskEvent::Error { metadata, .. } => Some(metadata),
            },
            SystemEvent::Plugin(plugin_event) => match plugin_event {
                PluginEvent::Load { metadata, .. } => Some(metadata),
                PluginEvent::Invoked { metadata, .. } => Some(metadata),
                PluginEvent::Result { metadata, .. } => Some(metadata),
                PluginEvent::Error { metadata, .. } => Some(metadata),
                PluginEvent::List => None,
            },
            SystemEvent::Agent(agent_event) => match agent_event {
                AgentEvent::Spawned { metadata, .. } => Some(metadata),
                AgentEvent::PartialOutput { metadata, .. } => Some(metadata),
                AgentEvent::Completed { metadata, .. } => Some(metadata),
                AgentEvent::Error { metadata, .. } => Some(metadata),
            },
        }
    }
}
