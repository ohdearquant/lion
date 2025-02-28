use crate::patterns::event::{
    DeliverySemantic, Event, EventAck, EventBroker, EventBrokerConfig, EventStatus,
    InMemoryEventStore,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_event_broker_retry() {
    // Create a broker with retry capability
    let config = EventBrokerConfig {
        delivery_semantic: DeliverySemantic::AtLeastOnce,
        max_retries: 3,
        ack_timeout: Duration::from_millis(100),
        ..Default::default()
    };

    let broker = EventBroker::new(config);

    // Subscribe to events
    let (mut event_rx, ack_tx) = broker
        .subscribe("test_event", "test_subscriber", None)
        .await
        .unwrap();

    // Create and publish an event
    let event = Event::new("test_event", serde_json::json!({"data": "test"}));
    let event_id = event.id.clone();

    let status = broker.publish(event).await.unwrap();
    assert_eq!(status, EventStatus::Sent);

    // Receive event
    let received = event_rx.recv().await.unwrap();
    assert_eq!(received.id, event_id);

    // Send failure acknowledgment to trigger retry
    let ack = EventAck::failure(&event_id, "test_subscriber", "Test failure");
    ack_tx.send(ack).await.unwrap();

    // Wait for the event to be enqueued for retry
    sleep(Duration::from_millis(200)).await;

    // Check if event was added to retry queue
    let retry_count = broker.get_retry_queue_size().await;
    assert!(retry_count > 0, "Event was not added to retry queue");

    // Process the retry queue
    broker.process_retry_queue().await;

    // Receive the retried event
    let retried = event_rx.recv().await.unwrap();
    assert_eq!(retried.id, event_id);
    assert_eq!(retried.retry_count, 1);

    // Send success acknowledgment
    let ack = EventAck::success(&event_id, "test_subscriber");
    ack_tx.send(ack).await.unwrap();

    // Wait for processing
    sleep(Duration::from_millis(200)).await;

    // Check if event was marked as processed
    assert!(broker.is_event_processed(&event_id).await);
}

#[tokio::test]
async fn test_event_broker_backpressure() {
    // Create a broker with small buffer size to test backpressure
    let config = EventBrokerConfig {
        delivery_semantic: DeliverySemantic::AtLeastOnce,
        channel_buffer_size: 5,
        enable_backpressure: true,
        max_in_flight: 10,
        ..Default::default()
    };

    let broker = EventBroker::new(config);

    // Subscribe to events
    let (_event_rx, _ack_tx) = broker
        .subscribe("test_event", "test_subscriber", None)
        .await
        .unwrap();

    // Publish many events to trigger backpressure
    let mut success_count = 0;
    let mut failure_count = 0;

    for i in 0..20 {
        let event = Event::new(
            "test_event",
            serde_json::json!({"data": format!("test-{}", i)}),
        );

        match broker.publish(event).await {
            Ok(_) => success_count += 1,
            Err(_) => failure_count += 1,
        }
    }

    // With backpressure, some events should be rejected
    assert!(success_count > 0, "No events were accepted");
    assert!(
        failure_count > 0 || broker.get_in_flight_count().await >= 10,
        "Backpressure did not trigger"
    );
}

#[tokio::test]
async fn test_event_store_integration() {
    // Create an event store
    let store = Arc::new(InMemoryEventStore::new());
    
    // Create a broker with the store
    let config = EventBrokerConfig::default();
    let broker = EventBroker::new(config).with_event_store(store.clone());

    // Create and publish an event
    let event = Event::new("test_event", serde_json::json!({"data": "test"}))
        .with_source("test_source")
        .with_correlation_id("corr-123");

    let event_id = event.id.clone();
    broker.publish(event).await.unwrap();

    // Verify the event was stored
    let loaded = store.load_event(&event_id).await.unwrap();
    assert_eq!(loaded.id, event_id);
    assert_eq!(loaded.source, "test_source");
    assert_eq!(loaded.correlation_id, Some("corr-123".to_string()));

    // Test replay functionality
    let replay_count = broker.replay_events(Some(vec!["test_event".to_string()])).await.unwrap();
    assert_eq!(replay_count, 1);
}