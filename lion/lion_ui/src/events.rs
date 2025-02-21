use axum::{
    extract::State,
    response::{sse::Event, Sse},
};
use futures::{stream::Stream, StreamExt};
use lion_core::SystemEvent;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::plugins::PluginInfo;

/// Shared state for the UI server
pub struct AppState {
    /// Channel for broadcasting log events to all connected clients
    pub logs_tx: broadcast::Sender<String>,
    /// The orchestrator instance
    pub orchestrator_sender: tokio::sync::mpsc::Sender<SystemEvent>,
    /// Active agents and their status
    pub agents: RwLock<HashMap<Uuid, String>>,
    /// Active plugins and their IDs
    pub plugins: RwLock<HashMap<Uuid, PluginInfo>>,
}

impl AppState {
    pub fn new(
        orchestrator_sender: tokio::sync::mpsc::Sender<SystemEvent>,
        channel_capacity: usize,
    ) -> Self {
        let (logs_tx, _) = broadcast::channel(channel_capacity);
        Self {
            logs_tx,
            orchestrator_sender,
            agents: RwLock::new(HashMap::new()),
            plugins: RwLock::new(HashMap::new()),
        }
    }

    pub fn new_with_logs(
        orchestrator_sender: tokio::sync::mpsc::Sender<SystemEvent>,
        logs_tx: broadcast::Sender<String>,
    ) -> Self {
        // Use provided logs_tx instead of creating a new one
        Self {
            logs_tx,
            orchestrator_sender,
            agents: RwLock::new(HashMap::new()),
            plugins: RwLock::new(HashMap::new()),
        }
    }
}

/// Handler for SSE events stream
pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.logs_tx.subscribe();

    let stream = BroadcastStream::new(rx).map(|msg| {
        let msg = msg.unwrap_or_else(|e| format!("Error receiving message: {}", e));
        Ok(Event::default().data(msg))
    });

    Sse::new(stream)
}
