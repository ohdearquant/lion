use crate::orchestrator::{
    events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent},
    handlers::EventHandler,
    types::*,
};
use crate::plugin_manager::PluginManifest;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info};

/// The core orchestrator that processes system events
pub struct Orchestrator {
    event_tx: EventSender,
    event_rx: EventReceiver,
    completion_tx: CompletionSender,
    handler: EventHandler,
}

impl Orchestrator {
    /// Create a new orchestrator instance with specified channel capacity
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(channel_capacity);
        let (completion_tx, _) = broadcast::channel(channel_capacity);
        debug!("Creating new orchestrator");

        Self {
            event_tx: tx,
            event_rx: rx,
            completion_tx,
            handler: EventHandler::new(),
        }
    }

    /// Get a sender that can be used to submit events to this orchestrator
    pub fn sender(&self) -> EventSender {
        self.event_tx.clone()
    }

    /// Get a receiver for completion events
    pub fn completion_receiver(&self) -> CompletionReceiver {
        self.completion_tx.subscribe()
    }

    /// Process a single event, returning a completion event if successful
    async fn process_event(&mut self, event: SystemEvent) -> Option<SystemEvent> {
        match event {
            SystemEvent::Task(task_event) => self.handler.handle_task(task_event),
            SystemEvent::Plugin(plugin_event) => self.handler.handle_plugin(plugin_event).await,
            SystemEvent::Agent(agent_event) => self.handler.handle_agent(agent_event),
        }
    }

    /// Run the orchestrator's event loop
    pub async fn run(mut self) {
        info!("Starting orchestrator event loop");

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
    use crate::orchestrator::metadata::EventMetadata;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_orchestrator_processes_task() {
        let orchestrator = Orchestrator::new(100);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Create and send a task
        let task_id = Uuid::new_v4();
        let event = SystemEvent::Task(TaskEvent::Submitted {
            task_id,
            payload: "test task".to_string(),
            metadata: EventMetadata::new(None),
        });

        sender.send(event).await.expect("Failed to send event");

        // Wait for completion
        if let Ok(SystemEvent::Task(TaskEvent::Completed {
            task_id: completed_id,
            ..
        })) = completion_rx.recv().await
        {
            assert_eq!(completed_id, task_id);
        } else {
            panic!("Expected TaskCompleted event");
        }
    }

    #[tokio::test]
    async fn test_orchestrator_processes_plugin() {
        let orchestrator = Orchestrator::new(100);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Create and send a plugin event
        let plugin_id = Uuid::new_v4();
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "Test plugin".to_string(),
        );

        let event = SystemEvent::Plugin(PluginEvent::Load {
            plugin_id,
            manifest,
            manifest_path: None,
            metadata: EventMetadata::new(None),
        });

        sender.send(event).await.expect("Failed to send event");

        // Wait for completion
        if let Ok(SystemEvent::Plugin(PluginEvent::Result {
            plugin_id: completed_id,
            ..
        })) = completion_rx.recv().await
        {
            assert_eq!(completed_id, plugin_id);
        } else {
            panic!("Expected PluginResult event");
        }
    }

    #[tokio::test]
    async fn test_orchestrator_processes_agent() {
        let orchestrator = Orchestrator::new(100);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Create and send an agent event
        let agent_id = Uuid::new_v4();
        let event = SystemEvent::Agent(AgentEvent::Spawned {
            agent_id,
            prompt: "test prompt".to_string(),
            metadata: EventMetadata::new(None),
        });

        sender.send(event).await.expect("Failed to send event");

        // Wait for completion
        if let Ok(SystemEvent::Agent(AgentEvent::Completed {
            agent_id: completed_id,
            ..
        })) = completion_rx.recv().await
        {
            assert_eq!(completed_id, agent_id);
        } else {
            panic!("Expected AgentCompleted event");
        }
    }
}
