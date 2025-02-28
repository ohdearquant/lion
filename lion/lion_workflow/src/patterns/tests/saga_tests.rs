use crate::model::Priority;
use crate::patterns::saga::{
    SagaDefinition, SagaError, SagaOrchestrator, SagaOrchestratorConfig, SagaStatus,
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
            Box::new(async move {
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
            Box::new(async move {
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
            Box::new(async move {
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
    assert_eq!(saga.status, SagaStatus::Compensated);
    assert_eq!(saga.steps["step1"].status, StepStatus::Compensated);

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
            Box::new(async move {
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
            Box::new(async move {
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
            Box::new(async move {
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
    for _ in 0..10 {
        if let Some(saga_lock) = orch.get_saga(&saga_id).await {
            let saga = saga_lock.read().await;

            if saga.status == SagaStatus::FailedWithErrors {
                saga_completed = true;
                break;
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    // Verify saga failed with compensation errors
    assert!(saga_completed, "Saga did not complete in time");

    let saga_lock = orch.get_saga(&saga_id).await.unwrap();
    let saga = saga_lock.read().await;

    assert_eq!(saga.status, SagaStatus::FailedWithErrors);
    assert_eq!(saga.steps["step1"].status, StepStatus::CompensationFailed);
    assert_eq!(saga.steps["step2"].status, StepStatus::Failed);

    // Stop the orchestrator
    orch.stop().await.unwrap();
}

#[tokio::test]
async fn test_saga_orchestrator_integration_with_events() {
    use crate::patterns::event::{Event, EventBroker, EventBrokerConfig};

    // Create event broker
    let event_broker = Arc::new(EventBroker::new(EventBrokerConfig::default()));

    // Create saga orchestrator
    let config = SagaOrchestratorConfig {
        channel_buffer_size: 10,
        ..Default::default()
    };
    let orch = SagaOrchestrator::new(config).with_event_broker(event_broker.clone());

    // Start the orchestrator
    orch.start().await.unwrap();

    // Register a step handler that uses the event broker
    orch.register_step_handler(
        "notification",
        "send",
        Arc::new(move |step| {
            let event_broker = event_broker.clone();
            Box::new(async move {
                // Create and publish an event
                let event = Event::new(
                    "notification.sent",
                    step.definition.command.clone(),
                )
                .with_source("saga");

                let _ = event_broker.publish(event).await;
                Ok(serde_json::json!({"notification_id": "789"}))
            })
        }),
    )
    .await;

    // Create a saga definition with a notification step
    let mut saga_def = SagaDefinition::new("notification-saga", "Notification Saga");
    let step = SagaStepDefinition::new(
        "notify",
        "Send Notification",
        "notification",
        "send",
        serde_json::json!({"message": "Hello world"}),
    );
    saga_def.add_step(step).unwrap();

    // Create and start a saga
    let saga_id = orch.create_saga(saga_def).await.unwrap();
    orch.start_saga(&saga_id).await.unwrap();

    // Wait for saga to complete
    sleep(Duration::from_millis(200)).await;

    // Check saga status
    let saga_lock = orch.get_saga(&saga_id).await.unwrap();
    let saga = saga_lock.read().await;

    assert_eq!(saga.status, SagaStatus::Completed);
    assert_eq!(saga.steps["notify"].status, StepStatus::Completed);

    // Stop the orchestrator
    orch.stop().await.unwrap();
}