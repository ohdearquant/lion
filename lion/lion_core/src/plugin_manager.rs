use crate::element::ElementData;
use crate::storage::FileStorage;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
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
    #[error("Failed to read manifest: {0}")]
    ManifestError(String),
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

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        debug!("Creating new PluginManager with no manifest directory");
        let storage = Arc::new(FileStorage::new("plugins/data"));
        Self {
            manifest_dir: None,
            storage,
        }
    }

    pub fn with_manifest_dir<P: AsRef<Path>>(manifest_dir: P) -> Self {
        let dir = manifest_dir.as_ref().to_path_buf();
        debug!(
            "Creating new PluginManager with manifest directory: {:?}",
            dir
        );

        // Create storage path relative to manifest directory
        let storage_path = if dir.ends_with("plugins") {
            dir.join("data")
        } else {
            dir.join("plugins").join("data")
        };

        debug!("Using storage path: {:?}", storage_path);
        let storage = Arc::new(FileStorage::new(storage_path));

        Self {
            manifest_dir: Some(dir),
            storage,
        }
    }

    pub fn discover_plugins(&self) -> Result<Vec<PluginManifest>, PluginError> {
        let manifest_dir = self.manifest_dir.as_ref().ok_or_else(|| {
            PluginError::ManifestError("No manifest directory configured".to_string())
        })?;

        debug!("Discovering plugins in directory: {:?}", manifest_dir);
        let mut manifests = Vec::new();

        if !manifest_dir.exists() {
            debug!(
                "Manifest directory does not exist: {}",
                manifest_dir.display()
            );
            return Err(PluginError::ManifestError(format!(
                "Manifest directory does not exist: {}",
                manifest_dir.display()
            )));
        }

        for entry in fs::read_dir(manifest_dir).map_err(|e| {
            PluginError::ManifestError(format!("Failed to read manifest directory: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                PluginError::ManifestError(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();
            debug!("Checking path: {:?}", path);

            if path.is_dir() {
                let manifest_path = path.join("manifest.toml");
                debug!("Looking for manifest at: {:?}", manifest_path);
                if manifest_path.exists() {
                    debug!("Found manifest file: {:?}", manifest_path);
                    let content = fs::read_to_string(&manifest_path).map_err(|e| {
                        PluginError::ManifestError(format!("Failed to read manifest file: {}", e))
                    })?;
                    debug!("Manifest content: {}", content);
                    let manifest: PluginManifest = toml::from_str(&content).map_err(|e| {
                        PluginError::ManifestError(format!("Failed to parse manifest: {}", e))
                    })?;
                    debug!("Successfully parsed manifest for plugin: {}", manifest.name);
                    manifests.push(manifest);
                }
            }
        }

        debug!("Discovered {} plugins", manifests.len());
        Ok(manifests)
    }

    fn resolve_entry_point(&self, entry_point: &str) -> PathBuf {
        // If entry_point is absolute, use it directly
        let path = PathBuf::from(entry_point);
        if path.is_absolute() {
            debug!("Using absolute entry point: {:?}", path);
            return path;
        }

        // Get the manifest directory or current directory as base
        let base_dir = self.manifest_dir.as_ref().map_or_else(
            || std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            |dir| dir.clone(),
        );
        debug!("Using base directory: {:?}", base_dir);

        // If it starts with ../ or ./, resolve relative to base directory
        if entry_point.starts_with("../") || entry_point.starts_with("./") {
            // Split the path into components
            let components: Vec<&str> = entry_point.split('/').collect();
            let mut current_path = base_dir;

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

        // Otherwise, resolve relative to base directory
        let resolved = base_dir.join(entry_point);
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
            .ok_or(PluginError::NotFound(plugin_id))?;

        let manifest: PluginManifest = serde_json::from_value(
            plugin
                .metadata
                .get("manifest")
                .ok_or_else(|| PluginError::InvokeError("Invalid plugin metadata".to_string()))?
                .clone(),
        )
        .map_err(|e| PluginError::InvokeError(format!("Failed to parse manifest: {}", e)))?;

        debug!("Retrieved plugin manifest: {:?}", manifest);

        let entry_point = self.resolve_entry_point(&manifest.entry_point);
        debug!("Resolved entry point for execution: {:?}", entry_point);

        // Execute the plugin as a subprocess
        let mut child = Command::new(&entry_point)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| PluginError::ProcessError(format!("Failed to spawn process: {}", e)))?;

        // Write input to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).map_err(|e| {
                PluginError::ProcessError(format!("Failed to write to stdin: {}", e))
            })?;
        }

        // Read output from stdout
        let output = child
            .wait_with_output()
            .map_err(|e| PluginError::ProcessError(format!("Failed to read output: {}", e)))?;

        if !output.status.success() {
            return Err(PluginError::ProcessError(format!(
                "Plugin process exited with status: {}",
                output.status
            )));
        }

        let result = String::from_utf8(output.stdout)
            .map_err(|e| PluginError::ProcessError(format!("Invalid UTF-8 output: {}", e)))?;

        Ok(result)
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

    pub fn clear(&self) {
        self.storage.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
    fn test_discover_plugins() {
        let temp_dir = tempdir().unwrap();
        let plugin_dir = temp_dir.path().join("plugin1");
        fs::create_dir(&plugin_dir).unwrap();

        let manifest = create_test_manifest();
        let manifest_content = toml::to_string(&manifest).unwrap();
        fs::write(plugin_dir.join("manifest.toml"), manifest_content).unwrap();

        let manager = PluginManager::with_manifest_dir(temp_dir.path());
        let discovered = manager.discover_plugins().unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].name, "test_plugin");
    }

    #[test]
    fn test_load_plugin_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let manager = PluginManager::with_manifest_dir(temp_dir.path());
        let manifest = create_test_manifest();
        let result = manager.load_plugin(manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_not_found() {
        let temp_dir = tempdir().unwrap();
        let manager = PluginManager::with_manifest_dir(temp_dir.path());
        let id = Uuid::new_v4();
        let result = manager.invoke_plugin(id, "test");
        assert!(matches!(result, Err(PluginError::NotFound(_))));
    }

    #[test]
    fn test_list_plugins() {
        let temp_dir = tempdir().unwrap();
        let manager = PluginManager::with_manifest_dir(temp_dir.path());
        manager.clear(); // Clear any existing plugins
        assert!(manager.list_plugins().is_empty());
    }
}
