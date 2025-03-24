use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod agents;
mod events;
mod logs;
mod plugins;
mod state;
mod utils;
mod wasm;

use agents::*;
use events::sse_handler;
use logs::{search_logs_handler, LogEntry};
use plugins::*;
use state::AppState;

#[tokio::main]
async fn main() {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .compact()
        .init();

    info!("Starting lion_ui server (Stage 2 - Phase 5)...");

    // Create broadcast channel for logs with 1000 capacity
    let (logs_tx, _logs_rx) = broadcast::channel::<LogEntry>(1000);

    // Create in-memory log buffer with 10,000 capacity for log search
    let log_buffer = Arc::new(RwLock::new(Vec::with_capacity(10000)));

    // Initialize shared application state
    let app_state = Arc::new(AppState::new(logs_tx.clone(), log_buffer.clone()));

    // Start a background task to collect logs and store in buffer
    let buffer_state = app_state.clone();
    tokio::spawn(async move {
        let mut rx = logs_tx.subscribe();
        while let Ok(log) = rx.recv().await {
            // Add to searchable buffer with a cap
            let mut buffer = buffer_state.log_buffer.write().await;
            buffer.push(log);

            // If buffer exceeds max size, remove oldest entries
            if buffer.len() > 10000 {
                *buffer = buffer.drain(1000..).collect();
            }
        }
    });

    // Build the router with our routes
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ping", get(ping_handler))
        .route("/events", get(sse_handler))
        .route("/api/agents", post(spawn_agent).get(list_agents))
        .route("/api/plugins", post(load_plugin_handler).get(list_plugins_handler))
        .route("/api/plugins/:plugin_id/invoke", post(invoke_plugin_handler))
        .route("/api/logs", get(search_logs_handler))
        .route("/api/wasm/plugins", post(wasm::load_wasm_plugin).get(wasm::list_wasm_plugins))
        .route("/api/wasm/plugins/:plugin_id", get(wasm::get_wasm_plugin_info))
        .route("/api/wasm/plugins/:plugin_id/invoke", post(wasm::invoke_wasm_plugin_function))
        // Serve static files from the frontend directory
        .nest_service("/assets", ServeDir::new("lion_ui/frontend/dist/assets"))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default()
                    .include_headers(true))
        )
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Bind to all interfaces on port 8080
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Server started successfully");

    // Serve the application
    axum::serve(listener, app).await.unwrap();
}

// Handler for the root path
async fn index_handler() -> impl IntoResponse {
    let html = include_str!("../frontend/dist/index.html");
    Html(html)
}

// Simple ping handler
async fn ping_handler() -> &'static str {
    "Pong from lion_ui (Phase 5)!"
}
