mod discovery;
mod error;
mod loader;
mod manifest;

pub use error::PluginError;
pub use manifest::{PluginFunction, PluginManifest};

use crate::storage::FileStorage;
use discovery::PluginDiscovery;
use loader::PluginLoader;
use std::path::Path;
use std::sync::Arc;

use tracing::{debug, error};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PluginManager {
    storage: Arc<FileStorage>,
    discovery: Option<PluginDiscovery>,
    loader: Option<PluginLoader>,
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
            storage,
            discovery: None,
            loader: None,
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

        // If the path ends with "plugins/data", use its parent as the manifest directory
        let manifest_dir =
            if dir.ends_with("data") && dir.parent().is_some_and(|p| p.ends_with("plugins")) {
                dir.parent().unwrap().parent().unwrap().to_path_buf()
            } else {
                dir
            };

        debug!("Using manifest directory: {:?}", manifest_dir);

        let discovery = PluginDiscovery::new(&manifest_dir);
        let loader = PluginLoader::new(&manifest_dir);

        Self {
            storage,
            discovery: Some(discovery),
            loader: Some(loader),
        }
    }

    pub fn discover_plugins(&self) -> Result<Vec<PluginManifest>, PluginError> {
        debug!("Attempting to discover plugins");
        let discovery = self.discovery.as_ref().ok_or_else(|| {
            PluginError::ManifestError("No manifest directory configured".to_string())
        })?;
        let manifests = discovery.discover_plugins()?;
        debug!("Discovered {} plugins", manifests.len());
        for (manifest, _) in &manifests {
            debug!(
                "Found plugin: {} at {}",
                manifest.name, manifest.entry_point
            );
        }
        Ok(manifests.into_iter().map(|(m, _)| m).collect())
    }

    pub fn load_plugin(&mut self, manifest: PluginManifest) -> Result<Uuid, PluginError> {
        debug!("Attempting to load plugin: {}", manifest.name);
        let loader = self.loader.as_ref().ok_or_else(|| {
            PluginError::LoadError("No manifest directory configured".to_string())
        })?;

        // Get the manifest path from discovery
        let discovery = self.discovery.as_ref().ok_or_else(|| {
            PluginError::LoadError("No manifest directory configured".to_string())
        })?;
        let manifests = discovery.discover_plugins()?;
        let manifest_path = manifests
            .iter()
            .find(|(m, _)| m.name == manifest.name)
            .map(|(_, p)| p.clone())
            .ok_or_else(|| {
                PluginError::LoadError(format!(
                    "Could not find manifest path for plugin {}",
                    manifest.name
                ))
            })?;

        // Store manifest path before loading plugin
        let manifest_path = manifest_path.to_path_buf();
        let (id, element) = loader.load_plugin(manifest, &manifest_path)?;
        debug!("Successfully loaded plugin with ID: {}", id);
        self.storage.store(element);
        Ok(id)
    }

    pub fn invoke_plugin(&self, plugin_id: Uuid, input: &str) -> Result<String, PluginError> {
        debug!("Attempting to invoke plugin: {}", plugin_id);
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

        debug!("Found plugin manifest for {}", manifest.name);
        let loader = self.loader.as_ref().ok_or_else(|| {
            PluginError::LoadError("No manifest directory configured".to_string())
        })?;

        // Get the manifest path from discovery
        let discovery = self.discovery.as_ref().ok_or_else(|| {
            PluginError::LoadError("No manifest directory configured".to_string())
        })?;
        let manifests = discovery.discover_plugins()?;
        let manifest_path_buf = manifests
            .iter()
            .find(|(m, _)| m.name == manifest.name)
            .map(|(_, p)| p.clone())
            .unwrap();

        loader.invoke_plugin(&manifest, manifest_path_buf.as_path(), input)
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
        let mut manager = PluginManager::with_manifest_dir(temp_dir.path());
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
