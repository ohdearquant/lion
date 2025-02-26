//! Lion CLI - Command-line interface for the Lion runtime
//!
//! This provides a command-line interface for interacting with the
//! Lion runtime, including loading plugins, calling functions, and
//! managing workflows.

use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};
use runtime::{Runtime, RuntimeConfig};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
    
    /// Manage workflows
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },
    
    /// Initialize a new configuration file
    Init {
        /// Path to the configuration file
        #[arg(value_name = "FILE")]
        config: PathBuf,
    },
}

#[derive(Subcommand)]
enum PluginCommands {
    /// Load a plugin
    Load {
        /// Path to the plugin file
        #[arg(value_name = "FILE")]
        path: PathBuf,
        
        /// Name of the plugin
        #[arg(short, long)]
        name: String,
        
        /// Version of the plugin
        #[arg(short, long, default_value = "1.0.0")]
        version: String,
        
        /// Description of the plugin
        #[arg(short, long, default_value = "")]
        description: String,
    },
    
    /// Call a function in a plugin
    Call {
        /// Plugin ID
        #[arg(value_name = "PLUGIN_ID")]
        plugin_id: String,
        
        /// Function name
        #[arg(value_name = "FUNCTION")]
        function: String,
        
        /// Parameters (as JSON)
        #[arg(value_name = "PARAMS")]
        params: Option<String>,
    },
    
    /// List loaded plugins
    List,
    
    /// Unload a plugin
    Unload {
        /// Plugin ID
        #[arg(value_name = "PLUGIN_ID")]
        plugin_id: String,
    },
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Create a workflow
    Create {
        /// Path to the workflow definition file
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
    
    /// Execute a workflow
    Execute {
        /// Workflow ID
        #[arg(value_name = "WORKFLOW_ID")]
        workflow_id: String,
        
        /// Input parameters (as JSON)
        #[arg(value_name = "PARAMS")]
        params: Option<String>,
    },
    
    /// List workflows
    List,
    
    /// Get workflow status
    Status {
        /// Workflow ID
        #[arg(value_name = "WORKFLOW_ID")]
        workflow_id: String,
    },
}

fn main() -> core::error::Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse();
    
    // Load configuration
    let config = match &cli.config {
        Some(path) => runtime::load_config(path)?,
        None => RuntimeConfig::default(),
    };
    
    // Create the runtime
    let runtime = Runtime::new(config)?;
    
    // Execute the command
    match cli.command {
        Commands::Plugin { command } => handle_plugin_command(&runtime, command)?,
        Commands::Workflow { command } => handle_workflow_command(&runtime, command)?,
        Commands::Init { config } => init_config(&config)?,
    }
    
    // Shut down the runtime
    runtime.shutdown()?;
    
    Ok(())
}

/// Handle plugin commands.
fn handle_plugin_command(runtime: &Runtime, command: PluginCommands) -> core::error::Result<()> {
    match command {
        PluginCommands::Load { path, name, version, description } => {
            let plugin_id = runtime.load_plugin_from_file(
                &path,
                &name,
                &version,
                &description,
            )?;
            
            println!("Plugin loaded with ID: {}", plugin_id);
        },
        
        PluginCommands::Call { plugin_id, function, params } => {
            // Parse plugin ID
            let plugin_id = plugin_id.parse::<uuid::Uuid>()
                .map_err(|_| core::error::PluginError::NotFound(core::types::PluginId(uuid::Uuid::nil())))?;
            
            let plugin_id = core::types::PluginId(plugin_id);
            
            // Parse parameters
            let params_bytes = match params {
                Some(params) => params.into_bytes(),
                None => b"{}".to_vec(),
            };
            
            // Call the function
            let result = runtime.plugin_manager().call_function(
                &plugin_id,
                &function,
                &params_bytes,
            )?;
            
            // Print the result
            let result_str = String::from_utf8_lossy(&result);
            println!("Result: {}", result_str);
        },
        
        PluginCommands::List => {
            let plugins = runtime.plugin_manager().list_plugins();
            
            println!("Loaded plugins:");
            for plugin in plugins {
                println!("  - {} ({}): {} [{}]", plugin.name, plugin.version, plugin.id, plugin.state);
            }
        },
        
        PluginCommands::Unload { plugin_id } => {
            // Parse plugin ID
            let plugin_id = plugin_id.parse::<uuid::Uuid>()
                .map_err(|_| core::error::PluginError::NotFound(core::types::PluginId(uuid::Uuid::nil())))?;
            
            let plugin_id = core::types::PluginId(plugin_id);
            
            // Unload the plugin
            runtime.plugin_manager().unload_plugin(&plugin_id)?;
            
            println!("Plugin unloaded: {}", plugin_id);
        },
    }
    
    Ok(())
}

/// Handle workflow commands.
fn handle_workflow_command(runtime: &Runtime, command: WorkflowCommands) -> core::error::Result<()> {
    match command {
        WorkflowCommands::Create { path } => {
            // Load the workflow definition
            let content = std::fs::read_to_string(&path)?;
            let workflow: workflow::Workflow = serde_json::from_str(&content)?;
            
            // Create the workflow
            let workflow_id = runtime.workflow_manager().create_workflow(workflow)?;
            
            println!("Workflow created with ID: {}", workflow_id);
        },
        
        WorkflowCommands::Execute { workflow_id, params } => {
            // Parse workflow ID
            let workflow_id = workflow_id.parse::<uuid::Uuid>()
                .map_err(|_| core::error::WorkflowError::WorkflowNotFound(workflow_id))?;
            
            let workflow_id = workflow::WorkflowId(workflow_id);
            
            // Parse parameters
            let params_json = match params {
                Some(params) => serde_json::from_str(&params)?,
                None => serde_json::json!({}),
            };
            
            // Execute the workflow
            let execution_id = runtime.workflow_manager().execute_workflow(
                &workflow_id,
                params_json,
                workflow::ExecutionOptions::default(),
            )?;
            
            println!("Workflow execution started with ID: {}", execution_id);
        },
        
        WorkflowCommands::List => {
            let workflows = runtime.workflow_manager().list_workflows();
            
            println!("Available workflows:");
            for workflow in workflows {
                println!("  - {} ({}): {}", workflow.name, workflow.version, workflow.id);
            }
        },
        
        WorkflowCommands::Status { workflow_id } => {
            // Parse workflow ID
            let workflow_id = workflow_id.parse::<uuid::Uuid>()
                .map_err(|_| core::error::WorkflowError::WorkflowNotFound(workflow_id))?;
            
            let workflow_id = workflow::WorkflowId(workflow_id);
            
            // Get the workflow
            let workflow = runtime.workflow_manager().get_workflow(&workflow_id)?;
            
            println!("Workflow: {} ({})", workflow.name, workflow.id);
            println!("Version: {}", workflow.version);
            println!("Description: {}", workflow.description);
            println!("Nodes: {}", workflow.nodes.len());
        },
    }
    
    Ok(())
}

/// Initialize a new configuration file.
fn init_config(path: &Path) -> core::error::Result<()> {
    // Create a default configuration
    let config = RuntimeConfig::default();
    
    // Save the configuration
    runtime::save_config(&config, path)?;
    
    println!("Configuration initialized at: {}", path.display());
    
    Ok(())
}