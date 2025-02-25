//! Host functions that are exposed to WebAssembly modules.
//!
//! This module defines the functions that plugins can call to interact with
//! the host system, such as logging, file access, network operations, and
//! inter-plugin communication.

use crate::error::WasmRuntimeError;
use crate::instance::HostState;
use lion_core::capability::CoreCapability;
use lion_core::message::TopicId;
use lion_core::plugin::PluginId;
use std::sync::Arc;
use wasmtime::{Caller, Extern, Linker, Memory, Store, Val, ValType};

/// Register all host functions with a linker
pub fn register_host_functions<T>(
    linker: &mut Linker<T>,
    plugin_id: PluginId,
    capability_manager: Arc<dyn lion_core::capability::CapabilityManager>,
    message_bus: Arc<dyn lion_core::message::MessageBus>,
) -> Result<(), WasmRuntimeError>
where
    T: Send + AsRef<HostState>,
{
    // Register logging functions
    register_logging_functions(linker)?;
    
    // Register file system functions if the plugin has the capability
    if capability_manager.has_capability(plugin_id, &CoreCapability::FileSystemRead { path: None }) ||
       capability_manager.has_capability(plugin_id, &CoreCapability::FileSystemWrite { path: None }) {
        register_filesystem_functions(linker, plugin_id, capability_manager.clone())?;
    }
    
    // Register network functions if the plugin has the capability
    if capability_manager.has_capability(plugin_id, &CoreCapability::NetworkClient { hosts: None }) {
        register_network_functions(linker, plugin_id, capability_manager.clone())?;
    }
    
    // Register messaging functions if the plugin has the capability
    if capability_manager.has_capability(plugin_id, &CoreCapability::InterPluginComm) {
        register_messaging_functions(linker, plugin_id, capability_manager.clone(), message_bus.clone())?;
    }
    
    // Register utility functions (always available)
    register_utility_functions(linker)?;
    
    Ok(())
}

/// Register logging functions
fn register_logging_functions<T>(linker: &mut Linker<T>) -> Result<(), WasmRuntimeError>
where
    T: Send + AsRef<HostState>,
{
    // log_message(level: i32, ptr: i32, len: i32) -> void
    linker.func_wrap("env", "log_message", |caller: Caller<'_, T>, level: i32, ptr: i32, len: i32| {
        let level = match level {
            0 => log::Level::Error,
            1 => log::Level::Warn,
            2 => log::Level::Info,
            3 => log::Level::Debug,
            _ => log::Level::Trace,
        };
        
        let state = caller.as_ref();
        let plugin_id = state.plugin_id;
        let plugin_name = state.plugin_name.clone();
        
        // Get the WebAssembly memory
        let memory = match get_memory(&caller) {
            Ok(memory) => memory,
            Err(e) => {
                log::error!("Failed to get memory in log_message: {}", e);
                return;
            }
        };
        
        // Read the message from WebAssembly memory
        let message = match read_string_from_memory(&memory, ptr, len) {
            Ok(message) => message,
            Err(e) => {
                log::error!("Failed to read message in log_message: {}", e);
                return;
            }
        };
        
        log::log!(level, "[Plugin {}:{}] {}", plugin_name, plugin_id.0, message);
    }).map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
    
    Ok(())
}

/// Register file system functions
fn register_filesystem_functions<T>(
    linker: &mut Linker<T>,
    plugin_id: PluginId,
    capability_manager: Arc<dyn lion_core::capability::CapabilityManager>,
) -> Result<(), WasmRuntimeError>
where
    T: Send + AsRef<HostState>,
{
    // fs_read_file(path_ptr: i32, path_len: i32) -> i32
    linker.func_wrap("env", "fs_read_file", move |caller: Caller<'_, T>, path_ptr: i32, path_len: i32| -> i32 {
        let state = caller.as_ref();
        let memory = match get_memory(&caller) {
            Ok(memory) => memory,
            Err(e) => {
                log::error!("Failed to get memory in fs_read_file: {}", e);
                return -1;
            }
        };
        
        // Read the path from WebAssembly memory
        let path = match read_string_from_memory(&memory, path_ptr, path_len) {
            Ok(path) => path,
            Err(e) => {
                log::error!("Failed to read path in fs_read_file: {}", e);
                return -1;
            }
        };
        
        // Check if the plugin has the capability to read files
        if !capability_manager.has_capability(plugin_id, &CoreCapability::FileSystemRead { path: None }) {
            log::warn!("Plugin {} attempted to read file without capability", plugin_id.0);
            return -2;  // Permission denied
        }
        
        // Implement actual file reading
        // For MVP, this can be simplistic
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                // Store the content in the plugin's state for retrieval
                // In a real implementation, this would be managed more efficiently
                state.store_file_content(&path, content);
                0  // Success
            }
            Err(e) => {
                log::error!("Failed to read file {}: {}", path, e);
                -3  // IO error
            }
        }
    }).map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
    
    // Add more file system functions here (write, delete, etc.)
    
    Ok(())
}

/// Register network functions
fn register_network_functions<T>(
    linker: &mut Linker<T>,
    plugin_id: PluginId,
    capability_manager: Arc<dyn lion_core::capability::CapabilityManager>,
) -> Result<(), WasmRuntimeError>
where
    T: Send + AsRef<HostState>,
{
    // For MVP, implement minimal network functionality
    // In a production system, this would be more comprehensive
    
    // http_get(url_ptr: i32, url_len: i32) -> i32
    linker.func_wrap("env", "http_get", move |caller: Caller<'_, T>, url_ptr: i32, url_len: i32| -> i32 {
        let state = caller.as_ref();
        let memory = match get_memory(&caller) {
            Ok(memory) => memory,
            Err(e) => {
                log::error!("Failed to get memory in http_get: {}", e);
                return -1;
            }
        };
        
        // Read the URL from WebAssembly memory
        let url = match read_string_from_memory(&memory, url_ptr, url_len) {
            Ok(url) => url,
            Err(e) => {
                log::error!("Failed to read URL in http_get: {}", e);
                return -1;
            }
        };
        
        // Check if the plugin has the capability to access the network
        if !capability_manager.has_capability(plugin_id, &CoreCapability::NetworkClient { hosts: None }) {
            log::warn!("Plugin {} attempted to access network without capability", plugin_id.0);
            return -2;  // Permission denied
        }
        
        // Implement actual HTTP GET
        // For MVP, this can be simplistic
        match ureq::get(&url).call() {
            Ok(response) => {
                match response.into_string() {
                    Ok(content) => {
                        // Store the content in the plugin's state for retrieval
                        state.store_http_response(&url, content);
                        0  // Success
                    }
                    Err(e) => {
                        log::error!("Failed to read HTTP response body: {}", e);
                        -3  // Response error
                    }
                }
            }
            Err(e) => {
                log::error!("HTTP request failed for {}: {}", url, e);
                -3  // Request error
            }
        }
    }).map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
    
    // Add more network functions here
    
    Ok(())
}

/// Register messaging functions
fn register_messaging_functions<T>(
    linker: &mut Linker<T>,
    plugin_id: PluginId,
    capability_manager: Arc<dyn lion_core::capability::CapabilityManager>,
    message_bus: Arc<dyn lion_core::message::MessageBus>,
) -> Result<(), WasmRuntimeError>
where
    T: Send + AsRef<HostState>,
{
    // publish_message(topic_ptr: i32, topic_len: i32, msg_ptr: i32, msg_len: i32) -> i32
    let message_bus_clone = message_bus.clone();
    linker.func_wrap("env", "publish_message", move |caller: Caller<'_, T>,
                                                     topic_ptr: i32, topic_len: i32,
                                                     msg_ptr: i32, msg_len: i32| -> i32 {
        let memory = match get_memory(&caller) {
            Ok(memory) => memory,
            Err(e) => {
                log::error!("Failed to get memory in publish_message: {}", e);
                return -1;
            }
        };
        
        // Read the topic from WebAssembly memory
        let topic = match read_string_from_memory(&memory, topic_ptr, topic_len) {
            Ok(topic) => topic,
            Err(e) => {
                log::error!("Failed to read topic in publish_message: {}", e);
                return -1;
            }
        };
        
        // Read the message from WebAssembly memory
        let message_str = match read_string_from_memory(&memory, msg_ptr, msg_len) {
            Ok(message) => message,
            Err(e) => {
                log::error!("Failed to read message in publish_message: {}", e);
                return -1;
            }
        };
        
        // Check if the plugin has the capability to publish messages
        if !capability_manager.has_capability(plugin_id, &CoreCapability::InterPluginComm) {
            log::warn!("Plugin {} attempted to publish message without capability", plugin_id.0);
            return -2;  // Permission denied
        }
        
        // Parse the message as JSON
        let message_json = match serde_json::from_str::<serde_json::Value>(&message_str) {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to parse message as JSON: {}", e);
                return -3;  // Invalid JSON
            }
        };
        
        // Publish the message
        match message_bus_clone.publish(plugin_id, TopicId(topic), message_json) {
            Ok(()) => 0,  // Success
            Err(e) => {
                log::error!("Failed to publish message: {}", e);
                -4  // Publish error
            }
        }
    }).map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
    
    // send_direct_message(target_plugin_id_ptr: i32, target_plugin_id_len: i32, msg_ptr: i32, msg_len: i32) -> i32
    let message_bus_clone = message_bus.clone();
    linker.func_wrap("env", "send_direct_message", move |caller: Caller<'_, T>,
                                                         target_plugin_id_ptr: i32, target_plugin_id_len: i32,
                                                         msg_ptr: i32, msg_len: i32| -> i32 {
        let memory = match get_memory(&caller) {
            Ok(memory) => memory,
            Err(e) => {
                log::error!("Failed to get memory in send_direct_message: {}", e);
                return -1;
            }
        };
        
        // Read the target plugin ID from WebAssembly memory
        let target_plugin_id_str = match read_string_from_memory(&memory, target_plugin_id_ptr, target_plugin_id_len) {
            Ok(id) => id,
            Err(e) => {
                log::error!("Failed to read target plugin ID in send_direct_message: {}", e);
                return -1;
            }
        };
        
        // Parse the target plugin ID
        let target_plugin_id = match uuid::Uuid::parse_str(&target_plugin_id_str) {
            Ok(uuid) => PluginId(uuid),
            Err(e) => {
                log::error!("Failed to parse target plugin ID: {}", e);
                return -1;
            }
        };
        
        // Read the message from WebAssembly memory
        let message_str = match read_string_from_memory(&memory, msg_ptr, msg_len) {
            Ok(message) => message,
            Err(e) => {
                log::error!("Failed to read message in send_direct_message: {}", e);
                return -1;
            }
        };
        
        // Check if the plugin has the capability to send messages
        if !capability_manager.has_capability(plugin_id, &CoreCapability::InterPluginComm) {
            log::warn!("Plugin {} attempted to send message without capability", plugin_id.0);
            return -2;  // Permission denied
        }
        
        // Parse the message as JSON
        let message_json = match serde_json::from_str::<serde_json::Value>(&message_str) {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to parse message as JSON: {}", e);
                return -3;  // Invalid JSON
            }
        };
        
        // Send the message
        match message_bus_clone.send_direct(plugin_id, target_plugin_id, message_json) {
            Ok(()) => 0,  // Success
            Err(e) => {
                log::error!("Failed to send direct message: {}", e);
                -4  // Send error
            }
        }
    }).map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
    
    // subscribe_to_topic(topic_ptr: i32, topic_len: i32) -> i32
    let message_bus_clone = message_bus.clone();
    linker.func_wrap("env", "subscribe_to_topic", move |caller: Caller<'_, T>,
                                                        topic_ptr: i32, topic_len: i32| -> i32 {
        let memory = match get_memory(&caller) {
            Ok(memory) => memory,
            Err(e) => {
                log::error!("Failed to get memory in subscribe_to_topic: {}", e);
                return -1;
            }
        };
        
        // Read the topic from WebAssembly memory
        let topic = match read_string_from_memory(&memory, topic_ptr, topic_len) {
            Ok(topic) => topic,
            Err(e) => {
                log::error!("Failed to read topic in subscribe_to_topic: {}", e);
                return -1;
            }
        };
        
        // Check if the plugin has the capability to subscribe to topics
        if !capability_manager.has_capability(plugin_id, &CoreCapability::InterPluginComm) {
            log::warn!("Plugin {} attempted to subscribe without capability", plugin_id.0);
            return -2;  // Permission denied
        }
        
        // Subscribe to the topic
        match message_bus_clone.subscribe(plugin_id, TopicId(topic)) {
            Ok(()) => 0,  // Success
            Err(e) => {
                log::error!("Failed to subscribe to topic: {}", e);
                -3  // Subscribe error
            }
        }
    }).map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
    
    Ok(())
}

/// Register utility functions
fn register_utility_functions<T>(linker: &mut Linker<T>) -> Result<(), WasmRuntimeError>
where
    T: Send + AsRef<HostState>,
{
    // current_time_ms() -> i64
    linker.func_wrap("env", "current_time_ms", |_caller: Caller<'_, T>| -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        now.as_millis() as i64
    }).map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
    
    // random_u32() -> i32
    linker.func_wrap("env", "random_u32", |_caller: Caller<'_, T>| -> i32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        // Simple random implementation for MVP
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        (now.as_nanos() % 0xFFFFFFFF) as i32
    }).map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
    
    Ok(())
}

/// Get the memory from a WebAssembly instance
fn get_memory<T>(caller: &Caller<'_, T>) -> Result<Memory, WasmRuntimeError> {
    let memory = caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or(WasmRuntimeError::MemoryNotFound)?;
    
    Ok(memory)
}

/// Read a string from WebAssembly memory
fn read_string_from_memory(memory: &Memory, ptr: i32, len: i32) -> Result<String, WasmRuntimeError> {
    if ptr < 0 || len < 0 {
        return Err(WasmRuntimeError::InvalidWebAssembly(
            "Negative pointer or length".to_string(),
        ));
    }
    
    let offset = ptr as usize;
    let length = len as usize;
    
    // Validate that the memory region is within bounds
    if offset + length > memory.data_size() {
        return Err(WasmRuntimeError::InvalidWebAssembly(
            "Memory access out of bounds".to_string(),
        ));
    }
    
    // Read the bytes from memory
    let mut buffer = vec![0u8; length];
    memory
        .read(offset, &mut buffer)
        .map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))?;
    
    // Convert bytes to a string
    String::from_utf8(buffer)
        .map_err(|e| WasmRuntimeError::InvalidWebAssembly(format!("Invalid UTF-8: {}", e)))
}

/// Write a string to WebAssembly memory
#[allow(dead_code)] // Will be used in future host functions
fn write_string_to_memory(memory: &Memory, ptr: i32, len: i32, data: &str) -> Result<(), WasmRuntimeError> {
    if ptr < 0 || len < 0 {
        return Err(WasmRuntimeError::InvalidWebAssembly(
            "Negative pointer or length".to_string(),
        ));
    }
    
    let offset = ptr as usize;
    let length = len as usize;
    
    // Validate that the memory region is within bounds
    if offset + length > memory.data_size() {
        return Err(WasmRuntimeError::InvalidWebAssembly(
            "Memory access out of bounds".to_string(),
        ));
    }
    
    // Ensure the string fits in the provided buffer
    if data.len() > length {
        return Err(WasmRuntimeError::InvalidWebAssembly(
            "String too long for buffer".to_string(),
        ));
    }
    
    // Write the string bytes to memory
    memory
        .write(offset, data.as_bytes())
        .map_err(|e| WasmRuntimeError::Wasmtime(e.to_string()))
}