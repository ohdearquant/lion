use lion_core::orchestrator::{
    events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent},
    metadata::EventMetadata,
};
use tracing::{debug, error, info};
use uuid::Uuid;

pub fn log_event(event: &SystemEvent) {
    match event {
        SystemEvent::Task(task_event) => match task_event {
            TaskEvent::Submitted {
                task_id,
                payload,
                metadata,
            } => {
                info!(
                    task_id = %task_id,
                    correlation_id = ?metadata.correlation_id,
                    "Task submitted: {}", payload
                );
            }
            TaskEvent::Completed {
                task_id,
                result,
                metadata,
            } => {
                info!(
                    task_id = %task_id,
                    correlation_id = ?metadata.correlation_id,
                    "Task completed: {}", result
                );
            }
            TaskEvent::Error {
                task_id,
                error,
                metadata,
            } => {
                error!(
                    task_id = %task_id,
                    correlation_id = ?metadata.correlation_id,
                    "Task error: {}", error
                );
            }
        },
        SystemEvent::Plugin(plugin_event) => match plugin_event {
            PluginEvent::Load {
                plugin_id,
                manifest,
                manifest_path,
                metadata,
            } => {
                info!(
                    plugin_id = %plugin_id,
                    correlation_id = ?metadata.correlation_id,
                    manifest_path = ?manifest_path,
                    "Loading plugin: {}", manifest.name
                );
            }
            PluginEvent::Invoked {
                plugin_id,
                input,
                metadata,
            } => {
                info!(
                    plugin_id = %plugin_id,
                    correlation_id = ?metadata.correlation_id,
                    "Plugin invoked: {}", input
                );
            }
            PluginEvent::Result {
                plugin_id,
                result,
                metadata,
            } => {
                info!(
                    plugin_id = %plugin_id,
                    correlation_id = ?metadata.correlation_id,
                    "Plugin result: {}", result
                );
            }
            PluginEvent::Error {
                plugin_id,
                error,
                metadata,
            } => {
                error!(
                    plugin_id = %plugin_id,
                    correlation_id = ?metadata.correlation_id,
                    "Plugin error: {}", error
                );
            }
            PluginEvent::List => {
                debug!("Listing plugins");
            }
        },
        SystemEvent::Agent(agent_event) => match agent_event {
            AgentEvent::Spawned {
                agent_id,
                prompt,
                metadata,
            } => {
                info!(
                    agent_id = %agent_id,
                    correlation_id = ?metadata.correlation_id,
                    "Agent spawned with prompt: {}", prompt
                );
            }
            AgentEvent::PartialOutput {
                agent_id,
                output,
                metadata,
            } => {
                debug!(
                    agent_id = %agent_id,
                    correlation_id = ?metadata.correlation_id,
                    "Agent partial output: {}", output
                );
            }
            AgentEvent::Completed {
                agent_id,
                result,
                metadata,
            } => {
                info!(
                    agent_id = %agent_id,
                    correlation_id = ?metadata.correlation_id,
                    "Agent completed: {}", result
                );
            }
            AgentEvent::Error {
                agent_id,
                error,
                metadata,
            } => {
                error!(
                    agent_id = %agent_id,
                    correlation_id = ?metadata.correlation_id,
                    "Agent error: {}", error
                );
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt::format::FmtSpan;

    fn init_test_logging() {
        let _ = tracing_subscriber::fmt()
            .with_span_events(FmtSpan::CLOSE)
            .with_thread_ids(true)
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            .try_init();
    }

    #[test]
    fn test_log_task_events() {
        init_test_logging();
        let task_id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());
        let metadata = EventMetadata::new(correlation_id);

        // Test TaskSubmitted
        log_event(&SystemEvent::Task(TaskEvent::Submitted {
            task_id,
            payload: "test task".into(),
            metadata: metadata.clone(),
        }));

        // Test TaskCompleted
        log_event(&SystemEvent::Task(TaskEvent::Completed {
            task_id,
            result: "test result".into(),
            metadata: metadata.clone(),
        }));

        // Test TaskError
        log_event(&SystemEvent::Task(TaskEvent::Error {
            task_id,
            error: "test error".into(),
            metadata,
        }));
    }

    #[test]
    fn test_log_plugin_events() {
        init_test_logging();
        let plugin_id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());
        let metadata = EventMetadata::new(correlation_id);

        // Test PluginInvoked
        log_event(&SystemEvent::Plugin(PluginEvent::Invoked {
            plugin_id,
            input: "test input".into(),
            metadata: metadata.clone(),
        }));

        // Test PluginResult
        log_event(&SystemEvent::Plugin(PluginEvent::Result {
            plugin_id,
            result: "test result".into(),
            metadata: metadata.clone(),
        }));

        // Test PluginError
        log_event(&SystemEvent::Plugin(PluginEvent::Error {
            plugin_id,
            error: "test error".into(),
            metadata,
        }));
    }

    #[test]
    fn test_log_agent_events() {
        init_test_logging();
        let agent_id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());
        let metadata = EventMetadata::new(correlation_id);

        // Test AgentSpawned
        log_event(&SystemEvent::Agent(AgentEvent::Spawned {
            agent_id,
            prompt: "test prompt".into(),
            metadata: metadata.clone(),
        }));

        // Test AgentPartialOutput
        log_event(&SystemEvent::Agent(AgentEvent::PartialOutput {
            agent_id,
            output: "partial output".into(),
            metadata: metadata.clone(),
        }));

        // Test AgentCompleted
        log_event(&SystemEvent::Agent(AgentEvent::Completed {
            agent_id,
            result: "test result".into(),
            metadata: metadata.clone(),
        }));

        // Test AgentError
        log_event(&SystemEvent::Agent(AgentEvent::Error {
            agent_id,
            error: "test error".into(),
            metadata,
        }));
    }
}
