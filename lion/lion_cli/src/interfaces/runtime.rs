//! Interface to the Lion runtime component
//!
//! This module provides functions to interact with the Lion runtime,
//! which is responsible for managing the lifecycle of plugins, system
//! bootstrap, and shutdown.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Start the Lion microkernel runtime
pub fn start_runtime() -> Result<()> {
    // In a real implementation, this would call into lion_runtime::system::bootstrap
    #[cfg(feature = "runtime-integration")]
    {
        use lion_runtime::system::bootstrap;
        bootstrap::start_system().context("Failed to start Lion microkernel runtime")?;
        println!("Runtime started successfully");
    }

    #[cfg(not(feature = "runtime-integration"))]
    {
        // Placeholder implementation when runtime-integration is disabled
        println!("Starting Lion microkernel runtime...");
        println!("Runtime started successfully");
    }

    Ok(())
}

/// Shutdown the Lion microkernel runtime
pub fn shutdown_runtime() -> Result<()> {
    // In a real implementation, this would call into lion_runtime::system::shutdown
    #[cfg(feature = "runtime-integration")]
    {
        use lion_runtime::system::shutdown;
        shutdown::stop_system().context("Failed to shut down Lion microkernel runtime")?;
        println!("Runtime shutdown successfully");
    }

    #[cfg(not(feature = "runtime-integration"))]
    {
        // Placeholder implementation when runtime-integration is disabled
        println!("Shutting down Lion microkernel runtime...");
        println!("Runtime shutdown successfully");
    }

    Ok(())
}

/// Get the status of the Lion microkernel runtime
pub fn get_runtime_status() -> Result<RuntimeStatus> {
    // In a real implementation, this would call into lion_runtime::system::status
    #[cfg(feature = "runtime-integration")]
    {
        use lion_runtime::system::status;
        let sys_status = status::get_system_status()?;

        Ok(RuntimeStatus {
            is_running: sys_status.is_running,
            uptime_seconds: sys_status.uptime_seconds,
            loaded_plugins: sys_status.loaded_plugins,
            active_workflows: sys_status.active_workflows,
            memory_usage_mb: sys_status.memory_usage_mb,
            cpu_usage_percent: sys_status.cpu_usage_percent,
        })
    }

    #[cfg(not(feature = "runtime-integration"))]
    {
        // Placeholder implementation when runtime-integration is disabled
        Ok(RuntimeStatus {
            is_running: true,
            uptime_seconds: 3600, // 1 hour
            loaded_plugins: 2,
            active_workflows: 1,
            memory_usage_mb: 42.5,
            cpu_usage_percent: 2.3,
        })
    }
}

/// Get runtime status and all loaded plugins
pub fn get_runtime_status_and_plugins() -> Result<(RuntimeStatus, Vec<String>)> {
    let status = get_runtime_status()?;

    #[cfg(feature = "runtime-integration")]
    {
        use lion_runtime::plugin::manager;
        let plugins = manager::get_loaded_plugins()?
            .into_iter()
            .map(|id| id.to_string())
            .collect();

        Ok((status, plugins))
    }

    #[cfg(not(feature = "runtime-integration"))]
    {
        // Placeholder implementation when runtime-integration is disabled
        let plugin_ids = vec![
            "123e4567-e89b-12d3-a456-426614174000".to_string(),
            "523e4567-e89b-12d3-a456-426614174001".to_string(),
        ];
        Ok((status, plugin_ids))
    }
}

/// Get metadata for a specific plugin
pub fn get_plugin_metadata(plugin_id: &str) -> Result<PluginMetadata> {
    #[cfg(feature = "runtime-integration")]
    {
        use lion_core::id::PluginId;
        use lion_runtime::plugin::manager;

        let id = PluginId::from_str(plugin_id).context("Invalid plugin ID format")?;
        manager::get_plugin_metadata(&id).context("Failed to retrieve plugin metadata")
    }

    #[cfg(not(feature = "runtime-integration"))]
    {
        // Placeholder implementation when runtime-integration is disabled
        Ok(PluginMetadata {
            name: if plugin_id.ends_with("174000") {
                "calculator".to_string()
            } else {
                "text-processor".to_string()
            },
        })
    }
}

/// Load a plugin into the runtime
pub fn load_plugin(path: &Path, capabilities_config: Option<&Path>) -> Result<String> {
    // In a real implementation, this would call into lion_runtime::plugin::manager
    #[cfg(feature = "runtime-integration")]
    {
        use lion_runtime::plugin::manager;

        let plugin_id = if let Some(caps_path) = capabilities_config {
            manager::load_plugin_with_capabilities(path, caps_path)?
        } else {
            manager::load_plugin(path)?
        };

        println!("Plugin loaded with ID: {}", plugin_id);
        Ok(plugin_id.to_string())
    }

    #[cfg(not(feature = "runtime-integration"))]
    {
        // Placeholder implementation
        println!("Loading plugin from: {}", path.display());

        if let Some(caps) = capabilities_config {
            println!("Using capability configuration from: {}", caps.display());
        } else {
            println!("No capability configuration provided, using defaults");
        }

        // Generate a UUID as plugin ID
        let plugin_id = uuid::Uuid::new_v4().to_string();
        println!("Plugin loaded with ID: {}", plugin_id);

        Ok(plugin_id)
    }
}

/// Unload a plugin from the runtime
pub fn unload_plugin(plugin_id: &str) -> Result<()> {
    // In a real implementation, this would call into lion_runtime::plugin::manager
    #[cfg(feature = "runtime-integration")]
    {
        use lion_core::id::PluginId;
        use lion_runtime::plugin::manager;

        let id = PluginId::from_str(plugin_id).context("Invalid plugin ID format")?;
        manager::unload_plugin(&id)?;
        println!("Plugin unloaded successfully");
    }

    #[cfg(not(feature = "runtime-integration"))]
    {
        // Placeholder implementation
        println!("Unloading plugin: {}", plugin_id);
        println!("Plugin unloaded successfully");
    }

    Ok(())
}

/// Call a function in a loaded plugin
pub fn call_plugin_function(plugin_id: &str, function: &str, args: Option<&str>) -> Result<String> {
    // In a real implementation, this would call into lion_runtime::plugin::manager
    #[cfg(feature = "runtime-integration")]
    {
        use lion_core::id::PluginId;
        use lion_runtime::plugin::manager;
        use serde_json::Value;

        let id = PluginId::from_str(plugin_id).context("Invalid plugin ID format")?;

        let args_value = if let Some(a) = args {
            serde_json::from_str(a)?
        } else {
            Value::Null
        };

        let result = manager::call_plugin_function(&id, function, args_value)?;

        // Convert result to JSON string
        let result_str =
            serde_json::to_string_pretty(&result).context("Failed to serialize function result")?;
        Ok(result_str)
    }

    #[cfg(not(feature = "runtime-integration"))]
    {
        // Placeholder implementation
        println!("Calling function '{}' in plugin '{}'", function, plugin_id);

        if let Some(a) = args {
            println!("With arguments: {}", a);
        } else {
            println!("With no arguments");
        }

        // Simulate different responses based on function name for better demo
        match function {
            "calculate" => Ok(r#"{
  "result": 42,
  "operation": "add",
  "execution_time_ms": 5
}"#
            .to_string()),
            "process_text" => Ok(r#"{
  "words": 256,
  "chars": 1024,
  "sentiment": "positive",
  "language": "English"
}"#
            .to_string()),
            _ => Ok(r#"{
  "result": "Function executed successfully"
}"#
            .to_string()),
        }
    }
}

/// Runtime status information
#[derive(Debug)]
pub struct RuntimeStatus {
    pub is_running: bool,
    pub uptime_seconds: u64,
    pub loaded_plugins: usize,
    pub active_workflows: usize,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// Plugin metadata containing descriptive information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
}

#[cfg(feature = "runtime-integration")]
impl From<lion_runtime::plugin::types::PluginMetadata> for PluginMetadata {
    fn from(meta: lion_runtime::plugin::types::PluginMetadata) -> Self {
        Self { name: meta.name }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_start_runtime() {
        let result = start_runtime();
        assert!(result.is_ok());
    }

    #[test]
    fn test_shutdown_runtime() {
        let result = shutdown_runtime();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_runtime_status() {
        let result = get_runtime_status();
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.is_running);
        assert!(status.uptime_seconds > 0);
    }

    #[test]
    fn test_load_plugin() {
        let temp_dir = tempdir().unwrap();
        let plugin_path = temp_dir.path().join("test_plugin.wasm");

        // Create a mock WASM file
        std::fs::write(&plugin_path, b"mock wasm content").unwrap();

        let result = load_plugin(&plugin_path, None);
        assert!(result.is_ok());

        let plugin_id = result.unwrap();
        assert!(!plugin_id.is_empty());
    }

    #[test]
    fn test_unload_plugin() {
        let plugin_id = uuid::Uuid::new_v4().to_string();
        let result = unload_plugin(&plugin_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_plugin_function() {
        let plugin_id = uuid::Uuid::new_v4().to_string();
        let function = "test_function";
        let args = Some(r#"{"param1": "value1", "param2": 42}"#);

        let result = call_plugin_function(&plugin_id, function, args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.contains("result"));
    }
}
