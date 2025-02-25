//! Lion CLI - Command-line interface for the Lion WebAssembly Plugin System.

mod commands;
mod error;
mod formatter;
mod system;

use clap::{command, ArgAction, Parser, Subcommand};
use colored::Colorize;
use error::CliError;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lion")]
#[command(author = "Lion Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Command-line interface for the Lion WebAssembly Plugin System", long_about = None)]
struct Cli {
    /// Sets the level of verbosity
    #[arg(short, long, action = ArgAction::Count)]
    verbose: u8,

    /// Configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Load a plugin from a manifest file
    Load {
        /// Path to the manifest file
        #[arg(value_name = "FILE")]
        manifest: PathBuf,
    },
    
    /// Load a plugin from a WebAssembly file
    LoadWasm {
        /// Path to the WebAssembly file
        #[arg(value_name = "FILE")]
        wasm_file: PathBuf,
        
        /// Plugin name
        #[arg(short, long)]
        name: String,
        
        /// Capabilities to grant (comma-separated)
        #[arg(short, long)]
        capabilities: Option<String>,
    },
    
    /// List all loaded plugins
    List,
    
    /// Show detailed information about a plugin
    Info {
        /// Plugin ID
        #[arg(value_name = "ID")]
        id: String,
    },
    
    /// Unload a plugin
    Unload {
        /// Plugin ID
        #[arg(value_name = "ID")]
        id: String,
    },
    
    /// Send a message to a plugin
    Send {
        /// Plugin ID
        #[arg(value_name = "ID")]
        id: String,
        
        /// Message content (JSON)
        #[arg(value_name = "MESSAGE")]
        message: String,
    },
    
    /// Create a plugin chain
    Chain {
        /// Comma-separated list of plugin IDs in the chain
        #[arg(value_name = "IDS")]
        ids: String,
        
        /// Input message for the first plugin (JSON)
        #[arg(short, long)]
        input: Option<String>,
    },
    
    /// Show the capabilities granted to a plugin
    Capabilities {
        /// Plugin ID
        #[arg(value_name = "ID")]
        id: String,
    },
    
    /// Show resource usage for a plugin
    Resources {
        /// Plugin ID
        #[arg(value_name = "ID")]
        id: String,
    },
    
    /// Run a demo with example plugins
    Demo {
        /// Name of the demo to run
        #[arg(value_name = "NAME")]
        name: Option<String>,
    },
}

fn main() -> Result<(), CliError> {
    // Parse command-line arguments
    let cli = Cli::parse();
    
    // Set up logging
    let log_level = match cli.verbose {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    
    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp(None)
        .init();
    
    // Initialize the system
    let system = system::LionSystem::initialize(cli.config)?;
    
    // Execute the command
    match cli.command {
        Commands::Load { manifest } => {
            commands::load::execute(&system, manifest)?;
        }
        Commands::LoadWasm { wasm_file, name, capabilities } => {
            commands::load_wasm::execute(&system, wasm_file, name, capabilities)?;
        }
        Commands::List => {
            commands::list::execute(&system)?;
        }
        Commands::Info { id } => {
            commands::info::execute(&system, &id)?;
        }
        Commands::Unload { id } => {
            commands::unload::execute(&system, &id)?;
        }
        Commands::Send { id, message } => {
            commands::send::execute(&system, &id, &message)?;
        }
        Commands::Chain { ids, input } => {
            commands::chain::execute(&system, &ids, input.as_deref())?;
        }
        Commands::Capabilities { id } => {
            commands::capabilities::execute(&system, &id)?;
        }
        Commands::Resources { id } => {
            commands::resources::execute(&system, &id)?;
        }
        Commands::Demo { name } => {
            commands::demo::execute(&system, name.as_deref())?;
        }
    }
    
    Ok(())
}
