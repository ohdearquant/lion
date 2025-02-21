use crate::events::AppState;
use axum::{
    extract::{Multipart, Path as AxumPath, State},
    response::Json,
    routing::post,
    Router,
};
use chrono::Utc;
use lion_core::orchestrator::{EventMetadata, SystemEvent};
use lion_core::plugin_manager::PluginManifest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path as StdPath;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error};
use uuid::Uuid;

/// Information about a loaded plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
}

impl From<&PluginManifest> for PluginInfo {
    fn from(manifest: &PluginManifest) -> Self {
        Self {
            id: Uuid::new_v4(), // This will be replaced with the actual ID after loading
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct InvokePluginRequest {
    function: String,
    args: serde_json::Value,
}

/// Handler for loading a plugin from a manifest file
pub async fn load_plugin_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<PluginInfo> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        if field.name().unwrap() == "manifest" {
            let content = field.bytes().await.unwrap();
            let content = String::from_utf8(content.to_vec()).unwrap();
            let manifest: PluginManifest = toml::from_str(&content).unwrap();

            // Create plugin info with temporary ID
            let info = PluginInfo::from(&manifest);

            // Send load plugin event to orchestrator
            let event = SystemEvent::TaskSubmitted {
                task_id: Uuid::new_v4(),
                payload: serde_json::to_string(&manifest).unwrap(),
                metadata: EventMetadata {
                    event_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    correlation_id: None,
                    context: serde_json::json!({}),
                },
            };
            state.orchestrator_sender.send(event).await.unwrap();

            // Return the info (ID will be updated when plugin is loaded)
            return Json(info);
        }
    }
    panic!("No manifest field found in request");
}

/// Handler for listing all loaded plugins
pub async fn list_plugins_handler(State(state): State<Arc<AppState>>) -> Json<Vec<PluginInfo>> {
    let plugins = state.plugins.read().await;
    Json(plugins.values().cloned().collect())
}

/// Handler for invoking a plugin function
pub async fn invoke_plugin_handler(
    State(state): State<Arc<AppState>>,
    AxumPath(plugin_id): AxumPath<Uuid>,
    Json(request): Json<InvokePluginRequest>,
) -> Json<serde_json::Value> {
    let input = serde_json::json!({
        "function": request.function,
        "args": request.args,
    });

    // Send plugin invocation event to orchestrator
    let event = SystemEvent::new_plugin_invocation(plugin_id, input.to_string(), None);
    state.orchestrator_sender.send(event).await.unwrap();

    // Wait for result from logs channel
    let mut logs_rx = state.logs_tx.subscribe();
    let result = wait_for_plugin_result(&mut logs_rx).await;

    Json(serde_json::from_str(&result).unwrap())
}

/// Create router for plugin endpoints
pub fn create_plugin_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/plugins",
            post(load_plugin_handler).get(list_plugins_handler),
        )
        .route("/plugins/{plugin_id}/invoke", post(invoke_plugin_handler))
}

/// Load a plugin manifest from a file
#[allow(dead_code)]
pub fn load_manifest<P: AsRef<StdPath>>(path: P) -> Option<PluginManifest> {
    let path = path.as_ref();
    debug!("Loading manifest from {}", path.display());

    match fs::read_to_string(path) {
        Ok(content) => {
            debug!("Read manifest content: {}", content);
            match toml::from_str(&content) {
                Ok(manifest) => {
                    debug!("Successfully parsed manifest");
                    Some(manifest)
                }
                Err(e) => {
                    error!("Failed to parse manifest: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            error!("Failed to read manifest file: {}", e);
            None
        }
    }
}

/// Save a plugin manifest to a file
#[allow(dead_code)]
pub fn save_manifest<P: AsRef<StdPath>>(manifest: &PluginManifest, path: P) -> std::io::Result<()> {
    let path = path.as_ref();
    debug!("Saving manifest to {}", path.display());

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(manifest).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to serialize manifest: {}", e),
        )
    })?;

    debug!("Writing manifest content: {}", content);
    fs::write(path, content)
}

#[allow(clippy::let_and_return)]
async fn wait_for_plugin_result(logs_rx: &mut broadcast::Receiver<String>) -> String {
    use std::time::Duration;
    use tokio::time::timeout;

    let timeout_duration = Duration::from_secs(5);
    let result = timeout(timeout_duration, async {
        while let Ok(log) = logs_rx.recv().await {
            debug!("Received log: {}", log);
            // Check for plugin output in the logs
            if log.contains(r#""result":"#) || log.contains(r#""error":"#) {
                // Extract the JSON part from the log
                let json_start = log.find('{').unwrap_or(0);
                let json_end = log.rfind('}').map(|i| i + 1).unwrap_or(log.len());
                let json = &log[json_start..json_end];
                return json.to_string();
            }
        }
        panic!("Plugin result not found in logs");
    })
    .await
    .expect("Plugin invocation timed out");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn create_test_manifest() -> PluginManifest {
        PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: "nonexistent".to_string(),
            permissions: vec![],
            driver: None,
            functions: HashMap::new(),
        }
    }

    #[test]
    fn test_save_and_load_manifest() {
        let temp_dir = tempdir().unwrap();
        let manifest_path = temp_dir.path().join("manifest.toml");

        let manifest = create_test_manifest();
        save_manifest(&manifest, &manifest_path).unwrap();

        let loaded = load_manifest(&manifest_path).unwrap();
        assert_eq!(loaded.name, manifest.name);
        assert_eq!(loaded.version, manifest.version);
        assert_eq!(loaded.description, manifest.description);
    }

    #[test]
    fn test_plugin_info_from_manifest() {
        let manifest = create_test_manifest();
        let info = PluginInfo::from(&manifest);
        assert_eq!(info.name, manifest.name);
        assert_eq!(info.version, manifest.version);
        assert_eq!(info.description, manifest.description);
    }
}
