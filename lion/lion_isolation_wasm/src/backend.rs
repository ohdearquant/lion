//! WebAssembly isolation backend implementation.
//!
//! This module provides the `WasmIsolationBackend` struct that implements
//! the `IsolationBackend` trait from `lion_core`.

use crate::config::{WasmInstanceConfig, WasmIsolationConfig};
use crate::error::WasmIsolationError;
use crate::instance::WasmInstance;
use crate::module::WasmModule;
use dashmap::DashMap;
use lion_core::capability::CapabilityManager;
use lion_core::isolation::IsolationBackend;
use lion_core::message::MessageBus;
use lion_core::plugin::{Plugin, PluginId, PluginManifest, PluginState};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use wasmtime::{Engine, Module};

/// A WebAssembly isolation backend using Wasmtime
pub struct WasmIsolationBackend {
    /// The WebAssembly engine
    engine: Engine,
    
    /// The capability manager
    capability_manager: Arc<dyn CapabilityManager>,
    
    /// The message bus
    message_bus: Arc<dyn MessageBus>,
    
    /// The configuration
    config: WasmIsolationConfig,
    
    /// Loaded modules (plugin_id -> module)
    modules: RwLock<DashMap<PluginId, WasmModule>>,
    
    /// Plugin instances (plugin_id -> instance)
    instances: RwLock<DashMap<PluginId, Arc<WasmInstance>>>,
    
    /// Plugin manifests (plugin_id -> manifest)
    manifests: RwLock<DashMap<PluginId, PluginManifest>>,
}

impl WasmIsolationBackend {
    /// Create a new WebAssembly isolation backend
    pub fn new(
        capability_manager: Arc<dyn CapabilityManager>,
        message_bus: Arc<dyn MessageBus>,
        config: WasmIsolationConfig,
    ) -> Result<Self, WasmIsolationError> {
        // Create a Wasmtime engine
        let mut engine_config = wasmtime::Config::default();
        engine_config.consume_fuel(config.enable_fuel_metering);
        let engine = Engine::new(&engine_config)
            .map_err(|e| WasmIsolationError::Wasmtime(e.to_string()))?;
        
        Ok(Self {
            engine,
            capability_manager,
            message_bus,
            config,
            modules: RwLock::new(DashMap::new()),
            instances: RwLock::new(DashMap::new()),
            manifests: RwLock::new(DashMap::new()),
        })
    }
}

impl IsolationBackend for WasmIsolationBackend {
    fn load_plugin(&self, manifest: &PluginManifest) -> Result<PluginId, lion_core::error::IsolationError> {
        // Generate a plugin ID
        let plugin_id = PluginId::new();
        
        // Load and compile the WebAssembly module
        let module = WasmModule::from_source(&self.engine, &manifest.source, &manifest.name)
            .map_err(|e| lion_core::error::IsolationError::InitializationFailed(
                format!("Failed to load module: {}", e)
            ))?;
        
        // Store the module
        {
            let modules = self.modules.read().unwrap();
            modules.insert(plugin_id, module.clone());
        }
        
        // Store the manifest
        {
            let manifests = self.manifests.read().unwrap();
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
        ).map_err(|e| lion_core::error::IsolationError::InitializationFailed(
            format!("Failed to create instance: {}", e)
        ))?;
        
        // Store the instance
        {
            let instances = self.instances.read().unwrap();
            instances.insert(plugin_id, Arc::new(instance));
        }
        
        Ok(plugin_id)
    }
    
    fn unload_plugin(&self, plugin_id: PluginId) -> Result<(), lion_core::error::IsolationError> {
        // Get the instance
        let instance = {
            let instances = self.instances.read().unwrap();
            instances.get(&plugin_id).cloned()
        };
        
        // Shutdown the instance
        if let Some(instance) = instance {
            instance.shutdown().map_err(|e| lion_core::error::IsolationError::ExecutionFailed(
                format!("Failed to shutdown instance: {}", e)
            ))?;
        }
        
        // Remove the instance
        {
            let instances = self.instances.write().unwrap();
            instances.remove(&plugin_id);
        }
        
        // Remove the module
        {
            let modules = self.modules.write().unwrap();
            modules.remove(&plugin_id);
        }
        
        // Remove the manifest
        {
            let manifests = self.manifests.write().unwrap();
            manifests.remove(&plugin_id);
        }
        
        Ok(())
    }
    
    fn initialize_plugin(&self, plugin_id: PluginId) -> Result<(), lion_core::error::IsolationError> {
        // Get the instance
        let instance = {
            let instances = self.instances.read().unwrap();
            instances.get(&plugin_id).cloned()
        };
        
        // Initialize the instance
        if let Some(instance) = instance {
            instance.initialize().map_err(|e| lion_core::error::IsolationError::InitializationFailed(
                format!("Failed to initialize instance: {}", e)
            ))?;
            Ok(())
        } else {
            Err(lion_core::error::IsolationError::ExecutionFailed(
                format!("Plugin not found: {}", plugin_id.0)
            ))
        }
    }
    
    fn handle_message(
        &self,
        plugin_id: PluginId,
        message: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, lion_core::error::IsolationError> {
        // Get the instance
        let instance = {
            let instances = self.instances.read().unwrap();
            instances.get(&plugin_id).cloned()
        };
        
        // Handle the message
        if let Some(instance) = instance {
            instance.handle_message(message).map_err(|e| lion_core::error::IsolationError::ExecutionFailed(
                format!("Failed to handle message: {}", e)
            ))
        } else {
            Err(lion_core::error::IsolationError::ExecutionFailed(
                format!("Plugin not found: {}", plugin_id.0)
            ))
        }
    }
    
    fn get_plugin(&self, plugin_id: PluginId) -> Option<Arc<dyn Plugin>> {
        let instances = self.instances.read().unwrap();
        instances.get(&plugin_id).cloned().map(|i| i as Arc<dyn Plugin>)
    }
    
    fn get_manifest(&self, plugin_id: PluginId) -> Option<PluginManifest> {
        let manifests = self.manifests.read().unwrap();
        manifests.get(&plugin_id).cloned()
    }
    
    fn list_plugins(&self) -> Vec<PluginId> {
        let instances = self.instances.read().unwrap();
        instances.iter().map(|entry| *entry.key()).collect()
    }
    
    fn get_memory_usage(&self, plugin_id: PluginId) -> Result<usize, lion_core::error::IsolationError> {
        let instances = self.instances.read().unwrap();
        if let Some(instance) = instances.get(&plugin_id) {
            Ok(instance.memory_usage())
        } else {
            Err(lion_core::error::IsolationError::ExecutionFailed(
                format!("Plugin not found: {}", plugin_id.0)
            ))
        }
    }
    
    fn get_execution_time(&self, plugin_id: PluginId) -> Result<Duration, lion_core::error::IsolationError> {
        let instances = self.instances.read().unwrap();
        if let Some(instance) = instances.get(&plugin_id) {
            Ok(instance.execution_time())
        } else {
            Err(lion_core::error::IsolationError::ExecutionFailed(
                format!("Plugin not found: {}", plugin_id.0)
            ))
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}