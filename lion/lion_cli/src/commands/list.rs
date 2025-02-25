//! Command to list all loaded plugins.

use crate::error::CliError;
use crate::formatter::{add_plugin_row, create_plugin_table};
use crate::system::LionSystem;
use colored::Colorize;

/// Execute the list command
pub fn execute(system: &LionSystem) -> Result<(), CliError> {
    // Get all plugins
    let plugins = system.list_plugins();
    
    if plugins.is_empty() {
        println!("{}", "No plugins loaded.".yellow());
        return Ok(());
    }
    
    println!("{} plugin(s) loaded:", plugins.len().to_string().cyan());
    
    // Create a table
    let mut table = create_plugin_table();
    
    // Add rows for each plugin
    for (plugin_id, name, state) in plugins {
        // Get resource usage if available
        let (memory, cpu) = match system.resource_monitor().get_usage(plugin_id) {
            Ok(usage) => (usage.memory_bytes, usage.cpu_usage),
            Err(_) => (0, 0.0),
        };
        
        add_plugin_row(&mut table, plugin_id, &name, state, memory, cpu);
    }
    
    // Print the table
    println!("{}", table);
    
    Ok(())
}
