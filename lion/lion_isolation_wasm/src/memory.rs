//! Utilities for working with WebAssembly memory.

use crate::error::WasmIsolationError;
use wasmtime::Memory;

/// Read a string from WebAssembly memory
pub fn read_string_from_memory(memory: &Memory, ptr: i32, len: i32) -> Result<String, WasmIsolationError> {
    if ptr < 0 || len < 0 {
        return Err(WasmIsolationError::InvalidWebAssembly(
            "Negative pointer or length".to_string(),
        ));
    }
    
    let offset = ptr as usize;
    let length = len as usize;
    
    // Validate that the memory region is within bounds
    if offset + length > memory.data_size() {
        return Err(WasmIsolationError::InvalidWebAssembly(
            "Memory access out of bounds".to_string(),
        ));
    }
    
    // Read the bytes from memory
    let mut buffer = vec![0u8; length];
    memory
        .read(offset, &mut buffer)
        .map_err(|e| WasmIsolationError::Wasmtime(e.to_string()))?;
    
    // Convert bytes to a string
    String::from_utf8(buffer)
        .map_err(|e| WasmIsolationError::InvalidWebAssembly(format!("Invalid UTF-8: {}", e)))
}

/// Write a string to WebAssembly memory
pub fn write_string_to_memory(memory: &Memory, ptr: i32, len: i32, data: &str) -> Result<(), WasmIsolationError> {
    if ptr < 0 || len < 0 {
        return Err(WasmIsolationError::InvalidWebAssembly(
            "Negative pointer or length".to_string(),
        ));
    }
    
    let offset = ptr as usize;
    let length = len as usize;
    
    // Validate that the memory region is within bounds
    if offset + length > memory.data_size() {
        return Err(WasmIsolationError::InvalidWebAssembly(
            "Memory access out of bounds".to_string(),
        ));
    }
    
    // Ensure the string fits in the provided buffer
    if data.len() > length {
        return Err(WasmIsolationError::InvalidWebAssembly(
            "String too long for buffer".to_string(),
        ));
    }
    
    // Write the string bytes to memory
    memory
        .write(offset, data.as_bytes())
        .map_err(|e| WasmIsolationError::Wasmtime(e.to_string()))
}

/// Get memory from a caller
pub fn get_memory_from_caller<T>(caller: &wasmtime::Caller<'_, T>) -> Result<Memory, WasmIsolationError> {
    caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or(WasmIsolationError::MemoryNotFound)
}

/// Allocate memory in a WebAssembly instance
pub fn allocate_in_instance(
    store: &mut wasmtime::Store<impl AsRef<HostState>>,
    instance: &wasmtime::Instance,
    size: usize,
) -> Result<i32, WasmIsolationError> {
    // Get the allocate function
    let allocate = instance
        .get_func(store, "allocate")
        .ok_or_else(|| WasmIsolationError::FunctionNotFound("allocate".to_string()))?;
    
    // Call the allocate function
    let mut results = [wasmtime::Val::I32(0)];
    allocate
        .call(store, &[wasmtime::Val::I32(size as i32)], &mut results)
        .map_err(|e| WasmIsolationError::ExecutionError(format!("Failed to allocate memory: {}", e)))?;
    
    match results[0].i32() {
        Some(ptr) => Ok(ptr),
        None => Err(WasmIsolationError::TypeMismatch(
            "Expected i32 return value from allocate".to_string(),
        )),
    }
}

/// Deallocate memory in a WebAssembly instance
pub fn deallocate_in_instance(
    store: &mut wasmtime::Store<impl AsRef<HostState>>,
    instance: &wasmtime::Instance,
    ptr: i32,
    size: usize,
) -> Result<(), WasmIsolationError> {
    // Get the deallocate function
    let deallocate = instance
        .get_func(store, "deallocate")
        .ok_or_else(|| WasmIsolationError::FunctionNotFound("deallocate".to_string()))?;
    
    // Call the deallocate function
    deallocate
        .call(
            store,
            &[wasmtime::Val::I32(ptr), wasmtime::Val::I32(size as i32)],
            &mut [],
        )
        .map_err(|e| WasmIsolationError::ExecutionError(format!("Failed to deallocate memory: {}", e)))?;
    
    Ok(())
}

/// State shared with host functions
pub struct HostState {
    /// The plugin ID
    pub plugin_id: lion_core::plugin::PluginId,
    
    /// The plugin name
    pub plugin_name: String,
    
    /// Cache for HTTP responses
    pub http_cache: dashmap::DashMap<String, String>,
    
    /// Cache for file contents
    pub file_cache: dashmap::DashMap<String, String>,
    
    /// Start time of current execution
    pub execution_start: std::sync::Mutex<Option<std::time::Instant>>,
    
    /// Total execution time
    pub total_execution_time: std::sync::Mutex<std::time::Duration>,
    
    /// Messages processed count
    pub messages_processed: std::sync::atomic::AtomicU64,
}

impl HostState {
    /// Create a new host state
    pub fn new(plugin_id: lion_core::plugin::PluginId, plugin_name: String) -> Self {
        Self {
            plugin_id,
            plugin_name,
            http_cache: dashmap::DashMap::new(),
            file_cache: dashmap::DashMap::new(),
            execution_start: std::sync::Mutex::new(None),
            total_execution_time: std::sync::Mutex::new(std::time::Duration::from_secs(0)),
            messages_processed: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    /// Start measuring execution time
    pub fn start_execution(&self) {
        let mut start = self.execution_start.lock().unwrap();
        *start = Some(std::time::Instant::now());
    }
    
    /// Stop measuring execution time and update total
    pub fn end_execution(&self) {
        let mut start = self.execution_start.lock().unwrap();
        if let Some(start_time) = *start {
            let duration = start_time.elapsed();
            let mut total = self.total_execution_time.lock().unwrap();
            *total += duration;
            *start = None;
        }
    }
    
    /// Get total execution time
    pub fn total_execution_time(&self) -> std::time::Duration {
        *self.total_execution_time.lock().unwrap()
    }
    
    /// Increment messages processed count
    pub fn increment_messages_processed(&self) {
        self.messages_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Get messages processed count
    pub fn messages_processed(&self) -> u64 {
        self.messages_processed.load(std::sync::atomic::Ordering::Relaxed)
    }
}