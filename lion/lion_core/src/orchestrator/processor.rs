use crate::{
    event_log::EventLog,
    events::sse::{NetworkEventSender, NetworkEvent},
    plugin_manager::{PluginManager, PluginManifest},
    types::traits::{LanguageMessage, LanguageMessageType},
    types::ParticipantState,
    collections::{Pile, Progression},
    orchestrator::agent_manager::AgentManager,
};
use super::{
    events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent},
    metadata::EventMetadata,
    types::*,
};
use tokio::sync::{broadcast, mpsc, RwLock, Mutex};
use tokio::time::{Duration, interval};
use tracing::{debug, info, warn, error};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use thiserror::Error;
use serde_json::json;

/// Errors that can occur in the orchestrator
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Channel error: {0}")]
    ChannelError(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(Uuid),
    #[error("Plugin not found: {0}")]
    PluginNotFound(Uuid),
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
    #[error("Scheduling error: {0}")]
    SchedulingError(String),
}

/// Configuration for the orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Maximum number of concurrent agents
    pub max_concurrent_agents: usize,
    /// Maximum time an agent can run
    pub agent_timeout: Duration,
    /// Channel capacity for events
    pub channel_capacity: usize,
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 10,
            agent_timeout: Duration::from_secs(300),
            channel_capacity: 1000,
            keep_alive_interval: Duration::from_secs(15),
        }
    }
}

/// Metrics for monitoring orchestrator performance
#[derive(Debug, Default)]
struct OrchestratorMetrics {
    completed_tasks: usize,
    failed_tasks: usize,
    messages_processed: usize,
    average_processing_time: f64,
}

/// The core orchestrator that processes system events and manages multi-agent concurrency
#[derive(Clone)]
pub struct Orchestrator {
    event_tx: EventSender,
    event_rx: Arc<Mutex<EventReceiver>>,
    completion_tx: CompletionSender,
    network_tx: NetworkEventSender,
    config: OrchestratorConfig,
    
    // Core components
    event_log: Arc<EventLog>,
    plugin_manager: Arc<PluginManager>,
    agent_manager: Arc<AgentManager>,
    
    // Message handling
    message_pile: Arc<Pile<LanguageMessage>>,
    task_progression: Arc<Progression>,
    
    // Metrics
    metrics: Arc<RwLock<OrchestratorMetrics>>,
}

impl Orchestrator {
    /// Create a new orchestrator instance with the specified configuration
    pub fn new(config: OrchestratorConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.channel_capacity);
        let (completion_tx, _) = broadcast::channel(config.channel_capacity);
        let network_tx = NetworkEventSender::new(config.channel_capacity);
        
        debug!("Creating new orchestrator with config: {:?}", config);

        let agent_manager = AgentManager::new(
            config.max_concurrent_agents,
            config.agent_timeout,
            network_tx.clone(),
        );

        Self {
            event_tx: tx,
            event_rx: Arc::new(Mutex::new(rx)),
            completion_tx,
            network_tx,
            config,
            event_log: Arc::new(EventLog::new()),
            plugin_manager: Arc::new(PluginManager::new()),
            agent_manager: Arc::new(agent_manager),
            message_pile: Arc::new(Pile::new()),
            task_progression: Arc::new(Progression::new()),
            metrics: Arc::new(RwLock::new(OrchestratorMetrics::default())),
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

    /// Get a sender for network events
    pub fn network_sender(&self) -> NetworkEventSender {
        self.network_tx.clone()
    }

    /// Store a language message in the message pile
    pub async fn store_message(&self, message: LanguageMessage) -> Result<(), OrchestratorError> {
        self.message_pile.insert(message.id, message)
            .map_err(|e| OrchestratorError::ChannelError(e.to_string()))
    }

    /// Retrieve a language message from the message pile
    pub async fn get_message(&self, message_id: Uuid) -> Result<Option<LanguageMessage>, OrchestratorError> {
        match self.message_pile.get(&message_id) {
            Ok(msg) => Ok(Some(msg)),
            Err(e) => Err(OrchestratorError::ChannelError(e.to_string()))
        }
    }

    /// Get all messages in order
    pub async fn get_messages(&self) -> Result<Vec<LanguageMessage>, OrchestratorError> {
        self.message_pile.get_ordered()
            .map_err(|e| OrchestratorError::ChannelError(e.to_string()))
    }

    /// Process a single event, returning a completion event if successful
    async fn process_event(&mut self, event: SystemEvent) -> Result<Option<SystemEvent>, OrchestratorError> {
        // Log the event
        self.event_log.append(event.clone());
        
        let start_time = tokio::time::Instant::now();
        
        let result = match event {
            SystemEvent::Task(task_event) => self.process_task(task_event).await?,
            SystemEvent::Plugin(plugin_event) => self.process_plugin(plugin_event).await?,
            SystemEvent::Agent(agent_event) => self.process_agent(agent_event).await?,
        };

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.messages_processed += 1;
        metrics.average_processing_time = (metrics.average_processing_time * (metrics.messages_processed - 1) as f64
            + start_time.elapsed().as_secs_f64()) / metrics.messages_processed as f64;

        Ok(result)
    }

    /// Process a task event
    async fn process_task(&mut self, event: TaskEvent) -> Result<Option<SystemEvent>, OrchestratorError> {
        match event {
            TaskEvent::Submitted { task_id, payload, metadata } => {
                // Record in progression with metadata
                self.task_progression.push(task_id, json!({
                    "payload": payload,
                    "timestamp": metadata.timestamp,
                })).map_err(|e| 
                    OrchestratorError::SchedulingError(e.to_string()))?;

                info!(task_id = %task_id, "Processing task: {}", payload);
                
                let mut metrics = self.metrics.write().await;
                metrics.completed_tasks += 1;
                
                Ok(Some(SystemEvent::Task(TaskEvent::Completed {
                    task_id,
                    result: format!("Processed: {}", payload),
                    metadata: EventMetadata::new(metadata.correlation_id),
                })))
            },
            TaskEvent::Error { task_id, error, metadata } => {
                let mut metrics = self.metrics.write().await;
                metrics.failed_tasks += 1;
                Ok(Some(SystemEvent::Task(TaskEvent::Error {
                    task_id,
                    error,
                    metadata,
                })))
            },
            TaskEvent::Completed { .. } => Ok(None),
        }
    }

    /// Process a plugin event
    async fn process_plugin(&mut self, event: PluginEvent) -> Result<Option<SystemEvent>, OrchestratorError> {
        match event {
            PluginEvent::Load { plugin_id, manifest, manifest_path, metadata } => {
                info!(plugin_id = %plugin_id, "Loading plugin: {}", manifest.name);

                // Notify network about plugin load
                self.network_tx.send_plugin_event(
                    plugin_id,
                    "load".to_string(),
                    serde_json::json!({
                        "manifest": manifest.clone(),
                        "path": manifest_path.clone()
                    }),
                ).map_err(|e| OrchestratorError::ChannelError(e.to_string()))?;

                match self.plugin_manager
                    .load_plugin_from_string(toml::to_string(&manifest).unwrap(), manifest_path)
                    .await
                {
                    Ok(_) => Ok(Some(SystemEvent::Plugin(PluginEvent::Result {
                        plugin_id,
                        result: format!("Plugin {} loaded successfully", manifest.name),
                        metadata: EventMetadata::new(metadata.correlation_id),
                    }))),
                    Err(e) => Ok(Some(SystemEvent::Plugin(PluginEvent::Error {
                        plugin_id,
                        error: e.to_string(),
                        metadata: EventMetadata::new(metadata.correlation_id),
                    }))),
                }
            },
            PluginEvent::Invoked { plugin_id, input, metadata } => {
                info!(plugin_id = %plugin_id, "Invoking plugin with input: {}", input);

                match self.plugin_manager.invoke_plugin(plugin_id, &input).await {
                    Ok(result) => Ok(Some(SystemEvent::Plugin(PluginEvent::Result {
                        plugin_id,
                        result,
                        metadata: EventMetadata::new(metadata.correlation_id),
                    }))),
                    Err(e) => Ok(Some(SystemEvent::Plugin(PluginEvent::Error {
                        plugin_id,
                        error: e.to_string(),
                        metadata: EventMetadata::new(metadata.correlation_id),
                    }))),
                }
            },
            PluginEvent::List => {
                let plugins = self.plugin_manager.list_plugins();
                let result = plugins
                    .iter()
                    .map(|p| format!("{} ({})", p.manifest.name, p.id))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(Some(SystemEvent::Plugin(PluginEvent::Result {
                    plugin_id: Uuid::new_v4(),
                    result,
                    metadata: EventMetadata::new(None),
                })))
            },
            PluginEvent::Result { .. } | PluginEvent::Error { .. } => Ok(None),
        }
    }

    /// Process an agent event
    async fn process_agent(&mut self, event: AgentEvent) -> Result<Option<SystemEvent>, OrchestratorError> {
        match event {
            AgentEvent::Spawned { agent_id, prompt, metadata } => {
                // Try to register the agent
                self.agent_manager.register_agent(agent_id).await?;

                info!(agent_id = %agent_id, "Processing agent prompt: {}", prompt);
                
                Ok(Some(SystemEvent::Agent(AgentEvent::Completed {
                    agent_id,
                    result: format!("Processed: {}", prompt),
                    metadata: EventMetadata::new(metadata.correlation_id),
                })))
            },
            AgentEvent::PartialOutput { agent_id, output, .. } => {
                // Update agent timeout
                self.agent_manager.update_agent_timeout(&agent_id).await;

                // Send partial output to network
                self.network_tx.send_partial_output(
                    agent_id,
                    output.clone(),
                    Uuid::new_v4(), // message ID for this chunk
                    0, // sequence number
                ).map_err(|e| OrchestratorError::ChannelError(e.to_string()))?;

                Ok(None)
            },
            AgentEvent::Completed { agent_id, result, metadata } => {
                // Remove agent and update metrics
                self.agent_manager.remove_agent(agent_id, "completed").await?;
                
                let mut metrics = self.metrics.write().await;
                metrics.completed_tasks += 1;

                Ok(Some(SystemEvent::Agent(AgentEvent::Completed {
                    agent_id,
                    result,
                    metadata,
                })))
            },
            AgentEvent::Error { agent_id, error, metadata } => {
                // Remove agent and update metrics
                self.agent_manager.remove_agent(agent_id, "error").await?;
                
                let mut metrics = self.metrics.write().await;
                metrics.failed_tasks += 1;

                Ok(Some(SystemEvent::Agent(AgentEvent::Error {
                    agent_id,
                    error,
                    metadata,
                })))
            },
        }
    }

    /// Run the orchestrator's event loop
    pub async fn run(self) {
        info!("Starting orchestrator event loop");

        // Spawn timeout monitor
        let agent_manager = self.agent_manager.clone();
        tokio::spawn(async move {
            agent_manager.monitor_timeouts().await;
        });

        // Process events
        while let Some(event) = self.event_rx.lock().await.recv().await {
            // Release the lock before processing
            let result = {
                let mut this = self.clone();
                this.process_event(event.clone()).await
            };
            
            match result {
                Ok(Some(completion_event)) => { let _ = self.completion_tx.send(completion_event); }
                Ok(None) => {}
                Err(e) => {
                    error!("Error processing event: {}", e);
                    
                    let mut metrics = self.metrics.write().await;
                    metrics.failed_tasks += 1;
                    
                    // Send error event back for agent events
                    if let SystemEvent::Agent(AgentEvent::Spawned { agent_id, metadata, .. }) = event {
                        let _ = self.completion_tx.send(SystemEvent::Agent(AgentEvent::Error {
                            agent_id,
                            error: e.to_string(),
                            metadata: EventMetadata::new(metadata.correlation_id),
                        }));
                    }
                }
            }
        }

        info!("Orchestrator shutting down");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_processes_task() {
        let config = OrchestratorConfig::default();
        let orchestrator = Orchestrator::new(config);
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
        let config = OrchestratorConfig::default();
        let orchestrator = Orchestrator::new(config);
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
        let config = OrchestratorConfig::default();
        let orchestrator = Orchestrator::new(config);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();
        let mut network_rx = orchestrator.network_sender().subscribe();

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

        // Wait for network event
        if let Ok(NetworkEvent::AgentStatus { agent_id: status_id, status, .. }) = network_rx.recv().await {
            assert_eq!(status_id, agent_id);
            assert_eq!(status, "spawned");
        } else {
            panic!("Expected AgentStatus event");
        }

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

    #[tokio::test]
    async fn test_orchestrator_concurrent_limit() {
        let config = OrchestratorConfig {
            max_concurrent_agents: 2,
            ..Default::default()
        };
        let orchestrator = Orchestrator::new(config);
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

        // Spawn the orchestrator
        tokio::spawn(orchestrator.run());

        // Try to spawn 3 agents (should fail on the third)
        for i in 0..3 {
            let agent_id = Uuid::new_v4();
            let event = SystemEvent::Agent(AgentEvent::Spawned {
                agent_id,
                prompt: format!("test prompt {}", i),
                metadata: EventMetadata::new(None),
            });

            sender.send(event).await.expect("Failed to send event");

            if i < 2 {
                // First two agents should complete successfully
                match completion_rx.recv().await {
                    Ok(SystemEvent::Agent(AgentEvent::Completed { .. })) => (),
                    _ => panic!("Expected agent completion"),
                }
            } else {
                // Third agent should fail with scheduling error
                match completion_rx.recv().await {
                    Ok(SystemEvent::Agent(AgentEvent::Error { error, .. })) => assert!(error.contains("Maximum concurrent agent limit")),
                    _ => panic!("Expected scheduling error"),
                }
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_message_pile() {
        let config = OrchestratorConfig::default();
        let orchestrator = Orchestrator::new(config);

        // Create test messages
        let message1 = LanguageMessage {
            id: Uuid::new_v4(),
            content: "test message 1".to_string(),
            sender_id: Uuid::new_v4(),
            recipient_ids: vec![Uuid::new_v4()].into_iter().collect(),
            message_type: LanguageMessageType::Text,
            metadata: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        };

        let message2 = LanguageMessage {
            id: Uuid::new_v4(),
            content: "test message 2".to_string(),
            sender_id: Uuid::new_v4(),
            recipient_ids: vec![Uuid::new_v4()].into_iter().collect(),
            message_type: LanguageMessageType::Text,
            metadata: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        };

        // Store messages
        orchestrator.store_message(message1.clone()).await.unwrap();
        orchestrator.store_message(message2.clone()).await.unwrap();

        // Retrieve individual messages
        let retrieved1 = orchestrator.get_message(message1.id).await.unwrap().unwrap();
        let retrieved2 = orchestrator.get_message(message2.id).await.unwrap().unwrap();
        assert_eq!(retrieved1.content, "test message 1");
        assert_eq!(retrieved2.content, "test message 2");

        // Get all messages in order
        let all_messages = orchestrator.get_messages().await.unwrap();
        assert_eq!(all_messages.len(), 2);
        assert_eq!(all_messages[0].content, "test message 1");
        assert_eq!(all_messages[1].content, "test message 2");
    }
}
