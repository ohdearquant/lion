use crate::element::ElementData;
use crate::event_log::EventLog;
use crate::plugin_manager::{PluginManager, PluginManifest};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Events that can occur in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    // Task events
    TaskSubmitted {
        task_id: Uuid,
        payload: String,
    },
    TaskCompleted {
        task_id: Uuid,
        result: String,
    },
    TaskFailed {
        task_id: Uuid,
        error: String,
    },

    // Agent events
    AgentSpawned {
        agent_id: Uuid,
        prompt: Option<String>,
    },
    AgentPartialOutput {
        agent_id: Uuid,
        chunk: String,
    },
    AgentCompleted {
        agent_id: Uuid,
        result: String,
    },
    AgentError {
        agent_id: Uuid,
        error: String,
    },

    // Plugin events
    PluginLoad {
        plugin_id: Uuid,
        manifest: PluginManifest,
        manifest_path: Option<String>,
    },
    PluginInvoked {
        plugin_id: Uuid,
        input: String,
    },
    PluginResult {
        plugin_id: Uuid,
        result: String,
    },
    PluginError {
        plugin_id: Uuid,
        error: String,
    },
    ListPlugins,
}

/// The orchestrator manages the system's event loop and coordinates components
pub struct Orchestrator {
    event_log: Arc<EventLog>,
    plugin_manager: Arc<PluginManager>,
    event_sender: mpsc::Sender<SystemEvent>,
    event_receiver: mpsc::Receiver<SystemEvent>,
    completion_sender: broadcast::Sender<SystemEvent>,
}

impl Orchestrator {
    pub fn new(channel_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(channel_size);
        let (completion_tx, _) = broadcast::channel(channel_size);

        // Create a plugin manager with a default plugins directory
        let plugin_manager = Arc::new(PluginManager::with_manifest_dir("plugins"));

        debug!(
            "Created new orchestrator with plugin manager: {:?}",
            plugin_manager
        );

        Self {
            event_log: Arc::new(EventLog::new(1000)), // Keep last 1000 events
            plugin_manager,
            event_sender: tx,
            event_receiver: rx,
            completion_sender: completion_tx,
        }
    }

    pub fn with_plugin_dir<P: AsRef<Path>>(channel_size: usize, plugin_dir: P) -> Self {
        let (tx, rx) = mpsc::channel(channel_size);
        let (completion_tx, _) = broadcast::channel(channel_size);

        let plugin_manager = Arc::new(PluginManager::with_manifest_dir(plugin_dir));

        debug!(
            "Created new orchestrator with custom plugin directory: {:?}",
            plugin_manager
        );

        Self {
            event_log: Arc::new(EventLog::new(1000)),
            plugin_manager,
            event_sender: tx,
            event_receiver: rx,
            completion_sender: completion_tx,
        }
    }

    /// Get a sender for submitting events to the orchestrator
    pub fn sender(&self) -> mpsc::Sender<SystemEvent> {
        self.event_sender.clone()
    }

    /// Get a receiver for completion events
    pub fn completion_receiver(&self) -> broadcast::Receiver<SystemEvent> {
        self.completion_sender.subscribe()
    }

    /// Get access to the event log
    pub fn event_log(&self) -> Arc<EventLog> {
        self.event_log.clone()
    }

    /// Run the orchestrator's event loop
    pub async fn run(mut self) {
        info!("Starting orchestrator event loop");

        while let Some(event) = self.event_receiver.recv().await {
            // Log the event
            self.event_log.log_event(event.clone());

            match &event {
                // Plugin events
                SystemEvent::PluginLoad {
                    plugin_id,
                    manifest,
                    manifest_path,
                } => {
                    info!(plugin_id = %plugin_id, "Loading plugin {}", manifest.name);

                    // If manifest_path is provided, update the plugin manager's manifest directory
                    if let Some(path) = manifest_path {
                        if let Some(parent) = Path::new(path).parent() {
                            debug!("Updating plugin manager directory to: {:?}", parent);
                            self.plugin_manager =
                                Arc::new(PluginManager::with_manifest_dir(parent));
                        }
                    }

                    match self.plugin_manager.load_plugin(manifest.clone()) {
                        Ok(_) => {
                            let result = SystemEvent::PluginResult {
                                plugin_id: *plugin_id,
                                result: format!("Plugin {} loaded successfully", manifest.name),
                            };
                            let _ = self.completion_sender.send(result.clone());
                            info!(plugin_id = %plugin_id, "Plugin {} loaded successfully", manifest.name);
                            self.event_log.log_event(result);
                        }
                        Err(e) => {
                            let error = SystemEvent::PluginError {
                                plugin_id: *plugin_id,
                                error: e.to_string(),
                            };
                            let _ = self.completion_sender.send(error.clone());
                            error!(plugin_id = %plugin_id, "Failed to load plugin: {}", e);
                            self.event_log.log_event(error);
                        }
                    }
                }
                SystemEvent::PluginInvoked { plugin_id, input } => {
                    info!(plugin_id = %plugin_id, "Invoking plugin");
                    match self.plugin_manager.invoke_plugin(*plugin_id, input) {
                        Ok(output) => {
                            let result = SystemEvent::PluginResult {
                                plugin_id: *plugin_id,
                                result: output,
                            };
                            let _ = self.completion_sender.send(result.clone());
                            self.event_log.log_event(result);
                        }
                        Err(e) => {
                            let error = SystemEvent::PluginError {
                                plugin_id: *plugin_id,
                                error: e.to_string(),
                            };
                            let _ = self.completion_sender.send(error.clone());
                            self.event_log.log_event(error);
                        }
                    }
                }
                SystemEvent::ListPlugins => {
                    info!("Listing plugins");
                    let plugins = self.plugin_manager.list_plugins();
                    debug!("Found {} plugins", plugins.len());

                    if plugins.is_empty() {
                        let result = SystemEvent::PluginResult {
                            plugin_id: Uuid::nil(),
                            result: "No plugins loaded".to_string(),
                        };
                        let _ = self.completion_sender.send(result.clone());
                        self.event_log.log_event(result);
                    } else {
                        for (id, manifest) in plugins {
                            let result = SystemEvent::PluginResult {
                                plugin_id: id,
                                result: serde_json::to_string(&manifest).unwrap_or_default(),
                            };
                            let _ = self.completion_sender.send(result.clone());
                            self.event_log.log_event(result);
                        }
                    }
                }

                // Agent events - broadcast them for real-time updates
                SystemEvent::AgentSpawned { .. }
                | SystemEvent::AgentPartialOutput { .. }
                | SystemEvent::AgentCompleted { .. }
                | SystemEvent::AgentError { .. } => {
                    let _ = self.completion_sender.send(event.clone());
                }

                // Task events
                SystemEvent::TaskSubmitted { task_id, payload } => {
                    info!(task_id = %task_id, "Task submitted: {}", payload);
                    // Here you might queue the task for execution
                }
                SystemEvent::TaskCompleted { task_id, result } => {
                    info!(task_id = %task_id, "Task completed: {}", result);
                    let _ = self.completion_sender.send(event.clone());
                }
                SystemEvent::TaskFailed { task_id, error } => {
                    error!(task_id = %task_id, "Task failed: {}", error);
                    let _ = self.completion_sender.send(event.clone());
                }

                // Plugin result/error events are generated internally
                SystemEvent::PluginResult { .. } | SystemEvent::PluginError { .. } => {
                    // These are generated by the orchestrator itself
                    // Just broadcast them
                    let _ = self.completion_sender.send(event.clone());
                }
            }
        }

        info!("Orchestrator event loop terminated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_orchestrator_events() {
        let orchestrator = Orchestrator::new(100);
        let sender = orchestrator.sender();
        let mut receiver = orchestrator.completion_receiver();

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Send a test event
        let task_id = Uuid::new_v4();
        sender
            .send(SystemEvent::TaskSubmitted {
                task_id,
                payload: "test task".to_string(),
            })
            .await
            .unwrap();

        // Send completion
        sender
            .send(SystemEvent::TaskCompleted {
                task_id,
                result: "done".to_string(),
            })
            .await
            .unwrap();

        // Should receive the completion event
        let received = tokio::time::timeout(Duration::from_secs(1), receiver.recv())
            .await
            .unwrap()
            .unwrap();

        match received {
            SystemEvent::TaskCompleted {
                task_id: id,
                result,
            } => {
                assert_eq!(id, task_id);
                assert_eq!(result, "done");
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_plugin_events() {
        let test_dir = tempdir().unwrap();
        let orchestrator = Orchestrator::with_plugin_dir(100, test_dir.path());
        let sender = orchestrator.sender();
        let mut receiver = orchestrator.completion_receiver();

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Create a test plugin manifest
        let plugin_id = Uuid::new_v4();
        let manifest = PluginManifest {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: "nonexistent".to_string(),
            permissions: vec![],
            driver: None,
            functions: std::collections::HashMap::new(),
        };

        // Try to load the plugin (should fail because entry_point doesn't exist)
        sender
            .send(SystemEvent::PluginLoad {
                plugin_id,
                manifest,
                manifest_path: Some(
                    test_dir
                        .path()
                        .join("manifest.toml")
                        .to_string_lossy()
                        .into_owned(),
                ),
            })
            .await
            .unwrap();

        // Should receive an error event
        let received = tokio::time::timeout(Duration::from_secs(1), receiver.recv())
            .await
            .unwrap()
            .unwrap();

        match received {
            SystemEvent::PluginError {
                plugin_id: id,
                error,
            } => {
                assert_eq!(id, plugin_id);
                assert!(error.contains("not found"));
            }
            _ => panic!("Expected PluginError event"),
        }
    }
}
