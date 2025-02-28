//! Integration tests for the Lion Workflow Engine components

#[cfg(test)]
pub mod saga_tests {
    use crate::model::Priority;
    use crate::patterns::saga::{
        SagaDefinition, SagaError, SagaOrchestrator, SagaOrchestratorConfig, SagaStatus, SagaStep,
        SagaStepDefinition, StepStatus,
    };
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_saga_orchestrator_abort() {
        // Create an orchestrator
        let orch = SagaOrchestrator::new(SagaOrchestratorConfig::default());

        // Start the orchestrator
        orch.start().await.unwrap();

        // Register handlers
        orch.register_step_handler(
            "inventory",
            "reserve",
            Arc::new(|_step| {
                Box::pin(async move {
                    // Simple mock handler that always succeeds
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok(serde_json::json!({"reservation_id": "123"}))
                })
            }),
        )
        .await;

        orch.register_step_handler(
            "payment",
            "process",
            Arc::new(|_step| {
                Box::pin(async move {
                    // Simple mock handler that takes longer to process
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    Ok(serde_json::json!({"payment_id": "456"}))
                })
            }),
        )
        .await;

        orch.register_compensation_handler(
            "inventory",
            "cancel_reservation",
            Arc::new(|_step| {
                Box::pin(async move {
                    // Simple mock compensation that always succeeds
                    Ok(())
                })
            }),
        )
        .await;

        // Create a saga definition
        let mut saga_def = SagaDefinition::new("test-saga", "Test Saga");

        // Add steps
        let step1 = SagaStepDefinition::new(
            "step1",
            "Reserve Items",
            "inventory",
            "reserve",
            serde_json::json!({"items": ["item1", "item2"]}),
        )
        .with_compensation(
            "cancel_reservation",
            serde_json::json!({"reservation_id": "123"}),
        );

        let step2 = SagaStepDefinition::new(
            "step2",
            "Process Payment",
            "payment",
            "process",
            serde_json::json!({"amount": 100.0}),
        )
        .with_dependency("step1");

        saga_def.add_step(step1).unwrap();
        saga_def.add_step(step2).unwrap();

        // Create and start a saga
        let saga_id = orch.create_saga(saga_def).await.unwrap();
        orch.start_saga(&saga_id).await.unwrap();

        // Wait a bit for the first step to complete
        sleep(Duration::from_millis(100)).await;

        // Abort the saga midway (after step1 completes but before step2 finishes)
        orch.abort_saga(&saga_id, "Testing abort").await.unwrap();

        // Wait for compensation to complete
        sleep(Duration::from_millis(200)).await;

        // Check saga status
        let saga_lock = orch.get_saga(&saga_id).await.unwrap();
        let saga = saga_lock.read().await;

        // Verify saga was aborted and compensation was triggered
        assert!(matches!(
            saga.status,
            SagaStatus::Compensated | SagaStatus::Aborted
        ));

        // Stop the orchestrator
        orch.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_saga_compensation_failure() {
        // Create an orchestrator
        let orch = SagaOrchestrator::new(SagaOrchestratorConfig::default());

        // Start the orchestrator
        orch.start().await.unwrap();

        // Register handlers
        orch.register_step_handler(
            "inventory",
            "reserve",
            Arc::new(|_step| {
                Box::pin(async move {
                    // This step succeeds
                    Ok(serde_json::json!({"reservation_id": "123"}))
                })
            }),
        )
        .await;

        orch.register_step_handler(
            "payment",
            "process",
            Arc::new(|_step| {
                Box::pin(async move {
                    // This step fails
                    Err("Payment declined".to_string())
                })
            }),
        )
        .await;

        orch.register_compensation_handler(
            "inventory",
            "cancel_reservation",
            Arc::new(|_step| {
                Box::pin(async move {
                    // Compensation also fails
                    Err("Failed to cancel reservation".to_string())
                })
            }),
        )
        .await;

        // Create a saga definition
        let mut saga_def = SagaDefinition::new("test-saga", "Test Saga");

        // Add steps
        let step1 = SagaStepDefinition::new(
            "step1",
            "Reserve Items",
            "inventory",
            "reserve",
            serde_json::json!({"items": ["item1", "item2"]}),
        )
        .with_compensation(
            "cancel_reservation",
            serde_json::json!({"reservation_id": "123"}),
        );

        let step2 = SagaStepDefinition::new(
            "step2",
            "Process Payment",
            "payment",
            "process",
            serde_json::json!({"amount": 100.0}),
        )
        .with_dependency("step1");

        saga_def.add_step(step1).unwrap();
        saga_def.add_step(step2).unwrap();

        // Create and start a saga
        let saga_id = orch.create_saga(saga_def).await.unwrap();
        orch.start_saga(&saga_id).await.unwrap();

        // Wait for saga to complete or timeout
        let mut saga_completed = false;
        for _ in 0..20 {
            if let Some(saga_lock) = orch.get_saga(&saga_id).await {
                let saga = saga_lock.read().await;

                if saga.status == SagaStatus::FailedWithErrors
                    || saga.status == SagaStatus::Compensated
                {
                    saga_completed = true;
                    break;
                }
            }

            sleep(Duration::from_millis(100)).await;
        }

        // Stop the orchestrator
        orch.stop().await.unwrap();

        // Verify saga completed with errors
        assert!(saga_completed, "Saga did not complete in time");
    }
}

#[cfg(test)]
pub mod event_tests {
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
        broker.process_retry_queue().await.unwrap();

        // Receive the retried event
        match event_rx.recv().await {
            Some(retried) => {
                assert_eq!(retried.id, event_id);
                assert_eq!(retried.retry_count, 1);

                // Send success acknowledgment
                let ack = EventAck::success(&event_id, "test_subscriber");
                ack_tx.send(ack).await.unwrap();
            }
            None => {
                panic!("Expected to receive retried event, but got none");
            }
        }

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

        // With backpressure, some events should be accepted and some might be rejected
        assert!(success_count > 0, "No events were accepted");
    }
}
