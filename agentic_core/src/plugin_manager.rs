use crate::element::ElementData;
use crate::storage::FileStorage;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(Uuid),
    #[error("Failed to load plugin: {0}")]
    LoadError(String),
    #[error("Failed to invoke plugin: {0}")]
    InvokeError(String),
    #[error("Plugin process error: {0}")]
    ProcessError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub entry_point: String,
    pub permissions: Vec<String>,
    pub driver: Option<String>,
    pub functions: std::collections::HashMap<String, PluginFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFunction {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct PluginManager {
    manifest_dir: Option<PathBuf>,
    storage: Arc<FileStorage>,
}

impl PluginManager {
    pub fn new() -> Self {
        debug!("Creating new PluginManager with no manifest directory");
        Self {
            manifest_dir: None,
            storage: Arc::new(FileStorage::new("plugins/data")),
        }
    }

    pub fn with_manifest_dir<P: AsRef<Path>>(manifest_dir: P) -> Self {
        let dir = manifest_dir.as_ref().to_path_buf();
        debug!(
            "Creating new PluginManager with manifest directory: {:?}",
            dir
        );
        Self {
            manifest_dir: Some(dir),
            storage: Arc::new(FileStorage::new("plugins/data")),
        }
    }

    fn resolve_entry_point(&self, entry_point: &str) -> PathBuf {
        // If entry_point is absolute, use it directly
        let path = PathBuf::from(entry_point);
        if path.is_absolute() {
            debug!("Using absolute entry point: {:?}", path);
            return path;
        }

        // Get the current working directory
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        debug!("Current working directory: {:?}", cwd);

        // If it starts with ../ or ./, resolve relative to current directory
        if entry_point.starts_with("../") || entry_point.starts_with("./") {
            // Split the path into components
            let components: Vec<&str> = entry_point.split('/').collect();
            let mut current_path = cwd.clone();

            // Handle .. components by going up directories
            for component in components {
                match component {
                    ".." => {
                        current_path = current_path.parent().unwrap_or(&current_path).to_path_buf();
                    }
                    "." => {}
                    "" => {}
                    _ => {
                        current_path = current_path.join(component);
                    }
                }
            }

            debug!(
                "Resolved relative entry point {:?} to {:?}",
                entry_point, current_path
            );
            return current_path;
        }

        // Otherwise, resolve relative to current directory
        let resolved = cwd.join(entry_point);
        debug!("Resolved entry point {:?} to {:?}", entry_point, resolved);
        resolved
    }

    pub fn load_plugin(&self, manifest: PluginManifest) -> Result<Uuid, PluginError> {
        debug!("Loading plugin {}", manifest.name);

        let entry_point = self.resolve_entry_point(&manifest.entry_point);
        debug!("Resolved entry point: {:?}", entry_point);

        // Check if entry point exists
        if !entry_point.exists() {
            error!("Entry point not found: {:?}", entry_point);
            return Err(PluginError::LoadError(format!(
                "Entry point not found: {}",
                entry_point.display()
            )));
        }

        debug!("Entry point exists checking if it's executable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = entry_point
                .metadata()
                .map_err(|e| PluginError::LoadError(format!("Failed to get metadata: {}", e)))?;
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            debug!("File permissions: {:o}", mode);
            if mode & 0o111 == 0 {
                error!("Entry point is not executable: {:?}", entry_point);
                return Err(PluginError::LoadError(format!(
                    "Entry point is not executable: {}",
                    entry_point.display()
                )));
            }
        }

        // Create an ElementData for this plugin
        let metadata = json!({
            "type": "plugin",
            "manifest": manifest,
            "status": "loaded"
        });
        let element = ElementData::new(metadata);
        let id = element.id;

        // Store the plugin in our storage
        self.storage.store(element);
        info!(
            "Plugin {} loaded successfully with ID {}",
            manifest.name, id
        );

        Ok(id)
    }

    pub fn invoke_plugin(&self, plugin_id: Uuid, input: &str) -> Result<String, PluginError> {
        debug!("Invoking plugin {} with input: {}", plugin_id, input);

        let plugin = self
            .storage
            .get(&plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id))?;

        let manifest: PluginManifest = serde_json::from_value(
            plugin
                .metadata
                .get("manifest")
                .ok_or_else(|| PluginError::InvokeError("Invalid plugin metadata".to_string()))?
                .clone(),
        )
        .map_err(|e| PluginError::InvokeError(format!("Failed to parse manifest: {}", e)))?;

        debug!("Retrieved plugin manifest: {:?}", manifest);

        // Parse input JSON
        let input_json: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| PluginError::InvokeError(format!("Invalid input JSON: {}", e)))?;

        debug!("Parsed input JSON: {:?}", input_json);

        // Get function name from input
        let function_name = input_json
            .get("function")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                PluginError::InvokeError("Missing 'function' field in input".to_string())
            })?;

        debug!("Function name from input: {}", function_name);

        // Check if function exists in manifest
        let function = manifest.functions.get(function_name).ok_or_else(|| {
            PluginError::InvokeError(format!("Function '{}' not found in plugin", function_name))
        })?;

        debug!("Found function in manifest: {:?}", function);

        let entry_point = self.resolve_entry_point(&manifest.entry_point);
        debug!("Resolved entry point: {:?}", entry_point);

        // Execute the plugin with the input via stdin
        let mut child = Command::new(&entry_point)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PluginError::ProcessError(format!("Failed to execute plugin: {}", e)))?;

        // Write input to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).map_err(|e| {
                PluginError::ProcessError(format!("Failed to write to plugin stdin: {}", e))
            })?;
            stdin.write_all(b"\n").map_err(|e| {
                PluginError::ProcessError(format!("Failed to write newline to plugin stdin: {}", e))
            })?;
        }

        // Get output
        let output = child.wait_with_output().map_err(|e| {
            PluginError::ProcessError(format!("Failed to get plugin output: {}", e))
        })?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            error!("Plugin execution failed: {}", error);
            return Err(PluginError::ProcessError(format!(
                "Plugin execution failed: {}",
                error
            )));
        }

        let output_str = String::from_utf8_lossy(&output.stdout).to_string();
        debug!("Plugin execution succeeded with output: {}", output_str);

        Ok(output_str)
    }

    pub fn list_plugins(&self) -> Vec<(Uuid, PluginManifest)> {
        debug!("Listing all plugins");
        let elements = self.storage.list();
        debug!("Found {} elements in storage", elements.len());

        elements
            .into_iter()
            .filter_map(|element| {
                if let Some(manifest) = element.metadata.get("manifest") {
                    match serde_json::from_value(manifest.clone()) {
                        Ok(manifest) => {
                            debug!("Successfully parsed manifest for plugin {}", element.id);
                            Some((element.id, manifest))
                        }
                        Err(e) => {
                            error!("Failed to parse manifest for plugin {}: {}", element.id, e);
                            None
                        }
                    }
                } else {
                    debug!("Element {} is not a plugin (no manifest)", element.id);
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_manifest() -> PluginManifest {
        PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: "nonexistent".to_string(),
            permissions: vec![],
            driver: None,
            functions: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_load_plugin_nonexistent() {
        let manager = PluginManager::new();
        let manifest = create_test_manifest();
        let result = manager.load_plugin(manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_not_found() {
        let manager = PluginManager::new();
        let id = Uuid::new_v4();
        let result = manager.invoke_plugin(id, "test");
        assert!(matches!(result, Err(PluginError::NotFound(_))));
    }

    #[test]
    fn test_list_plugins() {
        let manager = PluginManager::new();
        assert!(manager.list_plugins().is_empty());
    }
}
