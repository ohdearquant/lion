use axum::{
    extract::{Query, State},
    response::{sse::Event, Sse},
    Json,
};
use chrono::{DateTime, Utc};
use futures::{stream::Stream, StreamExt};
use lion_core::SystemEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::plugins::PluginInfo;

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub agent_id: Option<Uuid>,
    pub plugin_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct LogFilter {
    pub agent: Option<String>,
    pub plugin: Option<String>,
    pub text: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

const MAX_LOG_BUFFER: usize = 10000;

#[allow(dead_code)]
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
    /// Plugin manager manifest directory
    pub plugins_dir: RwLock<String>,
    /// Log buffer for searching
    pub log_buffer: RwLock<Vec<LogLine>>,
}

#[allow(dead_code)]
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
            plugins_dir: RwLock::new(String::from("plugins")),
            log_buffer: RwLock::new(Vec::with_capacity(MAX_LOG_BUFFER)),
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
            plugins_dir: RwLock::new(String::from("plugins")),
            log_buffer: RwLock::new(Vec::with_capacity(MAX_LOG_BUFFER)),
        }
    }

    pub async fn add_log(&self, line: LogLine) {
        let mut buffer = self.log_buffer.write().await;
        if buffer.len() >= MAX_LOG_BUFFER {
            buffer.remove(0); // Remove oldest log when buffer is full
        }

        // Also broadcast to real-time listeners
        let message = format!("[{}] {}", line.timestamp, line.message);
        let _ = self.logs_tx.send(message);

        // Store the log line after broadcasting
        buffer.push(line);
    }

    pub async fn search_logs(&self, filter: &LogFilter) -> Vec<LogLine> {
        let buffer = self.log_buffer.read().await;

        let mut results: Vec<LogLine> = buffer
            .iter()
            .filter(|line| {
                // Match agent ID if specified
                if let Some(agent) = &filter.agent {
                    if let Some(id) = line.agent_id {
                        if !id.to_string().contains(agent) {
                            return false;
                        }
                    }
                }

                // Match plugin ID if specified
                if let Some(plugin) = &filter.plugin {
                    if let Some(id) = line.plugin_id {
                        if !id.to_string().contains(plugin) {
                            return false;
                        }
                    }
                }

                // Match text if specified
                if let Some(text) = &filter.text {
                    if !line.message.contains(text) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        results.truncate(filter.limit);
        results
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

/// Handler for searching logs
pub async fn search_logs_handler(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<LogFilter>,
) -> Json<Vec<LogLine>> {
    Json(state.search_logs(&filter).await)
}

fn default_limit() -> usize {
    1000 // Default limit for log search results
}
