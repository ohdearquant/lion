//! Plugin management for the Lion runtime.
//!
//! This module provides plugin lifecycle management, including
//! loading, unloading, and function calls.

use std::sync::Arc;
use std::path::Path;
use std::time::Instant;

use dashmap::DashMap;
use parking_lot::RwLock;

use core::error::{Result, PluginError};
use core::types::{PluginId, PluginState, PluginType, PluginConfig, PluginMetadata, ResourceUsage};
use core::traits::{PluginManager as CorePluginManager, IsolationBackend, IsolationBackendFactory};
use capabilities::{CapabilityManager, Capability, FileCapability};
use policy::PolicyManager;
use isolation::WasmIsolationBackend;

use crate::config::RuntimeConfig;

/// Registry of loaded plugins.
pub struct PluginRegistry {
    /// Loaded plugin metadata.
    plugins: DashMap<PluginId, PluginMetadata>,
    
    /// Plugin configurations.
    configs: DashMap<PluginId, PluginConfig>,
}

impl PluginRegistry {
    /// Create a new plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
            configs: DashMap::new(),
        }
    }
    
    /// Register a plugin.
    pub fn register(
        &self,
        id: PluginId,
        metadata: PluginMetadata,
        config: PluginConfig,
    ) {
        self.plugins.insert(id.clone(), metadata);
        self.configs.insert(id, config);
    }
    
    /// Unregister a plugin.
    pub fn unregister(&self, id: &PluginId) {
        self.plugins.remove(id);
        self.configs.remove(id);
    }
    
    /// Get plugin metadata.
    pub fn get_metadata(&self, id: &PluginId) -> Option<PluginMetadata> {
        self.plugins.get(id).map(|p| p.clone())
    }
    
    /// Get plugin configuration.
    pub fn get_config(&self, id: &PluginId) -> Option<PluginConfig> {
        self.configs.get(id).map(|c| c.clone())
    }
    
    /// List all plugins.
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins.iter().map(|p| p.clone()).collect()
    }
    
    /// Update plugin state.
    pub fn update_state(&self, id: &PluginId, state: PluginState) -> Result<()> {
        if let Some(mut metadata) = self.plugins.get_mut(id) {
            metadata.state = state;
            metadata.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(PluginError::NotFound(id.clone()).into())
        }
    }
}

/// Manager for plugin lifecycle.
pub struct PluginManager {
    /// Plugin registry.
    registry: Arc<PluginRegistry>,
    
    /// Isolation backend.
    backend: Arc<dyn IsolationBackend>,
    
    /// Capability manager.
    capability_manager: Arc<CapabilityManager>,
    
    /// Policy manager.
    policy_manager: Arc<PolicyManager>,
    
    /// Runtime configuration.
    config: RuntimeConfig,
}

impl PluginManager {
    /// Create a new plugin manager.
    pub fn new(
        isolation_factory: impl IsolationBackendFactory,
        capability_manager: Arc<CapabilityManager>,
        policy_manager: Arc<PolicyManager>,
        config: RuntimeConfig,
    ) -> Result<Self> {
        // Create isolation backend
        let backend = isolation_factory.create_backend()?;
        
        Ok(Self {
            registry: Arc::new(PluginRegistry::new()),
            backend,
            capability_manager,
            policy_manager,
            config,
        })
    }
    
    /// Get the isolation backend.
    pub fn get_backend(&self) -> Arc<dyn IsolationBackend> {
        self.backend.clone()
    }
    
    /// Load a plugin.
    pub fn load_plugin(
        &self,
        name: &str,
        version: &str,
        description: &str,
        plugin_type: PluginType,
        code: Vec<u8>,
        config: PluginConfig,
    ) -> Result<PluginId> {
        // Create plugin ID
        let plugin_id = PluginId::new();
        
        // Create metadata
        let metadata = PluginMetadata {
            id: plugin_id.clone(),
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            plugin_type,
            state: PluginState::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        // Register the plugin
        self.registry.register(plugin_id.clone(), metadata, config.clone());
        
        // Configure the plugin in the backend
        self.backend.load_plugin(plugin_id.clone(), code, config)?;
        
        // Update state
        self.registry.update_state(&plugin_id, PluginState::Ready)?;
        
        // Grant basic capabilities
        let file_cap = Arc::new(FileCapability::new(true, false));
        self.capability_manager.grant_capability(&plugin_id, file_cap)?;
        
        Ok(plugin_id)
    }
    
    /// Call a function in a plugin.
    pub fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>> {
        // Check if plugin exists
        if let Some(metadata) = self.registry.get_metadata(plugin_id) {
            // Check if plugin is in a valid state
            match metadata.state {
                PluginState::Ready | PluginState::Running => {
                    // Update state
                    self.registry.update_state(plugin_id, PluginState::Running)?;
                    
                    // Measure execution time
                    let start_time = Instant::now();
                    
                    // Call the function
                    let result = self.backend.call_function(plugin_id, function, params);
                    
                    // Record execution time
                    let elapsed = start_time.elapsed();
                    
                    // Update state back to ready
                    self.registry.update_state(plugin_id, PluginState::Ready)?;
                    
                    // Record metrics if observability is enabled
                    #[cfg(feature = "observability")]
                    if self.config.enable_observability {
                        observability::metrics::record_plugin_call(
                            plugin_id,
                            function,
                            elapsed.as_millis() as u64,
                            result.is_ok(),
                        );
                    }
                    
                    result
                },
                PluginState::Paused => Err(PluginError::Paused.into()),
                PluginState::Failed => Err(PluginError::ExecutionError(
                    "Plugin is in a failed state".to_string()
                ).into()),
                PluginState::Terminated => Err(PluginError::ExecutionError(
                    "Plugin is terminated".to_string()
                ).into()),
                PluginState::Created => Err(PluginError::ExecutionError(
                    "Plugin is not initialized".to_string()
                ).into()),
                PluginState::Upgrading => Err(PluginError::Upgrading.into()),
            }
        } else {
            Err(PluginError::NotFound(plugin_id.clone()).into())
        }
    }
    
    /// Unload a plugin.
    pub fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        // Check if plugin exists
        if let Some(metadata) = self.registry.get_metadata(plugin_id) {
            // Unload from backend
            self.backend.unload_plugin(plugin_id)?;
            
            // Unregister the plugin
            self.registry.unregister(plugin_id);
            
            Ok(())
        } else {
            Err(PluginError::NotFound(plugin_id.clone()).into())
        }
    }
    
    /// Pause a plugin.
    pub fn pause_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        // Check if plugin exists and update state
        if self.registry.get_metadata(plugin_id).is_some() {
            self.registry.update_state(plugin_id, PluginState::Paused)?;
            Ok(())
        } else {
            Err(PluginError::NotFound(plugin_id.clone()).into())
        }
    }
    
    /// Resume a plugin.
    pub fn resume_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        // Check if plugin exists and update state
        if let Some(metadata) = self.registry.get_metadata(plugin_id) {
            if metadata.state == PluginState::Paused {
                self.registry.update_state(plugin_id, PluginState::Ready)?;
                Ok(())
            } else {
                Err(PluginError::InvalidState(metadata.state).into())
            }
        } else {
            Err(PluginError::NotFound(plugin_id.clone()).into())
        }
    }
    
    /// Get plugin state.
    pub fn get_plugin_state(&self, plugin_id: &PluginId) -> Result<PluginState> {
        if let Some(metadata) = self.registry.get_metadata(plugin_id) {
            Ok(metadata.state)
        } else {
            Err(PluginError::NotFound(plugin_id.clone()).into())
        }
    }
    
    /// Get plugin metadata.
    pub fn get_metadata(&self, plugin_id: &PluginId) -> Option<PluginMetadata> {
        self.registry.get_metadata(plugin_id)
    }
    
    /// List all plugins.
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.registry.list_plugins()
    }
    
    /// Get plugin resource usage.
    pub fn get_resource_usage(&self, plugin_id: &PluginId) -> Result<ResourceUsage> {
        self.backend.get_resource_usage(plugin_id)
    }
    
    /// Shutdown the plugin manager.
    pub fn shutdown(&self) -> Result<()> {
        // Unload all plugins
        let plugins = self.list_plugins();
        for plugin in plugins {
            let _ = self.unload_plugin(&plugin.id);
        }
        
        Ok(())
    }
}

impl CorePluginManager for PluginManager {
    fn load_plugin(
        &self,
        name: &str,
        version: &str,
        description: &str,
        plugin_type: core::types::PluginType,
        code: Vec<u8>,
        config: core::types::PluginConfig,
    ) -> core::error::Result<core::types::PluginId> {
        self.load_plugin(name, version, description, plugin_type, code, config)
    }
    
    fn call_function(
        &self,
        plugin_id: &core::types::PluginId,
        function: &str,
        params: &[u8],
    ) -> core::error::Result<Vec<u8>> {
        self.call_function(plugin_id, function, params)
    }
    
    fn get_metadata(&self, plugin_id: &core::types::PluginId) -> Option<core::types::PluginMetadata> {
        self.get_metadata(plugin_id)
    }
    
    fn unload_plugin(&self, plugin_id: &core::types::PluginId) -> core::error::Result<()> {
        self.unload_plugin(plugin_id)
    }
    
    fn pause_plugin(&self, plugin_id: &core::types::PluginId) -> core::error::Result<()> {
        self.pause_plugin(plugin_id)
    }
    
    fn resume_plugin(&self, plugin_id: &core::types::PluginId) -> core::error::Result<()> {
        self.resume_plugin(plugin_id)
    }
    
    fn list_plugins(&self) -> Vec<core::types::PluginMetadata> {
        self.list_plugins()
    }
    
    fn get_resource_usage(&self, plugin_id: &core::types::PluginId) -> core::error::Result<core::types::ResourceUsage> {
        self.get_resource_usage(plugin_id)
    }
    
    fn get_plugin_state(&self, plugin_id: &core::types::PluginId) -> core::error::Result<core::types::PluginState> {
        self.get_plugin_state(plugin_id)
    }
}