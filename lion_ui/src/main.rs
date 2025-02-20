use axum::{
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod agents;
mod events;
pub mod plugins;
pub mod state;

use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .compact()
        .init();

    info!("Starting lion_ui server...");

    // Create channels
    let (tx, _rx) = tokio::sync::mpsc::channel(100);
    let (logs_tx, _) = tokio::sync::broadcast::channel(100);

    // Create the shared state
    let state = Arc::new(AppState {
        orchestrator_tx: tx,
        plugins: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        logs_tx,
    });

    // Create the router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ping", get(ping_handler))
        .route("/events", get(events::sse_handler))
        .route(
            "/api/agents",
            get(agents::list_agents).post(agents::spawn_agent),
        )
        .route(
            "/api/plugins",
            get(plugins::list_plugins).post(plugins::load_plugin),
        )
        .route(
            "/api/plugins/:plugin_id/invoke",
            post(plugins::invoke_plugin),
        )
        .with_state(state);

    // Create the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    info!("Server started successfully");

    axum::serve(listener, app).await.unwrap();
}

// Handler for the root path
async fn index_handler() -> impl IntoResponse {
    let html = include_str!("../frontend/index.html");
    Html(html)
}

async fn ping_handler() -> &'static str {
    "Pong from lion_ui!"
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
    async fn test_index_endpoint() {
        let app = Router::new().route("/", get(index_handler));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("lion UI"));
    }

    #[tokio::test]
    async fn test_ping_endpoint() {
        let app = Router::new().route("/ping", get(ping_handler));

        let response = app
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(body_str, "Pong from lion_ui!");
    }
}
