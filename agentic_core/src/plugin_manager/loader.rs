use super::{error::PluginError, manifest::PluginManifest, registry::PluginRegistry, Result};
use crate::types::{plugin::PluginState, traits::Validatable};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error};
use uuid::Uuid;

/// Handles loading and initialization of plugins
#[derive(Debug)]
pub struct PluginLoader {
    registry: PluginRegistry,
    manifest_dir: Option<PathBuf>,
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new(storage_path: &str) -> Self {
        Self {
            registry: PluginRegistry::new(storage_path),
            manifest_dir: None,
        }
    }

    /// Create a new plugin loader with a manifest directory
    pub fn with_manifest_dir<P: AsRef<Path>>(storage_path: &str, manifest_dir: P) -> Self {
        Self {
            registry: PluginRegistry::new(storage_path),
            manifest_dir: Some(manifest_dir.as_ref().to_path_buf()),
        }
    }

    /// Load a plugin from a manifest file
    pub async fn load_from_file<P: AsRef<Path>>(&self, manifest_path: P) -> Result<Uuid> {
        let manifest_path = manifest_path.as_ref();
        debug!("Loading plugin from manifest: {:?}", manifest_path);

        // Read and parse manifest
        let manifest_content = fs::read_to_string(manifest_path)
            .await
            .map_err(|e| PluginError::LoadError(format!("Failed to read manifest: {}", e)))?;

        let manifest: PluginManifest = toml::from_str(&manifest_content)
            .map_err(|e| PluginError::LoadError(format!("Failed to parse manifest: {}", e)))?;

        self.load_plugin(manifest, Some(manifest_path.to_string_lossy().into_owned()))
            .await
    }

    /// Load a plugin from a manifest
    pub async fn load_plugin(
        &self,
        manifest: PluginManifest,
        manifest_path: Option<String>,
    ) -> Result<Uuid> {
        debug!("Loading plugin: {}", manifest.name);

        // Validate manifest
        manifest.validate().map_err(PluginError::InvalidManifest)?;

        // Check WASM file if specified
        if let Some(wasm_path) = manifest.wasm_path.as_ref() {
            let full_path = if let Some(manifest_dir) = &self.manifest_dir {
                manifest_dir.join(wasm_path)
            } else {
                PathBuf::from(wasm_path)
            };

            if !full_path.exists() {
                return Err(PluginError::LoadError(format!(
                    "WASM file not found: {}",
                    full_path.display()
                )));
            }
        }

        // Register plugin
        let plugin_id = self.registry.register(manifest.clone(), manifest_path)?;

        // Initialize plugin
        match self.initialize_plugin(plugin_id, &manifest).await {
            Ok(_) => {
                self.registry.update_state(plugin_id, PluginState::Ready)?;
                Ok(plugin_id)
            }
            Err(e) => {
                error!("Failed to initialize plugin: {}", e);
                self.registry.update_state(plugin_id, PluginState::Error)?;
                Err(e)
            }
        }
    }

    /// Initialize a plugin
    async fn initialize_plugin(
        &self,
        _plugin_id: Uuid,
        manifest: &PluginManifest,
    ) -> Result<String> {
        // TODO: Implement WASM module loading and initialization
        // For now, just verify the manifest and return success
        debug!("Initializing plugin: {}", manifest.name);
        Ok(format!("Plugin {} initialized successfully", manifest.name))
    }

    /// Get the plugin registry
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_plugin_loader() {
        let temp_dir = tempdir().unwrap();
        let loader = PluginLoader::with_manifest_dir(
            temp_dir.path().join("data").to_str().unwrap(),
            temp_dir.path(),
        );

        // Create test manifest
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        // Load plugin
        let plugin_id = loader.load_plugin(manifest.clone(), None).await.unwrap();

        // Verify plugin state
        let metadata = loader.registry().get(plugin_id).unwrap();
        assert_eq!(metadata.manifest.name, "test-plugin");
        assert_eq!(metadata.state, PluginState::Ready);
    }

    #[tokio::test]
    async fn test_invalid_manifest() {
        let temp_dir = tempdir().unwrap();
        let loader = PluginLoader::new(temp_dir.path().to_str().unwrap());

        // Create invalid manifest
        let manifest = PluginManifest::new("".to_string(), "1.0.0".to_string(), "".to_string());

        // Try to load plugin
        let result = loader.load_plugin(manifest, None).await;
        assert!(matches!(result, Err(PluginError::InvalidManifest(_))));
    }
}
