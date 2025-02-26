//! Isolation backends for plugin execution.
//!
//! This module provides concrete implementations of isolation backends,
//! including WebAssembly and potentially others.

use std::sync::Arc;

use dashmap::DashMap;
use wasmtime::Store;

use lion_core::error::{Result, IsolationError};
use lion_core::types::{PluginId, PluginConfig, ResourceUsage};
use lion_core::traits::IsolationBackend as CoreIsolationBackend;

use crate::wasm::{WasmIsolation, WasmContext, WasmModule};
use crate::store::ModuleStore;

/// Interface for isolation backends.
pub trait IsolationBackend: CoreIsolationBackend {
    /// Initialize the backend.
    fn initialize(&mut self) -> Result<()>;
    
    /// Shut down the backend.
    fn shutdown(&self) -> Result<()>;
}

/// WebAssembly isolation backend.
pub struct WasmIsolationBackend {
    /// Store for compiled modules.
    module_store: Arc<dyn ModuleStore>,
    
    /// WebAssembly engine.
    engine: wasmtime::Engine,
    
    /// Wasmtime linker.
    linker: wasmtime::Linker<WasmContext>,
    
    /// Loaded plugin instances.
    instances: DashMap<PluginId, WasmIsolation>,
    
    /// Maximum memory usage in bytes.
    max_memory: usize,
}

impl WasmIsolationBackend {
    /// Create a new WebAssembly isolation backend.
    pub fn new(
        module_store: Arc<dyn ModuleStore>,
        max_memory: usize,
    ) -> Result<Self> {
        // Create the Wasmtime engine
        let mut config = wasmtime::Config::default();
        config.wasm_multi_value(true);
        config.wasm_reference_types(true);
        config.wasm_bulk_memory(true);
        config.allocation_strategy(wasmtime::InstanceAllocationStrategy::OnDemand);
        
        let engine = wasmtime::Engine::new(&config)
            .map_err(|e| IsolationError::InstantiationFailed(format!("Failed to create Wasmtime engine: {}", e)))?;
        
        // Create the linker
        let mut linker = wasmtime::Linker::new(&engine);
        
        // Add WASI
        wasmtime_wasi::add_to_linker(&mut linker, |ctx| ctx.wasi.as_mut().unwrap())
            .map_err(|e| IsolationError::LinkingFailed(format!("Failed to add WASI to linker: {}", e)))?;
        
        // Add host functions (simplified for this example)
        // In a real implementation, we'd add more host functions here
        
        Ok(Self {
            module_store,
            engine,
            linker,
            instances: DashMap::new(),
            max_memory,
        })
    }
    
    /// Compile a WASM module.
    fn compile_module(&self, code: &[u8]) -> Result<Arc<WasmModule>> {
        // Check if we have a cached module
        if let Some(module) = self.module_store.get_module(code) {
            return Ok(module);
        }
        
        // Compile the module
        let module = wasmtime::Module::new(&self.engine, code)
            .map_err(|e| IsolationError::CompilationFailed(format!("Failed to compile WASM module: {}", e)))?;
        
        // Wrap in our module type
        let wasm_module = Arc::new(WasmModule::new(module));
        
        // Store in the cache
        self.module_store.add_module(code, wasm_module.clone())?;
        
        Ok(wasm_module)
    }
}

impl CoreIsolationBackend for WasmIsolationBackend {
    fn load_plugin(
        &self,
        plugin_id: PluginId,
        code: Vec<u8>,
        config: PluginConfig,
    ) -> Result<()> {
        // Check if plugin already exists
        if self.instances.contains_key(&plugin_id) {
            return Err(IsolationError::LoadFailed(
                format!("Plugin {} already loaded", plugin_id)
            ).into());
        }
        
        // Compile the module
        let module = self.compile_module(&code)?;
        
        // Create a new WASM isolation
        let isolation = WasmIsolation::new(
            plugin_id.clone(),
            module,
            &self.engine,
            &self.linker,
            config,
        )?;
        
        // Store the instance
        self.instances.insert(plugin_id, isolation);
        
        Ok(())
    }
    
    fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>> {
        // Get the instance
        let instance = self.instances.get(plugin_id)
            .ok_or_else(|| IsolationError::PluginNotLoaded(plugin_id.clone()))?;
        
        // Call the function
        instance.call_function(function, params)
    }
    
    fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        // Check if plugin exists
        if !self.instances.contains_key(plugin_id) {
            return Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into());
        }
        
        // Remove the instance
        self.instances.remove(plugin_id);
        
        Ok(())
    }
    
    fn list_functions(&self, plugin_id: &PluginId) -> Result<Vec<String>> {
        // Get the instance
        let instance = self.instances.get(plugin_id)
            .ok_or_else(|| IsolationError::PluginNotLoaded(plugin_id.clone()))?;
        
        // Get the functions
        instance.list_functions()
    }
    
    fn get_resource_usage(&self, plugin_id: &PluginId) -> Result<ResourceUsage> {
        // Get the instance
        let instance = self.instances.get(plugin_id)
            .ok_or_else(|| IsolationError::PluginNotLoaded(plugin_id.clone()))?;
        
        // Get the resource usage
        instance.get_resource_usage()
    }
}

impl IsolationBackend for WasmIsolationBackend {
    fn initialize(&mut self) -> Result<()> {
        // Nothing to do here
        Ok(())
    }
    
    fn shutdown(&self) -> Result<()> {
        // Unload all plugins
        let plugin_ids: Vec<PluginId> = self.instances.iter()
            .map(|entry| entry.key().clone())
            .collect();
        
        for plugin_id in plugin_ids {
            let _ = self.unload_plugin(&plugin_id);
        }
        
        Ok(())
    }
}