//! WebAssembly isolation implementation.
//!
//! This module provides a WebAssembly-based isolation mechanism
//! for running plugins securely.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, Duration};

use parking_lot::Mutex;
use wasmtime::{Instance, Module, Store, Memory, Caller};
use wasmtime_wasi::WasiCtx;

use lion_core::error::{Result, IsolationError, PluginError};
use lion_core::types::{PluginId, PluginConfig, ResourceUsage};

/// WebAssembly module wrapper.
pub struct WasmModule {
    /// The compiled module.
    module: Module,
    
    /// Reference count for cleanup.
    ref_count: AtomicUsize,
}

impl WasmModule {
    /// Create a new WebAssembly module.
    pub fn new(module: Module) -> Self {
        Self {
            module,
            ref_count: AtomicUsize::new(1),
        }
    }
    
    /// Get the underlying module.
    pub fn module(&self) -> &Module {
        &self.module
    }
    
    /// Increment the reference count.
    pub fn increment_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Decrement the reference count and return true if this was the last reference.
    pub fn decrement_ref(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::SeqCst) == 1
    }
}

/// WebAssembly execution context.
pub struct WasmContext {
    /// Plugin ID.
    pub plugin_id: PluginId,
    
    /// WASI context for sandboxed I/O.
    pub wasi: Option<WasiCtx>,
    
    /// Resource usage statistics.
    pub resources: ResourceUsage,
    
    /// Plugin configuration.
    pub config: PluginConfig,
    
    /// Start time of the current function call.
    pub call_start: Option<Instant>,
}

impl WasmContext {
    /// Create a new WebAssembly context.
    pub fn new(plugin_id: PluginId, config: PluginConfig) -> Self {
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build();
        
        Self {
            plugin_id,
            wasi: Some(wasi),
            resources: ResourceUsage::new(),
            config,
            call_start: None,
        }
    }
    
    /// Record the start of a function call.
    pub fn start_call(&mut self) {
        self.call_start = Some(Instant::now());
    }
    
    /// Record the end of a function call.
    pub fn end_call(&mut self) {
        if let Some(start) = self.call_start.take() {
            let duration = start.elapsed();
            self.resources.cpu_time_us += duration.as_micros() as u64;
            self.resources.function_calls += 1;
        }
    }
    
    /// Check if we've exceeded the function timeout.
    pub fn check_timeout(&self) -> Result<()> {
        if let (Some(start), Some(timeout_ms)) = (self.call_start, self.config.function_timeout_ms) {
            let elapsed = start.elapsed();
            if elapsed > Duration::from_millis(timeout_ms) {
                return Err(PluginError::Timeout(timeout_ms).into());
            }
        }
        
        Ok(())
    }
}

/// WebAssembly isolation for plugins.
pub struct WasmIsolation {
    /// Plugin ID.
    plugin_id: PluginId,
    
    /// WebAssembly module.
    module: Arc<WasmModule>,
    
    /// WebAssembly store with context.
    store: Mutex<Store<WasmContext>>,
    
    /// WebAssembly instance.
    instance: Mutex<Option<Instance>>,
    
    /// Available functions.
    functions: Vec<String>,
    
    /// Time when this isolation was created.
    created_at: Instant,
}

impl WasmIsolation {
    /// Create a new WebAssembly isolation.
    pub fn new(
        plugin_id: PluginId,
        module: Arc<WasmModule>,
        engine: &wasmtime::Engine,
        linker: &wasmtime::Linker<WasmContext>,
        config: PluginConfig,
    ) -> Result<Self> {
        // Create the context
        let wasm_ctx = WasmContext::new(plugin_id.clone(), config);
        
        // Create the store
        let mut store = Store::new(engine, wasm_ctx);
        
        // Instantiate the module
        let instance = linker.instantiate(&mut store, module.module())
            .map_err(|e| IsolationError::InstantiationFailed(format!("Failed to instantiate WASM module: {}", e)))?;
        
        // Get the available functions
        let functions = Self::get_exported_functions(&instance, &mut store);
        
        Ok(Self {
            plugin_id,
            module,
            store: Mutex::new(store),
            instance: Mutex::new(Some(instance)),
            functions,
            created_at: Instant::now(),
        })
    }
    
    /// Get the exported functions from the module.
    fn get_exported_functions(instance: &Instance, store: &mut Store<WasmContext>) -> Vec<String> {
        let mut functions = Vec::new();
        
        for export in instance.exports(store) {
            if export.ty(store).func().is_some() {
                functions.push(export.name().to_string());
            }
        }
        
        functions
    }
    
    /// Call a function in the module.
    pub fn call_function(&self, function: &str, params: &[u8]) -> Result<Vec<u8>> {
        // Lock the store
        let mut store_lock = self.store.lock();
        let store = &mut *store_lock;
        
        // Get the instance
        let instance_lock = self.instance.lock();
        let instance = instance_lock.as_ref()
            .ok_or_else(|| IsolationError::ExecutionTrap("Instance not initialized".to_string()))?;
        
        // Record the start time
        store.data_mut().start_call();
        
        // Find the function
        let func = instance.get_func(store, function)
            .ok_or_else(|| PluginError::FunctionNotFound(function.to_string()))?;
        
        // Get the alloc function
        let alloc = instance.get_func(store, "alloc")
            .ok_or_else(|| IsolationError::ExecutionTrap("Module does not export alloc function".to_string()))?;
        
        // Get the memory
        let memory = instance.get_memory(store, "memory")
            .ok_or_else(|| IsolationError::ExecutionTrap("Module does not export memory".to_string()))?;
        
        // Allocate memory for params
        let alloc_results = alloc.call(store, &[wasmtime::Val::I32(params.len() as i32)])
            .map_err(|e| IsolationError::ExecutionTrap(format!("Failed to allocate memory: {}", e)))?;
        
        let ptr = match alloc_results[0] {
            wasmtime::Val::I32(ptr) => ptr as usize,
            _ => return Err(IsolationError::ExecutionTrap("Invalid pointer returned from alloc".to_string()).into()),
        };
        
        // Write params to memory
        memory.write(store, ptr, params)
            .map_err(|e| IsolationError::ExecutionTrap(format!("Failed to write params to memory: {}", e)))?;
        
        // Call the function
        let results = func.call(store, &[wasmtime::Val::I32(ptr as i32), wasmtime::Val::I32(params.len() as i32)])
            .map_err(|e| IsolationError::ExecutionTrap(format!("Function call failed: {}", e)))?;
        
        // Check for timeout
        store.data_mut().check_timeout()?;
        
        // Parse the results
        let result_ptr = match results[0] {
            wasmtime::Val::I32(ptr) => ptr as usize,
            _ => return Err(IsolationError::ExecutionTrap("Invalid result pointer".to_string()).into()),
        };
        
        let result_len = match results[1] {
            wasmtime::Val::I32(len) => len as usize,
            _ => return Err(IsolationError::ExecutionTrap("Invalid result length".to_string()).into()),
        };
        
        // Read the result
        let mut result = vec![0u8; result_len];
        memory.read(store, result_ptr, &mut result)
            .map_err(|e| IsolationError::ExecutionTrap(format!("Failed to read result from memory: {}", e)))?;
        
        // Record the end time
        store.data_mut().end_call();
        
        Ok(result)
    }
    
    /// Get the list of available functions.
    pub fn list_functions(&self) -> Result<Vec<String>> {
        Ok(self.functions.clone())
    }
    
    /// Get the current resource usage.
    pub fn get_resource_usage(&self) -> Result<ResourceUsage> {
        let store = self.store.lock();
        Ok(store.data().resources.clone())
    }
}

impl Drop for WasmIsolation {
    fn drop(&mut self) {
        // Decrement the module reference count
        self.module.decrement_ref();
    }
}

// Helper to build WASI context
struct WasiCtxBuilder {
    inherit_stdio: bool,
}

impl WasiCtxBuilder {
    fn new() -> Self {
        Self {
            inherit_stdio: false,
        }
    }
    
    fn inherit_stdio(mut self) -> Self {
        self.inherit_stdio = true;
        self
    }
    
    fn build(self) -> WasiCtx {
        let mut builder = wasmtime_wasi::WasiCtxBuilder::new();
        
        if self.inherit_stdio {
            builder = builder.inherit_stdio();
        }
        
        builder.build()
    }
}