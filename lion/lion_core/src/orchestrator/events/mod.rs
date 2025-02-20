mod agent;
mod plugin;
mod task;

pub use agent::AgentEvent;
pub use plugin::PluginEvent;
pub use task::TaskEvent;

use serde::{Deserialize, Serialize};

/// System-wide events that can be processed by the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    /// Agent-related events
    Agent(AgentEvent),
    /// Plugin-related events
    Plugin(PluginEvent),
    /// Task-related events
    Task(TaskEvent),
}

impl SystemEvent {
    /// Create a new agent spawn event
    pub fn new_agent_spawn(
        agent_id: uuid::Uuid,
        prompt: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        AgentEvent::spawn(agent_id, prompt, correlation_id)
    }

    /// Create a new agent partial output event
    pub fn new_agent_partial_output(
        agent_id: uuid::Uuid,
        output: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        AgentEvent::partial_output(agent_id, output, correlation_id)
    }

    /// Create a new agent completion event
    pub fn new_agent_completion(
        agent_id: uuid::Uuid,
        result: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        AgentEvent::complete(agent_id, result, correlation_id)
    }

    /// Create a new agent error event
    pub fn new_agent_error(
        agent_id: uuid::Uuid,
        error: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        AgentEvent::error(agent_id, error, correlation_id)
    }

    /// Create a new task submission event
    pub fn new_task_submission(
        task_id: uuid::Uuid,
        payload: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        TaskEvent::submit(task_id, payload, correlation_id)
    }

    /// Create a new task completion event
    pub fn new_task_completion(
        task_id: uuid::Uuid,
        result: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        TaskEvent::complete(task_id, result, correlation_id)
    }

    /// Create a new task error event
    pub fn new_task_error(
        task_id: uuid::Uuid,
        error: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        TaskEvent::error(task_id, error, correlation_id)
    }

    /// Create a new plugin load event
    pub fn new_plugin_load(
        plugin_id: uuid::Uuid,
        manifest: crate::plugin_manager::PluginManifest,
        manifest_path: Option<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        PluginEvent::load(plugin_id, manifest, manifest_path, correlation_id)
    }

    /// Create a new plugin invocation event
    pub fn new_plugin_invocation(
        plugin_id: uuid::Uuid,
        input: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        PluginEvent::invoke(plugin_id, input, correlation_id)
    }

    /// Create a new plugin result event
    pub fn new_plugin_result(
        plugin_id: uuid::Uuid,
        result: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        PluginEvent::result(plugin_id, result, correlation_id)
    }

    /// Create a new plugin error event
    pub fn new_plugin_error(
        plugin_id: uuid::Uuid,
        error: impl Into<String>,
        correlation_id: Option<uuid::Uuid>,
    ) -> Self {
        PluginEvent::error(plugin_id, error, correlation_id)
    }

    /// Create a new plugin list event
    pub fn new_plugin_list() -> Self {
        PluginEvent::list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_manager::PluginManifest;
    use uuid::Uuid;

    #[test]
    fn test_system_events() {
        let id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());

        // Test agent events
        match SystemEvent::new_agent_spawn(id, "test prompt", correlation_id) {
            SystemEvent::Agent(AgentEvent::Spawned { agent_id, .. }) => assert_eq!(agent_id, id),
            _ => panic!("Expected Agent Spawned event"),
        }

        match SystemEvent::new_agent_partial_output(id, "test output", correlation_id) {
            SystemEvent::Agent(AgentEvent::PartialOutput { agent_id, .. }) => assert_eq!(agent_id, id),
            _ => panic!("Expected Agent PartialOutput event"),
        }

        match SystemEvent::new_agent_completion(id, "test result", correlation_id) {
            SystemEvent::Agent(AgentEvent::Completed { agent_id, .. }) => assert_eq!(agent_id, id),
            _ => panic!("Expected Agent Completed event"),
        }

        match SystemEvent::new_agent_error(id, "test error", correlation_id) {
            SystemEvent::Agent(AgentEvent::Error { agent_id, .. }) => assert_eq!(agent_id, id),
            _ => panic!("Expected Agent Error event"),
        }

        // Test task events
        match SystemEvent::new_task_submission(id, "test payload", correlation_id) {
            SystemEvent::Task(TaskEvent::Submitted { task_id, .. }) => assert_eq!(task_id, id),
            _ => panic!("Expected Task Submitted event"),
        }

        match SystemEvent::new_task_completion(id, "test result", correlation_id) {
            SystemEvent::Task(TaskEvent::Completed { task_id, .. }) => assert_eq!(task_id, id),
            _ => panic!("Expected Task Completed event"),
        }

        match SystemEvent::new_task_error(id, "test error", correlation_id) {
            SystemEvent::Task(TaskEvent::Error { task_id, .. }) => assert_eq!(task_id, id),
            _ => panic!("Expected Task Error event"),
        }

        // Test plugin events
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        match SystemEvent::new_plugin_load(
            id,
            manifest.clone(),
            Some("manifest.toml".to_string()),
            correlation_id,
        ) {
            SystemEvent::Plugin(PluginEvent::Load { plugin_id, .. }) => assert_eq!(plugin_id, id),
            _ => panic!("Expected Plugin Load event"),
        }

        match SystemEvent::new_plugin_invocation(id, "test input", correlation_id) {
            SystemEvent::Plugin(PluginEvent::Invoked { plugin_id, .. }) => assert_eq!(plugin_id, id),
            _ => panic!("Expected Plugin Invoked event"),
        }

        match SystemEvent::new_plugin_result(id, "test result", correlation_id) {
            SystemEvent::Plugin(PluginEvent::Result { plugin_id, .. }) => assert_eq!(plugin_id, id),
            _ => panic!("Expected Plugin Result event"),
        }

        match SystemEvent::new_plugin_error(id, "test error", correlation_id) {
            SystemEvent::Plugin(PluginEvent::Error { plugin_id, .. }) => assert_eq!(plugin_id, id),
            _ => panic!("Expected Plugin Error event"),
        }

        match SystemEvent::new_plugin_list() {
            SystemEvent::Plugin(PluginEvent::List) => (),
            _ => panic!("Expected Plugin List event"),
        }
    }
}