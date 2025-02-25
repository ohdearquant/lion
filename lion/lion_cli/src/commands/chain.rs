//! Command to create and execute a plugin chain.

use crate::error::CliError;
use crate::formatter::{format_json, format_plugin_id};
use crate::system::LionSystem;
use colored::Colorize;

/// Execute the chain command
pub fn execute(system: &LionSystem, ids: &str, input: Option<&str>) -> Result<(), CliError> {
    // Create the chain
    let chain = system.create_chain(ids)?;
    
    println!("Created plugin chain with {} plugin(s):", chain.len().to_string().cyan());
    
    // Print the chain
    for (i, plugin_id) in chain.iter().enumerate() {
        let plugin = system.plugin_manager().get_plugin(*plugin_id)
            .ok_or_else(|| CliError::InvalidPluginId(plugin_id.0.to_string()))?;
        
        println!("  {}. {} ({})", (i + 1).to_string().yellow(), plugin.name().cyan(), format_plugin_id(*plugin_id).cyan());
    }
    
    // Parse input if provided
    let input_value = if let Some(input_str) = input {
        let parsed: serde_json::Value = serde_json::from_str(input_str)?;
        println!("Input message: {}", format_json(&parsed).cyan());
        Some(parsed)
    } else {
        println!("No input message provided. Using empty object.");
        Some(serde_json::json!({}))
    };
    
    // Execute the chain
    println!("Executing chain...");
    let result = system.execute_chain(&chain, input)?;
    
    // Print the result
    if let Some(result_value) = result {
        println!("Chain execution completed. Final result:");
        println!("{}", format_json(&result_value).green());
    } else {
        println!("Chain execution completed with no result.");
    }
    
    Ok(())
}
