use tokio::sync::{mpsc, broadcast};
use tracing::{info, error};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

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
}

impl SystemEvent {
    /// Create a new TaskSubmitted event
    pub fn new_task(payload: String, correlation_id: Option<Uuid>) -> Self {
        SystemEvent::TaskSubmitted {
            task_id: Uuid::new_v4(),
            payload,
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                correlation_id,
                context: serde_json::json!({}),
            },
        }
    }

    /// Get the event's metadata
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            SystemEvent::TaskSubmitted { metadata, .. } => metadata,
            SystemEvent::TaskCompleted { metadata, .. } => metadata,
            SystemEvent::TaskError { metadata, .. } => metadata,
        }
    }
}

/// The core orchestrator that processes system events
pub struct Orchestrator {
    event_tx: mpsc::Sender<SystemEvent>,
    event_rx: mpsc::Receiver<SystemEvent>,
    completion_tx: broadcast::Sender<SystemEvent>,
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

    /// Process a single event, returning a completion event if successful
    async fn process_event(&self, event: SystemEvent) -> Option<SystemEvent> {
        match event {
            SystemEvent::TaskSubmitted { task_id, payload, metadata } => {
                info!(
                    task_id = %task_id,
                    correlation_id = ?metadata.correlation_id,
                    "Processing task"
                );

                // Simulate some processing
                let result = format!("Processed: {}", payload);

                Some(SystemEvent::TaskCompleted {
                    task_id,
                    result,
                    metadata: EventMetadata {
                        event_id: Uuid::new_v4(),
                        timestamp: chrono::Utc::now(),
                        correlation_id: metadata.correlation_id,
                        context: metadata.context,
                    },
                })
            }
            SystemEvent::TaskCompleted { task_id, result, metadata } => {
                info!(
                    task_id = %task_id,
                    correlation_id = ?metadata.correlation_id,
                    "Task completed: {}", result
                );
                None
            }
            SystemEvent::TaskError { task_id, error, metadata } => {
                error!(
                    task_id = %task_id,
                    correlation_id = ?metadata.correlation_id,
                    "Task error: {}", error
                );
                None
            }
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
    use tokio::time::timeout;
    use std::time::Duration;

    #[tokio::test]
    async fn test_orchestrator_processes_task() {
        let orchestrator = Orchestrator::new(100);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

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
        let completion = timeout(
            Duration::from_secs(1),
            completion_rx.recv()
        ).await.expect("Timeout waiting for completion")
         .expect("Channel closed");

        match completion {
            SystemEvent::TaskCompleted { task_id: completed_id, .. } => {
                assert_eq!(completed_id, task_id);
            }
            _ => panic!("Expected TaskCompleted event"),
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

        let completion = timeout(
            Duration::from_secs(1),
            completion_rx.recv()
        ).await.expect("Timeout waiting for completion")
         .expect("Channel closed");

        assert_eq!(
            completion.metadata().correlation_id,
            correlation_id,
            "Correlation ID should be preserved"
        );
    }
}