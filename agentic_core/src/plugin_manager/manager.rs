use super::{
    error::PluginError,
    loader::PluginLoader,
    manifest::PluginManifest,
    registry::{PluginMetadata, PluginState},
    Result,
};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Main plugin management interface that coordinates loading, initialization,
/// and invocation of plugins
#[derive(Debug)]
pub struct PluginManager {
    loader: PluginLoader,
    manifest_dir: Option<PathBuf>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        debug!("Creating new PluginManager with no manifest directory");
        Self {
            loader: PluginLoader::new("plugins/data"),
            manifest_dir: None,
        }
    }

    /// Create a new plugin manager with a manifest directory
    pub fn with_manifest_dir<P: AsRef<Path>>(manifest_dir: P) -> Self {
        let manifest_dir = manifest_dir.as_ref().to_path_buf();
        debug!(
            "Creating new PluginManager with manifest directory: {:?}",
            manifest_dir
        );
        Self {
            loader: PluginLoader::with_manifest_dir("plugins/data", &manifest_dir),
            manifest_dir: Some(manifest_dir),
        }
    }

    /// Get the manifest directory
    pub fn manifest_dir(&self) -> Option<&Path> {
        self.manifest_dir.as_deref()
    }

    /// Load a plugin from a manifest file
    pub async fn load_plugin<P: AsRef<Path>>(&self, manifest_path: P) -> Result<Uuid> {
        let manifest_path = manifest_path.as_ref();
        info!("Loading plugin from manifest: {:?}", manifest_path);

        self.loader.load_from_file(manifest_path).await
    }

    /// Load a plugin from a manifest string
    pub async fn load_plugin_from_string(
        &self,
        manifest: String,
        manifest_path: Option<String>,
    ) -> Result<Uuid> {
        info!("Loading plugin from string manifest");

        let manifest: PluginManifest = toml::from_str(&manifest)
            .map_err(|e| PluginError::LoadError(format!("Failed to parse manifest: {}", e)))?;

        self.loader.load_plugin(manifest, manifest_path).await
    }

    /// Get plugin metadata by ID
    pub fn get_plugin(&self, plugin_id: Uuid) -> Result<PluginMetadata> {
        self.loader.registry().get(plugin_id)
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.loader.registry().list()
    }

    /// Invoke a plugin with input
    pub async fn invoke_plugin(&self, plugin_id: Uuid, input: &str) -> Result<String> {
        debug!("Invoking plugin {} with input: {}", plugin_id, input);

        // Get plugin metadata
        let metadata = self.get_plugin(plugin_id)?;

        // Check plugin state
        match metadata.state {
            PluginState::Ready => (),
            PluginState::Loading => {
                return Err(PluginError::InvokeError(
                    "Plugin is still loading".to_string(),
                ))
            }
            PluginState::Failed(ref reason) => {
                return Err(PluginError::InvokeError(format!(
                    "Plugin failed to load: {}",
                    reason
                )))
            }
        }

        // TODO: Implement actual WASM invocation
        // For now, return a mock response
        Ok(format!(
            "Invoked plugin {} ({}) with input: {}",
            metadata.manifest.name, plugin_id, input
        ))
    }

    /// Remove a plugin
    pub async fn remove_plugin(&self, plugin_id: Uuid) -> Result<()> {
        info!("Removing plugin {}", plugin_id);

        // Get plugin metadata for logging
        if let Ok(metadata) = self.get_plugin(plugin_id) {
            debug!("Removing plugin: {}", metadata.manifest.name);
        }

        self.loader.registry().remove(plugin_id)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_plugin_manager() {
        let temp_dir = tempdir().unwrap();
        let manager = PluginManager::with_manifest_dir(temp_dir.path());

        // Create test manifest
        let manifest = toml::toml! {
            name = "test-plugin"
            version = "1.0.0"
            description = "A test plugin"
        };

        // Load plugin
        let plugin_id = manager
            .load_plugin_from_string(manifest.to_string(), None)
            .await
            .unwrap();

        // Get plugin
        let metadata = manager.get_plugin(plugin_id).unwrap();
        assert_eq!(metadata.manifest.name, "test-plugin");
        assert_eq!(metadata.manifest.version, "1.0.0");

        // List plugins
        let plugins = manager.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, plugin_id);

        // Invoke plugin
        let result = manager
            .invoke_plugin(plugin_id, "test input")
            .await
            .unwrap();
        assert!(result.contains("test-plugin"));
        assert!(result.contains("test input"));

        // Remove plugin
        manager.remove_plugin(plugin_id).await.unwrap();
        assert!(manager.get_plugin(plugin_id).is_err());
    }

    #[tokio::test]
    async fn test_invalid_plugin() {
        let manager = PluginManager::new();

        // Create invalid manifest
        let manifest = toml::toml! {
            name = ""
            version = "1.0.0"
            description = ""
        };

        // Try to load plugin
        let result = manager
            .load_plugin_from_string(manifest.to_string(), None)
            .await;
        assert!(matches!(result, Err(PluginError::InvalidManifest(_))));
    }
}
