use agentic_core::orchestrator::{
    events::{AgentEvent, SystemEvent},
    metadata::EventMetadata,
};
use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct AgentInfo {
    pub id: Uuid,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct SpawnAgentRequest {
    pub prompt: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

fn create_metadata(correlation_id: Option<Uuid>) -> EventMetadata {
    EventMetadata {
        event_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        correlation_id,
        context: json!({}),
    }
}

pub async fn spawn_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SpawnAgentRequest>,
) -> Json<ApiResponse<AgentInfo>> {
    info!("Spawning new agent");

    // Generate agent ID
    let agent_id = Uuid::new_v4();

    // Create the event using the constructor
    let event = SystemEvent::new_agent(req.prompt, None);

    // Send to orchestrator
    if let Err(e) = state.orchestrator_tx.send(event).await {
        return Json(ApiResponse::error(format!("Failed to spawn agent: {}", e)));
    }

    // Log the spawn event
    let _ = state.logs_tx.send(format!("Agent {} spawned", agent_id));

    // Create agent info
    let agent_info = AgentInfo {
        id: agent_id,
        status: "spawned".to_string(),
    };

    Json(ApiResponse::success(agent_info))
}

pub async fn list_agents(State(_state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<AgentInfo>>> {
    // For now, return empty list
    // In a real implementation, you would track agents in state
    Json(ApiResponse::success(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response() {
        let success = ApiResponse::success("test data");
        assert!(success.success);
        assert_eq!(success.data, Some("test data"));
        assert_eq!(success.error, None);

        let error = ApiResponse::<()>::error("test error");
        assert!(!error.success);
        assert_eq!(error.data, None);
        assert_eq!(error.error, Some("test error".to_string()));
    }
}
