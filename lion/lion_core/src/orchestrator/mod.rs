mod events;
mod processor;

pub use events::{EventMetadata, SystemEvent};
use processor::EventProcessor;

use crate::event_log::EventLog;
use crate::plugin_manager::PluginManager;
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

        // Find the plugins directory relative to CARGO_MANIFEST_DIR
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("lion")
                    .join("lion_core")
            });
        let plugins_dir = manifest_dir.join("..").join("..").join("plugins");

        let plugin_manager = PluginManager::with_manifest_dir(plugins_dir);
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
    use std::time::Duration;
    use tokio::time::timeout;
    use tracing::debug;
    use tracing_subscriber::fmt::format::FmtSpan;
    use uuid::Uuid;

    fn init_test_logging() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .with_span_events(FmtSpan::CLOSE)
            .try_init();
    }

    #[tokio::test]
    async fn test_orchestrator_processes_task() {
        init_test_logging();
        let orchestrator = Orchestrator::new(100);
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
        debug!("Starting plugin invocation test");

        let mut orchestrator = Orchestrator::new(100);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

        // Discover and load plugins
        debug!("Attempting to discover plugins");
        match orchestrator.plugin_manager().discover_plugins() {
            Ok(manifests) => {
                debug!("Discovered {} plugins", manifests.len());
                for manifest in manifests {
                    debug!("Found plugin manifest: {:?}", manifest);
                    if manifest.name == "calculator" {
                        debug!(
                            "Loading calculator plugin with entry point: {}",
                            manifest.entry_point
                        );
                        let plugin_id = orchestrator
                            .plugin_manager()
                            .load_plugin(manifest)
                            .expect("Failed to load calculator plugin");

                        // Spawn the orchestrator
                        tokio::spawn(orchestrator.run());

                        // Create and send a plugin invocation
                        let input = serde_json::json!({
                            "function": "add",
                            "args": {
                                "a": 5,
                                "b": 3
                            }
                        });

                        let event =
                            SystemEvent::new_plugin_invocation(plugin_id, input.to_string(), None);
                        sender.send(event).await.expect("Failed to send event");

                        // Wait for completion with timeout
                        let completion = timeout(Duration::from_secs(1), completion_rx.recv())
                            .await
                            .expect("Timeout waiting for completion")
                            .expect("Channel closed");

                        match completion {
                            SystemEvent::PluginResult {
                                plugin_id: completed_id,
                                output,
                                ..
                            } => {
                                assert_eq!(completed_id, plugin_id);
                                assert!(output.contains(r#""result":8.0"#));
                                return;
                            }
                            _ => panic!("Expected PluginResult event"),
                        }
                    }
                }
                debug!("Calculator plugin not found in discovered plugins");
                panic!("Calculator plugin not found");
            }
            Err(e) => panic!("Failed to discover plugins: {}", e),
        }
    }

    #[tokio::test]
    async fn test_correlation_id_propagation() {
        init_test_logging();
        let orchestrator = Orchestrator::new(100);
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
        let orchestrator = Orchestrator::new(100);
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
