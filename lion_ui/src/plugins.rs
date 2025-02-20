use crate::state::AppState;
use agentic_core::{PluginManifest, SystemEvent};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LoadPluginRequest {
    pub manifest: String,
    pub manifest_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PluginResponse {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
}

pub async fn load_plugin_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoadPluginRequest>,
) -> impl IntoResponse {
    debug!("Loading plugin from manifest");

    let manifest: PluginManifest = match toml::from_str(&req.manifest) {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to parse manifest: {}", e);
            return Json(serde_json::json!({
                "error": format!("Failed to parse manifest: {}", e)
            }));
        }
    };

    let plugin_id = Uuid::new_v4();
    let event = SystemEvent::PluginLoad {
        plugin_id,
        manifest,
        manifest_path: req.manifest_path,
    };

    if let Err(e) = state.orchestrator_sender.send(event).await {
        error!("Failed to send plugin load event: {}", e);
        return Json(serde_json::json!({
            "error": format!("Failed to load plugin: {}", e)
        }));
    }

    Json(serde_json::json!({
        "plugin_id": plugin_id.to_string(),
        "status": "loading"
    }))
}

pub async fn list_plugins_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Listing plugins");

    if let Err(e) = state
        .orchestrator_sender
        .send(SystemEvent::ListPlugins)
        .await
    {
        error!("Failed to send list plugins event: {}", e);
        return Json(serde_json::json!({
            "error": format!("Failed to list plugins: {}", e)
        }));
    }

    Json(serde_json::json!({
        "status": "listing"
    }))
}

pub async fn invoke_plugin_handler(
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<Uuid>,
    Json(input): Json<serde_json::Value>,
) -> impl IntoResponse {
    debug!("Invoking plugin {}", plugin_id);

    let event = SystemEvent::PluginInvoked {
        plugin_id,
        input: input.to_string(),
    };

    if let Err(e) = state.orchestrator_sender.send(event).await {
        error!("Failed to send plugin invoke event: {}", e);
        return Json(serde_json::json!({
            "error": format!("Failed to invoke plugin: {}", e)
        }));
    }

    Json(serde_json::json!({
        "status": "invoking"
    }))
}
