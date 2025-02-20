use crate::{
    orchestrator::{
        events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent},
        metadata::EventMetadata,
    },
    plugin_manager::PluginManager,
    EventLog,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Handler for system events that coordinates with the plugin manager and event log
#[derive(Debug, Clone)]
pub struct EventHandler {
    event_log: Arc<EventLog>,
    plugin_manager: Arc<PluginManager>,
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            event_log: Arc::new(EventLog::new()),
            plugin_manager: Arc::new(PluginManager::new()),
        }
    }

    pub fn event_log(&self) -> &EventLog {
        &self.event_log
    }

    pub fn plugin_manager(&self) -> &PluginManager {
        &self.plugin_manager
    }

    pub fn handle_task(&self, event: TaskEvent) -> Option<SystemEvent> {
        self.event_log.append(SystemEvent::Task(event.clone()));

        match event {
            TaskEvent::Submitted {
                task_id,
                payload,
                metadata,
            } => {
                info!(task_id = %task_id, "Processing task: {}", payload);
                Some(SystemEvent::Task(TaskEvent::Completed {
                    task_id,
                    result: format!("Processed: {}", payload),
                    metadata: EventMetadata::new(metadata.correlation_id),
                }))
            }
            TaskEvent::Completed { .. } | TaskEvent::Error { .. } => None,
        }
    }

    pub async fn handle_plugin(&self, event: PluginEvent) -> Option<SystemEvent> {
        self.event_log.append(SystemEvent::Plugin(event.clone()));

        match event {
            PluginEvent::Load {
                plugin_id,
                manifest,
                manifest_path,
                metadata,
            } => {
                info!(plugin_id = %plugin_id, "Loading plugin: {}", manifest.name);

                match self
                    .plugin_manager
                    .load_plugin_from_string(toml::to_string(&manifest).unwrap(), manifest_path)
                    .await
                {
                    Ok(_) => Some(SystemEvent::Plugin(PluginEvent::Result {
                        plugin_id,
                        result: format!("Plugin {} loaded successfully", manifest.name),
                        metadata: EventMetadata::new(metadata.correlation_id),
                    })),
                    Err(e) => Some(SystemEvent::Plugin(PluginEvent::Error {
                        plugin_id,
                        error: e.to_string(),
                        metadata: EventMetadata::new(metadata.correlation_id),
                    })),
                }
            }
            PluginEvent::Invoked {
                plugin_id,
                input,
                metadata,
            } => {
                info!(plugin_id = %plugin_id, "Invoking plugin with input: {}", input);

                match self.plugin_manager.invoke_plugin(plugin_id, &input).await {
                    Ok(result) => Some(SystemEvent::Plugin(PluginEvent::Result {
                        plugin_id,
                        result,
                        metadata: EventMetadata::new(metadata.correlation_id),
                    })),
                    Err(e) => Some(SystemEvent::Plugin(PluginEvent::Error {
                        plugin_id,
                        error: e.to_string(),
                        metadata: EventMetadata::new(metadata.correlation_id),
                    })),
                }
            }
            PluginEvent::List => {
                let plugins = self.plugin_manager.list_plugins();
                let result = plugins
                    .iter()
                    .map(|p| format!("{} ({})", p.manifest.name, p.id))
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(SystemEvent::Plugin(PluginEvent::Result {
                    plugin_id: Uuid::new_v4(),
                    result,
                    metadata: EventMetadata::new(None),
                }))
            }
            PluginEvent::Result { .. } | PluginEvent::Error { .. } => None,
        }
    }

    pub fn handle_agent(&self, event: AgentEvent) -> Option<SystemEvent> {
        self.event_log.append(SystemEvent::Agent(event.clone()));

        match event {
            AgentEvent::Spawned {
                agent_id,
                prompt,
                metadata,
            } => {
                info!(agent_id = %agent_id, "Processing agent prompt: {}", prompt);
                Some(SystemEvent::Agent(AgentEvent::Completed {
                    agent_id,
                    result: format!("Processed: {}", prompt),
                    metadata: EventMetadata::new(metadata.correlation_id),
                }))
            }
            AgentEvent::PartialOutput { .. }
            | AgentEvent::Completed { .. }
            | AgentEvent::Error { .. } => None,
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::plugin::PluginManifest;

    #[tokio::test]
    async fn test_plugin_handling() {
        let handler = EventHandler::new();
        let plugin_id = Uuid::new_v4();
        let metadata = EventMetadata::new(None);

        // Create test manifest
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        // Test plugin loading
        let load_event = PluginEvent::Load {
            plugin_id,
            manifest,
            manifest_path: None,
            metadata: metadata.clone(),
        };

        if let Some(SystemEvent::Plugin(PluginEvent::Result { result, .. })) =
            handler.handle_plugin(load_event).await
        {
            assert!(result.contains("loaded successfully"));
        } else {
            panic!("Expected successful plugin load");
        }

        // Test plugin invocation
        let invoke_event = PluginEvent::Invoked {
            plugin_id,
            input: "test input".to_string(),
            metadata,
        };

        if let Some(SystemEvent::Plugin(PluginEvent::Result { result, .. })) =
            handler.handle_plugin(invoke_event).await
        {
            assert!(result.contains("test input"));
        } else {
            panic!("Expected successful plugin invocation");
        }
    }

    #[test]
    fn test_task_handling() {
        let handler = EventHandler::new();
        let task_id = Uuid::new_v4();
        let metadata = EventMetadata::new(None);

        let event = TaskEvent::Submitted {
            task_id,
            payload: "test task".to_string(),
            metadata,
        };

        if let Some(SystemEvent::Task(TaskEvent::Completed { result, .. })) =
            handler.handle_task(event)
        {
            assert!(result.contains("test task"));
        } else {
            panic!("Expected task completion");
        }
    }

    #[test]
    fn test_agent_handling() {
        let handler = EventHandler::new();
        let agent_id = Uuid::new_v4();
        let metadata = EventMetadata::new(None);

        let event = AgentEvent::Spawned {
            agent_id,
            prompt: "test prompt".to_string(),
            metadata,
        };

        if let Some(SystemEvent::Agent(AgentEvent::Completed { result, .. })) =
            handler.handle_agent(event)
        {
            assert!(result.contains("test prompt"));
        } else {
            panic!("Expected agent completion");
        }
    }

    #[test]
    fn test_handler_clone() {
        let handler = EventHandler::new();
        let cloned = handler.clone();
        
        // Verify both handlers work
        let task_id = Uuid::new_v4();
        let metadata = EventMetadata::new(None);
        let event = TaskEvent::Submitted {
            task_id,
            payload: "test task".to_string(),
            metadata,
        };

        assert!(handler.handle_task(event.clone()).is_some());
        assert!(cloned.handle_task(event).is_some());
    }
}
