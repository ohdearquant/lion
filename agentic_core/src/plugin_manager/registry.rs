use super::{error::PluginError, manifest::PluginManifest, Result};
use crate::storage::{ElementId, FileStorage};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Registry for managing plugin metadata and state
#[derive(Debug, Clone)]
pub struct PluginRegistry {
    storage: Arc<FileStorage>,
    plugins: Arc<RwLock<HashMap<Uuid, PluginMetadata>>>,
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub id: Uuid,
    pub manifest: PluginManifest,
    pub manifest_path: Option<String>,
    pub state: PluginState,
}

#[derive(Debug, Clone)]
pub enum PluginState {
    Loading,
    Ready,
    Failed(String),
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new(storage_path: &str) -> Self {
        Self {
            storage: Arc::new(FileStorage::new(storage_path)),
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new plugin
    pub fn register(
        &self,
        manifest: PluginManifest,
        manifest_path: Option<String>,
    ) -> Result<Uuid> {
        // Validate manifest
        manifest.validate().map_err(PluginError::InvalidManifest)?;

        let plugin_id = Uuid::new_v4();

        // Create plugin data before moving manifest_path
        let plugin_data = json!({
            "id": plugin_id.to_string(),
            "name": manifest.name,
            "version": manifest.version,
            "description": manifest.description,
            "wasm_path": manifest.wasm_path,
            "manifest_path": manifest_path,
        });

        let metadata = PluginMetadata {
            id: plugin_id,
            manifest: manifest.clone(),
            manifest_path,
            state: PluginState::Loading,
        };

        // Store in memory
        if let Ok(mut plugins) = self.plugins.write() {
            plugins.insert(plugin_id, metadata);
        } else {
            return Err(PluginError::LoadError(
                "Failed to acquire write lock".into(),
            ));
        }

        // Store in persistent storage
        self.storage
            .set(ElementId(plugin_id), plugin_data)
            .map_err(|e| PluginError::LoadError(format!("Failed to store plugin data: {}", e)))?;

        Ok(plugin_id)
    }

    /// Get plugin metadata by ID
    pub fn get(&self, plugin_id: Uuid) -> Result<PluginMetadata> {
        // Try memory first
        if let Ok(plugins) = self.plugins.read() {
            if let Some(metadata) = plugins.get(&plugin_id) {
                return Ok(metadata.clone());
            }
        }

        // Try storage
        let element = self
            .storage
            .get(ElementId(plugin_id))
            .ok_or_else(|| PluginError::NotFound(plugin_id))?;

        let data = &element.data.content;
        let manifest = PluginManifest::new(
            data.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            data.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            data.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        );

        let metadata = PluginMetadata {
            id: plugin_id,
            manifest,
            manifest_path: data
                .get("manifest_path")
                .and_then(|v| v.as_str())
                .map(String::from),
            state: PluginState::Ready,
        };

        Ok(metadata)
    }

    /// List all registered plugins
    pub fn list(&self) -> Vec<PluginMetadata> {
        let mut plugins = Vec::new();

        // Get from memory
        if let Ok(memory_plugins) = self.plugins.read() {
            plugins.extend(memory_plugins.values().cloned());
        }

        // Get from storage
        for element in self.storage.list() {
            let data = &element.data.content;
            if let Some(id_str) = data.get("id").and_then(|v| v.as_str()) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    // Skip if already in memory
                    if plugins.iter().any(|p| p.id == id) {
                        continue;
                    }

                    let manifest = PluginManifest::new(
                        data.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        data.get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        data.get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                    );

                    let metadata = PluginMetadata {
                        id,
                        manifest,
                        manifest_path: data
                            .get("manifest_path")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        state: PluginState::Ready,
                    };

                    plugins.push(metadata);
                }
            }
        }

        plugins
    }

    /// Update plugin state
    pub fn update_state(&self, plugin_id: Uuid, state: PluginState) -> Result<()> {
        if let Ok(mut plugins) = self.plugins.write() {
            if let Some(metadata) = plugins.get_mut(&plugin_id) {
                metadata.state = state;
                return Ok(());
            }
        }
        Err(PluginError::NotFound(plugin_id))
    }

    /// Remove a plugin from the registry
    pub fn remove(&self, plugin_id: Uuid) -> Result<()> {
        // Remove from memory
        if let Ok(mut plugins) = self.plugins.write() {
            plugins.remove(&plugin_id);
        }

        // Remove from storage
        if !self.storage.remove(ElementId(plugin_id)) {
            return Err(PluginError::NotFound(plugin_id));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_plugin_registry() {
        let temp_dir = tempdir().unwrap();
        let registry = PluginRegistry::new(temp_dir.path().to_str().unwrap());

        // Create test manifest
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        // Register plugin
        let plugin_id = registry
            .register(manifest.clone(), Some("manifest.toml".to_string()))
            .unwrap();

        // Get plugin
        let metadata = registry.get(plugin_id).unwrap();
        assert_eq!(metadata.manifest.name, "test-plugin");
        assert_eq!(metadata.manifest.version, "1.0.0");

        // List plugins
        let plugins = registry.list();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, plugin_id);

        // Update state
        registry
            .update_state(plugin_id, PluginState::Ready)
            .unwrap();
        let metadata = registry.get(plugin_id).unwrap();
        match metadata.state {
            PluginState::Ready => (),
            _ => panic!("Unexpected plugin state"),
        }

        // Remove plugin
        registry.remove(plugin_id).unwrap();
        assert!(registry.get(plugin_id).is_err());
    }
}
