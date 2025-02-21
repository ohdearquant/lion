use super::events::{EventMetadata, SystemEvent};
use crate::agent::{AgentEvent, AgentProtocol, MockStreamingAgent};
use crate::event_log::EventLog;
use crate::plugin_manager::PluginManager;
use chrono::Utc;
use tracing::info;
use uuid::Uuid;

/// Handles the processing of system events
pub struct EventProcessor {
    pub(super) event_log: EventLog,
    pub(super) plugin_manager: PluginManager,
}

impl EventProcessor {
    pub fn new(event_log: EventLog, plugin_manager: PluginManager) -> Self {
        Self {
            event_log,
            plugin_manager,
        }
    }

    /// Process a single event, returning a completion event if successful
    pub async fn process_event(&self, event: SystemEvent) -> Option<SystemEvent> {
        // Log the incoming event
        self.event_log.append(event.clone());

        match event {
            SystemEvent::TaskSubmitted {
                task_id,
                payload,
                metadata,
            } => self.process_task(task_id, payload, metadata),

            SystemEvent::PluginInvoked {
                plugin_id,
                input,
                metadata,
            } => self.process_plugin(plugin_id, input, metadata),

            SystemEvent::AgentSpawned {
                agent_id,
                prompt,
                metadata,
            } => self.process_agent(agent_id, prompt, metadata).await,

            // These events don't require processing
            SystemEvent::TaskCompleted { .. }
            | SystemEvent::TaskError { .. }
            | SystemEvent::PluginResult { .. }
            | SystemEvent::PluginError { .. }
            | SystemEvent::AgentPartialOutput { .. }
            | SystemEvent::AgentCompleted { .. }
            | SystemEvent::AgentError { .. } => None,
        }
    }

    fn process_task(
        &self,
        task_id: Uuid,
        payload: String,
        metadata: EventMetadata,
    ) -> Option<SystemEvent> {
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

    fn process_plugin(
        &self,
        plugin_id: Uuid,
        input: String,
        metadata: EventMetadata,
    ) -> Option<SystemEvent> {
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
                    error: e.to_string(), // PluginError already formats errors consistently
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

    async fn process_agent(
        &self,
        agent_id: Uuid,
        prompt: String,
        metadata: EventMetadata,
    ) -> Option<SystemEvent> {
        info!(
            agent_id = %agent_id,
            correlation_id = ?metadata.correlation_id,
            "Agent spawned"
        );

        // Create a mock streaming agent
        let mut agent = MockStreamingAgent::new(agent_id);

        // Start the agent and get initial event
        let start_evt = AgentEvent::Start {
            agent_id,
            prompt: prompt.clone(),
        };

        let mut current_evt = agent.on_event(start_evt);
        let mut final_result = None;

        // Process all agent events
        while let Some(evt) = current_evt {
            match evt {
                AgentEvent::PartialOutput { agent_id, chunk } => {
                    // Log partial output
                    let partial = SystemEvent::AgentPartialOutput {
                        agent_id,
                        chunk: chunk.clone(),
                        metadata: EventMetadata {
                            event_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            correlation_id: metadata.correlation_id,
                            context: metadata.context.clone(),
                        },
                    };
                    self.event_log.append(partial);

                    // Get next event
                    current_evt = agent.on_event(AgentEvent::PartialOutput { agent_id, chunk });
                }
                AgentEvent::Done { final_output, .. } => {
                    final_result = Some(final_output);
                    current_evt = None;
                }
                AgentEvent::Error { error, .. } => {
                    let error_evt = SystemEvent::AgentError {
                        agent_id,
                        error,
                        metadata: EventMetadata {
                            event_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            correlation_id: metadata.correlation_id,
                            context: metadata.context,
                        },
                    };
                    self.event_log.append(error_evt.clone());
                    return Some(error_evt);
                }
                _ => current_evt = None,
            }
        }

        // Create completion event
        if let Some(result) = final_result {
            let completion = SystemEvent::AgentCompleted {
                agent_id,
                result,
                metadata: EventMetadata {
                    event_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    correlation_id: metadata.correlation_id,
                    context: metadata.context,
                },
            };
            self.event_log.append(completion.clone());
            Some(completion)
        } else {
            let error_evt = SystemEvent::AgentError {
                agent_id,
                error: "Agent failed to produce final output".into(),
                metadata: EventMetadata {
                    event_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    correlation_id: metadata.correlation_id,
                    context: metadata.context,
                },
            };
            self.event_log.append(error_evt.clone());
            Some(error_evt)
        }
    }
}
