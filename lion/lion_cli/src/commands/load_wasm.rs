//! Command to load a plugin from a WebAssembly file.

use crate::error::CliError;
use crate::system::LionSystem;
use colored::Colorize;
use std::path::PathBuf;

/// Execute the load_wasm command
pub fn execute(
    system: &LionSystem,
    wasm_file: PathBuf,
    name: String,
    capabilities: Option<String>,
) -> Result<(), CliError> {
    println!("Loading WebAssembly plugin from file: {}", wasm_file.display().to_string().cyan());
    println!("Plugin name: {}", name.cyan());
    
    if let Some(caps) = &capabilities {
        println!("Requested capabilities: {}", caps.cyan());
    } else {
        println!("Requested capabilities: {}", "default (InterPluginComm)".cyan());
    }
    
    // Load the plugin
    let plugin_id = system.load_wasm_plugin(&wasm_file, &name, capabilities.as_deref())?;
    
    println!("Plugin loaded successfully with ID: {}", plugin_id.0.to_string().green());
    
    // Get the plugin
    let plugin = system.plugin_manager().get_plugin(plugin_id)
        .ok_or_else(|| CliError::Other("Plugin loaded but not found in manager".to_string()))?;
    
    println!("Plugin state: {}", plugin.state().to_string().yellow());
    
    // Initialize the plugin
    println!("Initializing plugin...");
    let mut plugin_clone = plugin.clone();
    plugin_clone.initialize()?;
    println!("Plugin initialized successfully. State: {}", plugin_clone.state().to_string().green());
    
    Ok(())
}