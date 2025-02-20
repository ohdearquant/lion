mod agents;
mod events;

use crate::{
    agents::{list_agents, spawn_agent},
    events::sse_handler,
};
use agentic_core::{Orchestrator, SystemEvent};
use axum::{
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use events::AppState;
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub async fn index_handler() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <title>lion UI</title>
    <style>
        #logs {
            height: 300px;
            overflow-y: auto;
            border: 1px solid #ccc;
            padding: 10px;
            margin: 10px 0;
            font-family: monospace;
            white-space: pre-wrap;
        }
        .agent-list {
            margin: 20px 0;
        }
        .agent-item {
            margin: 5px 0;
            padding: 5px;
            border: 1px solid #eee;
        }
    </style>
</head>
<body>
    <h1>lion UI - Agent Management</h1>
    
    <div>
        <h2>Spawn New Agent</h2>
        <input type="text" id="promptInput" placeholder="Enter agent prompt" />
        <button id="spawnBtn">Spawn Agent</button>
    </div>

    <div class="agent-list">
        <h2>Active Agents</h2>
        <div id="agentList"></div>
    </div>

    <div>
        <h2>Real-time Logs</h2>
        <div id="logs"></div>
    </div>

    <script>
        // Set up SSE for real-time logs
        const evtSource = new EventSource("/events");
        const logsDiv = document.getElementById("logs");
        
        evtSource.onmessage = (event) => {
            const newLog = document.createElement("div");
            newLog.textContent = event.data;
            logsDiv.appendChild(newLog);
            logsDiv.scrollTop = logsDiv.scrollHeight;
        };

        // Function to spawn a new agent
        async function spawnAgent() {
            const promptInput = document.getElementById("promptInput");
            const prompt = promptInput.value.trim();
            
            if (!prompt) {
                alert("Please enter a prompt");
                return;
            }

            try {
                const res = await fetch("/api/agents", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ prompt })
                });
                
                const data = await res.json();
                if (data.error) {
                    alert(data.error);
                } else {
                    promptInput.value = "";
                    fetchAgents(); // Refresh agent list
                }
            } catch (e) {
                alert("Error spawning agent: " + e);
            }
        }

        // Function to fetch and display active agents
        async function fetchAgents() {
            try {
                const res = await fetch("/api/agents");
                const agents = await res.json();
                
                const agentList = document.getElementById("agentList");
                agentList.innerHTML = agents.map(agent => `
                    <div class="agent-item">
                        Agent ${agent.id} - Status: ${agent.status}
                    </div>
                `).join("");
            } catch (e) {
                console.error("Error fetching agents:", e);
            }
        }

        // Set up event listeners
        document.getElementById("spawnBtn").onclick = spawnAgent;
        
        // Initial agent list fetch
        fetchAgents();
        
        // Refresh agent list periodically
        setInterval(fetchAgents, 5000);
    </script>
</body>
</html>"#;
    Html(html)
}

/// Handler for the /ping endpoint, returns a simple response from the microkernel
///
/// # Examples
///
/// ```no_run
/// use axum::{
///     Router,
///     routing::get,
/// };
///
/// async fn test_ping() {
///     let app = Router::new().route("/ping", get(lion_ui::ping_handler));
///     // In a real server, we would bind and serve
/// }
/// ```
pub async fn ping_handler() -> &'static str {
    // TODO: In future phases, this will call actual microkernel functions
    "Pong from lion_ui microkernel!"
}

#[tokio::main]
async fn main() {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .compact()
        .init();

    info!("Starting lion_ui server...");

    // Initialize orchestrator
    let orchestrator = Orchestrator::new(100);
    let orchestrator_sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator in the background
    tokio::spawn(orchestrator.run());

    // Create shared state
    let state = Arc::new(AppState::new(orchestrator_sender, 100));
    let state_clone = Arc::clone(&state);

    // Spawn a task to forward orchestrator completion events to UI
    tokio::spawn(async move {
        while let Ok(event) = completion_rx.recv().await {
            match &event {
                SystemEvent::AgentPartialOutput {
                    agent_id, chunk, ..
                } => {
                    let _ = state_clone
                        .logs_tx
                        .send(format!("Agent {}: {}", agent_id, chunk));
                }
                SystemEvent::AgentCompleted {
                    agent_id, result, ..
                } => {
                    let _ = state_clone
                        .logs_tx
                        .send(format!("Agent {} completed: {}", agent_id, result));
                    let mut agents = state_clone.agents.write().await;
                    agents.insert(*agent_id, "completed".to_string());
                }
                SystemEvent::AgentError {
                    agent_id, error, ..
                } => {
                    let _ = state_clone
                        .logs_tx
                        .send(format!("Agent {} error: {}", agent_id, error));
                    let mut agents = state_clone.agents.write().await;
                    agents.insert(*agent_id, "error".to_string());
                }
                _ => {}
            }
        }
    });

    // Build our application router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/events", get(sse_handler))
        .route("/api/agents", post(spawn_agent).get(list_agents))
        .with_state(state);

    // Run it on localhost:8080
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Server started successfully");

    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_ping_endpoint() {
        let app = Router::new().route("/ping", get(ping_handler));

        let response = app
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
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

        assert_eq!(body, "Pong from lion_ui microkernel!");
    }

    #[tokio::test]
    async fn test_index_endpoint() {
        let app = Router::new().route("/", get(index_handler));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
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

        // Check that the response contains our expected HTML elements
        assert!(body.contains("lion UI - Agent Management"));
        assert!(body.contains("Spawn New Agent"));
        assert!(body.contains("Real-time Logs"));
    }
}
