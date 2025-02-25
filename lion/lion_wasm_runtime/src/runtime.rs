//! WebAssembly runtime implementation.
//!
//! This module provides the main runtime for loading and executing
//! WebAssembly plugins.

use crate::error::WasmRuntimeError;
use crate::instance::{WasmInstance, WasmInstanceConfig};
use crate::module::WasmModule;
use lion_core::capability::CapabilityManager;
use lion_core::message::MessageBus;
use lion_core::plugin::{Plugin, PluginId, PluginManifest, PluginSource};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use wasmtime::Engine;

/// Configuration for the WebAssembly runtime
#[derive(Clone)]
pub struct WasmRuntimeConfig {
    /// Configuration for WebAssembly instances
    pub instance_config: WasmInstanceConfig,
    
    /// Whether to enable fuel-based metering
    pub enable_fuel_metering: bool,
    
    /// Default fuel limit
    pub default_fuel_limit: Option<u64>,
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            instance_config: WasmInstanceConfig::default(),
            enable_fuel_metering: true,
            default_fuel_limit: Some(10_000_000), // 10 million instructions
        }
    }
}

/// WebAssembly runtime for loading and executing plugins
pub struct WasmRuntime {
    /// Wasmtime engine
    engine: Engine,
    
    /// Capability manager for checking capabilities
    capability_manager: Arc<dyn CapabilityManager>,
    
    /// Message bus for inter-plugin communication
    message_bus: Arc<dyn MessageBus>,
    
    /// Configuration
    config: WasmRuntimeConfig,
    
    /// Loaded modules (plugin_id -> module)
    modules: RwLock<HashMap<PluginId, WasmModule>>,
    
    /// Plugin instances (plugin_id -> instance)
    instances: RwLock<HashMap<PluginId, Arc<Mutex<WasmInstance>>>>,
    
    /// Plugin manifests (plugin_id -> manifest)
    manifests: RwLock<HashMap<PluginId, PluginManifest>>,
}

impl WasmRuntime {
    /// Create a new WebAssembly runtime
    pub fn new(
        capability_manager: Arc<dyn CapabilityManager>,
        message_bus: Arc<dyn MessageBus>,
        config: WasmRuntimeConfig,
    ) -> Result<Self, WasmRuntimeError> {
        // Create a Wasmtime engine
        let mut engine_config = wasmtime::Config::default();
        engine_config.consume_fuel(config.enable_fuel_metering);
        let engine = Engine::new(&engine_config)
            .map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
        
        Ok(Self {
            engine,
            capability_manager,
            message_bus,
            config,
            modules: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            manifests: RwLock::new(HashMap::new()),
        })
    }
    
    /// Load a plugin from a manifest
    pub fn load_plugin(&self, manifest: &PluginManifest) -> Result<PluginId, WasmRuntimeError> {
        // Generate a plugin ID
        let plugin_id = PluginId::new();
        
        // Load and compile the WebAssembly module
        let module = WasmModule::from_source(&self.engine, &manifest.source, &manifest.name)?;
        
        // Store the module
        {
            let mut modules = self.modules.write().unwrap();
            modules.insert(plugin_id, module.clone());
        }
        
        // Store the manifest
        {
            let mut manifests = self.manifests.write().unwrap();
            manifests.insert(plugin_id, manifest.clone());
        }
        
        // Create a WebAssembly instance
        let instance = WasmInstance::new(
            module,
            plugin_id,
            manifest.name.clone(),
            self.capability_manager.clone(),
            self.message_bus.clone(),
            self.config.instance_config.clone(),
        )?;
        
        // Store the instance
        {
            let mut instances = self.instances.write().unwrap();
            instances.insert(plugin_id, Arc::new(Mutex::new(instance)));
        }
        
        Ok(plugin_id)
    }
    
    /// Unload a plugin
    pub fn unload_plugin(&self, plugin_id: PluginId) -> Result<(), WasmRuntimeError> {
        // Get the instance
        let instance = {
            let instances = self.instances.read().unwrap();
            instances.get(&plugin_id).cloned()
        };
        
        // Shutdown the instance
        if let Some(instance) = instance {
            let mut instance = instance.lock().unwrap();
            instance.shutdown()?;
        }
        
        // Remove the instance
        {
            let mut instances = self.instances.write().unwrap();
            instances.remove(&plugin_id);
        }
        
        // Remove the module
        {
            let mut modules = self.modules.write().unwrap();
            modules.remove(&plugin_id);
        }
        
        // Remove the manifest
        {
            let mut manifests = self.manifests.write().unwrap();
            manifests.remove(&plugin_id);
        }
        
        Ok(())
    }
    
    /// Initialize a plugin
    pub fn initialize_plugin(&self, plugin_id: PluginId) -> Result<(), WasmRuntimeError> {
        // Get the instance
        let instance = {
            let instances = self.instances.read().unwrap();
            instances.get(&plugin_id).cloned()
        };
        
        // Initialize the instance
        if let Some(instance) = instance {
            let mut instance = instance.lock().unwrap();
            instance.initialize()?;
            Ok(())
        } else {
            Err(WasmRuntimeError::InvalidWebAssembly(format!(
                "Plugin not found: {}",
                plugin_id.0
            )))
        }
    }
    
    /// Handle a message for a plugin
    pub fn handle_message(
        &self,
        plugin_id: PluginId,
        message: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, WasmRuntimeError> {
        // Get the instance
        let instance = {
            let instances = self.instances.read().unwrap();
            instances.get(&plugin_id).cloned()
        };
        
        // Handle the message
        if let Some(instance) = instance {
            let mut instance = instance.lock().unwrap();
            instance.handle_message(message)
        } else {
            Err(WasmRuntimeError::InvalidWebAssembly(format!(
                "Plugin not found: {}",
                plugin_id.0
            )))
        }
    }
    
    /// Get a plugin instance
    pub fn get_plugin(&self, plugin_id: PluginId) -> Option<Arc<dyn Plugin>> {
        let instances = self.instances.read().unwrap();
        instances.get(&plugin_id).cloned().map(|instance| -> Arc<dyn Plugin> {
            // We need to do some type magic here to convert from
            // Arc<Mutex<WasmInstance>> to Arc<dyn Plugin>
            let instance: Arc<Mutex<dyn Plugin>> = instance as Arc<Mutex<dyn Plugin>>;
            Arc::new(InstanceWrapper(instance))
        })
    }
    
    /// Get a plugin's manifest
    pub fn get_manifest(&self, plugin_id: PluginId) -> Option<PluginManifest> {
        let manifests = self.manifests.read().unwrap();
        manifests.get(&plugin_id).cloned()
    }
    
    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginId> {
        let instances = self.instances.read().unwrap();
        instances.keys().cloned().collect()
    }
}

/// Wrapper to convert Arc<Mutex<dyn Plugin>> to Arc<dyn Plugin>
struct InstanceWrapper(Arc<Mutex<dyn Plugin>>);

impl Plugin for InstanceWrapper {
    fn id(&self) -> PluginId {
        self.0.lock().unwrap().id()
    }
    
    fn name(&self) -> &str {
        // This is a bit of a hack - we need to return a reference
        // but we can't hold the lock across the function call boundary
        // In a real implementation, we would store the name separately
        // or use a different approach
        "Plugin"
    }
    
    fn state(&self) -> lion_core::plugin::PluginState {
        self.0.lock().unwrap().state()
    }
    
    fn initialize(&mut self) -> Result<(), lion_core::error::PluginError> {
        self.0.lock().unwrap().initialize()
    }
    
    fn handle_message(
        &mut self,
        message: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, lion_core::error::PluginError> {
        self.0.lock().unwrap().handle_message(message)
    }
    
    fn shutdown(&mut self) -> Result<(), lion_core::error::PluginError> {
        self.0.lock().unwrap().shutdown()
    }
}