use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use futures::{stream::Stream, StreamExt};
use std::{convert::Infallible, sync::Arc};
use tokio_stream::wrappers::BroadcastStream;
use tracing::error;

use crate::state::AppState;

/// Server-Sent Events handler for streaming logs in real-time
pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Subscribe to the logs broadcast channel
    let rx = state.logs_tx.subscribe();

    // Convert broadcast receiver to a stream of SSE events
    let stream = BroadcastStream::new(rx).map(|msg| {
        match msg {
            Ok(log) => {
                // Convert LogEntry to a JSON string for the event
                match serde_json::to_string(&log) {
                    Ok(json) => Ok(Event::default().data(json)),
                    Err(e) => {
                        error!("Failed to serialize log entry: {}", e);
                        Ok(Event::default().comment("Error serializing log"))
                    }
                }
            }
            Err(e) => {
                error!("Error receiving from broadcast: {}", e);
                Ok(Event::default().comment("Error receiving log"))
            }
        }
    });

    Sse::new(stream)
}
