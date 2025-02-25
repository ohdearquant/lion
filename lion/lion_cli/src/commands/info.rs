//! Command to show detailed information about a plugin.

use crate::error::CliError;
use crate::formatter::{format_plugin_id, format_plugin_state, format_resource_usage};
use crate::system::LionSystem;
use colored::Colorize;

/// Execute the info command
pub fn execute(system: &LionSystem, id: &str) -> Result<(), CliError> {
    // Get the plugin
    let plugin = system.get_plugin(id)?;
    
    println!("Plugin information:");
    println!("ID: {}", format_plugin_id(plugin.id()).cyan());
    println!("Name: {}", plugin.name().cyan());
    println!("State: {}", format_plugin_state(plugin.state()));
    
    // Get the manifest
    if let Ok(manifest) = system.get_manifest(id) {
        println!("Version: {}", manifest.version.cyan());
        
        if let Some(description) = manifest.description {
            println!("Description: {}", description.cyan());
        }
        
        if let Some(author) = manifest.author {
            println!("Author: {}", author.cyan());
        }
        
        println!("Requested capabilities:");
        for capability in &manifest.requested_capabilities {
            println!("  - {}", format!("{:?}", capability).cyan());
        }
        
        println!("Source: {}", format!("{:?}", manifest.source).cyan());
    }
    
    // Get resource usage
    if let Ok(usage) = system.get_resource_usage(id) {
        println!("Resource usage:");
        println!("{}", format_resource_usage(&usage).cyan());
    }
    
    Ok(())
}
