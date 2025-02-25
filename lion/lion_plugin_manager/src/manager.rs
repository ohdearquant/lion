//! Main plugin manager implementation.

use crate::error::PluginManagerError;
use crate::loader::PluginLoader;
use crate::manifest::ManifestParser;
use crate::resource_monitor::ResourceMonitorImpl;
use dashmap::DashMap;
use lion_core::capability::{CapabilityManager, CoreCapability};
use lion_core::message::MessageBus;
use lion_core::plugin::{Plugin, PluginId, PluginManager as PluginManagerTrait, PluginManifest};
use lion_wasm_runtime::{WasmRuntime, WasmRuntimeConfig};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Configuration for the plugin manager
#[derive(Clone)]
pub struct PluginManagerConfig {
    /// Base directory for plugin files
    pub plugin_dir: String,
    
    /// Default memory limit for plugins in bytes
    pub default_memory_limit: usize,
    
    /// Default execution time limit for plugins
    pub default_execution_time_limit: Duration,
    
    /// Whether to initialize plugins on load
    pub initialize_on_load: bool,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self {
            plugin_dir: "plugins".to_string(),
            default_memory_limit: 100 * 1024 * 1024, // 100 MB
            default_execution_time_limit: Duration::from_secs(5),
            initialize_on_load: true,
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
    
    /// WebAssembly runtime
    wasm_runtime: Arc<Mutex<WasmRuntime>>,
    
    /// Plugin loader
    loader: PluginLoader,
    
    /// Manifest parser
    manifest_parser: ManifestParser,
    
    /// Resource monitor
    resource_monitor: ResourceMonitorImpl,
    
    /// Loaded plugin manifests
    manifests: DashMap<PluginId, PluginManifest>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(
        capability_manager: Arc<dyn CapabilityManager>,
        message_bus: Arc<dyn MessageBus>,
        config: PluginManagerConfig,
    ) -> Self {
        // Create WebAssembly runtime config
        let runtime_config = WasmRuntimeConfig {
            instance_config: lion_wasm_runtime::WasmInstanceConfig {
                memory_limit: config.default_memory_limit,
                execution_time_limit: config.default_execution_time_limit,
                fuel_limit: Some(10_000_000),
            },
            enable_fuel_metering: true,
            default_fuel_limit: Some(10_000_000),
        };
        
        // Create WebAssembly runtime
        let wasm_runtime = WasmRuntime::new(
            capability_manager.clone(),
            message_bus.clone(),
            runtime_config,
        )
        .expect("Failed to create WebAssembly runtime");
        
        Self {
            config,
            capability_manager: capability_manager.clone(),
            message_bus: message_bus.clone(),
            wasm_runtime: Arc::new(Mutex::new(wasm_runtime)),
            loader: PluginLoader::new(),
            manifest_parser: ManifestParser::new(),
            resource_monitor: ResourceMonitorImpl::new(),
            manifests: DashMap::new(),
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
    
    /// Get resource usage for a plugin
    pub fn get_resource_usage(
        &self,
        plugin_id: PluginId,
    ) -> Result<lion_core::resource::ResourceUsage, PluginManagerError> {
        self.resource_monitor
            .get_usage(plugin_id)
            .map_err(|e| PluginManagerError::Internal(e.to_string()))
    }
    
    /// Set resource limits for a plugin
    pub fn set_resource_limits(
        &self,
        plugin_id: PluginId,
        limits: lion_core::resource::ResourceLimits,
    ) -> Result<(), PluginManagerError> {
        self.resource_monitor
            .set_limits(plugin_id, limits)
            .map_err(|e| PluginManagerError::Internal(e.to_string()))
    }
}

impl PluginManagerTrait for PluginManager {
    fn load_plugin(&self, manifest: &PluginManifest) -> Result<PluginId, lion_core::error::PluginError> {
        // Grant requested capabilities
        let mut granted_ids = Vec::new();
        for capability in &manifest.requested_capabilities {
            match self.capability_manager.grant_capability(
                PluginId::new(), // Temporary ID until we get the real one
                capability.clone(),
            ) {
                Ok(id) => granted_ids.push(id),
                Err(e) => {
                    return Err(PluginManagerError::CapabilityError(format!(
                        "Failed to grant capability {:?}: {}",
                        capability, e
                    ))
                    .into());
                }
            }
        }
        
        // Load the plugin in the WebAssembly runtime
        let runtime = self.wasm_runtime.lock().unwrap();
        let plugin_id = runtime.load_plugin(manifest).map_err(|e| {
            PluginManagerError::RuntimeError(format!("Failed to load plugin: {}", e))
        })?;
        
        // Store the manifest
        self.manifests.insert(plugin_id, manifest.clone());
        
        // Initialize the plugin if configured to do so
        if self.config.initialize_on_load {
            runtime.initialize_plugin(plugin_id).map_err(|e| {
                PluginManagerError::InitializationFailure(format!(
                    "Failed to initialize plugin: {}",
                    e
                ))
            })?;
        }
        
        // Set up resource monitoring
        self.resource_monitor.register_plugin(plugin_id);
        
        Ok(plugin_id)
    }
    
    fn unload_plugin(&self, plugin_id: PluginId) -> Result<(), lion_core::error::PluginError> {
        // Unload the plugin from the WebAssembly runtime
        let runtime = self.wasm_runtime.lock().unwrap();
        runtime.unload_plugin(plugin_id).map_err(|e| {
            PluginManagerError::RuntimeError(format!("Failed to unload plugin: {}", e))
        })?;
        
        // Remove the manifest
        self.manifests.remove(&plugin_id);
        
        // Clean up resource monitoring
        self.resource_monitor.unregister_plugin(plugin_id);
        
        Ok(())
    }
    
    fn get_plugin(&self, plugin_id: PluginId) -> Option<Arc<dyn Plugin>> {
        let runtime = self.wasm_runtime.lock().unwrap();
        runtime.get_plugin(plugin_id)
    }
    
    fn list_plugins(&self) -> Vec<PluginId> {
        let runtime = self.wasm_runtime.lock().unwrap();
        runtime.list_plugins()
    }
    
    fn get_manifest(&self, plugin_id: PluginId) -> Option<PluginManifest> {
        self.manifests.get(&plugin_id).map(|m| m.clone())
    }
}
