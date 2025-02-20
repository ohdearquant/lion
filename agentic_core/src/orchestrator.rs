use crate::event_log::EventLog;
use crate::plugin_manager::PluginManager;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{broadcast, mpsc};
use tracing::info;
use uuid::Uuid;

/// Represents system-level events that flow through the orchestrator.
/// Each event carries metadata for tracking and correlation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique identifier for this event
    pub event_id: Uuid,
    /// When this event was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional correlation ID to track related events
    pub correlation_id: Option<Uuid>,
    /// Additional context as key-value pairs
    pub context: serde_json::Value,
}

/// System events that can be processed by the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    /// A new task has been submitted for processing
    TaskSubmitted {
        task_id: Uuid,
        payload: String,
        metadata: EventMetadata,
    },
    /// A task has been completed
    TaskCompleted {
        task_id: Uuid,
        result: String,
        metadata: EventMetadata,
    },
    /// An error occurred during task processing
    TaskError {
        task_id: Uuid,
        error: String,
        metadata: EventMetadata,
    },
    /// A plugin is being invoked
    PluginInvoked {
        plugin_id: Uuid,
        input: String,
        metadata: EventMetadata,
    },
    /// A plugin has produced a result
    PluginResult {
        plugin_id: Uuid,
        output: String,
        metadata: EventMetadata,
    },
    /// A plugin invocation resulted in an error
    PluginError {
        plugin_id: Uuid,
        error: String,
        metadata: EventMetadata,
    },
}

impl SystemEvent {
    /// Create a new TaskSubmitted event
    pub fn new_task(payload: String, correlation_id: Option<Uuid>) -> Self {
        SystemEvent::TaskSubmitted {
            task_id: Uuid::new_v4(),
            payload,
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id,
                context: json!({}),
            },
        }
    }

    /// Create a new PluginInvoked event
    pub fn new_plugin_invocation(
        plugin_id: Uuid,
        input: String,
        correlation_id: Option<Uuid>,
    ) -> Self {
        SystemEvent::PluginInvoked {
            plugin_id,
            input,
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                correlation_id,
                context: json!({}),
            },
        }
    }

    /// Get the event's metadata
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            SystemEvent::TaskSubmitted { metadata, .. } => metadata,
            SystemEvent::TaskCompleted { metadata, .. } => metadata,
            SystemEvent::TaskError { metadata, .. } => metadata,
            SystemEvent::PluginInvoked { metadata, .. } => metadata,
            SystemEvent::PluginResult { metadata, .. } => metadata,
            SystemEvent::PluginError { metadata, .. } => metadata,
        }
    }
}

/// The core orchestrator that processes system events
pub struct Orchestrator {
    event_tx: mpsc::Sender<SystemEvent>,
    event_rx: mpsc::Receiver<SystemEvent>,
    completion_tx: broadcast::Sender<SystemEvent>,
    event_log: EventLog,
    plugin_manager: PluginManager,
}

impl Orchestrator {
    /// Create a new orchestrator instance with specified channel capacity
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(channel_capacity);
        let (completion_tx, _) = broadcast::channel(channel_capacity);
        Self {
            event_tx: tx,
            event_rx: rx,
            completion_tx,
            event_log: EventLog::new(),
            plugin_manager: PluginManager::new(),
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
        &self.event_log
    }

    /// Get a reference to the plugin manager
    pub fn plugin_manager(&mut self) -> &mut PluginManager {
        &mut self.plugin_manager
    }

    /// Process a single event, returning a completion event if successful
    async fn process_event(&self, event: SystemEvent) -> Option<SystemEvent> {
        // Log the incoming event
        self.event_log.append(event.clone());

        match event {
            SystemEvent::TaskSubmitted {
                task_id,
                payload,
                metadata,
            } => {
                info!(
                    task_id = %task_id,
                    correlation_id = ?metadata.correlation_id,
                    "Processing task"
                );

                // Simulate some processing
                let result = format!("Processed: {}", payload);

                let completion = SystemEvent::TaskCompleted {
                    task_id,
                    result,
                    metadata: EventMetadata {
                        event_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        correlation_id: metadata.correlation_id,
                        context: metadata.context,
                    },
                };

                // Log the completion event
                self.event_log.append(completion.clone());
                Some(completion)
            }
            SystemEvent::PluginInvoked {
                plugin_id,
                input,
                metadata,
            } => {
                info!(
                    plugin_id = %plugin_id,
                    correlation_id = ?metadata.correlation_id,
                    "Invoking plugin"
                );

                match self.plugin_manager.invoke_plugin(plugin_id, &input) {
                    Ok(output) => {
                        let result = SystemEvent::PluginResult {
                            plugin_id,
                            output,
                            metadata: EventMetadata {
                                event_id: Uuid::new_v4(),
                                timestamp: Utc::now(),
                                correlation_id: metadata.correlation_id,
                                context: metadata.context,
                            },
                        };
                        self.event_log.append(result.clone());
                        Some(result)
                    }
                    Err(e) => {
                        let error_event = SystemEvent::PluginError {
                            plugin_id,
                            error: e.to_string(),
                            metadata: EventMetadata {
                                event_id: Uuid::new_v4(),
                                timestamp: Utc::now(),
                                correlation_id: metadata.correlation_id,
                                context: metadata.context,
                            },
                        };
                        self.event_log.append(error_event.clone());
                        Some(error_event)
                    }
                }
            }
            SystemEvent::TaskCompleted { .. }
            | SystemEvent::TaskError { .. }
            | SystemEvent::PluginResult { .. }
            | SystemEvent::PluginError { .. } => None,
        }
    }

    /// Run the orchestrator's event loop
    pub async fn run(mut self) {
        info!("Orchestrator starting");

        while let Some(event) = self.event_rx.recv().await {
            if let Some(completion_event) = self.process_event(event).await {
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
    use crate::plugin_manager::PluginManifest;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_orchestrator_processes_task() {
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
        let mut orchestrator = Orchestrator::new(100);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

        // Create a test plugin
        let manifest = PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            entry_point: "/dev/null".to_string(), // dummy path for testing
            permissions: vec![],
        };

        let plugin_id = orchestrator
            .plugin_manager()
            .load_plugin(manifest)
            .expect("Failed to load plugin");

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Create and send a plugin invocation
        let event = SystemEvent::new_plugin_invocation(plugin_id, "test input".to_string(), None);
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
                assert!(output.contains("test input"));
            }
            _ => panic!("Expected PluginResult event"),
        }
    }

    #[tokio::test]
    async fn test_correlation_id_propagation() {
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
}
