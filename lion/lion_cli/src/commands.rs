use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lion-cli", version = "0.0.1")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
