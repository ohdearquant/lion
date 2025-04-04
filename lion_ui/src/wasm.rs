use anyhow::Result;
use axum::{
    extract::{Json, Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;
use wasmtime::{Linker, Module, Store};

use crate::logs::{LogEntry, LogLevel};
use crate::state::{AppState, PluginInfo, WasmPluginInstance};

/// WASM plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub exports: Vec<String>,
}

/// Plugin function parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginParam {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<PluginParam>),
    Object(serde_json::Map<String, serde_json::Value>),
    Null,
}

/// Result of a plugin method invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

/// A context for WASM execution
pub struct WasmContext {
    pub plugin_id: Uuid,
    pub resource_limits: ResourceLimits,
}

/// Resource limits for WASM execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub memory_pages: u32,
    pub max_execution_time_ms: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_pages: 100, // 6.4MB (64KB * 100)
            max_execution_time_ms: 1000,
        }
    }
}

/// Request to load a WASM plugin
#[derive(Debug, Deserialize)]
pub struct LoadWasmPluginRequest {
    /// Path to the plugin file
    pub path: String,

    /// Optional plugin name (if not provided, derived from file name)
    pub name: Option<String>,
}

/// Request to invoke a WASM plugin function
#[derive(Debug, Deserialize)]
pub struct InvokeWasmFunctionRequest {
    /// Function name to call
    pub function: String,

    /// Parameters to pass to the function
    pub params: Vec<PluginParam>,
}

/// Load a WASM plugin from a file
pub async fn load_plugin(
    state: &Arc<AppState>,
    path: &str,
    custom_name: Option<String>,
) -> Result<PluginInfo, String> {
    info!("Loading WASM plugin from: {}", path);

    // Ensure the file exists
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Err(format!("Plugin file not found: {}", path));
    }

    // Get the engine or return an error
    let engine = state
        .wasm_engine
        .as_ref()
        .ok_or_else(|| "WebAssembly engine not initialized".to_string())?;

    // Read the WASM file
    let wasm_bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("Failed to read WASM file: {}", e)),
    };

    // Extract file name for default plugin name
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    let plugin_name = custom_name.unwrap_or_else(|| file_name.to_string());
    let plugin_id = Uuid::new_v4();

    // Try to compile the WASM module
    let module = match Module::new(engine, &wasm_bytes) {
        Ok(module) => module,
        Err(e) => return Err(format!("Failed to compile WASM module: {}", e)),
    };

    // Create a linker for host functions
    let linker = Linker::new(engine);

    // TODO: Add host functions to the linker

    // Create a WASM context
    let wasm_context = WasmContext {
        plugin_id,
        resource_limits: ResourceLimits::default(),
    };

    // Create a store
    let mut store = Store::new(engine, wasm_context);

    // Try to instantiate the module asynchronously since async_support is enabled
    let instance = match linker.instantiate_async(&mut store, &module).await {
        Ok(instance) => instance,
        Err(e) => return Err(format!("Failed to instantiate WASM module: {}", e)),
    };

    // Extract exported functions by name only, to avoid borrow checker issues
    let export_names: Vec<_> = instance
        .exports(&mut store)
        .map(|export| export.name().to_string())
        .collect();

    // Filter for functions
    let exports = export_names;

    // Create a plugin instance
    let wasm_instance = WasmPluginInstance {
        id: plugin_id,
        path: path.to_string(),
        module: Some(module),
        instance: Some(instance),
        memory: None, // Memory will be obtained when needed
        exports,
    };

    // Register the WASM plugin instance
    {
        let mut plugins_wasm = state.plugins_wasm.write().await;
        plugins_wasm.insert(plugin_id, wasm_instance);
    }

    // Create plugin info for the API
    let plugin_info = PluginInfo {
        id: plugin_id,
        name: plugin_name.clone(),
        version: "1.0.0".to_string(), // Default version
        description: "WASM Plugin".to_string(),
    };

    // Register in the main plugins registry
    {
        let mut plugins = state.plugins.write().await;
        plugins.insert(plugin_id, plugin_info.clone());
    }

    // Log the plugin loading
    let log_entry = LogEntry::new(
        LogLevel::Info,
        format!("WASM Plugin '{}' loaded from {}", plugin_name, path),
        "wasm-engine",
    )
    .with_plugin_id(plugin_id);

    state.log(log_entry).await;

    Ok(plugin_info)
}

/// Invoke a WASM plugin function
pub async fn invoke_plugin_function(
    state: &Arc<AppState>,
    plugin_id: Uuid,
    function_name: &str,
    params: Vec<PluginParam>,
) -> Result<PluginResult, String> {
    debug!(
        "Invoking function '{}' on plugin {}",
        function_name, plugin_id
    );

    // Get the plugin instance
    let plugins_wasm = state.plugins_wasm.read().await;
    let plugin = plugins_wasm
        .get(&plugin_id)
        .ok_or_else(|| format!("Plugin with ID {} not found", plugin_id))?;

    // Check if the function exists
    if !plugin.exports.contains(&function_name.to_string()) {
        return Err(format!("Function '{}' not found in plugin", function_name));
    }

    // Get the engine or return an error
    let engine = state
        .wasm_engine
        .as_ref()
        .ok_or_else(|| "WebAssembly engine not initialized".to_string())?;

    // Create a context
    let wasm_context = WasmContext {
        plugin_id,
        resource_limits: ResourceLimits::default(),
    };

    // Create a store
    let mut store = Store::new(engine, wasm_context);

    // Get the instance
    let instance = plugin
        .instance
        .as_ref()
        .ok_or_else(|| "Plugin instance not available".to_string())?;

    // Get the function
    let func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, function_name)
        .map_err(|e| format!("Failed to get function: {}", e))?;

    // For simplicity in this implementation, we'll only support functions
    // that take two i32 parameters and return an i32
    // In a real implementation, we would need to handle different function signatures

    // Convert params to i32
    let param1 = match params.get(0) {
        Some(PluginParam::Integer(i)) => *i as i32,
        _ => 0,
    };

    let param2 = match params.get(1) {
        Some(PluginParam::Integer(i)) => *i as i32,
        _ => 0,
    };

    // Call the function asynchronously since async_support is enabled
    let result = match func.call_async(&mut store, (param1, param2)).await {
        Ok(result) => result,
        Err(e) => return Err(format!("Failed to call function: {}", e)),
    };

    // Log the function call
    let log_entry = LogEntry::new(
        LogLevel::Info,
        format!(
            "Called function '{}' on plugin {} with result: {}",
            function_name, plugin_id, result
        ),
        "wasm-engine",
    )
    .with_plugin_id(plugin_id);

    state.log(log_entry).await;

    // Return the result
    Ok(PluginResult {
        success: true,
        data: serde_json::json!(result),
        error: None,
    })
}

/// Get information about a WASM plugin
pub async fn get_plugin_info(
    state: &Arc<AppState>,
    plugin_id: Uuid,
) -> Result<WasmPluginInfo, String> {
    // Get the WASM plugin instance
    let plugins_wasm = state.plugins_wasm.read().await;
    let wasm_plugin = plugins_wasm
        .get(&plugin_id)
        .ok_or_else(|| format!("Plugin with ID {} not found", plugin_id))?;

    // Get the regular plugin info
    let plugins = state.plugins.read().await;
    let plugin_info = plugins
        .get(&plugin_id)
        .ok_or_else(|| format!("Plugin info with ID {} not found", plugin_id))?
        .clone();

    // Create the WASM plugin info
    let wasm_plugin_info = WasmPluginInfo {
        id: plugin_id,
        name: plugin_info.name,
        version: plugin_info.version,
        description: plugin_info.description,
        path: wasm_plugin.path.clone(),
        exports: wasm_plugin.exports.clone(),
    };

    Ok(wasm_plugin_info)
}

/// Detailed WASM plugin information for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginInfo {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
    pub path: String,
    pub exports: Vec<String>,
}

// ------ API Handlers ------

/// Handler for loading a WASM plugin
pub async fn load_wasm_plugin(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoadWasmPluginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match load_plugin(&state, &request.path, request.name).await {
        Ok(plugin_info) => (
            StatusCode::CREATED,
            Json(serde_json::to_value(plugin_info).unwrap_or_default()),
        ),
        Err(e) => {
            error!("Failed to load WASM plugin: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        }
    }
}

/// Handler for listing all WASM plugins
pub async fn list_wasm_plugins(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Get all WASM plugin info
    let mut plugin_infos = Vec::new();

    // Get all plugins
    let plugins = state.plugins.read().await;
    let plugins_wasm = state.plugins_wasm.read().await;

    // Filter for WASM plugins
    for (id, plugin) in plugins.iter() {
        if plugins_wasm.contains_key(id) {
            let wasm_plugin = plugins_wasm.get(id).unwrap();

            plugin_infos.push(WasmPluginInfo {
                id: *id,
                name: plugin.name.clone(),
                version: plugin.version.clone(),
                description: plugin.description.clone(),
                path: wasm_plugin.path.clone(),
                exports: wasm_plugin.exports.clone(),
            });
        }
    }

    Json(plugin_infos)
}

/// Handler for getting WASM plugin info
pub async fn get_wasm_plugin_info(
    State(state): State<Arc<AppState>>,
    AxumPath(plugin_id): AxumPath<Uuid>,
) -> (StatusCode, Json<serde_json::Value>) {
    match get_plugin_info(&state, plugin_id).await {
        Ok(plugin_info) => (
            StatusCode::OK,
            Json(serde_json::to_value(plugin_info).unwrap_or_default()),
        ),
        Err(e) => {
            error!("Failed to get WASM plugin info: {}", e);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        }
    }
}

/// Handler for invoking a WASM plugin function
pub async fn invoke_wasm_plugin_function(
    State(state): State<Arc<AppState>>,
    AxumPath(plugin_id): AxumPath<Uuid>,
    Json(request): Json<InvokeWasmFunctionRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match invoke_plugin_function(&state, plugin_id, &request.function, request.params).await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::to_value(result).unwrap_or_default()),
        ),
        Err(e) => {
            error!("Failed to invoke WASM plugin function: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        }
    }
}
