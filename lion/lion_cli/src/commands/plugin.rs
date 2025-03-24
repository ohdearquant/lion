//! Plugin management commands
//!
//! This module contains commands for plugin management.
//! It is currently under development and not all features are implemented.

use super::interfaces::{capability, isolation, runtime};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::path::Path;

/// Load a WASM plugin from the specified path
pub fn load_plugin(path: &Path, caps_path: Option<&Path>) -> Result<String> {
    // Use the runtime interface to load the plugin
    runtime::load_plugin(path, caps_path)
        .context("Failed to load plugin")
        .map_err(|e| {
            println!("Error: {}", e);
            e
        })
}

/// List all loaded plugins
pub fn list_plugins() -> Result<()> {
    // Get runtime status to get loaded plugins
    let (status, plugin_ids) = runtime::get_runtime_status_and_plugins()?;

    println!("Listing all loaded plugins ({})", status.loaded_plugins);
    println!("ID                                    | Name           | Status  | Capabilities");
    println!("--------------------------------------|----------------|---------|-------------");

    // For each plugin, get its metadata and display information
    for plugin_id in &plugin_ids {
        // Get capabilities for this plugin
        let capabilities = capability::list_capabilities(&plugin_id)?;

        // Get resource usage for this plugin
        let usage = isolation::get_resource_usage(&plugin_id)?;

        // Get plugin metadata (includes name)
        let metadata = runtime::get_plugin_metadata(&plugin_id)?;

        // Display plugin information
        let status_str = if usage.cpu_percent > 0.1 {
            "RUNNING".green()
        } else if usage.cpu_percent > 0.0 {
            "IDLE".yellow()
        } else {
            "STOPPED".red()
        };

        // Display the first capability (if any)
        let cap_str = if let Some(cap) = capabilities.first() {
            cap.description.clone()
        } else {
            "none".dimmed().to_string()
        };

        println!(
            "{} | {:<14} | {:<7} | {}",
            plugin_id, metadata.name, status_str, cap_str
        );
    }

    Ok(())
}

/// Call a function in a loaded plugin
pub fn call_plugin(plugin_id: &str, function: &str, args: Option<&str>) -> Result<()> {
    // Use the runtime interface to call the plugin function
    let result = runtime::call_plugin_function(plugin_id, function, args)?;

    println!(
        "Function {} in plugin {} executed successfully",
        function.bright_green(),
        plugin_id.bright_blue()
    );

    // Try to pretty-print the result if it's valid JSON
    if let Ok(parsed) = serde_json::from_str::<Value>(&result) {
        if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
            println!("Result: \n{}", pretty);
        } else {
            println!("Result: {}", result);
        }
    } else {
        println!("Result: {}", result);
    }

    // If we got a significant result, suggest next steps
    if result.len() > 10 {
        println!("\n{}", "Suggested next steps:".bold());
        println!(
            "  - To check plugin status: {}",
            "lion-cli plugin list".italic()
        );
        println!(
            "  - To grant more capabilities: {}",
            format!(
                "lion-cli plugin grant-cap --plugin {} --cap-type [capability] --params [json]",
                plugin_id
            )
            .italic()
        );
    }

    Ok(())
}

/// Unload a plugin
pub fn unload_plugin(plugin_id: &str) -> Result<()> {
    // Use the runtime interface to unload the plugin
    runtime::unload_plugin(plugin_id)
}

/// Grant capabilities to a plugin
pub fn grant_capability(plugin_id: &str, cap_type: &str, params: &str) -> Result<()> {
    // Use the capability interface to grant the capability
    capability::grant_capability(plugin_id, cap_type, params)?;

    println!(
        "Capability {} granted to plugin {}",
        cap_type.bright_green(),
        plugin_id.bright_blue()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_plugin() {
        let path = PathBuf::from("/tmp/test_plugin.wasm");
        let result = load_plugin(&path, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_plugins() {
        let result = list_plugins();
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_plugin() {
        let plugin_id = "123e4567-e89b-12d3-a456-426614174000";
        let function = "calculate";
        let args = Some(r#"{"x": 5, "y": 3, "operation": "add"}"#);

        let result = call_plugin(plugin_id, function, args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_plugin_invalid_json() {
        let plugin_id = "123e4567-e89b-12d3-a456-426614174000";
        let function = "calculate";
        let args = Some(r#"{"x": 5, "y": 3, "operation": "add""#); // Invalid JSON

        let result = call_plugin(plugin_id, function, args);
        assert!(result.is_err());
    }

    #[test]
    fn test_unload_plugin() {
        let plugin_id = "123e4567-e89b-12d3-a456-426614174000";
        let result = unload_plugin(plugin_id);
        assert!(result.is_ok());
    }
}
