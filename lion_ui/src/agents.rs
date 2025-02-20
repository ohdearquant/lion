use agentic_core::SystemEvent;
use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
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
    debug!(
        "Handling spawn_agent request with prompt: {}",
        payload.prompt
    );

    // Create a new agent spawn event
    let event = SystemEvent::new_agent(payload.prompt, None);

    let agent_id = match &event {
        SystemEvent::AgentSpawned { agent_id, .. } => *agent_id,
        _ => unreachable!(),
    };

    debug!("Created agent with ID: {}", agent_id);

    // Store the agent in our registry
    {
        let mut agents = state.agents.write().await;
        agents.insert(agent_id, "spawned".to_string());
        debug!("Stored agent {} in registry with status: spawned", agent_id);
    }

    // Send the event to the orchestrator
    match state.orchestrator_sender.send(event).await {
        Ok(_) => {
            info!("Successfully spawned agent {}", agent_id);
            // Log the spawn
            let _ = state.logs_tx.send(format!("Agent {} spawned", agent_id));

            Json(serde_json::json!({
                "agent_id": agent_id.to_string(),
                "status": "spawned"
            }))
        }
        Err(e) => {
            debug!("Failed to spawn agent: {}", e);
            Json(serde_json::json!({
                "error": format!("Failed to spawn agent: {}", e)
            }))
        }
    }
}

/// Handler for listing active agents
pub async fn list_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Handling list_agents request");
    let agents = state.agents.read().await;
    let agent_list: Vec<AgentInfo> = agents
        .iter()
        .map(|(id, status)| AgentInfo {
            id: *id,
            status: status.clone(),
        })
        .collect();

    debug!("Returning {} agents", agent_list.len());
    Json(agent_list)
}
