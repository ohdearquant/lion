mod commands;
mod handlers;
mod logging;

use clap::Parser;
use commands::{Cli, Commands};
use tracing_subscriber::fmt::format::FmtSpan;

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
            handlers::handle_ci();
        }
        Commands::TestCli => {
            handlers::handle_test_cli();
        }
        Commands::SpawnAgent {
            prompt,
            correlation_id,
        } => {
            handlers::handle_spawn_agent(prompt, correlation_id).await;
        }
        Commands::Demo {
            data,
            correlation_id,
        } => {
            handlers::handle_demo(data, correlation_id).await;
        }
        Commands::LoadPlugin { manifest } => {
            handlers::handle_load_plugin(manifest);
        }
        Commands::InvokePlugin {
            plugin_id,
            input,
            correlation_id,
        } => {
            handlers::handle_invoke_plugin(plugin_id, input, correlation_id).await;
        }
    }
}
