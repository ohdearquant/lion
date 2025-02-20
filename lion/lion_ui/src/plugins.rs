use crate::state::AppState;
use agentic_core::{
    orchestrator::{
        events::{PluginEvent, SystemEvent},
        metadata::create_metadata,
    },
    plugin_manager::PluginManifest,
    types::plugin::PluginResponse,
};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LoadPluginRequest {
    pub manifest: String,
    pub manifest_path: Option<String>,
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
            return Json(PluginResponse::error(format!(
                "Failed to parse manifest: {}",
                e
            )));
        }
    };

    let plugin_id = manifest.id;
    let event = SystemEvent::new_plugin_load(plugin_id, manifest.clone(), req.manifest_path, None);

    if let Err(e) = state.orchestrator_tx.send(event).await {
        error!("Failed to send plugin load event: {}", e);
        return Json(PluginResponse::error(format!(
            "Failed to load plugin: {}",
            e
        )));
    }

    Json(manifest.into_response().with_status("loading"))
}

pub async fn list_plugins_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Listing plugins");

    if let Err(e) = state
        .orchestrator_tx
        .send(SystemEvent::new_plugin_list())
        .await
    {
        error!("Failed to send list plugins event: {}", e);
        return Json(PluginResponse::error(format!(
            "Failed to list plugins: {}",
            e
        )));
    }

    Json(
        PluginResponse::new(Uuid::nil(), String::new(), String::new(), String::new())
            .with_status("listing"),
    )
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
        return Json(PluginResponse::error(format!(
            "Failed to invoke plugin: {}",
            e
        )));
    }

    Json(
        PluginResponse::new(plugin_id, String::new(), String::new(), String::new())
            .with_status("invoking"),
    )
}
