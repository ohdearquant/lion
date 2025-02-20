use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub entry_point: String,
    pub permissions: Vec<String>,
    /// A brief description of the plugin
    pub description: String,
    /// A map of function names to their descriptions
    pub functions: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PluginHandle {
    pub id: Uuid,
    pub manifest: PluginManifest,
    // For Phase 4, we'll keep this simple without actual WASM/process handles
    // In a future phase, we might add:
    // wasm_instance: Option<WasmInstance>,
    // process_handle: Option<Child>,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Failed to load plugin: {0}")]
    LoadFailure(String),
    #[error("Failed to invoke plugin: {0}")]
    InvokeFailure(String),
}

#[derive(Debug, Default)]
pub struct PluginManager {
    plugins: Arc<Mutex<HashMap<Uuid, PluginHandle>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn load_plugin(&self, manifest: PluginManifest) -> Result<Uuid, PluginError> {
        // Validate the manifest
        if manifest.name.is_empty() || manifest.version.is_empty() {
            return Err(PluginError::InvalidManifest(
                "Name or version is empty".into(),
            ));
        }

        // Basic permission check - deny "forbidden" permission
        if manifest.permissions.iter().any(|p| p == "forbidden") {
            return Err(PluginError::PermissionDenied(
                "Plugin requested forbidden permission".into(),
            ));
        }

        // Check if entry point exists
        if !Path::new(&manifest.entry_point).exists() {
            return Err(PluginError::LoadFailure(format!(
                "Entry point {} not found",
                manifest.entry_point
            )));
        }

        let id = Uuid::new_v4();
        let handle = PluginHandle {
            id,
            manifest: manifest.clone(),
        };

        info!(
            plugin_name = manifest.name,
            plugin_version = manifest.version,
            plugin_id = %id,
            "Loading plugin"
        );

        let mut plugins = self.plugins.lock().unwrap();
        plugins.insert(id, handle);
        Ok(id)
    }

    pub fn invoke_plugin(&self, plugin_id: Uuid, input: &str) -> Result<String, PluginError> {
        let plugins = self.plugins.lock().unwrap();
        let handle = plugins
            .get(&plugin_id)
            .ok_or_else(|| PluginError::InvokeFailure("Plugin not found".into()))?;

        info!(
            plugin_id = %plugin_id,
            plugin_name = handle.manifest.name,
            "Invoking plugin"
        );

        // For Phase 4, we'll simulate plugin execution
        // In a real implementation, this would:
        // 1. For WASM: Load and call the WASM module
        // 2. For subprocess: Spawn a process and communicate via IPC
        Ok(format!(
            "Hello from plugin {} (version {}) with input: {}",
            handle.manifest.name, handle.manifest.version, input
        ))
    }

    pub fn get_plugin(&self, plugin_id: &Uuid) -> Option<PluginHandle> {
        let plugins = self.plugins.lock().unwrap();
        plugins.get(plugin_id).cloned()
    }

    pub fn list_plugins(&self) -> Vec<(Uuid, PluginManifest)> {
        let plugins = self.plugins.lock().unwrap();
        plugins
            .iter()
            .map(|(id, handle)| (*id, handle.manifest.clone()))
            .collect()
    }
}

impl Clone for PluginManager {
    fn clone(&self) -> Self {
        Self {
            plugins: Arc::clone(&self.plugins),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_plugin_file() -> (tempfile::TempDir, String) {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_plugin.wasm");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "mock wasm content").unwrap();
        (dir, file_path.to_string_lossy().into_owned())
    }

    #[test]
    fn test_load_plugin_ok() {
        let (dir, entry_point) = create_test_plugin_file();
        let mgr = PluginManager::new();

        let manifest = PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            entry_point,
            permissions: vec!["net".to_string()],
            description: "A bad plugin".to_string(),
            functions: HashMap::new(),
        };

        let result = mgr.load_plugin(manifest);
        assert!(result.is_ok());
        drop(dir); // Clean up temp directory
    }

    #[test]
    fn test_load_plugin_forbidden_permission() {
        let mgr = PluginManager::new();
        let manifest = PluginManifest {
            name: "bad_plugin".to_string(),
            version: "0.1.0".to_string(),
            entry_point: "dummy".to_string(),
            permissions: vec!["forbidden".to_string()],
            description: "A test plugin".to_string(),
            functions: HashMap::new(),
        };

        let result = mgr.load_plugin(manifest);
        assert!(matches!(result, Err(PluginError::PermissionDenied(_))));
    }

    #[test]
    fn test_invoke_plugin() {
        let (dir, entry_point) = create_test_plugin_file();
        let mgr = PluginManager::new();

        let manifest = PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            entry_point,
            permissions: vec![],
            description: "A test plugin".to_string(),
            functions: HashMap::new(),
        };

        let plugin_id = mgr.load_plugin(manifest).unwrap();
        let result = mgr.invoke_plugin(plugin_id, "test input");

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("test input"));
        drop(dir);
    }

    #[test]
    fn test_invoke_nonexistent_plugin() {
        let mgr = PluginManager::new();
        let result = mgr.invoke_plugin(Uuid::new_v4(), "test");
        assert!(matches!(result, Err(PluginError::InvokeFailure(_))));
    }

    #[test]
    fn test_plugin_manager_clone() {
        let mgr1 = PluginManager::new();
        let mgr2 = mgr1.clone();

        // Load a plugin in mgr1
        let (dir, entry_point) = create_test_plugin_file();
        let manifest = PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            entry_point,
            permissions: vec![],
            description: "A test plugin".to_string(),
            functions: HashMap::new(),
        };

        let plugin_id = mgr1.load_plugin(manifest.clone()).unwrap();

        // Verify the plugin is accessible in mgr2
        let plugins2 = mgr2.list_plugins();
        assert_eq!(plugins2.len(), 1);
        assert_eq!(plugins2[0].0, plugin_id);
        drop(dir);
    }
}
