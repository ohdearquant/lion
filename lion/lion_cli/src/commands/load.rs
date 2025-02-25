//! Command to load a plugin from a manifest file.

use crate::error::CliError;
use crate::system::LionSystem;
use colored::Colorize;
use std::path::PathBuf;

/// Execute the load command
pub fn execute(system: &LionSystem, manifest_path: PathBuf) -> Result<(), CliError> {
    println!("Loading plugin from manifest: {}", manifest_path.display().to_string().cyan());
    
    // Load the plugin
    let plugin_id = system.load_plugin(&manifest_path)?;
    
    println!("Plugin loaded successfully with ID: {}", plugin_id.0.to_string().green());
    
    // Get the plugin
    let plugin = system.plugin_manager().get_plugin(plugin_id)
        .ok_or_else(|| CliError::Other("Plugin loaded but not found in manager".to_string()))?;
    
    println!("Plugin name: {}", plugin.name().cyan());
    println!("Plugin state: {}", plugin.state().to_string().yellow());
    
    // Initialize the plugin
    println!("Initializing plugin...");
    let mut plugin_clone = plugin.clone();
    plugin_clone.initialize()?;
    println!("Plugin initialized successfully. State: {}", plugin_clone.state().to_string().green());
    
    Ok(())
}
