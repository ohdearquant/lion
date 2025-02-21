mod agents;
mod events;
mod plugins;

use crate::{
    agents::{list_agents, spawn_agent},
    events::sse_handler,
    plugins::PluginInfo,
};
use axum::{
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use events::AppState;
use lion_core::{Orchestrator, PluginManifest, SystemEvent};
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

const INDEX_HTML: &str = include_str!("../templates/index.html");

pub async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
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
                    agent_id,
                    chunk,
                    metadata: _,
                } => {
                    let _ = state_clone
                        .logs_tx
                        .send(format!("Agent {}: {}", agent_id, chunk));
                }
                SystemEvent::AgentCompleted {
                    agent_id,
                    result,
                    metadata: _,
                } => {
                    let _ = state_clone
                        .logs_tx
                        .send(format!("Agent {} completed: {}", agent_id, result));
                    let mut agents = state_clone.agents.write().await;
                    agents.insert(*agent_id, "completed".to_string());
                }
                SystemEvent::AgentError {
                    agent_id,
                    error,
                    metadata: _,
                } => {
                    let _ = state_clone
                        .logs_tx
                        .send(format!("Agent {} error: {}", agent_id, error));
                    let mut agents = state_clone.agents.write().await;
                    agents.insert(*agent_id, "error".to_string());
                }
                SystemEvent::PluginInvoked {
                    plugin_id,
                    input,
                    metadata: _,
                } => {
                    let _ = state_clone.logs_tx.send(format!(
                        "Plugin {} invoked with input: {}",
                        plugin_id, input
                    ));

                    // If this is a load operation, track the plugin
                    if let Some(manifest_str) = input.strip_prefix("load:") {
                        let mut plugins = state_clone.plugins.write().await;
                        if let Ok(manifest) = toml::from_str::<PluginManifest>(manifest_str) {
                            plugins.insert(
                                *plugin_id,
                                PluginInfo {
                                    id: *plugin_id,
                                    name: manifest.name,
                                    version: manifest.version,
                                    description: manifest.description,
                                },
                            );
                        } else {
                            let _ = state_clone
                                .logs_tx
                                .send(format!("Failed to parse manifest for plugin {}", plugin_id));
                        }
                    }
                }
                SystemEvent::PluginResult {
                    plugin_id,
                    output,
                    metadata: _,
                } => {
                    let _ = state_clone
                        .logs_tx
                        .send(format!("Plugin {} result: {}", plugin_id, output));
                }
                SystemEvent::PluginError {
                    plugin_id,
                    error,
                    metadata: _,
                } => {
                    let _ = state_clone
                        .logs_tx
                        .send(format!("Plugin {} error: {}", plugin_id, error));
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
        .nest("/api", plugins::create_plugin_router())
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
