use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use futures::{Stream, StreamExt};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

use crate::state::AppState;

pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("Client connected to SSE stream");

    // Subscribe to the logs channel
    let rx = state.logs_tx.subscribe();

    // Convert the broadcast receiver into a stream of SSE events
    let stream = BroadcastStream::new(rx).map(|msg| {
        let data = msg.unwrap_or_else(|e| format!("Error receiving message: {}", e));
        Ok::<_, Infallible>(Event::default().data(data))
    });

    // Add keep-alive for long-running connections
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use http_body_util::BodyExt;
    use tokio::sync::broadcast;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_sse_endpoint() {
        // Create test state
        let (tx, _) = broadcast::channel(100);
        let state = Arc::new(AppState {
            orchestrator_tx: tokio::sync::mpsc::channel(100).0,
            plugins: tokio::sync::RwLock::new(std::collections::HashMap::new()),
            logs_tx: tx,
        });

        let app = Router::new()
            .route("/events", get(sse_handler))
            .with_state(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/events")
                    .header("Accept", "text/event-stream")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["content-type"],
            "text/event-stream;charset=utf-8"
        );
    }
}
