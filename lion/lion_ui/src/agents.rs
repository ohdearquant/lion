use axum::{extract::State, response::IntoResponse, Json};
use lion_core::SystemEvent;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::events::AppState;

#[derive(Debug, Deserialize)]
pub struct SpawnAgentRequest {
    pub prompt: String,
}

#[derive(Debug, Serialize)]
pub struct AgentInfo {
    pub id: Uuid,
    pub status: String,
}

/// Handler for spawning a new agent
pub async fn spawn_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SpawnAgentRequest>,
) -> impl IntoResponse {
    // Create a new agent spawn event
    let event = SystemEvent::new_agent(payload.prompt, None);

    let agent_id = match &event {
        SystemEvent::AgentSpawned { agent_id, .. } => *agent_id,
        _ => unreachable!(),
    };

    // Store the agent in our registry
    {
        let mut agents = state.agents.write().await;
        agents.insert(agent_id, "spawned".to_string());
    }

    // Send the event to the orchestrator
    if let Err(e) = state.orchestrator_sender.send(event).await {
        return Json(serde_json::json!({
            "error": format!("Failed to spawn agent: {}", e)
        }));
    }

    // Log the spawn
    let _ = state.logs_tx.send(format!("Agent {} spawned", agent_id));

    Json(serde_json::json!({
        "agent_id": agent_id.to_string(),
        "status": "spawned"
    }))
}

/// Handler for listing active agents
pub async fn list_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let agents = state.agents.read().await;
    let agent_list: Vec<AgentInfo> = agents
        .iter()
        .map(|(id, status)| AgentInfo {
            id: *id,
            status: status.clone(),
        })
        .collect();

    Json(agent_list)
}
