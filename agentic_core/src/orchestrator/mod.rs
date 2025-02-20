pub mod events;
pub mod handlers;
pub mod metadata;
pub mod processor;
pub mod types;

pub use events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent};
pub use handlers::EventHandler;
pub use metadata::EventMetadata;
pub use processor::Orchestrator;
pub use types::*;

// Provide constructor functions for backward compatibility
impl SystemEvent {
    pub fn task_submitted(
        task_id: uuid::Uuid,
        payload: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Task(TaskEvent::Submitted {
            task_id,
            payload,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn task_completed(
        task_id: uuid::Uuid,
        result: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Task(TaskEvent::Completed {
            task_id,
            result,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn task_error(
        task_id: uuid::Uuid,
        error: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Task(TaskEvent::Error {
            task_id,
            error,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn plugin_load(
        plugin_id: uuid::Uuid,
        manifest: crate::plugin_manager::PluginManifest,
        manifest_path: Option<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Plugin(PluginEvent::Load {
            plugin_id,
            manifest,
            manifest_path,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn plugin_invoked(
        plugin_id: uuid::Uuid,
        input: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Plugin(PluginEvent::Invoked {
            plugin_id,
            input,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn plugin_result(
        plugin_id: uuid::Uuid,
        result: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Plugin(PluginEvent::Result {
            plugin_id,
            result,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn plugin_error(
        plugin_id: uuid::Uuid,
        error: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Plugin(PluginEvent::Error {
            plugin_id,
            error,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn list_plugins() -> Self {
        SystemEvent::Plugin(PluginEvent::List)
    }

    pub fn agent_spawned(
        agent_id: uuid::Uuid,
        prompt: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Agent(AgentEvent::Spawned {
            agent_id,
            prompt,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn agent_partial_output(
        agent_id: uuid::Uuid,
        chunk: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Agent(AgentEvent::PartialOutput {
            agent_id,
            chunk,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn agent_completed(
        agent_id: uuid::Uuid,
        result: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Agent(AgentEvent::Completed {
            agent_id,
            result,
            metadata: EventMetadata::new(correlation_id),
        })
    }

    pub fn agent_error(
        agent_id: uuid::Uuid,
        error: String,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        SystemEvent::Agent(AgentEvent::Error {
            agent_id,
            error,
            metadata: EventMetadata::new(correlation_id),
        })
    }
}
