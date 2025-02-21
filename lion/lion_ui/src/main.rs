mod agents;
mod events;
mod plugins;

use crate::{
    agents::{list_agents, spawn_agent},
    events::{search_logs_handler, sse_handler, LogLine},
    plugins::PluginInfo,
};
use axum::{
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use events::AppState;
use lion_core::{Orchestrator, SystemEvent};
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

pub async fn run_server() {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .compact()
        .init();

    info!("Starting lion_ui server...");

    // Initialize plugin manager with plugins directory
    let plugin_manager =
        lion_core::plugin_manager::PluginManager::with_manifest_dir("../../plugins");

    // Initialize orchestrator with plugin manager
    let orchestrator = Orchestrator::with_plugin_manager(100, plugin_manager);
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
                    state_clone
                        .add_log(LogLine {
                            timestamp: chrono::Utc::now(),
                            agent_id: Some(*agent_id),
                            plugin_id: None,
                            correlation_id: None,
                            message: format!("Agent {}: {}", agent_id, chunk),
                        })
                        .await;

                    // Agent status is still tracked separately
                    let mut agents = state_clone.agents.write().await;
                    agents.insert(*agent_id, "running".to_string());
                }
                SystemEvent::AgentCompleted {
                    agent_id,
                    result,
                    metadata: _,
                } => {
                    state_clone
                        .add_log(LogLine {
                            timestamp: chrono::Utc::now(),
                            agent_id: Some(*agent_id),
                            plugin_id: None,
                            correlation_id: None,
                            message: format!("Agent {} completed: {}", agent_id, result),
                        })
                        .await;

                    let mut agents = state_clone.agents.write().await;
                    agents.insert(*agent_id, "completed".to_string());
                }
                SystemEvent::AgentError {
                    agent_id,
                    error,
                    metadata: _,
                } => {
                    state_clone
                        .add_log(LogLine {
                            timestamp: chrono::Utc::now(),
                            agent_id: Some(*agent_id),
                            plugin_id: None,
                            correlation_id: None,
                            message: format!("Agent {} error: {}", agent_id, error),
                        })
                        .await;

                    // Update agent status
                    let mut agents = state_clone.agents.write().await;
                    agents.insert(*agent_id, "error".to_string());
                }
                SystemEvent::PluginInvoked {
                    plugin_id,
                    input,
                    metadata: _,
                } => {
                    state_clone
                        .add_log(LogLine {
                            timestamp: chrono::Utc::now(),
                            agent_id: None,
                            plugin_id: Some(*plugin_id),
                            correlation_id: None,
                            message: format!("Plugin {} invoked with input: {}", plugin_id, input),
                        })
                        .await;
                }
                SystemEvent::PluginLoadRequested {
                    plugin_id,
                    manifest: _,
                    metadata: _,
                } => {
                    state_clone
                        .add_log(LogLine {
                            timestamp: chrono::Utc::now(),
                            agent_id: None,
                            plugin_id: Some(*plugin_id),
                            correlation_id: None,
                            message: format!("Loading plugin {}", plugin_id),
                        })
                        .await;
                }
                SystemEvent::PluginLoaded {
                    plugin_id,
                    name,
                    version,
                    description,
                    metadata: _,
                } => {
                    let mut plugins = state_clone.plugins.write().await;
                    plugins.insert(
                        *plugin_id,
                        PluginInfo {
                            id: *plugin_id,
                            name: name.clone(),
                            version: version.clone(),
                            description: description.clone(),
                        },
                    );
                    state_clone
                        .add_log(LogLine {
                            timestamp: chrono::Utc::now(),
                            agent_id: None,
                            plugin_id: Some(*plugin_id),
                            correlation_id: None,
                            message: format!("Plugin {} loaded successfully", name),
                        })
                        .await;
                }
                SystemEvent::PluginResult {
                    plugin_id,
                    output,
                    metadata: _,
                } => {
                    state_clone
                        .add_log(LogLine {
                            timestamp: chrono::Utc::now(),
                            agent_id: None,
                            plugin_id: Some(*plugin_id),
                            correlation_id: None,
                            message: format!("Plugin {} result: {}", plugin_id, output),
                        })
                        .await;
                }
                SystemEvent::PluginError {
                    plugin_id,
                    error,
                    metadata: _,
                } => {
                    state_clone
                        .add_log(LogLine {
                            timestamp: chrono::Utc::now(),
                            agent_id: None,
                            plugin_id: Some(*plugin_id),
                            correlation_id: None,
                            message: format!("Plugin {} error: {}", plugin_id, error),
                        })
                        .await;
                }
                _ => {}
            }
        }
    });

    // Build our application router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/events", get(sse_handler))
        .route("/api/logs", get(search_logs_handler))
        .route("/api/agents", post(spawn_agent).get(list_agents))
        .nest("/api/plugins", plugins::create_plugin_router())
        .fallback(get(index_handler))  // Serve index.html for all unmatched routes
        .with_state(state);

    // Run it on localhost:8080
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Server started successfully");

    axum::serve(listener, app).await.unwrap();
}

#[tokio::main]
async fn main() {
    run_server().await;
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
