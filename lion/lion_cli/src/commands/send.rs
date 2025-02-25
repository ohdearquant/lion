//! Command to send a message to a plugin.

use crate::error::CliError;
use crate::formatter::format_json;
use crate::system::LionSystem;
use colored::Colorize;

/// Execute the send command
pub fn execute(system: &LionSystem, id: &str, message: &str) -> Result<(), CliError> {
    // Parse the message as JSON to validate it
    let parsed: serde_json::Value = serde_json::from_str(message)?;
    
    // Get the plugin name
    let name = system.get_plugin(id)?.name().to_string();
    
    println!("Sending message to plugin: {} ({})", name.cyan(), id.cyan());
    println!("Message: {}", format_json(&parsed).cyan());
    
    // Send the message
    let response = system.send_message(id, message)?;
    
    // Print the response
    if let Some(response_value) = response {
        println!("Response received:");
        println!("{}", format_json(&response_value).green());
    } else {
        println!("{}", "No response received.".yellow());
    }
    
    Ok(())
}
