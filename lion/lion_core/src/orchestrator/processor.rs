use crate::orchestrator::{
    events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent},
    handlers::EventHandler,
    types::*,
};
use crate::plugin_manager::PluginManifest;
use crate::types::traits::{LanguageMessage, LanguageMessageType, ParticipantState};
use crate::collections::{Pile, Progression};
use crate::events::sse::{NetworkEventSender, NetworkEvent};
use tokio::sync::{broadcast, mpsc, RwLock, Mutex};
use tokio::time::{Duration, interval};
use tracing::{debug, info, warn, error};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;
use thiserror::Error;

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
    active_agents: usize,
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
    handler: EventHandler,
    config: OrchestratorConfig,
    
    // Concurrent state management
    active_agents: Arc<RwLock<HashMap<Uuid, ParticipantState>>>,
    agent_timeouts: Arc<RwLock<HashMap<Uuid, tokio::time::Instant>>>,
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

        Self {
            event_tx: tx,
            event_rx: Arc::new(Mutex::new(rx)),
            completion_tx,
            network_tx,
            handler: EventHandler::new(),
            config,
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            agent_timeouts: Arc::new(RwLock::new(HashMap::new())),
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

    /// Process a single event, returning a completion event if successful
    async fn process_event(&mut self, event: SystemEvent) -> Result<Option<SystemEvent>, OrchestratorError> {
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
                // Record in progression
                self.task_progression.push(task_id).map_err(|e| 
                    OrchestratorError::SchedulingError(e.to_string()))?;

                // Delegate to handler
                if let Some(result) = self.handler.handle_task(TaskEvent::Submitted {
                    task_id,
                    payload,
                    metadata,
                }) {
                    let mut metrics = self.metrics.write().await;
                    metrics.completed_tasks += 1;
                    Ok(Some(result))
                } else {
                    Ok(None)
                }
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
            _ => Ok(self.handler.handle_task(event)),
        }
    }

    /// Process a plugin event
    async fn process_plugin(&mut self, event: PluginEvent) -> Result<Option<SystemEvent>, OrchestratorError> {
        match event {
            PluginEvent::Load { plugin_id, manifest, manifest_path, metadata } => {
                // Notify network about plugin load
                self.network_tx.send_plugin_event(
                    plugin_id,
                    "load".to_string(),
                    serde_json::json!({
                        "manifest": manifest.clone(),
                        "path": manifest_path
                    }),
                ).map_err(|e| OrchestratorError::ChannelError(e.to_string()))?;

                Ok(self.handler.handle_plugin(PluginEvent::Load {
                    plugin_id,
                    manifest,
                    manifest_path,
                    metadata,
                }).await)
            },
            _ => Ok(self.handler.handle_plugin(event).await),
        }
    }

    /// Process an agent event
    async fn process_agent(&mut self, event: AgentEvent) -> Result<Option<SystemEvent>, OrchestratorError> {
        match event {
            AgentEvent::Spawned { agent_id, prompt, metadata } => {
                // Check concurrent agent limit
                let active_count = self.active_agents.read().await.len();
                if active_count >= self.config.max_concurrent_agents {
                    return Err(OrchestratorError::SchedulingError(
                        "Maximum concurrent agent limit reached".to_string()
                    ));
                }

                // Register agent
                self.active_agents.write().await.insert(agent_id, ParticipantState::Initializing);
                self.agent_timeouts.write().await.insert(agent_id, tokio::time::Instant::now());

                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.active_agents += 1;

                // Notify network
                self.network_tx.send_agent_status(agent_id, "spawned".to_string())
                    .map_err(|e| OrchestratorError::ChannelError(e.to_string()))?;

                Ok(self.handler.handle_agent(AgentEvent::Spawned {
                    agent_id,
                    prompt,
                    metadata,
                }))
            },
            AgentEvent::PartialOutput { agent_id, output, metadata } => {
                // Update agent timeout
                if let Some(timeout) = self.agent_timeouts.write().await.get_mut(&agent_id) {
                    *timeout = tokio::time::Instant::now();
                }

                // Send partial output to network
                self.network_tx.send_partial_output(
                    agent_id,
                    output.clone(),
                    Uuid::new_v4(), // message ID for this chunk
                    0, // sequence number
                ).map_err(|e| OrchestratorError::ChannelError(e.to_string()))?;

                Ok(self.handler.handle_agent(AgentEvent::PartialOutput {
                    agent_id,
                    output,
                    metadata,
                }))
            },
            AgentEvent::Completed { agent_id, result, metadata } => {
                // Update state
                self.active_agents.write().await.remove(&agent_id);
                self.agent_timeouts.write().await.remove(&agent_id);

                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.active_agents -= 1;
                metrics.completed_tasks += 1;

                // Notify network
                self.network_tx.send_agent_status(agent_id, "completed".to_string())
                    .map_err(|e| OrchestratorError::ChannelError(e.to_string()))?;

                Ok(self.handler.handle_agent(AgentEvent::Completed {
                    agent_id,
                    result,
                    metadata,
                }))
            },
            AgentEvent::Error { agent_id, error, metadata } => {
                // Update state
                self.active_agents.write().await.remove(&agent_id);
                self.agent_timeouts.write().await.remove(&agent_id);

                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.active_agents -= 1;
                metrics.failed_tasks += 1;

                // Notify network
                self.network_tx.send_agent_status(agent_id, "error".to_string())
                    .map_err(|e| OrchestratorError::ChannelError(e.to_string()))?;

                Ok(self.handler.handle_agent(AgentEvent::Error {
                    agent_id,
                    error,
                    metadata,
                }))
            },
        }
    }

    /// Monitor agent timeouts
    async fn monitor_timeouts(&self) {
        let mut interval = interval(Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            let mut timeouts = self.agent_timeouts.write().await;
            let mut active_agents = self.active_agents.write().await;
            
            let now = tokio::time::Instant::now();
            let mut timed_out = Vec::new();
            
            for (&agent_id, &last_active) in timeouts.iter() {
                if now.duration_since(last_active) > self.config.agent_timeout {
                    timed_out.push(agent_id);
                }
            }
            
            for agent_id in timed_out {
                warn!("Agent {} timed out", agent_id);
                timeouts.remove(&agent_id);
                active_agents.remove(&agent_id);
                
                // Notify about timeout
                if let Err(e) = self.network_tx.send_agent_status(agent_id, "timeout".to_string()) {
                    error!("Failed to send timeout notification: {}", e);
                }
            }
        }
    }

    /// Run the orchestrator's event loop
    pub async fn run(mut self) {
        info!("Starting orchestrator event loop");

        // Spawn timeout monitor
        let timeout_monitor = self.clone();
        tokio::spawn(async move {
            timeout_monitor.monitor_timeouts().await;
        });

        let mut rx = self.event_rx.lock().await;
        while let Some(event) = rx.recv().await {
            match self.process_event(event).await {
                Ok(Some(completion_event)) => {
                    // Broadcast the completion event
                    let _ = self.completion_tx.send(completion_event);
                }
                Ok(None) => {}
                Err(e) => {
                    error!("Error processing event: {}", e);
                    let mut metrics = self.metrics.write().await;
                    metrics.failed_tasks += 1;
                }
            }
        }

        info!("Orchestrator shutting down");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::metadata::EventMetadata;

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

            let result = sender.send(event).await;
            if i < 2 {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        }
    }
}
