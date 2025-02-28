//! Plugin Manager for Lion Runtime
//!
//! Manages the loading, unloading, and lifecycle of plugins in the Lion system.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::plugin::lifecycle::LifecycleManager;
use anyhow::Result;
use lion_core::id::PluginId;
use lion_core::types::plugin::PluginState;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::lifecycle::{PluginLifecycle, PluginMetadata};
use super::registry::PluginRegistry;
use crate::capabilities::manager::CapabilityManager;
use crate::system::config::RuntimeConfig;

/// Errors that can occur in plugin manager operations
#[derive(thiserror::Error, Debug)]
pub enum PluginManagerError {
    #[error("Plugin {0} not found")]
    NotFound(PluginId),

    #[error("Plugin {0} already exists")]
    AlreadyExists(PluginId),

    #[error("Plugin {0} is not in the expected state")]
    InvalidState(PluginId),

    #[error("Plugin directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),
}

/// Configuration for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Path to the plugin file
    pub path: String,

    /// Configuration for the plugin
    pub config: serde_json::Value,

    /// Whether to automatically start the plugin
    pub autostart: bool,
}

/// The plugin manager handles loading, unloading, and managing plugins
pub struct PluginManager {
    /// Map of plugin IDs to lifecycle managers
    plugins: RwLock<HashMap<PluginId, Arc<PluginLifecycle>>>,

    /// Plugin registry for discovering and registering plugins
    registry: Arc<PluginRegistry>,

    /// Capability manager for granting capabilities to plugins
    capability_manager: Arc<CapabilityManager>,

    /// Runtime configuration
    config: RuntimeConfig,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(config: RuntimeConfig, capability_manager: Arc<CapabilityManager>) -> Result<Self> {
        let registry = Arc::new(PluginRegistry::new()?);

        Ok(Self {
            plugins: RwLock::new(HashMap::new()),
            registry,
            capability_manager,
            config,
        })
    }

    /// Start the plugin manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting plugin manager");

        // Load plugins from the configured plugin directory
        self.load_plugins_from_directory(&self.config.plugin_directory)
            .await?;

        // Load and start autostart plugins
        for (plugin_name, plugin_config) in &self.config.plugins {
            if plugin_config.autostart {
                info!("Auto-starting plugin: {}", plugin_name);

                let plugin_id = self.registry.get_plugin_id(plugin_name)?;
                self.start_plugin(&plugin_id).await?;
            }
        }

        info!("Plugin manager started");

        Ok(())
    }

    /// Load all plugins from a directory
    pub async fn load_plugins_from_directory(&self, directory: &str) -> Result<()> {
        info!("Loading plugins from directory: {}", directory);

        let dir_path = Path::new(directory);
        if !dir_path.exists() || !dir_path.is_dir() {
            return Err(PluginManagerError::DirectoryNotFound(directory.to_string()).into());
        }

        // Discover plugins using the registry
        let discovered_plugins = self.registry.discover_plugins(dir_path).await?;

        for plugin_metadata in discovered_plugins {
            info!("Found plugin: {}", plugin_metadata.name);

            // Register the plugin
            self.registry
                .register_plugin(plugin_metadata.clone())
                .await?;

            // Create the plugin lifecycle
            let isolation_manager = Arc::new(LifecycleManager::new(&plugin_metadata.path)?);
            let lifecycle =
                PluginLifecycle::new(plugin_metadata.clone(), isolation_manager).await?;

            // Load the plugin
            self.plugins
                .write()
                .await
                .insert(plugin_metadata.id, Arc::new(lifecycle));
        }

        info!(
            "Loaded {} plugins from directory",
            self.plugins.read().await.len()
        );

        Ok(())
    }

    /// Register a plugin manually
    pub async fn register_plugin(&self, metadata: PluginMetadata, path: &str) -> Result<PluginId> {
        info!("Registering plugin: {}", metadata.name);

        // Check if the plugin already exists
        if self.registry.has_plugin(&metadata.id).await? {
            return Err(PluginManagerError::AlreadyExists(metadata.id).into());
        }

        // Register the plugin in the registry
        self.registry.register_plugin(metadata.clone()).await?;

        // Create the plugin lifecycle
        let isolation_manager = Arc::new(LifecycleManager::new(path)?);
        let lifecycle = PluginLifecycle::new(metadata.clone(), isolation_manager).await?;

        // Store the plugin
        self.plugins
            .write()
            .await
            .insert(metadata.id, Arc::new(lifecycle));

        Ok(metadata.id)
    }

    /// Load a plugin
    pub async fn load_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        info!("Loading plugin: {:?}", plugin_id);

        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))?;

        // Load the plugin
        plugin.load().await?;

        Ok(())
    }

    /// Initialize a plugin with configuration
    pub async fn initialize_plugin(
        &self,
        plugin_id: &PluginId,
        config: serde_json::Value,
    ) -> Result<()> {
        info!("Initializing plugin: {:?}", plugin_id);

        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))?;

        // Initialize the plugin
        plugin.initialize(config).await?;

        Ok(())
    }

    /// Start a plugin
    pub async fn start_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        info!("Starting plugin: {:?}", plugin_id);

        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))?;

        // Get plugin state
        let state = plugin.get_state().await;

        // Initialize if needed
        if state == PluginState::Ready {
            // Get config from runtime config if available
            let metadata = plugin.get_metadata().await;
            let config = self
                .config
                .plugins
                .get(&metadata.name)
                .map(|p| p.config.clone())
                .unwrap_or_else(|| serde_json::json!({}));

            plugin.initialize(config).await?;
        }

        // Start the plugin
        plugin.start().await?;

        Ok(())
    }

    /// Pause a plugin
    pub async fn pause_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        info!("Pausing plugin: {:?}", plugin_id);

        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))?;

        // Pause the plugin
        plugin.pause().await?;

        Ok(())
    }

    /// Stop a plugin
    pub async fn stop_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        info!("Stopping plugin: {:?}", plugin_id);

        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))?;

        // Stop the plugin
        plugin.stop().await?;

        Ok(())
    }

    /// Unload a plugin
    pub async fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        info!("Unloading plugin: {:?}", plugin_id);

        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))?;

        // Get plugin state
        let state = plugin.get_state().await;

        // Stop if needed
        if state == PluginState::Running || state == PluginState::Paused {
            plugin.stop().await?;
        }

        // Unload the plugin
        plugin.unload().await?;

        Ok(())
    }

    /// Remove a plugin
    pub async fn remove_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        info!("Removing plugin: {:?}", plugin_id);

        // Unload first if needed
        if let Ok(plugin) = self
            .plugins
            .read()
            .await
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))
        {
            let state = plugin.get_state().await;

            if state != PluginState::Terminated {
                self.unload_plugin(plugin_id).await?;
            }
        }

        // Remove from registry
        self.registry.unregister_plugin(plugin_id).await?;

        // Remove from plugins
        self.plugins.write().await.remove(plugin_id);

        info!("Plugin removed: {:?}", plugin_id);

        Ok(())
    }

    /// Call a function in a plugin
    pub async fn call_plugin_function(
        &self,
        plugin_id: &PluginId,
        function_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        debug!(
            "Calling function '{}' in plugin '{:?}'",
            function_name, plugin_id
        );

        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))?;

        // Call the function
        let result = plugin.call_function(function_name, params).await?;

        Ok(result)
    }

    /// Get all loaded plugins
    pub async fn get_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        let mut result = Vec::new();

        for plugin in plugins.values() {
            result.push(plugin.get_metadata().await);
        }

        result
    }

    /// Get a plugin by ID
    pub async fn get_plugin(&self, plugin_id: &PluginId) -> Result<PluginMetadata> {
        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))?;

        Ok(plugin.get_metadata().await)
    }

    /// Grant a capability to a plugin
    pub async fn grant_capability(
        &self,
        plugin_id: &PluginId,
        object: &str,
        rights: Vec<String>,
    ) -> Result<()> {
        info!(
            "Granting capability for object '{}' to plugin '{:?}'",
            object, plugin_id
        );

        let plugins = self.plugins.read().await;
        let _plugin = plugins
            .get(plugin_id)
            .ok_or(PluginManagerError::NotFound(*plugin_id))?;

        // Use the capability manager to grant the capability
        let _cap_id = self
            .capability_manager
            .grant_capability(plugin_id.to_string(), object.to_string(), rights)
            .await?;

        Ok(())
    }

    /// Shutdown all plugins
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down all plugins");

        let plugins = self.plugins.read().await;

        // Stop each plugin
        for (plugin_id, plugin) in plugins.iter() {
            let state = plugin.get_state().await;

            if state == PluginState::Running || state == PluginState::Paused {
                info!("Stopping plugin: {:?}", plugin_id);

                if let Err(e) = plugin.stop().await {
                    warn!("Error stopping plugin {:?}: {}", plugin_id, e);
                    // Continue with next plugin
                }
            }
        }

        info!("All plugins shut down");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_manager_lifecycle() {
        // Create a temp directory for plugins
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();

        // Create a config
        let mut config = RuntimeConfig::default();
        config.plugin_directory = temp_path.clone();

        // Create a capability manager
        let capability_manager = Arc::new(CapabilityManager::new().unwrap());

        // Create the plugin manager
        let manager = PluginManager::new(config, capability_manager).unwrap();

        // Create a test plugin
        let plugin_id = PluginId::new();
        let metadata = PluginMetadata {
            id: plugin_id.clone(),
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            path: format!("{}/test-plugin", temp_path),
            state: PluginState::Created,
            required_capabilities: vec![],
        };

        // Create test file
        std::fs::create_dir_all(&format!("{}", temp_path)).unwrap();
        std::fs::write(&format!("{}/test-plugin", temp_path), b"test plugin").unwrap();

        // Register the plugin
        manager
            .register_plugin(metadata, &format!("{}/test-plugin", temp_path))
            .await
            .unwrap();

        // Verify the plugin is registered
        let plugins = manager.get_plugins().await;
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, plugin_id);

        // Start the manager
        manager.start().await.unwrap();

        // Shutdown
        manager.shutdown().await.unwrap();
    }
}
