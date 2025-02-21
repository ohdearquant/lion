use super::core::EventLog;
use crate::orchestrator::SystemEvent;
use std::collections::HashMap;
use uuid::Uuid;

pub struct ReplaySummary {
    pub total_events: usize,
    pub tasks_submitted: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub plugins_invoked: usize,
    pub plugins_loaded: usize,
    pub plugins_load_requested: usize,
    pub plugins_completed: usize,
    pub plugins_failed: usize,
    pub agents_spawned: usize,
    pub agents_completed: usize,
    pub agents_failed: usize,
    pub task_statuses: HashMap<Uuid, Vec<String>>,
    pub plugin_statuses: HashMap<Uuid, Vec<String>>,
    pub agent_statuses: HashMap<Uuid, Vec<String>>,
}

impl EventLog {
    pub fn replay_summary(&self) -> String {
        let records = self.all();
        if records.is_empty() {
            return "No events to replay.".to_string();
        }

        let mut summary = ReplaySummary {
            total_events: records.len(),
            tasks_submitted: 0,
            tasks_completed: 0,
            tasks_failed: 0,
            plugins_invoked: 0,
            plugins_loaded: 0,
            plugins_load_requested: 0,
            plugins_completed: 0,
            plugins_failed: 0,
            agents_spawned: 0,
            agents_completed: 0,
            agents_failed: 0,
            task_statuses: HashMap::new(),
            plugin_statuses: HashMap::new(),
            agent_statuses: HashMap::new(),
        };

        // Process all events
        for record in &records {
            match &record.event {
                SystemEvent::TaskSubmitted {
                    task_id, payload, ..
                } => {
                    summary.tasks_submitted += 1;
                    summary
                        .task_statuses
                        .entry(*task_id)
                        .or_default()
                        .push(format!("Submitted with payload: {}", payload));
                }
                SystemEvent::TaskCompleted {
                    task_id, result, ..
                } => {
                    summary.tasks_completed += 1;
                    summary
                        .task_statuses
                        .entry(*task_id)
                        .or_default()
                        .push(format!("Completed with result: {}", result));
                }
                SystemEvent::TaskError { task_id, error, .. } => {
                    summary.tasks_failed += 1;
                    summary
                        .task_statuses
                        .entry(*task_id)
                        .or_default()
                        .push(format!("Failed with error: {}", error));
                }
                SystemEvent::PluginLoadRequested {
                    plugin_id,
                    manifest,
                    ..
                } => {
                    summary.plugins_load_requested += 1;
                    summary
                        .plugin_statuses
                        .entry(*plugin_id)
                        .or_default()
                        .push(format!("Load requested with manifest: {}", manifest));
                }
                SystemEvent::PluginLoaded {
                    plugin_id,
                    name,
                    version,
                    description,
                    ..
                } => {
                    summary.plugins_loaded += 1;
                    summary
                        .plugin_statuses
                        .entry(*plugin_id)
                        .or_default()
                        .push(format!(
                            "Loaded plugin {} v{}: {}",
                            name, version, description
                        ));
                }
                SystemEvent::PluginInvoked {
                    plugin_id, input, ..
                } => {
                    summary.plugins_invoked += 1;
                    summary
                        .plugin_statuses
                        .entry(*plugin_id)
                        .or_default()
                        .push(format!("Invoked with input: {}", input));
                }
                SystemEvent::PluginResult {
                    plugin_id, output, ..
                } => {
                    summary.plugins_completed += 1;
                    summary
                        .plugin_statuses
                        .entry(*plugin_id)
                        .or_default()
                        .push(format!("Completed with output: {}", output));
                }
                SystemEvent::PluginError {
                    plugin_id, error, ..
                } => {
                    summary.plugins_failed += 1;
                    summary
                        .plugin_statuses
                        .entry(*plugin_id)
                        .or_default()
                        .push(format!("Failed with error: {}", error));
                }
                SystemEvent::AgentSpawned {
                    agent_id, prompt, ..
                } => {
                    summary.agents_spawned += 1;
                    summary
                        .agent_statuses
                        .entry(*agent_id)
                        .or_default()
                        .push(format!("Spawned with prompt: {}", prompt));
                }
                SystemEvent::AgentPartialOutput {
                    agent_id, chunk, ..
                } => {
                    summary
                        .agent_statuses
                        .entry(*agent_id)
                        .or_default()
                        .push(format!("Partial output: {}", chunk));
                }
                SystemEvent::AgentCompleted {
                    agent_id, result, ..
                } => {
                    summary.agents_completed += 1;
                    summary
                        .agent_statuses
                        .entry(*agent_id)
                        .or_default()
                        .push(format!("Completed with result: {}", result));
                }
                SystemEvent::AgentError {
                    agent_id, error, ..
                } => {
                    summary.agents_failed += 1;
                    summary
                        .agent_statuses
                        .entry(*agent_id)
                        .or_default()
                        .push(format!("Failed with error: {}", error));
                }
            }
        }

        format_summary(&summary)
    }
}

fn format_summary(summary: &ReplaySummary) -> String {
    let mut output = String::new();

    output.push_str(&format!("Total Events: {}\n", summary.total_events));

    output.push_str("\nTask Statistics:\n");
    output.push_str("---------------\n");
    output.push_str(&format!("Tasks Submitted: {}\n", summary.tasks_submitted));
    output.push_str(&format!("Tasks Completed: {}\n", summary.tasks_completed));
    output.push_str(&format!("Tasks Failed: {}\n", summary.tasks_failed));

    output.push_str("\nPlugin Statistics:\n");
    output.push_str("-----------------\n");
    output.push_str(&format!("Plugins Invoked: {}\n", summary.plugins_invoked));
    output.push_str(&format!(
        "Plugins Load Requested: {}\n",
        summary.plugins_load_requested
    ));
    output.push_str(&format!("Plugins Loaded: {}\n", summary.plugins_loaded));
    output.push_str(&format!(
        "Plugins Completed: {}\n",
        summary.plugins_completed
    ));
    output.push_str(&format!("Plugins Failed: {}\n", summary.plugins_failed));

    output.push_str("\nAgent Statistics:\n");
    output.push_str("----------------\n");
    output.push_str(&format!("Agents Spawned: {}\n", summary.agents_spawned));
    output.push_str(&format!("Agents Completed: {}\n", summary.agents_completed));
    output.push_str(&format!("Agents Failed: {}\n", summary.agents_failed));

    if !summary.task_statuses.is_empty() {
        output.push_str("\nTask Status Summary:\n");
        output.push_str("------------------\n");
        for (task_id, statuses) in &summary.task_statuses {
            output.push_str(&format!("Task {}:\n", task_id));
            for status in statuses {
                output.push_str(&format!("  {}\n", status));
            }
        }
    }

    if !summary.plugin_statuses.is_empty() {
        output.push_str("\nPlugin Status Summary:\n");
        output.push_str("--------------------\n");
        for (plugin_id, statuses) in &summary.plugin_statuses {
            output.push_str(&format!("Plugin {}:\n", plugin_id));
            for status in statuses {
                output.push_str(&format!("  {}\n", status));
            }
        }
    }

    if !summary.agent_statuses.is_empty() {
        output.push_str("\nAgent Status Summary:\n");
        output.push_str("-------------------\n");
        for (agent_id, statuses) in &summary.agent_statuses {
            output.push_str(&format!("Agent {}:\n", agent_id));
            for status in statuses {
                output.push_str(&format!("  {}\n", status));
            }
        }
    }

    output
}
