use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path as FilePath, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::time::timeout;
use toml;
use tracing::{debug, info};
use uuid::Uuid;

use crate::AppState;

const PLUGINS_DIR: &str = "plugins";

#[derive(Debug, Deserialize)]
struct PluginManifest {
    name: String,
    version: String,
    description: String,
    entry_point: String,
    permissions: Vec<String>,
    functions: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct PluginInfo {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub functions: std::collections::HashMap<String, String>,
    pub manifest_path: String,
    pub entry_point: String,
}

impl PluginInfo {
    fn from_manifest(manifest: PluginManifest, manifest_path: PathBuf) -> Self {
        // Generate a deterministic UUID based on name and version
        let id_string = format!("{}:{}", manifest.name, manifest.version);
        let id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, id_string.as_bytes());

        Self {
            id,
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            permissions: manifest.permissions,
            functions: manifest.functions,
            manifest_path: manifest_path.to_string_lossy().to_string(),
            entry_point: manifest.entry_point,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoadPluginRequest {
    pub plugin_id: String,
}

#[derive(Debug, Deserialize)]
pub struct InvokePluginRequest {
    pub function: String,
    pub args: serde_json::Value,
}

/// Scan the plugins directory and read all plugin manifests
fn discover_plugins() -> Vec<PluginInfo> {
    let mut plugins = Vec::new();
    let plugins_dir = FilePath::new(PLUGINS_DIR);

    debug!("Scanning plugins directory: {}", PLUGINS_DIR);

    if !plugins_dir.exists() {
        debug!("Plugins directory does not exist");
        return plugins;
    }

    // Read each subdirectory in the plugins directory
    if let Ok(entries) = fs::read_dir(plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                debug!("Found plugin directory: {}", path.display());
                // Look for manifest.toml in each plugin directory
                let manifest_path = path.join("manifest.toml");
                if manifest_path.exists() {
                    debug!("Found manifest file: {}", manifest_path.display());
                    if let Ok(content) = fs::read_to_string(&manifest_path) {
                        match toml::from_str::<PluginManifest>(&content) {
                            Ok(manifest) => {
                                debug!(
                                    "Successfully loaded plugin manifest: {} v{}",
                                    manifest.name, manifest.version
                                );
                                plugins.push(PluginInfo::from_manifest(manifest, manifest_path));
                            }
                            Err(e) => {
                                debug!("Failed to parse manifest: {}", e);
                            }
                        }
                    } else {
                        debug!("Failed to read manifest file: {}", manifest_path.display());
                    }
                } else {
                    debug!("No manifest.toml found in: {}", path.display());
                }
            }
        }
    }

    // Sort plugins by name for consistent order
    plugins.sort_by(|a, b| a.name.cmp(&b.name));

    debug!("Discovered {} plugins", plugins.len());
    plugins
}

pub async fn list_plugins_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Handling list_plugins request");
    // Discover and return all plugins from the plugins directory
    let plugins = discover_plugins();
    debug!("Returning {} plugins", plugins.len());
    Json(plugins)
}

pub async fn load_plugin_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoadPluginRequest>,
) -> impl IntoResponse {
    debug!("Handling load_plugin request for id: {}", req.plugin_id);
    // Find the plugin in our discovered plugins
    let plugins = discover_plugins();
    let plugin = plugins.iter().find(|p| p.id.to_string() == req.plugin_id);

    match plugin {
        Some(plugin) => {
            debug!("Found plugin: {} v{}", plugin.name, plugin.version);
            // Read the manifest content
            if let Ok(manifest_content) = fs::read_to_string(&plugin.manifest_path) {
                // Parse the manifest content
                match toml::from_str::<PluginManifest>(&manifest_content) {
                    Ok(manifest) => {
                        // Create a plugin invocation event with metadata
                        let event = agentic_core::SystemEvent::new_plugin_invocation(
                            plugin.id,
                            manifest_content,
                            None, // No correlation ID for now
                        );

                        match state.orchestrator_sender.send(event).await {
                            Ok(_) => {
                                info!(
                                    "Successfully submitted load request for plugin: {}",
                                    plugin.name
                                );
                                // Log the plugin load attempt
                                let _ = state.logs_tx.send(format!(
                                    "Loading plugin {} from {}",
                                    plugin.name, plugin.manifest_path
                                ));
                                Json(serde_json::json!({
                                    "status": "success",
                                    "message": format!("Plugin {} load request submitted", plugin.name)
                                }))
                            }
                            Err(e) => {
                                debug!("Failed to submit plugin load request: {}", e);
                                Json(serde_json::json!({
                                    "status": "error",
                                    "message": format!("Failed to submit plugin load request: {}", e)
                                }))
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to parse manifest: {}", e);
                        Json(serde_json::json!({
                            "status": "error",
                            "message": format!("Failed to parse plugin manifest: {}", e)
                        }))
                    }
                }
            } else {
                debug!("Failed to read manifest file: {}", plugin.manifest_path);
                Json(serde_json::json!({
                    "status": "error",
                    "message": "Failed to read plugin manifest file"
                }))
            }
        }
        None => {
            debug!("Plugin not found with id: {}", req.plugin_id);
            Json(serde_json::json!({
                "status": "error",
                "message": "Plugin not found"
            }))
        }
    }
}

pub async fn invoke_plugin_handler(
    Path(plugin_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<InvokePluginRequest>,
) -> impl IntoResponse {
    debug!(
        "Handling invoke_plugin request for id: {} function: {}",
        plugin_id, req.function
    );
    // Parse plugin_id string to Uuid
    match Uuid::parse_str(&plugin_id) {
        Ok(id) => {
            // Find the plugin to verify the function exists
            let plugins = discover_plugins();
            let plugin = plugins.iter().find(|p| p.id == id);

            match plugin {
                Some(plugin) if plugin.functions.contains_key(&req.function) => {
                    debug!("Found plugin {} and function {}", plugin.name, req.function);

                    // First, load the plugin manifest
                    if let Ok(manifest_content) = fs::read_to_string(&plugin.manifest_path) {
                        // Create a new orchestrator instance for this invocation
                        let orchestrator = agentic_core::Orchestrator::new(100);
                        let orchestrator_sender = orchestrator.sender();
                        let mut completion_rx = orchestrator.completion_receiver();

                        // Spawn orchestrator in the background
                        tokio::spawn(orchestrator.run());

                        // Create the load event
                        let load_event = agentic_core::SystemEvent::new_plugin_invocation(
                            id,
                            manifest_content,
                            None,
                        );

                        // Send load event and wait for completion
                        match orchestrator_sender.send(load_event).await {
                            Ok(_) => {
                                // Wait for the load event to complete
                                match timeout(Duration::from_secs(5), completion_rx.recv()).await {
                                    Ok(Ok(completion)) => {
                                        match completion {
                                            agentic_core::SystemEvent::PluginResult { .. } => {
                                                // Plugin loaded successfully, now invoke it
                                                // Create the invocation input as a JSON string
                                                let input = serde_json::json!({
                                                    "function": req.function,
                                                    "args": req.args
                                                });

                                                // Create a plugin invocation event with metadata
                                                let invoke_event = agentic_core::SystemEvent::new_plugin_invocation(
                                                    id,
                                                    input.to_string(),
                                                    None, // No correlation ID for now
                                                );

                                                match orchestrator_sender.send(invoke_event).await {
                                                    Ok(_) => {
                                                        // Wait for the invoke event to complete
                                                        match timeout(
                                                            Duration::from_secs(5),
                                                            completion_rx.recv(),
                                                        )
                                                        .await
                                                        {
                                                            Ok(Ok(completion)) => {
                                                                match completion {
                                                                    agentic_core::SystemEvent::PluginResult { output, .. } => {
                                                                        info!(
                                                                            "Successfully invoked function {} on plugin {}",
                                                                            req.function, plugin.name
                                                                        );
                                                                        // Log the invocation attempt
                                                                        let _ = state.logs_tx.send(format!(
                                                                            "Plugin {} invoked function {} with result: {}",
                                                                            plugin.name, req.function, output
                                                                        ));
                                                                        Json(serde_json::json!({
                                                                            "status": "success",
                                                                            "message": format!("Plugin {} function {} invocation completed", plugin.name, req.function),
                                                                            "result": output
                                                                        }))
                                                                    }
                                                                    agentic_core::SystemEvent::PluginError { error, .. } => {
                                                                        Json(serde_json::json!({
                                                                            "status": "error",
                                                                            "message": format!("Failed to invoke plugin: {}", error)
                                                                        }))
                                                                    }
                                                                    _ => {
                                                                        Json(serde_json::json!({
                                                                            "status": "error",
                                                                            "message": "Unexpected response from plugin invocation"
                                                                        }))
                                                                    }
                                                                }
                                                            }
                                                            Ok(Err(e)) => Json(serde_json::json!({
                                                                "status": "error",
                                                                "message": format!("Failed to receive plugin invocation completion: {}", e)
                                                            })),
                                                            Err(_) => Json(serde_json::json!({
                                                                "status": "error",
                                                                "message": "Timeout waiting for plugin invocation completion"
                                                            })),
                                                        }
                                                    }
                                                    Err(e) => {
                                                        debug!("Failed to invoke plugin: {}", e);
                                                        Json(serde_json::json!({
                                                            "status": "error",
                                                            "message": format!("Failed to invoke plugin: {}", e)
                                                        }))
                                                    }
                                                }
                                            }
                                            agentic_core::SystemEvent::PluginError {
                                                error,
                                                ..
                                            } => Json(serde_json::json!({
                                                "status": "error",
                                                "message": format!("Failed to load plugin: {}", error)
                                            })),
                                            _ => Json(serde_json::json!({
                                                "status": "error",
                                                "message": "Unexpected response from plugin load"
                                            })),
                                        }
                                    }
                                    Ok(Err(e)) => Json(serde_json::json!({
                                        "status": "error",
                                        "message": format!("Failed to receive plugin load completion: {}", e)
                                    })),
                                    Err(_) => Json(serde_json::json!({
                                        "status": "error",
                                        "message": "Timeout waiting for plugin load completion"
                                    })),
                                }
                            }
                            Err(e) => Json(serde_json::json!({
                                "status": "error",
                                "message": format!("Failed to load plugin: {}", e)
                            })),
                        }
                    } else {
                        Json(serde_json::json!({
                            "status": "error",
                            "message": "Failed to read plugin manifest file"
                        }))
                    }
                }
                Some(_) => {
                    debug!("Function {} not found in plugin", req.function);
                    Json(serde_json::json!({
                        "status": "error",
                        "message": format!("Function {} not found in plugin", req.function)
                    }))
                }
                None => {
                    debug!("Plugin not found with id: {}", id);
                    Json(serde_json::json!({
                        "status": "error",
                        "message": "Plugin not found"
                    }))
                }
            }
        }
        Err(_) => {
            debug!("Invalid plugin ID format: {}", plugin_id);
            Json(serde_json::json!({
                "status": "error",
                "message": "Invalid plugin ID format"
            }))
        }
    }
}
