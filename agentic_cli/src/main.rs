use agentic_core::orchestrator::{Orchestrator, SystemEvent};
use clap::{Parser, Subcommand};
use std::time::Duration;
use tokio::time::timeout;
use tracing_subscriber::fmt::format::FmtSpan;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "lion-cli", version = "0.0.1a")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Submit a task to the orchestrator
    SubmitTask {
        /// The task data/payload
        #[arg(long)]
        data: String,

        /// Optional correlation ID for tracking related tasks
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

    // Create orchestrator with a channel capacity of 100
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn the orchestrator's event loop
    tokio::spawn(orchestrator.run());

    match cli.command {
        Commands::SubmitTask {
            data,
            correlation_id,
        } => {
            let correlation_uuid = correlation_id
                .map(|id| Uuid::parse_str(&id))
                .transpose()
                .expect("Invalid correlation ID format");

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
        }
    }
}
