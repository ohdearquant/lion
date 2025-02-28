//! Plugin Registry
//!
//! Manages the discovery and registration of plugins in the Lion system.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{error, info};

use lion_core::id::PluginId;
use lion_core::types::plugin::PluginState;

use super::lifecycle::PluginMetadata;

/// Errors that can occur in plugin registry operations
#[derive(thiserror::Error, Debug)]
pub enum RegistryError {
    #[error("Plugin {0} not found")]
    NotFound(PluginId),

    #[error("Plugin {0} already exists")]
    AlreadyExists(PluginId),

    #[error("Plugin with name {0} not found")]
    NameNotFound(String),

    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),
}

/// The plugin registry manages plugin discovery and registration
pub struct PluginRegistry {
    /// Map of plugin IDs to metadata
    plugins: RwLock<HashMap<PluginId, PluginMetadata>>,

    /// Map of plugin names to IDs
    plugin_names: RwLock<HashMap<String, PluginId>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Result<Self> {
        Ok(Self {
            plugins: RwLock::new(HashMap::new()),
            plugin_names: RwLock::new(HashMap::new()),
        })
    }

    /// Register a plugin
    pub async fn register_plugin(&self, metadata: PluginMetadata) -> Result<()> {
        let id = metadata.id;
        let name = metadata.name.clone();

        // Check if plugin already exists
        {
            let plugins = self.plugins.read().await;
            if plugins.contains_key(&id) {
                return Err(RegistryError::AlreadyExists(id).into());
            }
        }

        // Store plugin metadata
        self.plugins.write().await.insert(id, metadata);
        self.plugin_names.write().await.insert(name.clone(), id);

        info!("Registered plugin: {}", name);

        Ok(())
    }

    /// Unregister a plugin
    pub async fn unregister_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        // Get the plugin to find its name
        let name = {
            let plugins = self.plugins.read().await;
            let metadata = plugins
                .get(plugin_id)
                .ok_or(RegistryError::NotFound(*plugin_id))?;
            metadata.name.clone()
        };

        // Remove the plugin
        self.plugins.write().await.remove(plugin_id);
        self.plugin_names.write().await.remove(&name);

        info!("Unregistered plugin: {}", name);

        Ok(())
    }

    /// Check if plugin exists
    pub async fn has_plugin(&self, plugin_id: &PluginId) -> Result<bool> {
        Ok(self.plugins.read().await.contains_key(plugin_id))
    }

    /// Get plugin metadata by ID
    pub async fn get_plugin(&self, plugin_id: &PluginId) -> Result<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins
            .get(plugin_id)
            .cloned()
            .ok_or(RegistryError::NotFound(*plugin_id).into())
    }

    /// Get plugin ID by name
    pub fn get_plugin_id(&self, name: &str) -> Result<PluginId> {
        let plugin_names = self.plugin_names.blocking_read();
        plugin_names
            .get(name)
            .cloned()
            .ok_or_else(|| RegistryError::NameNotFound(name.to_string()).into())
    }

    /// Get all plugins
    pub async fn get_all_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins.read().await.values().cloned().collect()
    }

    /// Discover plugins in a directory
    pub async fn discover_plugins(&self, dir_path: &Path) -> Result<Vec<PluginMetadata>> {
        // In a real implementation, this would scan the directory for plugins
        // and extract metadata from them. For now, we'll return an empty list.

        info!("Discovering plugins in directory: {:?}", dir_path);

        // Mock implementation
        let mut discovered = Vec::new();

        if dir_path.exists() && dir_path.is_dir() {
            // Read the directory
            if let Ok(entries) = std::fs::read_dir(dir_path) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_file() {
                        // For now, treat every file as a potential plugin
                        // In a real implementation, we'd check file extensions or content
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            let plugin_id = PluginId::new();
                            let metadata = PluginMetadata {
                                id: plugin_id,
                                name: file_name.to_string(),
                                version: "0.1.0".to_string(),
                                description: "Auto-discovered plugin".to_string(),
                                author: "Unknown".to_string(),
                                path: path.to_string_lossy().to_string(),
                                state: PluginState::Created,
                                required_capabilities: Vec::new(),
                            };
                            discovered.push(metadata);
                        }
                    }
                }
            }
        }

        info!("Discovered {} plugins", discovered.len());

        Ok(discovered)
    }
}
