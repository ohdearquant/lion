use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Emitter, Window};
use uuid::Uuid;

/// Structure for creating a log entry
#[derive(Debug, Serialize, Deserialize)]
pub struct LogRequest {
    level: String,
    message: String,
    source: String,
    metadata: Option<serde_json::Value>,
}

/// Structure for spawning an agent
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRequest {
    name: String,
    agent_type: String,
    config: serde_json::Value,
}

/// Structure for loading a plugin
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginRequest {
    path: String,
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginCallRequest {
    plugin_id: String,
    function: String,
    args: Option<String>,
}

/// Simple ping command to test if bridge is working
#[tauri::command]
pub fn ping() -> String {
    "pong from lion_ui (Tauri bridge)".to_string()
}

/// Create a log entry in the Lion UI backend
#[tauri::command]
pub async fn create_log(window: Window, request: LogRequest) -> Result<String, String> {
    // In a real implementation, this would make a request to the Lion UI server
    // For now, we just print the log and return a success message

    println!(
        "Log [{}]: {} (from {})",
        request.level, request.message, request.source
    );

    // Emit an event to the frontend about the log creation
    let _ = window.emit_to(
        &window.label(),
        "log-created",
        serde_json::json!({
            "id": Uuid::new_v4().to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "level": request.level,
            "message": request.message,
            "source": request.source,
            "metadata": request.metadata
        }),
    );

    Ok("Log created successfully".to_string())
}

/// Spawn a new agent through the Lion UI backend
#[tauri::command]
pub async fn spawn_agent(window: Window, request: AgentRequest) -> Result<String, String> {
    // In a real implementation, this would make a request to the Lion UI server
    // For now, we just print the agent details and return a mock ID

    println!(
        "Spawning agent: {} (type: {})",
        request.name, request.agent_type
    );

    let agent_id = Uuid::new_v4().to_string();

    // Emit an event to the frontend about the agent creation
    let _ = window.emit_to(
        &window.label(),
        "agent-spawned",
        serde_json::json!({
            "id": agent_id,
            "name": request.name,
            "agent_type": request.agent_type,
            "status": "spawned"
        }),
    );

    Ok(agent_id)
}

/// Load a plugin through the Lion UI backend
#[tauri::command]
pub async fn load_plugin_integrated(
    window: Window,
    request: PluginRequest,
) -> Result<String, String> {
    let path_buf = PathBuf::from(&request.path);

    // Call the CLI library function directly
    let plugin_id = lion_cli::commands::plugin::load_plugin(&path_buf, None)
        .map_err(|e| format!("Failed to load plugin: {}", e))?;

    let plugin_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Emit the event to inform frontend
    let _ = window.emit_to(
        &window.label(),
        "plugin-loaded",
        serde_json::json!({
            "id": plugin_id,
            "path": request.path,
            "name": plugin_name,
            "description": request.description.unwrap_or_default(),
            "status": "loaded"
        }),
    );

    Ok(plugin_id)
}

/// List all loaded plugins
#[tauri::command]
pub async fn list_plugins_integrated(window: Window) -> Result<Vec<serde_json::Value>, String> {
    let (_status, plugin_ids) = lion_cli::interfaces::runtime::get_runtime_status_and_plugins()
        .map_err(|e| format!("Failed to list plugins: {}", e))?;

    let mut plugins = Vec::new();

    for plugin_id in plugin_ids {
        let metadata = lion_cli::interfaces::runtime::get_plugin_metadata(&plugin_id)
            .map_err(|e| format!("Failed to get plugin metadata: {}", e))?;

        plugins.push(serde_json::json!({
            "id": plugin_id,
            "name": metadata.name,
            "status": "loaded"
        }));
    }

    let _ = window.emit_to(
        &window.label(),
        "plugins-listed",
        serde_json::json!({
            "count": plugins.len(),
            "plugins": plugins.clone()
        }),
    );

    Ok(plugins)
}

/// Call a plugin function
#[tauri::command]
pub async fn call_plugin_integrated(
    window: Window,
    request: PluginCallRequest,
) -> Result<String, String> {
    let result = lion_cli::commands::plugin::call_plugin(
        &request.plugin_id,
        &request.function,
        request.args.as_deref(),
    )
    .map_err(|e| format!("Failed to call plugin function: {}", e))?;

    let _ = window.emit_to(
        &window.label(),
        "plugin-call-completed",
        serde_json::json!({
            "plugin_id": request.plugin_id,
            "function": request.function,
            "result": result
        }),
    );

    Ok("Success".to_string())
}
