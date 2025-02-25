//! Command to show the capabilities granted to a plugin.

use crate::error::CliError;
use crate::formatter::{add_capability_row, create_capability_table, format_plugin_id};
use crate::system::LionSystem;
use colored::Colorize;

/// Execute the capabilities command
pub fn execute(system: &LionSystem, id: &str) -> Result<(), CliError> {
    // Get the plugin
    let plugin = system.get_plugin(id)?;
    
    println!("Capabilities for plugin: {} ({})", plugin.name().cyan(), format_plugin_id(plugin.id()).cyan());
    
    // Get the capabilities
    let capabilities = system.get_capabilities(id)?;
    
    if capabilities.is_empty() {
        println!("{}", "No capabilities granted.".yellow());
        return Ok(());
    }
    
    println!("{} capability/capabilities granted:", capabilities.len().to_string().cyan());
    
    // Create a table
    let mut table = create_capability_table();
    
    // Add rows for each capability
    for capability in capabilities {
        add_capability_row(&mut table, &capability);
    }
    
    // Print the table
    println!("{}", table);
    
    Ok(())
}
