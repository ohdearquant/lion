mod events;
mod processor;

pub use events::{EventMetadata, SystemEvent};
use processor::EventProcessor;

use crate::event_log::EventLog;
use crate::plugin_manager::{Config, PluginManager, PluginsConfig};
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc};
use tracing::info;

/// The core orchestrator that processes system events
pub struct Orchestrator {
    event_tx: mpsc::Sender<SystemEvent>,
    event_rx: mpsc::Receiver<SystemEvent>,
    completion_tx: broadcast::Sender<SystemEvent>,
    processor: EventProcessor,
}

impl Orchestrator {
    /// Create a new orchestrator instance with specified channel capacity
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(channel_capacity);
        let (completion_tx, _) = broadcast::channel(channel_capacity);

        let event_log = EventLog::new();

        // Try to load config from Lion.toml, fall back to default paths
        let plugin_manager = match Config::from_project_root() {
            Ok(config) => PluginManager::with_config(config),
            Err(_) => {
                // Fall back to default paths
                let plugins_dir = PathBuf::from("plugins");
                PluginManager::with_config(Config {
                    plugins: PluginsConfig {
                        data_dir: plugins_dir.join("data"),
                        calculator_manifest: plugins_dir.join("calculator").join("manifest.toml"),
                    },
                })
            }
        };

        let processor = EventProcessor::new(event_log, plugin_manager);

        Self {
            event_tx: tx,
            event_rx: rx,
            completion_tx,
            processor,
        }
    }

    /// Create a new orchestrator instance with a custom plugin manager
    pub fn with_plugin_manager(channel_capacity: usize, plugin_manager: PluginManager) -> Self {
        let (tx, rx) = mpsc::channel(channel_capacity);
        let (completion_tx, _) = broadcast::channel(channel_capacity);

        let event_log = EventLog::new();
        let processor = EventProcessor::new(event_log, plugin_manager);

        Self {
            event_tx: tx,
            event_rx: rx,
            completion_tx,
            processor,
        }
    }

    /// Get a sender that can be used to submit events to this orchestrator
    pub fn sender(&self) -> mpsc::Sender<SystemEvent> {
        self.event_tx.clone()
    }

    /// Get a receiver for completion events
    pub fn completion_receiver(&self) -> broadcast::Receiver<SystemEvent> {
        self.completion_tx.subscribe()
    }

    /// Get a reference to the event log
    pub fn event_log(&self) -> &EventLog {
        &self.processor.event_log
    }

    /// Get a reference to the plugin manager
    pub fn plugin_manager(&mut self) -> &mut PluginManager {
        &mut self.processor.plugin_manager
    }

    /// Run the orchestrator's event loop
    pub async fn run(mut self) {
        info!("Orchestrator starting");

        while let Some(event) = self.event_rx.recv().await {
            if let Some(completion_event) = self.processor.process_event(event).await {
                // Broadcast the completion event
                let _ = self.completion_tx.send(completion_event);
            }
        }

        info!("Orchestrator shutting down");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_manager::init_test_logging;
    use std::time::Duration;
    use tokio::time::timeout;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_orchestrator_processes_task() {
        let plugin_manager = PluginManager::new();
        let orchestrator = Orchestrator::with_plugin_manager(100, plugin_manager);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();
        let event_log = orchestrator.event_log().clone();

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Create and send a task
        let event = SystemEvent::new_task("test payload".to_string(), None);
        let task_id = match &event {
            SystemEvent::TaskSubmitted { task_id, .. } => *task_id,
            _ => panic!("Unexpected event type"),
        };

        sender.send(event).await.expect("Failed to send event");

        // Wait for completion with timeout
        let completion = timeout(Duration::from_secs(1), completion_rx.recv())
            .await
            .expect("Timeout waiting for completion")
            .expect("Channel closed");

        match completion {
            SystemEvent::TaskCompleted {
                task_id: completed_id,
                ..
            } => {
                assert_eq!(completed_id, task_id);
            }
            _ => panic!("Expected TaskCompleted event"),
        }

        // Verify events were logged
        tokio::time::sleep(Duration::from_millis(100)).await;
        let records = event_log.all();
        assert_eq!(
            records.len(),
            2,
            "Should have submission and completion events"
        );
    }

    #[tokio::test]
    async fn test_plugin_invocation() {
        init_test_logging();
        // Create orchestrator with default config from Lion.toml
        let plugin_manager = PluginManager::new();
        let mut orchestrator = Orchestrator::with_plugin_manager(100, plugin_manager);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

        // Discover plugins
        let manifests = orchestrator
            .plugin_manager()
            .discover_plugins()
            .expect("Failed to discover plugins");

        // Find and load calculator plugin
        let calculator = manifests
            .iter()
            .find(|m| m.name == "calculator")
            .expect("Calculator plugin not found");

        let plugin_id = orchestrator
            .plugin_manager()
            .load_plugin(calculator.clone())
            .expect("Failed to load calculator plugin");

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Create and send a plugin invocation
        let input = serde_json::json!({
            "function": "add",
            "args": { "a": 5, "b": 3 }
        });

        let event = SystemEvent::new_plugin_invocation(plugin_id, input.to_string(), None);
        sender.send(event).await.expect("Failed to send event");

        // Wait for completion
        let completion = timeout(Duration::from_secs(1), completion_rx.recv())
            .await
            .expect("Timeout waiting for completion")
            .expect("Channel closed");
        match completion {
            SystemEvent::PluginResult {
                plugin_id: id,
                output,
                ..
            } if id == plugin_id && output.contains(r#""result":8.0"#) => (),
            _ => panic!("Expected plugin result"),
        }
    }

    #[tokio::test]
    async fn test_correlation_id_propagation() {
        init_test_logging();
        let plugin_manager = PluginManager::new();
        let orchestrator = Orchestrator::with_plugin_manager(100, plugin_manager);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        let correlation_id = Some(Uuid::new_v4());
        let event = SystemEvent::new_task("test payload".to_string(), correlation_id);

        sender.send(event).await.expect("Failed to send event");

        let completion = timeout(Duration::from_secs(1), completion_rx.recv())
            .await
            .expect("Timeout waiting for completion")
            .expect("Channel closed");

        assert_eq!(
            completion.metadata().correlation_id,
            correlation_id,
            "Correlation ID should be preserved"
        );
    }

    #[tokio::test]
    async fn test_agent_spawn_and_completion() {
        init_test_logging();
        let plugin_manager = PluginManager::new();
        let orchestrator = Orchestrator::with_plugin_manager(100, plugin_manager);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();
        let event_log = orchestrator.event_log().clone();

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Create and send an agent spawn event
        let event = SystemEvent::new_agent("test prompt".to_string(), None);
        let agent_id = match &event {
            SystemEvent::AgentSpawned { agent_id, .. } => *agent_id,
            _ => panic!("Unexpected event type"),
        };

        sender.send(event).await.expect("Failed to send event");

        // Wait for completion with timeout
        let completion = timeout(Duration::from_secs(1), completion_rx.recv())
            .await
            .expect("Timeout waiting for completion")
            .expect("Channel closed");

        match completion {
            SystemEvent::AgentCompleted {
                agent_id: completed_id,
                result,
                ..
            } => {
                assert_eq!(completed_id, agent_id);
                assert!(result.contains("test prompt"));
            }
            _ => panic!("Expected AgentCompleted event"),
        }

        // Verify events were logged
        tokio::time::sleep(Duration::from_millis(100)).await;
        let records = event_log.all();
        assert_eq!(
            records.len(),
            4,
            "Should have spawn, partial outputs, and completion events"
        );

        // Verify event sequence
        assert!(
            matches!(records[0].event, SystemEvent::AgentSpawned { .. }),
            "First event should be AgentSpawned"
        );
        assert!(
            matches!(records[1].event, SystemEvent::AgentPartialOutput { .. }),
            "Second event should be AgentPartialOutput"
        );
        assert!(
            matches!(records[2].event, SystemEvent::AgentPartialOutput { .. }),
            "Third event should be AgentPartialOutput"
        );
        assert!(
            matches!(records[3].event, SystemEvent::AgentCompleted { .. }),
            "Fourth event should be AgentCompleted"
        );
    }

    mod state_consistency;
}
