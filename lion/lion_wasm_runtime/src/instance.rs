//! WebAssembly instance management.
//!
//! This module provides functionality for creating and managing instances
//! of WebAssembly modules, including managing their state and resources.

use crate::error::WasmRuntimeError;
use crate::host_functions::register_host_functions;
use crate::module::WasmModule;
use lion_core::capability::CapabilityManager;
use lion_core::message::MessageBus;
use lion_core::plugin::{Plugin, PluginId, PluginState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use wasmtime::{Instance, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

/// Configuration for a WebAssembly instance
#[derive(Clone)]
pub struct WasmInstanceConfig {
    /// Maximum memory size in bytes
    pub memory_limit: usize,
    
    /// Maximum execution time for a single call
    pub execution_time_limit: Duration,
    
    /// Optional fuel limit (for instruction counting)
    pub fuel_limit: Option<u64>,
}

impl Default for WasmInstanceConfig {
    fn default() -> Self {
        Self {
            memory_limit: 100 * 1024 * 1024, // 100 MB
            execution_time_limit: Duration::from_secs(5),
            fuel_limit: Some(10_000_000), // 10 million instructions
        }
    }
}

/// Host state for WebAssembly instances
pub struct HostState {
    /// The ID of the plugin this instance belongs to
    pub plugin_id: PluginId,
    
    /// The name of the plugin
    pub plugin_name: String,
    
    /// Store HTTP responses
    http_responses: Mutex<HashMap<String, String>>,
    
    /// Store file contents
    file_contents: Mutex<HashMap<String, String>>,
    
    /// Store for last execution time
    last_execution_start: Mutex<Option<Instant>>,
    
    /// Store for total execution time
    total_execution_time: Mutex<Duration>,
}

impl HostState {
    /// Create a new host state
    pub fn new(plugin_id: PluginId, plugin_name: String) -> Self {
        Self {
            plugin_id,
            plugin_name,
            http_responses: Mutex::new(HashMap::new()),
            file_contents: Mutex::new(HashMap::new()),
            last_execution_start: Mutex::new(None),
            total_execution_time: Mutex::new(Duration::from_secs(0)),
        }
    }
    
    /// Store an HTTP response
    pub fn store_http_response(&self, url: &str, content: String) {
        let mut responses = self.http_responses.lock().unwrap();
        responses.insert(url.to_string(), content);
    }
    
    /// Get an HTTP response
    pub fn get_http_response(&self, url: &str) -> Option<String> {
        let responses = self.http_responses.lock().unwrap();
        responses.get(url).cloned()
    }
    
    /// Store file content
    pub fn store_file_content(&self, path: &str, content: String) {
        let mut contents = self.file_contents.lock().unwrap();
        contents.insert(path.to_string(), content);
    }
    
    /// Get file content
    pub fn get_file_content(&self, path: &str) -> Option<String> {
        let contents = self.file_contents.lock().unwrap();
        contents.get(path).cloned()
    }
    
    /// Mark the start of an execution
    pub fn start_execution(&self) {
        let mut start = self.last_execution_start.lock().unwrap();
        *start = Some(Instant::now());
    }
    
    /// Mark the end of an execution and update total time
    pub fn end_execution(&self) {
        let mut start = self.last_execution_start.lock().unwrap();
        if let Some(start_time) = *start {
            let elapsed = start_time.elapsed();
            let mut total = self.total_execution_time.lock().unwrap();
            *total += elapsed;
            *start = None;
        }
    }
    
    /// Get the total execution time
    pub fn total_execution_time(&self) -> Duration {
        *self.total_execution_time.lock().unwrap()
    }
}

/// A WebAssembly instance
pub struct WasmInstance {
    /// The plugin ID
    plugin_id: PluginId,
    
    /// The plugin name
    plugin_name: String,
    
    /// The WebAssembly module
    module: WasmModule,
    
    /// The WebAssembly store
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
    ) -> Result<Self, WasmRuntimeError> {
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
                .map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
        }
        
        // Create a linker
        let mut linker = Linker::new(&engine);
        
        // Register host functions
        register_host_functions(&mut linker, plugin_id, capability_manager, message_bus)?;
        
        // Instantiate the module
        let instance = linker
            .instantiate(&mut store, module.module())
            .map_err(|e| WasmRuntimeError::InstantiationFailed(e.to_string()))?;
        
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
    pub fn initialize(&self) -> Result<(), WasmRuntimeError> {
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
                    .map_err(|e| WasmRuntimeError::ExecutionError(e.to_string()))?;
                
                // End timing
                store.data().end_execution();
                
                // Check result (assuming it returns a status code)
                if !result.is_empty() {
                    if let Some(status) = result[0].i32() {
                        if status != 0 {
                            let mut state = self.state.lock().unwrap();
                            *state = PluginState::Failed;
                            return Err(WasmRuntimeError::ExecutionError(
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
    pub fn handle_message(&self, message: serde_json::Value) -> Result<Option<serde_json::Value>, WasmRuntimeError> {
        // Check state
        {
            let mut state = self.state.lock().unwrap();
            if *state != PluginState::Ready {
                return Err(WasmRuntimeError::ExecutionError(
                    format!("Plugin is not in Ready state: {:?}", *state)
                ));
            }
            *state = PluginState::Processing;
        }
        
        let mut store = self.store.lock().unwrap();
        
        // Start timing
        store.data().start_execution();
        
        // Convert message to string
        let message_str = serde_json::to_string(&message)
            .map_err(|e| WasmRuntimeError::Serialization(e.to_string()))?;
        
        // Get the handle_message function
        let handle_message = self.instance
            .get_func(&mut *store, "handle_message")
            .ok_or_else(|| WasmRuntimeError::FunctionNotFound("handle_message".to_string()))?;
        
        // Allocate memory for the message
        // In a more robust implementation, we would use exported memory allocation functions
        // For simplicity in this MVP, we'll assume the function directly takes string pointers/lengths
        
        // Call the function with the message JSON
        // The exact parameters depend on the WebAssembly interface
        let result = handle_message
            .call(&mut *store, &[], &mut [])
            .map_err(|e| WasmRuntimeError::ExecutionError(e.to_string()))?;
        
        // End timing
        store.data().end_execution();
        
        // Update state
        let mut state = self.state.lock().unwrap();
        *state = PluginState::Ready;
        
        // Process result
        // This is a simplified version - in a real implementation, we would
        // handle result values properly based on the WebAssembly interface
        if !result.is_empty() {
            if let Some(status) = result[0].i32() {
                if status != 0 {
                    return Err(WasmRuntimeError::ExecutionError(
                        format!("Message handling failed with status: {}", status)
                    ));
                }
            }
        }
        
        // For MVP, we'll return a default response
        // In a real implementation, we would extract the actual response from WebAssembly
        Ok(Some(serde_json::json!({
            "status": "processed",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })))
    }
    
    /// Shutdown the instance
    pub fn shutdown(&self) -> Result<(), WasmRuntimeError> {
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
        // This is an approximation - in a real implementation we would
        // measure actual memory usage more precisely
        store.data_size()
    }
    
    /// Get total execution time
    pub fn execution_time(&self) -> Duration {
        let store = self.store.lock().unwrap();
        store.data().total_execution_time()
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
        self.initialize().map_err(Into::into)
    }
    
    fn handle_message(
        &mut self,
        message: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, lion_core::error::PluginError> {
        self.handle_message(message).map_err(Into::into)
    }
    
    fn shutdown(&mut self) -> Result<(), lion_core::error::PluginError> {
        self.shutdown().map_err(Into::into)
    }
}