use axum::{
    extract::{Path, State},
    Json,
};
use lion_core::{orchestrator::EventMetadata, PluginManifest, SystemEvent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LoadPluginRequest {
    pub manifest: String,
}

#[derive(Debug, Serialize, Deserialize)]
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
            let error_msg = format!("Invalid manifest format: {}", e.to_string());
            println!("Handler sending error: {}", error_msg);
            let _ = state.orchestrator_sender.send(SystemEvent::PluginError {
                plugin_id: Uuid::new_v4(),
                error: error_msg.clone(),
                metadata: EventMetadata {
                    event_id: Uuid::new_v4(),
                    timestamp: chrono::Utc::now(),
                    correlation_id: None,
                    context: serde_json::json!({"action": "load"}),
                },
            });
            let _ = state.logs_tx.send(error_msg.clone());
            return Err(error_msg.clone());
        }
    };

    // Load plugin via orchestrator
    let plugin_id = Uuid::new_v4();
    let _ = state
        .orchestrator_sender
        .send(SystemEvent::PluginInvoked {
            plugin_id,
            input: format!("load:{}", toml::to_string(&manifest).unwrap()),
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                correlation_id: None,
                context: serde_json::json!({"action": "load"}),
            },
        })
        .await
        .map_err(|e| format!("Failed to send load request: {}", e))?;

    // Track the plugin
    let mut plugins = state.plugins.write().await;
    plugins.insert(
        plugin_id,
        PluginInfo {
            id: plugin_id,
            loaded: true,
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            permissions: manifest.permissions.clone(),
        },
    );

    // Send event to UI logs
    let _ = state.logs_tx.send(format!(
        "Plugin {} invoked: load:{}",
        manifest.name, manifest.version
    ));

    Ok(Json(PluginInfo {
        id: plugin_id,
        loaded: true,
        name: manifest.name,
        version: manifest.version,
        permissions: manifest.permissions,
    }))
}

pub async fn list_plugins_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PluginInfo>>, String> {
    let plugins = state.plugins.read().await;

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
    let _ = state
        .logs_tx
        .send(format!("Plugin {} invoked: {}", plugin_id, req.input));

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

    async fn setup_test_app() -> Router {
        let orchestrator = Orchestrator::new(100);
        let orchestrator_tx = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();
        let (logs_tx, _) = broadcast::channel::<String>(100);

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
                        let _ = logs_tx.send(format!("Plugin {} invoked: {}", plugin_id, input));
                    }
                    SystemEvent::PluginResult {
                        plugin_id, output, ..
                    } => {
                        let _ = logs_tx.send(format!("Plugin {} result: {}", plugin_id, output));
                    }
                    SystemEvent::PluginError {
                        plugin_id, error, ..
                    } => {
                        let _ = logs_tx.send(format!("Plugin {} error: {}", plugin_id, error));
                    }
                    _ => {}
                }
            }
        });

        Router::new()
            .route(
                "/api/plugins",
                axum::routing::post(load_plugin_handler).get(list_plugins_handler),
            )
            .route(
                "/api/plugins/{plugin_id}/invoke",
                axum::routing::post(invoke_plugin_handler),
            )
            .with_state(state)
    }

    #[tokio::test]
    async fn test_load_plugin() {
        let app = setup_test_app().await;
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

        let plugin_info: PluginInfo = serde_json::from_str(&body).unwrap();
        assert_eq!(plugin_info.name, "test_plugin");
        assert_eq!(plugin_info.version, "0.1.0");
        assert_eq!(plugin_info.permissions, vec!["net"]);
    }

    #[tokio::test]
    async fn test_list_plugins() {
        let app = setup_test_app().await;

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
        let app = setup_test_app().await;

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
