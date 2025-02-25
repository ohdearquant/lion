//! Command to unload a plugin.

use crate::error::CliError;
use crate::system::LionSystem;
use colored::Colorize;

/// Execute the unload command
pub fn execute(system: &LionSystem, id: &str) -> Result<(), CliError> {
    // Get the plugin name before unloading
    let name = system.get_plugin(id).map(|p| p.name().to_string()).unwrap_or_default();
    
    println!("Unloading plugin: {} ({})", name.cyan(), id.cyan());
    
    // Unload the plugin
    system.unload_plugin(id)?;
    
    println!("{}", "Plugin unloaded successfully.".green());
    
    Ok(())
}
