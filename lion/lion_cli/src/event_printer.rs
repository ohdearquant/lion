use lion_core::{orchestrator::SystemEvent, EventLog};
use std::time::Duration;

pub async fn print_event_log(event_log: &EventLog) {
    // Give more time for events to be processed
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("\n=== Event Log ===");
    let records = event_log.all();
    if records.is_empty() {
        println!("No events recorded.");
        return;
    }

    for (i, record) in records.iter().enumerate() {
        println!("{}. Event at {}:", i + 1, record.timestamp);
        match &record.event {
            SystemEvent::TaskSubmitted {
                task_id,
                payload,
                metadata,
            } => {
                println!("  Type: TaskSubmitted");
                println!("  Task ID: {}", task_id);
                println!("  Payload: {}", payload);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::TaskCompleted {
                task_id,
                result,
                metadata,
            } => {
                println!("  Type: TaskCompleted");
                println!("  Task ID: {}", task_id);
                println!("  Result: {}", result);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::TaskError {
                task_id,
                error,
                metadata,
            } => {
                println!("  Type: TaskError");
                println!("  Task ID: {}", task_id);
                println!("  Error: {}", error);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::PluginInvoked {
                plugin_id,
                input,
                metadata,
            } => {
                println!("  Type: PluginInvoked");
                println!("  Plugin ID: {}", plugin_id);
                println!("  Input: {}", input);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::PluginResult {
                plugin_id,
                output,
                metadata,
            } => {
                println!("  Type: PluginResult");
                println!("  Plugin ID: {}", plugin_id);
                println!("  Output: {}", output);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::PluginError {
                plugin_id,
                error,
                metadata,
            } => {
                println!("  Type: PluginError");
                println!("  Plugin ID: {}", plugin_id);
                println!("  Error: {}", error);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::PluginLoadRequested {
                plugin_id,
                manifest,
                metadata,
            } => {
                println!("  Type: PluginLoadRequested");
                println!("  Plugin ID: {}", plugin_id);
                println!("  Manifest: {}", manifest);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::PluginLoaded {
                plugin_id,
                name,
                version,
                description,
                metadata,
            } => {
                println!("  Type: PluginLoaded");
                println!("  Plugin ID: {}", plugin_id);
                println!("  Name: {}", name);
                println!("  Version: {}", version);
                println!("  Description: {}", description);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::AgentSpawned {
                agent_id,
                prompt,
                metadata,
            } => {
                println!("  Type: AgentSpawned");
                println!("  Agent ID: {}", agent_id);
                println!("  Prompt: {}", prompt);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::AgentPartialOutput {
                agent_id,
                chunk,
                metadata: _,
            } => {
                println!("  Type: AgentPartialOutput");
                println!("  Agent ID: {}", agent_id);
                println!("  Chunk: {}", chunk);
            }
            SystemEvent::AgentCompleted {
                agent_id,
                result,
                metadata,
            } => {
                println!("  Type: AgentCompleted");
                println!("  Agent ID: {}", agent_id);
                println!("  Result: {}", result);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
            SystemEvent::AgentError {
                agent_id,
                error,
                metadata,
            } => {
                println!("  Type: AgentError");
                println!("  Agent ID: {}", agent_id);
                println!("  Error: {}", error);
                if let Some(corr_id) = metadata.correlation_id {
                    println!("  Correlation ID: {}", corr_id);
                }
            }
        }
        println!();
    }

    println!("=== Event Replay Summary ===");
    let summary = event_log.replay_summary();
    println!("{}", summary);

    // Keep the program running for a moment to ensure all logs are flushed
    tokio::time::sleep(Duration::from_millis(100)).await;
}
