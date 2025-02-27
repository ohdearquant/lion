//! Plugin Registry for Lion Runtime
//!
//! Handles plugin discovery, registration, and lookups.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use lion_core::id::PluginId;
use lion_core::traits::plugin::PluginState;
use serde_json::Value;
use thiserror::Error;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::lifecycle::PluginMetadata;

/// Errors that can occur in plugin registry operations
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Plugin {0} not found")]
    NotFound(PluginId),
    
    #[error("Plugin name {0} not found")]
    NameNotFound(String),
    
    #[error("Plugin {0} already exists")]
    AlreadyExists(PluginId),
    
    #[error("Plugin registry IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Plugin discovery error: {0}")]
    DiscoveryError(String),
}

/// Registry for managing plugins
pub struct PluginRegistry {
    /// Map of plugin IDs to metadata
    plugins: RwLock<HashMap<PluginId, PluginMetadata>>,
    
    /// Map of plugin names to IDs
    name_to_id: RwLock<HashMap<String, PluginId>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Result<Self> {
        Ok(Self {
            plugins: RwLock::new(HashMap::new()),
            name_to_id: RwLock::new(HashMap::new()),
        })
    }
    
    /// Discover plugins in a directory
    pub async fn discover_plugins(&self, directory: &Path) -> Result<Vec<PluginMetadata>> {
        info!("Discovering plugins in directory: {:?}", directory);
        
        let mut discovered = Vec::new();
        
        // Check if the directory exists
        if !directory.exists() || !directory.is_dir() {
            return Err(RegistryError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory not found: {:?}", directory),
            )).into());
        }
        
        // Read all entries in the directory
        let mut entries = fs::read_dir(directory).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // Skip directories
            if path.is_dir() {
                continue;
            }
            
            // Check if this is a plugin file (e.g., .wasm)
            // In a real implementation, this would parse the file metadata or manifest
            if let Some(extension) = path.extension() {
                if extension == "wasm" {
                    // Create plugin metadata
                    let file_name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    let metadata = PluginMetadata {
                        id: PluginId(Uuid::new_v4().to_string()),
                        name: file_name.clone(),
                        version: "1.0.0".to_string(), // Default version
                        description: format!("Plugin discovered at {:?}", path),
                        author: "Unknown".to_string(),
                        path: path.to_str().unwrap_or("").to_string(),
                        state: PluginState::Created,
                        required_capabilities: Vec::new(),
                    };
                    
                    discovered.push(metadata);
                }
            }
        }
        
        info!("Discovered {} plugins", discovered.len());
        
        Ok(discovered)
    }
    
    /// Register a plugin in the registry
    pub async fn register_plugin(&self, metadata: PluginMetadata) -> Result<()> {
        info!("Registering plugin: {:?} ({})", metadata.id, metadata.name);
        
        let mut plugins = self.plugins.write().await;
        
        // Check if plugin ID already exists
        if plugins.contains_key(&metadata.id) {
            return Err(RegistryError::AlreadyExists(metadata.id).into());
        }
        
        // Add to maps
        plugins.insert(metadata.id.clone(), metadata.clone());
        self.name_to_id.write().await.insert(metadata.name.clone(), metadata.id.clone());
        
        Ok(())
    }
    
    /// Unregister a plugin from the registry
    pub async fn unregister_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        info!("Unregistering plugin: {:?}", plugin_id);
        
        let mut plugins = self.plugins.write().await;
        
        // Check if plugin exists
        if !plugins.contains_key(plugin_id) {
            return Err(RegistryError::NotFound(plugin_id.clone()).into());
        }
        
        // Get the plugin name
        let name = plugins.get(plugin_id).map(|p| p.name.clone()).unwrap_or_default();
        
        // Remove from maps
        plugins.remove(plugin_id);
        self.name_to_id.write().await.remove(&name);
        
        Ok(())
    }
    
    /// Check if a plugin exists in the registry
    pub async fn has_plugin(&self, plugin_id: &PluginId) -> Result<bool> {
        let plugins = self.plugins.read().await;
        Ok(plugins.contains_key(plugin_id))
    }
    
    /// Get a plugin's metadata by ID
    pub async fn get_plugin(&self, plugin_id: &PluginId) -> Result<PluginMetadata> {
        let plugins = self.plugins.read().await;
        
        plugins.get(plugin_id)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(plugin_id.clone()).into())
    }
    
    /// Get a plugin's ID by name
    pub fn get_plugin_id(&self, name: &str) -> Result<PluginId> {
        let name_to_id = self.name_to_id.blocking_read();
        
        name_to_id.get(name)
            .cloned()
            .ok_or_else(|| RegistryError::NameNotFound(name.to_string()).into())
    }
    
    /// Get a list of all registered plugins
    pub async fn get_all_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values().cloned().collect()
    }
    
    /// Get plugin manifest by parsing the plugin file
    pub async fn get_plugin_manifest(&self, plugin_path: &Path) -> Result<Value> {
        // In a real implementation, this would extract metadata from the plugin file
        // For now, we just return a placeholder manifest
        Ok(serde_json::json!({
            "name": plugin_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown"),
            "version": "1.0.0",
            "description": "Plugin discovered at runtime",
            "author": "Unknown",
            "capabilities": []
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_plugin_registry() {
        // Create a registry
        let registry = PluginRegistry::new().unwrap();
        
        // Create a plugin metadata
        let plugin_id = PluginId(Uuid::new_v4().to_string());
        let metadata = PluginMetadata {
            id: plugin_id.clone(),
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            path: "/path/to/plugin".to_string(),
            state: PluginState::Created,
            required_capabilities: vec![],
        };
        
        // Register the plugin
        registry.register_plugin(metadata.clone()).await.unwrap();
        
        // Check if plugin exists
        assert!(registry.has_plugin(&plugin_id).await.unwrap());
        
        // Get plugin by ID
        let retrieved = registry.get_plugin(&plugin_id).await.unwrap();
        assert_eq!(retrieved.id, plugin_id);
        
        // Get plugin by name
        let retrieved_id = registry.get_plugin_id("test-plugin").unwrap();
        assert_eq!(retrieved_id, plugin_id);
        
        // Unregister the plugin
        registry.unregister_plugin(&plugin_id).await.unwrap();
        
        // Check plugin no longer exists
        assert!(!registry.has_plugin(&plugin_id).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_discover_plugins() {
        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create a few test plugin files
        fs::write(temp_path.join("plugin1.wasm"), b"test plugin 1").await.unwrap();
        fs::write(temp_path.join("plugin2.wasm"), b"test plugin 2").await.unwrap();
        fs::write(temp_path.join("not_a_plugin.txt"), b"not a plugin").await.unwrap();
        
        // Create a registry
        let registry = PluginRegistry::new().unwrap();
        
        // Discover plugins
        let discovered = registry.discover_plugins(temp_path).await.unwrap();
        
        // Check that we found 2 plugins
        assert_eq!(discovered.len(), 2);
        
        // Verify plugin names
        let names: Vec<String> = discovered.iter().map(|p| p.name.clone()).collect();
        assert!(names.contains(&"plugin1".to_string()));
        assert!(names.contains(&"plugin2".to_string()));
    }
}