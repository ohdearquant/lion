use super::config::{Config, PluginsConfig};
use super::discovery::PluginDiscovery;
use super::error::PluginError;
use super::loader::PluginLoader;
use super::manifest::PluginManifest;
use crate::storage::{ElementStore, FileStorage};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PluginManager {
    storage: Arc<FileStorage>,
    discovery: Option<PluginDiscovery>,
    loader: Option<PluginLoader>,
    config: PluginsConfig,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        debug!("Creating new PluginManager");
        match Config::from_project_root() {
            Ok(config) => Self::with_config(config),
            Err(e) => {
                debug!("Failed to load config, using defaults: {}", e);
                let storage = Arc::new(FileStorage::new("plugins/data"));
                Self {
                    storage,
                    discovery: None,
                    loader: None,
                    config: PluginsConfig {
                        data_dir: PathBuf::from("plugins/data"),
                        calculator_manifest: PathBuf::from("plugins/calculator/manifest.toml"),
                    },
                }
            }
        }
    }

    pub fn with_config(config: Config) -> Self {
        debug!("Creating PluginManager with config");
        let storage = Arc::new(FileStorage::new(&config.plugins.data_dir));

        // Get manifest directory from calculator manifest path
        let manifest_dir = config
            .plugins
            .calculator_manifest
            .parent()
            .unwrap_or_else(|| Path::new("plugins"))
            .to_path_buf();

        let discovery = PluginDiscovery::new(&manifest_dir);
        let loader = PluginLoader::new(&manifest_dir);

        Self {
            storage,
            discovery: Some(discovery),
            loader: Some(loader),
            config: config.plugins,
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
        let storage = Arc::new(FileStorage::new(&storage_path));

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
            config: PluginsConfig {
                data_dir: storage_path.clone(),
                calculator_manifest: manifest_dir.join("calculator").join("manifest.toml"),
            },
        }
    }

    fn get_manifest_path(&self, plugin_name: &str) -> Result<PathBuf, PluginError> {
        if plugin_name == "calculator" {
            return Ok(self.config.calculator_manifest.clone());
        }

        let discovery = self.discovery.as_ref().ok_or_else(|| {
            PluginError::LoadError("No manifest directory configured".to_string())
        })?;
        let manifests = discovery.discover_plugins()?;
        manifests
            .iter()
            .find(|(m, _)| m.name == plugin_name)
            .map(|(_, p)| p.clone())
            .ok_or_else(|| {
                PluginError::LoadError(format!(
                    "Could not find manifest path for plugin {}",
                    plugin_name
                ))
            })
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

        // Get manifest path
        let manifest_path = self.get_manifest_path(&manifest.name)?;

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

        // Get manifest path
        let manifest_path_buf = self.get_manifest_path(&manifest.name)?;

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
        debug!("Cleared plugin storage");
    }
}
