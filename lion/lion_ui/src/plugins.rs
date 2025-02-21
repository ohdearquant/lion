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
use uuid::Uuid;
use tracing::debug;

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
                if let Ok(plugin_id) = plugin_manager.load_plugin(manifest.clone()) {
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

    Ok(Json(plugin_list))
}

pub async fn invoke_plugin_handler(
    Path(plugin_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<InvokePluginRequest>,
) -> Result<String, String> {
    // Check if plugin exists
    let plugins = state.plugins.read().await;
    if !plugins.contains_key(&plugin_id) {
        return Err("Plugin not found".to_string());
    }

    state
        .orchestrator_sender
        .send(SystemEvent::PluginInvoked {
            plugin_id,
            input: req.input.clone(),
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
        .send(format!("Plugin {} invoked: {}", plugin_id, req.input))
        .map_err(|e| format!("Failed to log invocation: {}", e))?;

    Ok("Plugin invocation sent".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use http_body_util::BodyExt;
    use lion_core::Orchestrator;
    use std::sync::Arc;
    use tokio::sync::broadcast;
    use tower::ServiceExt;

    async fn setup_test_app() -> (Router, broadcast::Receiver<String>) {
        let plugin_manager = PluginManager::with_manifest_dir("plugins");
        let orchestrator = Orchestrator::with_plugin_manager(100, plugin_manager);
        let orchestrator_tx = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();
        let (logs_tx, _) = broadcast::channel::<String>(100);
        let logs_tx_clone = logs_tx.clone();

        let state = Arc::new(AppState::new_with_logs(orchestrator_tx, logs_tx.clone()));

        // Spawn orchestrator
        tokio::spawn(orchestrator.run());

        // Forward completion events to logs
        tokio::spawn(async move {
            while let Ok(event) = completion_rx.recv().await {
                match &event {
                    SystemEvent::PluginInvoked {
                        plugin_id, input, ..
                    } => {
                        logs_tx_clone
                            .send(format!("Plugin {} invoked: {}", plugin_id, input))
                            .map_err(|e| format!("Failed to log invocation: {}", e))
                            .ok();
                    }
                    SystemEvent::PluginResult {
                        plugin_id, output, ..
                    } => {
                        logs_tx_clone
                            .send(format!("Plugin {} result: {}", plugin_id, output))
                            .map_err(|e| format!("Failed to log result: {}", e))
                            .ok();
                    }
                    SystemEvent::PluginError {
                        plugin_id, error, ..
                    } => {
                        logs_tx_clone
                            .send(format!("Plugin {} error: {}", plugin_id, error))
                            .map_err(|e| format!("Failed to log error: {}", e))
                            .ok();
                    }
                    _ => {}
                }
            }
        });

        let app = Router::new()
            .route(
                "/api/plugins",
                axum::routing::post(load_plugin_handler).get(list_plugins_handler),
            )
            .route(
                "/api/plugins/{plugin_id}/invoke",
                axum::routing::post(invoke_plugin_handler),
            )
            .with_state(state);

        (app, logs_tx.subscribe())
    }

    #[tokio::test]
    async fn test_load_plugin() {
        let (app, _logs_rx) = setup_test_app().await;
        let manifest = r#"
name = "test_plugin"
version = "0.1.0"
entry_point = "test.wasm"
permissions = ["net"]
"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plugins")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "manifest": manifest
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        println!("Load response body: {}", body);
        let plugin_info: PluginInfo = serde_json::from_str(&body).unwrap();
        assert_eq!(plugin_info.name, "test_plugin");
        assert_eq!(plugin_info.version, "0.1.0");
        assert_eq!(plugin_info.permissions, vec!["net"]);
    }

    #[tokio::test]
    async fn test_list_plugins() {
        let (app, _logs_rx) = setup_test_app().await;

        // First load a plugin
        let manifest = r#"
name = "test_plugin"
version = "0.1.0"
entry_point = "test.wasm"
permissions = ["net"]
"#;

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plugins")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "manifest": manifest
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then list plugins
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/plugins")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();

        let plugins: Vec<PluginInfo> = serde_json::from_str(&body).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "test_plugin");
    }

    #[tokio::test]
    async fn test_invoke_plugin() {
        let (app, _logs_rx) = setup_test_app().await;

        // First load a plugin
        let manifest = r#"
name = "test_plugin"
version = "0.1.0"
entry_point = "test.wasm"
permissions = ["net"]
"#;

        let load_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plugins")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "manifest": manifest
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = String::from_utf8(
            load_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        println!("Load response body: {}", body);
        let plugin_info: PluginInfo = serde_json::from_str(&body).unwrap();

        // Then invoke the plugin
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/plugins/{}/invoke", plugin_info.id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "input": "test input"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();

        assert!(body.contains("Plugin invocation sent"));
    }
}
