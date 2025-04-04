use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::logs::{LogEntry, LogLevel};
use crate::state::{AppState, PluginInfo};

/// Request to load a plugin
#[derive(Debug, Deserialize)]
pub struct LoadPluginRequest {
    /// Path to the plugin file
    pub path: String,

    /// Optional plugin name (if not provided, derived from file name)
    pub name: Option<String>,

    /// Optional plugin description
    pub description: Option<String>,
}

/// Request to invoke a plugin method
#[derive(Debug, Deserialize)]
pub struct InvokePluginRequest {
    /// Method name to invoke
    pub method: String,

    /// Parameters for the method
    pub params: serde_json::Value,
}

/// Load a plugin from a file
pub async fn load_plugin_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoadPluginRequest>,
) -> impl IntoResponse {
    // TODO: Implement actual plugin loading with lion runtime

    // Extract file name for default plugin name
    let file_name = request.path.split('/').last().unwrap_or("unknown");
    let plugin_name = request.name.unwrap_or_else(|| file_name.to_string());
    let plugin_id = Uuid::new_v4();

    // Create plugin info
    let plugin_info = PluginInfo {
        id: plugin_id,
        name: plugin_name.clone(),
        version: "0.1.0".to_string(), // Default version
        description: request.description.unwrap_or_else(|| "".to_string()),
    };

    // Register the plugin
    {
        let mut plugins = state.plugins.write().await;
        plugins.insert(plugin_id, plugin_info.clone());
    }

    // Log the plugin loading
    let log_entry = LogEntry::new(
        LogLevel::Info,
        format!("Plugin '{}' loaded from {}", plugin_name, request.path),
        "system",
    )
    .with_plugin_id(plugin_id);

    state.log(log_entry).await;

    info!("Plugin '{}' loaded with ID {}", plugin_name, plugin_id);

    // Return success with plugin info
    (StatusCode::CREATED, Json(plugin_info))
}

/// List all loaded plugins
pub async fn list_plugins_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let plugins = state.plugins.read().await;
    let plugins_vec: Vec<_> = plugins.values().cloned().collect();

    Json(plugins_vec)
}

/// Invoke a method on a loaded plugin
pub async fn invoke_plugin_handler(
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<Uuid>,
    Json(request): Json<InvokePluginRequest>,
) -> impl IntoResponse {
    // Find the plugin
    let plugin_name = {
        let plugins = state.plugins.read().await;
        match plugins.get(&plugin_id) {
            Some(plugin) => plugin.name.clone(),
            None => {
                error!("Plugin with ID {} not found", plugin_id);
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": format!("Plugin with ID {} not found", plugin_id)
                    })),
                );
            }
        }
    };

    // TODO: Implement actual plugin method invocation with lion runtime

    // Log the plugin invocation
    let log_entry = LogEntry::new(
        LogLevel::Info,
        format!(
            "Plugin '{}' method '{}' invoked",
            plugin_name, request.method
        ),
        "system",
    )
    .with_plugin_id(plugin_id);

    state.log(log_entry).await;

    info!(
        "Plugin '{}' method '{}' invoked with ID {}",
        plugin_name, request.method, plugin_id
    );

    // Return a mock result
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "plugin_id": plugin_id,
            "method": request.method,
            "result": "Method executed successfully",
            "mock_data": true
        })),
    )
}
