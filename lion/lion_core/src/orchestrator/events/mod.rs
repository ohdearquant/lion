mod agent;
mod plugin;
mod task;

pub use agent::AgentEvent;
pub use plugin::PluginEvent;
pub use task::TaskEvent;

use serde::{Deserialize, Serialize};
use std::fmt;

/// System-wide events that can be processed by the orchestrator.
/// Events are created through their respective type constructors:
/// - `AgentEvent::spawn()`, `AgentEvent::complete()`, etc.
/// - `TaskEvent::submit()`, `TaskEvent::complete()`, etc.
/// - `PluginEvent::load()`, `PluginEvent::invoke()`, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    /// Agent-related events
    Agent(AgentEvent),
    /// Plugin-related events
    Plugin(PluginEvent),
    /// Task-related events
    Task(TaskEvent),
}

impl fmt::Display for SystemEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemEvent::Agent(event) => write!(f, "Agent: {}", event),
            SystemEvent::Plugin(event) => write!(f, "Plugin: {}", event),
            SystemEvent::Task(event) => write!(f, "Task: {}", event),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_manager::PluginManifest;
    use uuid::Uuid;

    #[test]
    fn test_event_creation() {
        let id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());

        // Test agent events
        match AgentEvent::spawn(id, "test prompt", correlation_id) {
            SystemEvent::Agent(AgentEvent::Spawned { agent_id, .. }) => assert_eq!(agent_id, id),
            _ => panic!("Expected Agent Spawned event"),
        }

        match AgentEvent::partial_output(id, "test output", correlation_id) {
            SystemEvent::Agent(AgentEvent::PartialOutput { agent_id, .. }) => assert_eq!(agent_id, id),
            _ => panic!("Expected Agent PartialOutput event"),
        }

        match AgentEvent::complete(id, "test result", correlation_id) {
            SystemEvent::Agent(AgentEvent::Completed { agent_id, .. }) => assert_eq!(agent_id, id),
            _ => panic!("Expected Agent Completed event"),
        }

        match AgentEvent::error(id, "test error", correlation_id) {
            SystemEvent::Agent(AgentEvent::Error { agent_id, .. }) => assert_eq!(agent_id, id),
            _ => panic!("Expected Agent Error event"),
        }

        // Test task events
        match TaskEvent::submit(id, "test payload", correlation_id) {
            SystemEvent::Task(TaskEvent::Submitted { task_id, .. }) => assert_eq!(task_id, id),
            _ => panic!("Expected Task Submitted event"),
        }

        match TaskEvent::complete(id, "test result", correlation_id) {
            SystemEvent::Task(TaskEvent::Completed { task_id, .. }) => assert_eq!(task_id, id),
            _ => panic!("Expected Task Completed event"),
        }

        match TaskEvent::error(id, "test error", correlation_id) {
            SystemEvent::Task(TaskEvent::Error { task_id, .. }) => assert_eq!(task_id, id),
            _ => panic!("Expected Task Error event"),
        }

        // Test plugin events
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        match PluginEvent::load(
            id,
            manifest.clone(),
            Some("manifest.toml".to_string()),
            correlation_id,
        ) {
            SystemEvent::Plugin(PluginEvent::Load { plugin_id, .. }) => assert_eq!(plugin_id, id),
            _ => panic!("Expected Plugin Load event"),
        }

        match PluginEvent::invoke(id, "test input", correlation_id) {
            SystemEvent::Plugin(PluginEvent::Invoked { plugin_id, .. }) => assert_eq!(plugin_id, id),
            _ => panic!("Expected Plugin Invoked event"),
        }

        match PluginEvent::result(id, "test result", correlation_id) {
            SystemEvent::Plugin(PluginEvent::Result { plugin_id, .. }) => assert_eq!(plugin_id, id),
            _ => panic!("Expected Plugin Result event"),
        }

        match PluginEvent::error(id, "test error", correlation_id) {
            SystemEvent::Plugin(PluginEvent::Error { plugin_id, .. }) => assert_eq!(plugin_id, id),
            _ => panic!("Expected Plugin Error event"),
        }

        match PluginEvent::list() {
            SystemEvent::Plugin(PluginEvent::List) => (),
            _ => panic!("Expected Plugin List event"),
        }
    }
}