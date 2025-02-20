use crate::orchestrator::events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct EventStats {
    pub tasks_submitted: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub plugins_invoked: usize,
    pub plugins_completed: usize,
    pub plugins_failed: usize,
    pub agents_spawned: usize,
    pub agents_completed: usize,
    pub agents_failed: usize,
    pub task_statuses: HashMap<Uuid, String>,
    pub plugin_statuses: HashMap<Uuid, String>,
    pub agent_statuses: HashMap<Uuid, String>,
}

impl EventStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_event(&mut self, event: &SystemEvent) {
        match event {
            SystemEvent::Task(task_event) => self.process_task_event(task_event),
            SystemEvent::Plugin(plugin_event) => self.process_plugin_event(plugin_event),
            SystemEvent::Agent(agent_event) => self.process_agent_event(agent_event),
        }
    }

    fn process_task_event(&mut self, event: &TaskEvent) {
        match event {
            TaskEvent::Submitted {
                task_id, payload, ..
            } => {
                self.tasks_submitted += 1;
                self.task_statuses
                    .insert(*task_id, format!("Submitted with payload: {}", payload));
            }
            TaskEvent::Completed {
                task_id, result, ..
            } => {
                self.tasks_completed += 1;
                self.task_statuses
                    .insert(*task_id, format!("Completed with result: {}", result));
            }
            TaskEvent::Error { task_id, error, .. } => {
                self.tasks_failed += 1;
                self.task_statuses
                    .insert(*task_id, format!("Failed with error: {}", error));
            }
        }
    }

    fn process_plugin_event(&mut self, event: &PluginEvent) {
        match event {
            PluginEvent::Invoked {
                plugin_id, input, ..
            } => {
                self.plugins_invoked += 1;
                self.plugin_statuses
                    .insert(*plugin_id, format!("Invoked with input: {}", input));
            }
            PluginEvent::Result {
                plugin_id, result, ..
            } => {
                self.plugins_completed += 1;
                self.plugin_statuses
                    .insert(*plugin_id, format!("Completed with result: {}", result));
            }
            PluginEvent::Error {
                plugin_id, error, ..
            } => {
                self.plugins_failed += 1;
                self.plugin_statuses
                    .insert(*plugin_id, format!("Failed with error: {}", error));
            }
            PluginEvent::Load {
                plugin_id,
                manifest,
                ..
            } => {
                self.plugin_statuses
                    .insert(*plugin_id, format!("Loading plugin: {}", manifest.name));
            }
            PluginEvent::List => {}
        }
    }

    fn process_agent_event(&mut self, event: &AgentEvent) {
        match event {
            AgentEvent::Spawned {
                agent_id, prompt, ..
            } => {
                self.agents_spawned += 1;
                self.agent_statuses
                    .insert(*agent_id, format!("Spawned with prompt: {}", prompt));
            }
            AgentEvent::PartialOutput {
                agent_id, chunk, ..
            } => {
                let status = self
                    .agent_statuses
                    .entry(*agent_id)
                    .or_insert_with(String::new);
                if !status.is_empty() {
                    status.push_str("\n  ");
                }
                status.push_str(&format!("Partial output: {}", chunk));
            }
            AgentEvent::Completed {
                agent_id, result, ..
            } => {
                self.agents_completed += 1;
                let status = self
                    .agent_statuses
                    .entry(*agent_id)
                    .or_insert_with(String::new);
                if !status.is_empty() {
                    status.push_str("\n  ");
                }
                status.push_str(&format!("Completed with result: {}", result));
            }
            AgentEvent::Error {
                agent_id, error, ..
            } => {
                self.agents_failed += 1;
                self.agent_statuses
                    .insert(*agent_id, format!("Failed with error: {}", error));
            }
        }
    }
}
                    .entry(*agent_id)
                    .or_insert_with(String::new);
                if !status.is_empty() {
                    status.push_str("\n  ");
                }
                status.push_str(&format!("Partial output: {}", chunk));
            }
            AgentEvent::Completed {
                agent_id, result, ..
            } => {
                self.agents_completed += 1;
                let status = self
                    .agent_statuses
                    .entry(*agent_id)
                    .or_insert_with(String::new);
                if !status.is_empty() {
                    status.push_str("\n  ");
                }
                status.push_str(&format!("Completed with result: {}", result));
            }
            AgentEvent::Error {
                agent_id, error, ..
            } => {
                self.agents_failed += 1;
                self.agent_statuses
                    .insert(*agent_id, format!("Failed with error: {}", error));
            }
        }
    }
}
