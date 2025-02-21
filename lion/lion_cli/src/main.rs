use clap::{Parser, Subcommand};
use lion_core::plugin_manager::PluginManifest;
use lion_core::{
    orchestrator::{Orchestrator, SystemEvent},
    EventLog,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing_subscriber::fmt::format::FmtSpan;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "lion-cli", version = "0.0.1")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run CI checks
    Ci,
    /// Run CLI tests
    TestCli,
    /// Submit a task and show event log
    Demo {
        /// The task data/payload
        #[arg(long)]
        data: String,

        /// Optional correlation ID for tracking related tasks
        #[arg(long)]
        correlation_id: Option<String>,
    },
    /// Load a plugin from a manifest file
    LoadPlugin {
        /// Path to the plugin manifest file
        #[arg(long)]
        manifest: String,
    },
    /// Invoke a loaded plugin
    InvokePlugin {
        /// Plugin ID (UUID)
        #[arg(long)]
        plugin_id: String,
        /// Input data for the plugin
        #[arg(long)]
        input: String,
        /// Optional correlation ID
        #[arg(long)]
        correlation_id: Option<String>,
    },
    /// Spawn an agent with a prompt
    SpawnAgent {
        /// The prompt for the agent
        #[arg(long)]
        prompt: String,
        /// Optional correlation ID
        #[arg(long)]
        correlation_id: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_thread_ids(true)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Ci => {
            let status = std::process::Command::new("./scripts/ci.sh")
                .status()
                .expect("Failed to execute CI script");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Commands::TestCli => {
            let status = std::process::Command::new("./scripts/test_cli.sh")
                .status()
                .expect("Failed to execute test script");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Commands::SpawnAgent {
            prompt,
            correlation_id,
        } => {
            // Create orchestrator with a channel capacity of 100
            let orchestrator = Orchestrator::new(100);
            let sender = orchestrator.sender();
            let mut completion_rx = orchestrator.completion_receiver();

            // Get a clone of the event log before moving orchestrator
            let event_log = Arc::new(orchestrator.event_log().clone());

            // Spawn the orchestrator's event loop
            tokio::spawn(orchestrator.run());

            let correlation_uuid = correlation_id
                .map(|id| Uuid::parse_str(&id))
                .transpose()
                .expect("Invalid correlation ID format");

            println!("\n=== Spawning Agent ===");
            let event = SystemEvent::new_agent(prompt, correlation_uuid);

            // Extract agent_id before moving event
            let agent_id = match &event {
                SystemEvent::AgentSpawned { agent_id, .. } => *agent_id,
                _ => panic!("Unexpected event type"),
            };

            // Send the event
            if let Err(e) = sender.send(event).await {
                eprintln!("Failed to spawn agent: {}", e);
                std::process::exit(1);
            }

            println!("Agent spawned successfully with ID: {}", agent_id);

            // Wait for completion with timeout
            match timeout(Duration::from_secs(5), completion_rx.recv()).await {
                Ok(Ok(completion)) => match completion {
                    SystemEvent::AgentCompleted { result, .. } => {
                        println!("Agent completed successfully!");
                        println!("Result: {}", result);
                    }
                    SystemEvent::AgentError { error, .. } => {
                        eprintln!("Agent failed: {}", error);
                        std::process::exit(1);
                    }
                    _ => {}
                },
                Ok(Err(e)) => {
                    eprintln!("Error receiving completion: {}", e);
                    std::process::exit(1);
                }
                Err(_) => {
                    eprintln!("Timeout waiting for agent completion");
                    std::process::exit(1);
                }
            }

            print_event_log(&event_log).await;
        }
        Commands::Demo {
            data,
            correlation_id,
        } => {
            // Create orchestrator with a channel capacity of 100
            let orchestrator = Orchestrator::new(100);
            let sender = orchestrator.sender();
            let mut completion_rx = orchestrator.completion_receiver();

            // Get a clone of the event log before moving orchestrator
            let event_log = Arc::new(orchestrator.event_log().clone());

            // Spawn the orchestrator's event loop
            tokio::spawn(orchestrator.run());

            let correlation_uuid = correlation_id
                .map(|id| Uuid::parse_str(&id))
                .transpose()
                .expect("Invalid correlation ID format");

            println!("\n=== Submitting Task ===");
            let event = SystemEvent::new_task(data, correlation_uuid);

            // Extract task_id before moving event
            let task_id = match &event {
                SystemEvent::TaskSubmitted { task_id, .. } => *task_id,
                _ => panic!("Unexpected event type"),
            };

            // Send the event
            if let Err(e) = sender.send(event).await {
                eprintln!("Failed to submit task: {}", e);
                std::process::exit(1);
            }

            println!("Task submitted successfully with ID: {}", task_id);

            // Wait for completion with timeout
            match timeout(Duration::from_secs(5), completion_rx.recv()).await {
                Ok(Ok(completion)) => match completion {
                    SystemEvent::TaskCompleted { result, .. } => {
                        println!("Task completed successfully!");
                        println!("Result: {}", result);
                    }
                    SystemEvent::TaskError { error, .. } => {
                        eprintln!("Task failed: {}", error);
                        std::process::exit(1);
                    }
                    _ => {}
                },
                Ok(Err(e)) => {
                    eprintln!("Error receiving completion: {}", e);
                    std::process::exit(1);
                }
                Err(_) => {
                    eprintln!("Timeout waiting for task completion");
                    std::process::exit(1);
                }
            }

            print_event_log(&event_log).await;
        }
        Commands::LoadPlugin { manifest } => {
            let mut orchestrator = Orchestrator::new(100);

            // Read and parse the manifest file
            let manifest_content =
                std::fs::read_to_string(manifest).expect("Failed to read manifest file");
            let manifest: PluginManifest =
                toml::from_str(&manifest_content).expect("Failed to parse manifest");

            // Load the plugin
            match orchestrator.plugin_manager().load_plugin(manifest) {
                Ok(plugin_id) => {
                    println!("Plugin loaded successfully!");
                    println!("Plugin ID: {}", plugin_id);
                }
                Err(e) => {
                    eprintln!("Failed to load plugin: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::InvokePlugin {
            plugin_id,
            input,
            correlation_id,
        } => {
            let orchestrator = Orchestrator::new(100);
            let sender = orchestrator.sender();
            let mut completion_rx = orchestrator.completion_receiver();
            let event_log = Arc::new(orchestrator.event_log().clone());

            // Spawn the orchestrator
            tokio::spawn(orchestrator.run());

            let plugin_uuid = Uuid::parse_str(&plugin_id).expect("Invalid plugin ID format");
            let correlation_uuid = correlation_id
                .map(|id| Uuid::parse_str(&id))
                .transpose()
                .expect("Invalid correlation ID format");

            println!("\n=== Invoking Plugin ===");
            let event = SystemEvent::new_plugin_invocation(plugin_uuid, input, correlation_uuid);

            // Send the event
            if let Err(e) = sender.send(event).await {
                eprintln!("Failed to invoke plugin: {}", e);
                std::process::exit(1);
            }

            println!("Plugin invocation sent successfully");

            // Wait for completion with timeout
            match timeout(Duration::from_secs(5), completion_rx.recv()).await {
                Ok(Ok(completion)) => match completion {
                    SystemEvent::PluginResult { output, .. } => {
                        println!("Plugin completed successfully!");
                        println!("Output: {}", output);
                    }
                    SystemEvent::PluginError { error, .. } => {
                        eprintln!("Plugin failed: {}", error);
                        std::process::exit(1);
                    }
                    _ => {}
                },
                Ok(Err(e)) => {
                    eprintln!("Error receiving completion: {}", e);
                    std::process::exit(1);
                }
                Err(_) => {
                    eprintln!("Timeout waiting for plugin completion");
                    std::process::exit(1);
                }
            }

            print_event_log(&event_log).await;
        }
    }
}

async fn print_event_log(event_log: &EventLog) {
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
