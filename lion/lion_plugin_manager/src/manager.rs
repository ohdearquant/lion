//! Main plugin manager implementation.

use crate::error::PluginManagerError;
use crate::loader::PluginLoader;
use crate::manifest::ManifestParser;
use dashmap::DashMap;
use lion_core::capability::CapabilityManager;
use lion_core::isolation::IsolationBackend;
use lion_core::message::MessageBus;
use lion_core::plugin::{Plugin, PluginId, PluginManager as PluginManagerTrait, PluginManifest, PluginSource, PluginState};
use lion_core::resource::ResourceMonitor;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Configuration for the plugin manager
#[derive(Clone, Debug)]
pub struct PluginManagerConfig {
    /// Base directory for plugin files
    pub plugin_dir: String,
    
    /// Default memory limit for plugins in bytes
    pub default_memory_limit: usize,
    
    /// Default execution time limit for plugins in seconds
    pub default_execution_time_limit: u64,
    
    /// Whether to initialize plugins on load
    pub initialize_on_load: bool,
    
    /// Whether to cache downloaded plugins
    pub cache_downloads: bool,
    
    /// Path to the cache directory
    pub cache_dir: Option<String>,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self {
            plugin_dir: "plugins".to_string(),
            default_memory_limit: 100 * 1024 * 1024, // 100 MB
            default_execution_time_limit: 5, // 5 seconds
            initialize_on_load: true,
            cache_downloads: true,
            cache_dir: None,
        }
    }
}

/// The main plugin manager
pub struct PluginManager {
    /// Configuration
    config: PluginManagerConfig,
    
    /// Capability manager
    capability_manager: Arc<dyn CapabilityManager>,
    
    /// Message bus
    message_bus: Arc<dyn MessageBus>,
    
    /// Isolation backend
    isolation_backend: Arc<dyn IsolationBackend>,
    
    /// Resource monitor
    resource_monitor: Arc<dyn ResourceMonitor>,
    
    /// Plugin loader
    loader: PluginLoader,
    
    /// Manifest parser
    manifest_parser: ManifestParser,
    
    /// Loaded plugin manifests
    manifests: RwLock<DashMap<PluginId, PluginManifest>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(
        capability_manager: Arc<dyn CapabilityManager>,
        message_bus: Arc<dyn MessageBus>,
        isolation_backend: Arc<dyn IsolationBackend>,
        resource_monitor: Arc<dyn ResourceMonitor>,
        config: PluginManagerConfig,
    ) -> Self {
        // Create the plugin loader
        let loader = if config.cache_downloads {
            if let Some(cache_dir) = &config.cache_dir {
                PluginLoader::with_cache_dir(cache_dir)
            } else {
                PluginLoader::with_cache_dir(format!("{}/cache", config.plugin_dir))
            }
        } else {
            PluginLoader::new()
        };
        
        Self {
            config,
            capability_manager,
            message_bus,
            isolation_backend,
            resource_monitor,
            loader,
            manifest_parser: ManifestParser::new(),
            manifests: RwLock::new(DashMap::new()),
        }
    }
    
    /// Load a plugin from a manifest file
    pub fn load_from_file<P: AsRef<Path>>(
        &self,
        manifest_path: P,
    ) -> Result<PluginId, PluginManagerError> {
        // Parse the manifest file
        let manifest = self.manifest_parser.parse_file(manifest_path)?;
        
        // Load the plugin
        self.load_plugin(&manifest)
    }
    
    /// Load a plugin from a manifest string
    pub fn load_from_string(
        &self,
        manifest_str: &str,
    ) -> Result<PluginId, PluginManagerError> {
        // Parse the manifest string
        let manifest = self.manifest_parser.parse_string(manifest_str)?;
        
        // Load the plugin
        self.load_plugin(&manifest)
    }
    
    /// Load a plugin from raw WebAssembly bytes
    pub fn load_from_bytes(
        &self,
        bytes: Vec<u8>,
        name: &str,
        capabilities: &[lion_core::capability::CoreCapability],
    ) -> Result<PluginId, PluginManagerError> {
        // Create a manifest
        let manifest = PluginManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: None,
            author: None,
            source: PluginSource::InMemory(bytes),
            requested_capabilities: capabilities.to_vec(),
        };
        
        // Load the plugin
        self.load_plugin(&manifest)
    }
    
    /// Get the capability manager
    pub fn capability_manager(&self) -> Arc<dyn CapabilityManager> {
        self.capability_manager.clone()
    }
    
    /// Get the message bus
    pub fn message_bus(&self) -> Arc<dyn MessageBus> {
        self.message_bus.clone()
    }
    
    /// Get the isolation backend
    pub fn isolation_backend(&self) -> Arc<dyn IsolationBackend> {
        self.isolation_backend.clone()
    }
    
    /// Get the resource monitor
    pub fn resource_monitor(&self) -> Arc<dyn ResourceMonitor> {
        self.resource_monitor.clone()
    }
    
    /// Create a plugin chain
    pub fn create_chain(&self, plugin_ids: &[PluginId]) -> Result<crate::workflow::PluginChain, PluginManagerError> {
        if plugin_ids.is_empty() {
            return Err(PluginManagerError::WorkflowError("Chain is empty".to_string()));
        }
        
        // Check that all plugins exist
        for &plugin_id in plugin_ids {
            if self.get_plugin(plugin_id).is_none() {
                return Err(PluginManagerError::PluginNotFound(plugin_id.0.to_string()));
            }
        }
        
        Ok(crate::workflow::PluginChain::with_steps(plugin_ids.to_vec()))
    }
    
    /// Execute a plugin chain
    pub fn execute_chain(
        &self,
        chain: &crate::workflow::PluginChain,
        input: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, PluginManagerError> {
        chain.execute(self, input)
    }
}

impl PluginManagerTrait for PluginManager {
    fn load_plugin(&self, manifest: &PluginManifest) -> Result<PluginId, lion_core::error::PluginError> {
        // Grant requested capabilities
        for capability in &manifest.requested_capabilities {
            // We need a plugin ID to grant capabilities, but we don't have one yet.
            // So we'll load the plugin first, then grant capabilities.
        }
        
        // Load the plugin in the isolation backend
        let plugin_id = self.isolation_backend.load_plugin(manifest)?;
        
        // Store the manifest
        {
            let manifests = self.manifests.write().unwrap();
            manifests.insert(plugin_id, manifest.clone());
        }
        
        // Grant requested capabilities
        for capability in &manifest.requested_capabilities {
            self.capability_manager
                .grant_capability(plugin_id, capability.clone())
                .map_err(|e| lion_core::error::PluginError::ExecutionError(format!(
                    "Failed to grant capability {:?}: {}",
                    capability, e
                )))?;
        }
        
        // Register the plugin with the resource monitor
        if let Err(e) = self.resource_monitor.register_plugin(plugin_id) {
            log::warn!("Failed to register plugin with resource monitor: {}", e);
        }
        
        // Initialize the plugin if configured to do so
        if self.config.initialize_on_load {
            self.isolation_backend.initialize_plugin(plugin_id)?;
        }
        
        Ok(plugin_id)
    }
    
    fn unload_plugin(&self, plugin_id: PluginId) -> Result<(), lion_core::error::PluginError> {
        // Unload the plugin from the isolation backend
        self.isolation_backend.unload_plugin(plugin_id)?;
        
        // Remove the manifest
        {
            let manifests = self.manifests.write().unwrap();
            manifests.remove(&plugin_id);
        }
        
        // Unregister the plugin from the resource monitor
        if let Err(e) = self.resource_monitor.unregister_plugin(plugin_id) {
            log::warn!("Failed to unregister plugin from resource monitor: {}", e);
        }
        
        Ok(())
    }
    
    fn get_plugin(&self, plugin_id: PluginId) -> Option<Arc<dyn Plugin>> {
        self.isolation_backend.get_plugin(plugin_id)
    }
    
    fn list_plugins(&self) -> Vec<PluginId> {
        self.isolation_backend.list_plugins()
    }
    
    fn get_manifest(&self, plugin_id: PluginId) -> Option<PluginManifest> {
        let manifests = self.manifests.read().unwrap();
        if let Some(manifest) = manifests.get(&plugin_id) {
            Some(manifest.clone())
        } else {
            self.isolation_backend.get_manifest(plugin_id)
        }
    }
}