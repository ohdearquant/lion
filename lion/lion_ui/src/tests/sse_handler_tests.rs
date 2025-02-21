use crate::events::{AppState, LogLine};
use axum::{routing::get, Router};
use futures::StreamExt;
use reqwest::Client;
use reqwest_eventsource::{Event as EventSourceEvent, RequestBuilderExt};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tokio::time::sleep;

async fn setup_test_server() -> (SocketAddr, Arc<AppState>) {
    // Create app state
    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(AppState::new_with_logs(
        tokio::sync::mpsc::channel(100).0,
        tx,
    ));

    // Build router
    let app = Router::new()
        .route("/api/logs/stream", get(crate::events::sse_handler))
        .with_state(state.clone());

    // Bind to random port
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn server
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Wait for server to start
    sleep(Duration::from_millis(100)).await;

    (addr, state)
}

#[tokio::test]
async fn test_sse_initialization() {
    let (addr, _state) = setup_test_server().await;

    // Connect to SSE endpoint
    let client = Client::new();
    let request = client.get(format!("http://{}/api/logs/stream", addr));
    let mut es = request.eventsource().unwrap();

    // No events should be received initially
    let event = tokio::time::timeout(Duration::from_millis(100), es.next()).await;
    assert!(event.is_err(), "Expected timeout with no events");
}

#[tokio::test]
async fn test_sse_multiple_subscribers() {
    let (addr, state) = setup_test_server().await;
    let url = format!("http://{}/api/logs/stream", addr);

    // Create two subscribers
    let client = Client::new();
    let mut es1 = client.get(&url).eventsource().unwrap();
    let mut es2 = client.get(&url).eventsource().unwrap();

    // Add a log entry
    state
        .add_log(LogLine {
            timestamp: chrono::Utc::now(),
            agent_id: None,
            plugin_id: None,
            correlation_id: None,
            message: "Test message".to_string(),
        })
        .await;

    // Both subscribers should receive the message
    let event1 = es1.next().await.unwrap().unwrap();
    let event2 = es2.next().await.unwrap().unwrap();

    match (event1, event2) {
        (EventSourceEvent::Message(msg1), EventSourceEvent::Message(msg2)) => {
            assert!(msg1.data.contains("Test message"));
            assert!(msg2.data.contains("Test message"));
        }
        _ => panic!("Expected Message events"),
    }
}

#[tokio::test]
async fn test_sse_error_handling() {
    let (addr, state) = setup_test_server().await;
    let url = format!("http://{}/api/logs/stream", addr);
    let client = Client::new();
    let mut es = client.get(&url).eventsource().unwrap();

    // Fill the channel to cause errors
    for i in 0..10 {
        state
            .add_log(LogLine {
                timestamp: chrono::Utc::now(),
                agent_id: None,
                plugin_id: None,
                correlation_id: None,
                message: format!("Message {}", i),
            })
            .await;
    }

    // Should still receive messages or error messages
    let event = es.next().await.unwrap().unwrap();
    match event {
        EventSourceEvent::Message(msg) => {
            assert!(msg.data.contains("Message") || msg.data.contains("Error receiving message"));
        }
        _ => panic!("Expected Message event"),
    }
}

#[tokio::test]
async fn test_sse_stream_backpressure() {
    let (addr, state) = setup_test_server().await;
    let url = format!("http://{}/api/logs/stream", addr);
    let client = Client::new();
    let mut es = client.get(&url).eventsource().unwrap();

    // Spawn task to continuously add logs
    let state_clone = state.clone();
    tokio::spawn(async move {
        for i in 0..10 {
            state_clone
                .add_log(LogLine {
                    timestamp: chrono::Utc::now(),
                    agent_id: None,
                    plugin_id: None,
                    correlation_id: None,
                    message: format!("Message {}", i),
                })
                .await;
            sleep(Duration::from_millis(10)).await;
        }
    });

    // Collect messages with delay to test backpressure
    let mut messages = Vec::new();
    while let Ok(Some(Ok(event))) =
        tokio::time::timeout(Duration::from_millis(100), es.next()).await
    {
        if let EventSourceEvent::Message(msg) = event {
            messages.push(msg);
            if messages.len() >= 5 {
                break;
            }
        }
        sleep(Duration::from_millis(50)).await;
    }

    assert!(!messages.is_empty());
    assert!(messages.len() <= 5);
}

#[tokio::test]
async fn test_sse_message_ordering() {
    let (addr, state) = setup_test_server().await;
    let url = format!("http://{}/api/logs/stream", addr);
    let client = Client::new();
    let mut es = client.get(&url).eventsource().unwrap();

    // Add logs with sequential numbers
    for i in 0..5 {
        state
            .add_log(LogLine {
                timestamp: chrono::Utc::now(),
                agent_id: None,
                plugin_id: None,
                correlation_id: None,
                message: format!("Message {}", i),
            })
            .await;
        sleep(Duration::from_millis(10)).await;
    }

    // Collect and verify messages
    let mut messages = Vec::new();
    for _ in 0..5 {
        if let Ok(Some(Ok(EventSourceEvent::Message(msg)))) =
            tokio::time::timeout(Duration::from_millis(100), es.next()).await
        {
            messages.push(msg);
        }
    }

    assert_eq!(messages.len(), 5);
    for (i, msg) in messages.iter().enumerate() {
        assert!(msg.data.contains(&format!("Message {}", i)));
    }
}
