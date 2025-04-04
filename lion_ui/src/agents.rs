use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::logs::{LogEntry, LogLevel};
use crate::state::AppState;

/// Request to spawn a new agent
#[derive(Debug, Deserialize)]
pub struct SpawnAgentRequest {
    /// Name of the agent
    pub name: String,

    /// Agent type
    pub agent_type: String,

    /// Configuration
    pub config: serde_json::Value,
}

/// Response for a successful agent spawn
#[derive(Debug, Serialize)]
pub struct SpawnAgentResponse {
    /// Agent ID
    pub id: Uuid,

    /// Name of the agent
    pub name: String,

    /// Status message
    pub status: String,
}

/// Spawns a new agent
pub async fn spawn_agent(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SpawnAgentRequest>,
) -> impl IntoResponse {
    // TODO: Implement actual agent spawning with lion runtime

    // For now, just create a placeholder agent
    let agent_id = Uuid::new_v4();

    // Add agent to our state
    {
        let mut agents = state.agents.write().await;
        agents.insert(agent_id, request.name.clone());
    }

    // Log the agent creation
    let log_entry = LogEntry::new(
        LogLevel::Info,
        format!("Agent '{}' spawned", request.name),
        "system",
    )
    .with_agent_id(agent_id);

    state.log(log_entry).await;

    info!("Agent '{}' spawned with ID {}", request.name, agent_id);

    // Return success
    (
        StatusCode::CREATED,
        Json(SpawnAgentResponse {
            id: agent_id,
            name: request.name,
            status: "spawned".to_string(),
        }),
    )
}

/// Lists all agents
pub async fn list_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let agents = state.agents.read().await;

    let response: Vec<_> = agents
        .iter()
        .map(|(id, name)| {
            serde_json::json!({
                "id": id,
                "name": name,
            })
        })
        .collect();

    Json(response)
}
