use crate::orchestrator::SystemEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub timestamp: DateTime<Utc>,
    pub event: SystemEvent,
}

#[derive(Debug, Clone)]
pub struct EventLog {
    records: Arc<Mutex<Vec<EventRecord>>>,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn append(&self, event: SystemEvent) {
        let record = EventRecord {
            timestamp: Utc::now(),
            event,
        };
        if let Ok(mut records) = self.records.lock() {
            records.push(record);
        }
    }

    pub fn all(&self) -> Vec<EventRecord> {
        self.records
            .lock()
            .map(|records| records.clone())
            .unwrap_or_default()
    }

    pub fn replay_summary(&self) -> String {
        let records = self.all();
        if records.is_empty() {
            return "No events to replay.".to_string();
        }

        let mut summary = String::new();
        let mut tasks_submitted = 0;
        let mut tasks_completed = 0;
        let mut tasks_failed = 0;
        let mut plugins_invoked = 0;
        let mut plugins_completed = 0;
        let mut plugins_failed = 0;
        let mut agents_spawned = 0;
        let mut agents_completed = 0;
        let mut agents_failed = 0;
        let mut task_statuses = std::collections::HashMap::new();
        let mut plugin_statuses = std::collections::HashMap::new();
        let mut agent_statuses = std::collections::HashMap::new();

        // Use reference to avoid moving records
        for record in &records {
            match &record.event {
                SystemEvent::TaskSubmitted {
                    task_id, payload, ..
                } => {
                    tasks_submitted += 1;
                    task_statuses.insert(*task_id, format!("Submitted with payload: {}", payload));
                }
                SystemEvent::TaskCompleted {
                    task_id, result, ..
                } => {
                    tasks_completed += 1;
                    task_statuses.insert(*task_id, format!("Completed with result: {}", result));
                }
                SystemEvent::TaskError { task_id, error, .. } => {
                    tasks_failed += 1;
                    task_statuses.insert(*task_id, format!("Failed with error: {}", error));
                }
                SystemEvent::PluginInvoked {
                    plugin_id, input, ..
                } => {
                    plugins_invoked += 1;
                    plugin_statuses.insert(*plugin_id, format!("Invoked with input: {}", input));
                }
                SystemEvent::PluginResult {
                    plugin_id, output, ..
                } => {
                    plugins_completed += 1;
                    plugin_statuses
                        .insert(*plugin_id, format!("Completed with output: {}", output));
                }
                SystemEvent::PluginError {
                    plugin_id, error, ..
                } => {
                    plugins_failed += 1;
                    plugin_statuses.insert(*plugin_id, format!("Failed with error: {}", error));
                }
                SystemEvent::AgentSpawned {
                    agent_id, prompt, ..
                } => {
                    agents_spawned += 1;
                    agent_statuses.insert(*agent_id, format!("Spawned with prompt: {}", prompt));
                }
                SystemEvent::AgentPartialOutput {
                    agent_id, chunk, ..
                } => {
                    let status = agent_statuses.entry(*agent_id).or_insert_with(String::new);
                    if !status.is_empty() {
                        status.push_str("\n  ");
                    }
                    status.push_str(&format!("Partial output: {}", chunk));
                }
                SystemEvent::AgentCompleted {
                    agent_id, result, ..
                } => {
                    agents_completed += 1;
                    let status = agent_statuses.entry(*agent_id).or_insert_with(String::new);
                    if !status.is_empty() {
                        status.push_str("\n  ");
                    }
                    status.push_str(&format!("Completed with result: {}", result));
                }
                SystemEvent::AgentError {
                    agent_id, error, ..
                } => {
                    agents_failed += 1;
                    agent_statuses.insert(*agent_id, format!("Failed with error: {}", error));
                }
            }
        }

        summary.push_str(&format!("Total Events: {}\n", records.len()));

        summary.push_str("\nTask Statistics:\n");
        summary.push_str("---------------\n");
        summary.push_str(&format!("Tasks Submitted: {}\n", tasks_submitted));
        summary.push_str(&format!("Tasks Completed: {}\n", tasks_completed));
        summary.push_str(&format!("Tasks Failed: {}\n", tasks_failed));

        summary.push_str("\nPlugin Statistics:\n");
        summary.push_str("-----------------\n");
        summary.push_str(&format!("Plugins Invoked: {}\n", plugins_invoked));
        summary.push_str(&format!("Plugins Completed: {}\n", plugins_completed));
        summary.push_str(&format!("Plugins Failed: {}\n", plugins_failed));

        summary.push_str("\nAgent Statistics:\n");
        summary.push_str("----------------\n");
        summary.push_str(&format!("Agents Spawned: {}\n", agents_spawned));
        summary.push_str(&format!("Agents Completed: {}\n", agents_completed));
        summary.push_str(&format!("Agents Failed: {}\n", agents_failed));

        if !task_statuses.is_empty() {
            summary.push_str("\nTask Status Summary:\n");
            summary.push_str("------------------\n");
            for (task_id, status) in task_statuses {
                summary.push_str(&format!("Task {}: {}\n", task_id, status));
            }
        }

        if !plugin_statuses.is_empty() {
            summary.push_str("\nPlugin Status Summary:\n");
            summary.push_str("--------------------\n");
            for (plugin_id, status) in plugin_statuses {
                summary.push_str(&format!("Plugin {}: {}\n", plugin_id, status));
            }
        }

        if !agent_statuses.is_empty() {
            summary.push_str("\nAgent Status Summary:\n");
            summary.push_str("-------------------\n");
            for (agent_id, status) in agent_statuses {
                summary.push_str(&format!("Agent {}: {}\n", agent_id, status));
            }
        }

        summary
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {:?}", self.timestamp, self.event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::EventMetadata;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn test_event_log_basic_flow() {
        let log = EventLog::new();
        let task_id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());

        // Submit task
        log.append(SystemEvent::TaskSubmitted {
            task_id,
            payload: "test task".into(),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id,
                context: json!({}),
            },
        });

        // Complete task
        log.append(SystemEvent::TaskCompleted {
            task_id,
            result: "test result".into(),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id,
                context: json!({}),
            },
        });

        // Verify events were logged
        let records = log.all();
        assert_eq!(records.len(), 2, "Should have logged 2 events");

        // Check first event is TaskSubmitted
        match &records[0].event {
            SystemEvent::TaskSubmitted {
                task_id: t,
                payload,
                metadata,
                ..
            } => {
                assert_eq!(*t, task_id);
                assert_eq!(payload, "test task");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("First event should be TaskSubmitted"),
        }

        // Check second event is TaskCompleted
        match &records[1].event {
            SystemEvent::TaskCompleted {
                task_id: t,
                result,
                metadata,
                ..
            } => {
                assert_eq!(*t, task_id);
                assert_eq!(result, "test result");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Second event should be TaskCompleted"),
        }

        // Verify replay summary
        let summary = log.replay_summary();
        assert!(summary.contains("Total Events: 2"));
        assert!(summary.contains("Tasks Submitted: 1"));
        assert!(summary.contains("Tasks Completed: 1"));
        assert!(summary.contains("Tasks Failed: 0"));
        assert!(summary.contains(&task_id.to_string()));
    }

    #[test]
    fn test_event_log_with_error() {
        let log = EventLog::new();
        let task_id = Uuid::new_v4();

        // Submit task
        log.append(SystemEvent::TaskSubmitted {
            task_id,
            payload: "test task".into(),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id: None,
                context: json!({}),
            },
        });

        // Task fails
        log.append(SystemEvent::TaskError {
            task_id,
            error: "test error".into(),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id: None,
                context: json!({}),
            },
        });

        let summary = log.replay_summary();
        assert!(summary.contains("Tasks Failed: 1"));
        assert!(summary.contains("Failed with error: test error"));
    }

    #[test]
    fn test_event_log_with_plugin() {
        let log = EventLog::new();
        let plugin_id = Uuid::new_v4();

        // Invoke plugin
        log.append(SystemEvent::PluginInvoked {
            plugin_id,
            input: "test input".into(),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id: None,
                context: json!({}),
            },
        });

        // Plugin completes
        log.append(SystemEvent::PluginResult {
            plugin_id,
            output: "test output".into(),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id: None,
                context: json!({}),
            },
        });

        let summary = log.replay_summary();
        assert!(summary.contains("Plugins Invoked: 1"));
        assert!(summary.contains("Plugins Completed: 1"));
        assert!(summary.contains("test output"));
    }

    #[test]
    fn test_event_log_with_agent() {
        let log = EventLog::new();
        let agent_id = Uuid::new_v4();

        // Spawn agent
        log.append(SystemEvent::AgentSpawned {
            agent_id,
            prompt: "test prompt".into(),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id: None,
                context: json!({}),
            },
        });

        // Agent produces partial output
        log.append(SystemEvent::AgentPartialOutput {
            agent_id,
            chunk: "partial result".into(),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id: None,
                context: json!({}),
            },
        });

        // Agent completes
        log.append(SystemEvent::AgentCompleted {
            agent_id,
            result: "final result".into(),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id: None,
                context: json!({}),
            },
        });

        let summary = log.replay_summary();
        assert!(summary.contains("Agents Spawned: 1"));
        assert!(summary.contains("Agents Completed: 1"));
        assert!(summary.contains("partial result"));
        assert!(summary.contains("final result"));
    }
}
