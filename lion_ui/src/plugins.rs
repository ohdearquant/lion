use crate::state::AppState;
use agentic_core::orchestrator::{
    events::{PluginEvent, SystemEvent},
    metadata::EventMetadata,
};
use agentic_core::plugin_manager::PluginManifest;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

fn create_metadata(correlation_id: Option<Uuid>) -> EventMetadata {
    EventMetadata {
        event_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        correlation_id,
        context: json!({}),
    }
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
    let event = SystemEvent::Plugin(PluginEvent::Load {
        plugin_id,
        manifest,
        manifest_path: req.manifest_path,
        metadata: create_metadata(None),
    });

    if let Err(e) = state.orchestrator_tx.send(event).await {
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
        .orchestrator_tx
        .send(SystemEvent::Plugin(PluginEvent::List))
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

    let event = SystemEvent::new_plugin_invocation(plugin_id, input.to_string(), None);

    if let Err(e) = state.orchestrator_tx.send(event).await {
        error!("Failed to send plugin invoke event: {}", e);
        return Json(serde_json::json!({
            "error": format!("Failed to invoke plugin: {}", e)
        }));
    }

    Json(serde_json::json!({
        "status": "invoking"
    }))
}
