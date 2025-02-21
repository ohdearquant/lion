use axum::{
    extract::{Path, State},
    Json,
};
use lion_core::{
    orchestrator::EventMetadata,
    plugin_manager::{PluginManager, PluginManifest},
    SystemEvent,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LoadPluginRequest {
    pub manifest: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginInfo {
    pub id: Uuid,
    pub loaded: bool,
    pub name: String,
    pub version: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct InvokePluginRequest {
    pub input: String,
}

pub async fn load_plugin_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoadPluginRequest>,
) -> Result<Json<PluginInfo>, String> {
    // Parse manifest string as TOML
    let manifest: PluginManifest = match toml::from_str(&req.manifest) {
        Ok(manifest) => manifest,
        Err(e) => {
            let error_msg = format!("Invalid manifest format: {}", e);
            println!("Handler sending error: {}", error_msg);
            state
                .orchestrator_sender
                .send(SystemEvent::PluginError {
                    plugin_id: Uuid::new_v4(),
                    error: error_msg.clone(),
                    metadata: EventMetadata {
                        event_id: Uuid::new_v4(),
                        timestamp: chrono::Utc::now(),
                        correlation_id: None,
                        context: serde_json::json!({"action": "load"}),
                    },
                })
                .await
                .map_err(|e| format!("Failed to send error: {}", e))?;
            state
                .logs_tx
                .send(error_msg.clone())
                .map_err(|e| format!("Failed to log error: {}", e))?;
            return Err(error_msg);
        }
    };

    // Initialize plugin manager
    let plugin_manager = PluginManager::with_manifest_dir("plugins");

    // Load the plugin
    let plugin_id = match plugin_manager.load_plugin(manifest.clone()) {
        Ok(id) => id,
        Err(e) => {
            let error_msg = format!("Failed to load plugin: {}", e);
            state
                .orchestrator_sender
                .send(SystemEvent::PluginError {
                    plugin_id: Uuid::new_v4(),
                    error: error_msg.clone(),
                    metadata: EventMetadata {
                        event_id: Uuid::new_v4(),
                        timestamp: chrono::Utc::now(),
                        correlation_id: None,
                        context: serde_json::json!({"action": "load"}),
                    },
                })
                .await
                .map_err(|e| format!("Failed to send error: {}", e))?;
            return Err(error_msg);
        }
    };

    // Track the plugin
    let mut plugins = state.plugins.write().await;
    let plugin_info = PluginInfo {
        id: plugin_id,
        loaded: true,
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        permissions: manifest.permissions.clone(),
    };
    plugins.insert(plugin_id, plugin_info.clone());

    // Send event to UI logs
    state
        .logs_tx
        .send(format!(
            "Plugin {} loaded successfully with ID {}",
            manifest.name, plugin_id
        ))
        .map_err(|e| format!("Failed to log plugin load: {}", e))?;

    Ok(Json(plugin_info))
}

pub async fn list_plugins_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PluginInfo>>, String> {
    // Initialize plugin manager and discover plugins
    let plugin_manager = PluginManager::with_manifest_dir("plugins");

    // Clear existing plugins
    let mut plugins = state.plugins.write().await;
    plugins.clear();

    // Discover and load available plugins
    match plugin_manager.discover_plugins() {
        Ok(manifests) => {
            debug!("Discovered {} plugins", manifests.len());
            for manifest in manifests {
                debug!("Loading plugin: {}", manifest.name);
                if let Ok(plugin_id) = plugin_manager.load_plugin(manifest.clone()) {
                    debug!("Successfully loaded plugin with ID: {}", plugin_id);
                    plugins.insert(
                        plugin_id,
                        PluginInfo {
                            id: plugin_id,
                            loaded: true,
                            name: manifest.name,
                            version: manifest.version,
                            permissions: manifest.permissions,
                        },
                    );
                }
            }
        }
        Err(e) => {
            return Err(format!("Failed to discover plugins: {}", e));
        }
    }

    // Get plugins from storage
    let stored_plugins = plugin_manager.list_plugins();
    debug!("Found {} plugins in storage", stored_plugins.len());
    for (id, manifest) in stored_plugins {
        if !plugins.contains_key(&id) {
            debug!("Adding stored plugin: {} ({})", manifest.name, id);
            plugins.insert(
                id,
                PluginInfo {
                    id,
                    loaded: true,
                    name: manifest.name,
                    version: manifest.version,
                    permissions: manifest.permissions,
                },
            );
        }
    }

    let plugin_list: Vec<PluginInfo> = plugins
        .iter()
        .map(|(_, info)| PluginInfo {
            id: info.id,
            loaded: info.loaded,
            name: info.name.clone(),
            version: info.version.clone(),
            permissions: info.permissions.clone(),
        })
        .collect();

    debug!("Returning {} plugins", plugin_list.len());
    Ok(Json(plugin_list))
}

pub async fn invoke_plugin_handler(
    Path(plugin_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<InvokePluginRequest>,
) -> Result<String, String> {
    debug!("Invoking plugin {} with input: {}", plugin_id, req.input);

    // Check if plugin exists
    let plugins = state.plugins.read().await;
    if !plugins.contains_key(&plugin_id) {
        return Err("Plugin not found".to_string());
    }

    state
        .orchestrator_sender
        .send(SystemEvent::PluginInvoked {
            plugin_id,
            input: req.input,
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                correlation_id: None,
                context: serde_json::json!({"action": "invoke"}),
            },
        })
        .await
        .map_err(|e| format!("Failed to invoke plugin: {}", e))?;

    // Log the invocation
    state
        .logs_tx
        .send(format!("Plugin {} invoked", plugin_id))
        .map_err(|e| format!("Failed to log invocation: {}", e))?;

    Ok("Plugin invocation sent".to_string())
}
