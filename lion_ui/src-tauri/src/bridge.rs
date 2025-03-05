use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::{command, Window};
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

/// Simple ping command to test if bridge is working
#[command]
pub fn ping() -> String {
    "pong from lion_ui (Tauri bridge)".to_string()
}

/// Create a log entry in the Lion UI backend
#[command]
pub async fn create_log(window: Window, request: LogRequest) -> Result<String, String> {
    // In a real implementation, this would make a request to the Lion UI server
    // For now, we just print the log and return a success message
    
    println!(
        "Log [{}]: {} (from {})",
        request.level, request.message, request.source
    );
    
    // Emit an event to the frontend about the log creation
    window
        .emit(
            "log-created",
            serde_json::json!({
                "id": Uuid::new_v4().to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "level": request.level,
                "message": request.message,
                "source": request.source,
                "metadata": request.metadata
            }),
        )
        .map_err(|e| e.to_string())?;
    
    Ok("Log created successfully".to_string())
}

/// Spawn a new agent through the Lion UI backend
#[command]
pub async fn spawn_agent(window: Window, request: AgentRequest) -> Result<String, String> {
    // In a real implementation, this would make a request to the Lion UI server
    // For now, we just print the agent details and return a mock ID
    
    println!(
        "Spawning agent: {} (type: {})",
        request.name, request.agent_type
    );
    
    let agent_id = Uuid::new_v4().to_string();
    
    // Emit an event to the frontend about the agent creation
    window
        .emit(
            "agent-spawned",
            serde_json::json!({
                "id": agent_id,
                "name": request.name,
                "agent_type": request.agent_type,
                "status": "spawned"
            }),
        )
        .map_err(|e| e.to_string())?;
    
    Ok(agent_id)
}

/// Load a plugin through the Lion UI backend
#[command]
pub async fn load_plugin(window: Window, request: PluginRequest) -> Result<String, String> {
    // In a real implementation, this would make a request to the Lion UI server
    // For now, we just print the plugin details and return a mock ID
    
    let plugin_name = request.name.unwrap_or_else(|| {
        request
            .path
            .split('/')
            .last()
            .unwrap_or("unknown")
            .to_string()
    });
    
    println!("Loading plugin: {} from {}", plugin_name, request.path);
    
    let plugin_id = Uuid::new_v4().to_string();
    
    // Emit an event to the frontend about the plugin loading
    window
        .emit(
            "plugin-loaded",
            serde_json::json!({
                "id": plugin_id,
                "name": plugin_name,
                "path": request.path,
                "description": request.description.unwrap_or_default(),
                "version": "0.1.0" // Default version
            }),
        )
        .map_err(|e| e.to_string())?;
    
    Ok(plugin_id)
}
