//! WebAssembly instance management.
//!
//! This module provides functionality for creating and managing instances
//! of WebAssembly modules, including managing their state and resources.

use crate::config::WasmInstanceConfig;
use crate::error::WasmIsolationError;
use crate::host_functions::register_host_functions;
use crate::memory::HostState;
use crate::module::WasmModule;
use lion_core::capability::CapabilityManager;
use lion_core::message::MessageBus;
use lion_core::plugin::{Plugin, PluginId, PluginState};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use wasmtime::{Instance, Linker, Store, StoreLimits, StoreLimitsBuilder};

/// A WebAssembly instance
pub struct WasmInstance {
    /// The plugin ID
    plugin_id: PluginId,
    
    /// The plugin name
    plugin_name: String,
    
    /// The WebAssembly module
    module: WasmModule,
    
    /// The WebAssembly store with host state
    store: Mutex<Store<HostState>>,
    
    /// The WebAssembly instance
    instance: Instance,
    
    /// The current state of the plugin
    state: Mutex<PluginState>,
    
    /// The configuration
    config: WasmInstanceConfig,
}

impl WasmInstance {
    /// Create a new WebAssembly instance
    pub fn new(
        module: WasmModule,
        plugin_id: PluginId,
        plugin_name: String,
        capability_manager: Arc<dyn CapabilityManager>,
        message_bus: Arc<dyn MessageBus>,
        config: WasmInstanceConfig,
    ) -> Result<Self, WasmIsolationError> {
        // Create store limits
        let limits = StoreLimitsBuilder::new()
            .memory_size(config.memory_limit)
            .build();
        
        // Create the engine and store
        let engine = wasmtime::Engine::default();
        let mut store = Store::new(
            &engine,
            HostState::new(plugin_id, plugin_name.clone()),
        );
        
        // Set resource limits
        store.limiter(|_| limits.clone());
        
        // Set fuel if enabled
        if let Some(fuel) = config.fuel_limit {
            store.add_fuel(fuel)
                .map_err(|e| WasmIsolationError::Wasmtime(e.to_string()))?;
        }
        
        // Create a linker
        let mut linker = Linker::new(&engine);
        
        // Register host functions
        register_host_functions(&mut linker, plugin_id, capability_manager, message_bus)?;
        
        // Instantiate the module
        let instance = linker
            .instantiate(&mut store, module.module())
            .map_err(|e| WasmIsolationError::InstantiationFailed(e.to_string()))?;
        
        Ok(Self {
            plugin_id,
            plugin_name,
            module,
            store: Mutex::new(store),
            instance,
            state: Mutex::new(PluginState::Created),
            config,
        })
    }
    
    /// Initialize the instance
    pub fn initialize(&self) -> Result<(), WasmIsolationError> {
        // Update state
        {
            let mut state = self.state.lock().unwrap();
            *state = PluginState::Initializing;
        }
        
        let mut store = self.store.lock().unwrap();
        
        // Start timing
        store.data().start_execution();
        
        // Call the initialize function
        match self.instance.get_func(&mut *store, "initialize") {
            Some(func) => {
                let result = func
                    .call(&mut *store, &[], &mut [])
                    .map_err(|e| WasmIsolationError::ExecutionError(e.to_string()))?;
                
                // End timing
                store.data().end_execution();
                
                // Check result (assuming it returns a status code)
                if !result.is_empty() {
                    if let Some(status) = result[0].i32() {
                        if status != 0 {
                            let mut state = self.state.lock().unwrap();
                            *state = PluginState::Failed;
                            return Err(WasmIsolationError::ExecutionError(
                                format!("Initialization failed with status: {}", status)
                            ));
                        }
                    }
                }
                
                // Update state
                let mut state = self.state.lock().unwrap();
                *state = PluginState::Ready;
                
                Ok(())
            }
            None => {
                // If no initialize function, just mark as ready
                store.data().end_execution();
                let mut state = self.state.lock().unwrap();
                *state = PluginState::Ready;
                Ok(())
            }
        }
    }
    
    /// Handle a message
    pub fn handle_message(&self, message: serde_json::Value) -> Result<Option<serde_json::Value>, WasmIsolationError> {
        // Check state
        {
            let mut state = self.state.lock().unwrap();
            if *state != PluginState::Ready {
                return Err(WasmIsolationError::ExecutionError(
                    format!("Plugin is not in Ready state: {:?}", *state)
                ));
            }
            *state = PluginState::Processing;
        }
        
        let mut store = self.store.lock().unwrap();
        
        // Start timing
        store.data().start_execution();
        store.data().increment_messages_processed();
        
        // Convert message to string
        let message_str = serde_json::to_string(&message)
            .map_err(|e| WasmIsolationError::Serialization(e.to_string()))?;
        
        // Get the handle_message function
        let handle_message = self.instance
            .get_func(&mut *store, "handle_message")
            .ok_or_else(|| WasmIsolationError::FunctionNotFound("handle_message".to_string()))?;
        
        // In a real implementation, we would:
        // 1. Allocate memory in the WebAssembly instance
        // 2. Write the message JSON to that memory
        // 3. Call handle_message with the pointer and length
        // 4. Read the result from memory
        // 5. Deallocate the memory
        
        // For simplicity in this example, we'll just call the function with no args
        // and return a mock response
        let result = handle_message
            .call(&mut *store, &[], &mut [])
            .map_err(|e| WasmIsolationError::ExecutionError(e.to_string()))?;
        
        // End timing
        store.data().end_execution();
        
        // Update state
        let mut state = self.state.lock().unwrap();
        *state = PluginState::Ready;
        
        // Process result
        // This is a simplified version - in a real implementation, we would
        // handle result values properly based on the WebAssembly interface
        
        // For MVP, return a default response
        Ok(Some(serde_json::json!({
            "status": "processed",
            "plugin_id": self.plugin_id.0.to_string(),
            "plugin_name": self.plugin_name,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })))
    }
    
    /// Shutdown the instance
    pub fn shutdown(&self) -> Result<(), WasmIsolationError> {
        // Update state
        {
            let mut state = self.state.lock().unwrap();
            if *state == PluginState::Terminated {
                return Ok(());  // Already terminated
            }
            *state = PluginState::Terminated;
        }
        
        let mut store = self.store.lock().unwrap();
        
        // Start timing
        store.data().start_execution();
        
        // Call the shutdown function
        match self.instance.get_func(&mut *store, "shutdown") {
            Some(func) => {
                let _ = func.call(&mut *store, &[], &mut []);
                // Ignore errors during shutdown
            }
            None => {
                // No shutdown function, nothing to do
            }
        }
        
        // End timing
        store.data().end_execution();
        
        Ok(())
    }
    
    /// Get the plugin ID
    pub fn plugin_id(&self) -> PluginId {
        self.plugin_id
    }
    
    /// Get the plugin name
    pub fn plugin_name(&self) -> &str {
        &self.plugin_name
    }
    
    /// Get the current state
    pub fn state(&self) -> PluginState {
        *self.state.lock().unwrap()
    }
    
    /// Get memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        let store = self.store.lock().unwrap();
        store.data_size()
    }
    
    /// Get total execution time
    pub fn execution_time(&self) -> Duration {
        let store = self.store.lock().unwrap();
        store.data().total_execution_time()
    }
    
    /// Get messages processed count
    pub fn messages_processed(&self) -> u64 {
        let store = self.store.lock().unwrap();
        store.data().messages_processed()
    }
}

impl Plugin for WasmInstance {
    fn id(&self) -> PluginId {
        self.plugin_id
    }
    
    fn name(&self) -> &str {
        &self.plugin_name
    }
    
    fn state(&self) -> PluginState {
        *self.state.lock().unwrap()
    }
    
    fn initialize(&mut self) -> Result<(), lion_core::error::PluginError> {
        self.initialize().map_err(|e| 
            lion_core::error::PluginError::InitializationFailure(e.to_string())
        )
    }
    
    fn handle_message(
        &mut self,
        message: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, lion_core::error::PluginError> {
        self.handle_message(message).map_err(|e| 
            lion_core::error::PluginError::ExecutionError(e.to_string())
        )
    }
    
    fn shutdown(&mut self) -> Result<(), lion_core::error::PluginError> {
        self.shutdown().map_err(|e| 
            lion_core::error::PluginError::ExecutionError(e.to_string())
        )
    }
    
    fn clone_box(&self) -> Box<dyn Plugin> {
        // We can't actually clone a WasmInstance due to the mutex and store,
        // so we'll return an error. In a real implementation, you might
        // want to handle this differently.
        panic!("WasmInstance cannot be cloned directly");
    }
}