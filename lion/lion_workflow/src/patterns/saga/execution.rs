use crate::patterns::saga::step::SagaStep;
use crate::patterns::saga::types::{SagaError, StepResult, StepStatus};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::timeout;

/// Helper functions for saga step execution
pub struct StepExecutor;

impl StepExecutor {
    /// Execute a step with timeout
    pub async fn execute_step<F, Fut>(
        step: &SagaStep,
        executor: F,
        timeout_duration: Duration,
    ) -> Result<StepResult, SagaError>
    where
        F: FnOnce(&SagaStep) -> Fut,
        Fut: Future<Output = Result<serde_json::Value, String>>,
    {
        // Execute the step with timeout
        match timeout(timeout_duration, executor(step)).await {
            Ok(Ok(data)) => {
                // Step succeeded
                Ok(StepResult {
                    step_id: step.definition.id.clone(),
                    status: StepStatus::Completed,
                    data: Some(data),
                    error: None,
                    saga_id: "".to_string(), // Will be set by the caller
                })
            }
            Ok(Err(error)) => {
                // Step failed
                Ok(StepResult {
                    step_id: step.definition.id.clone(),
                    status: StepStatus::Failed,
                    data: None,
                    error: Some(error),
                    saga_id: "".to_string(), // Will be set by the caller
                })
            }
            Err(_) => {
                // Timeout
                Err(SagaError::Timeout(format!(
                    "Step timed out after {}ms",
                    timeout_duration.as_millis()
                )))
            }
        }
    }

    /// Execute compensation for a step
    pub async fn execute_compensation<F, Fut>(
        step: &SagaStep,
        compensator: F,
        timeout_duration: Duration,
    ) -> Result<(), SagaError>
    where
        F: FnOnce(&SagaStep) -> Fut,
        Fut: Future<Output = Result<(), String>>,
    {
        // Execute the compensation with timeout
        match timeout(timeout_duration, compensator(step)).await {
            Ok(Ok(_)) => {
                // Compensation succeeded
                Ok(())
            }
            Ok(Err(error)) => {
                // Compensation failed
                Err(SagaError::CompensationFailed(error))
            }
            Err(_) => {
                // Timeout
                Err(SagaError::Timeout(format!(
                    "Compensation timed out after {}ms",
                    timeout_duration.as_millis()
                )))
            }
        }
    }
}

/// Helpers for ensuring futures are properly boxed
pub struct FutureHelper;

impl FutureHelper {
    /// Box an async function returning a serde_json::Value
    pub fn box_step_handler<F>(
        f: F,
    ) -> Box<dyn Future<Output = Result<serde_json::Value, String>> + Send + Unpin>
    where
        F: Future<Output = Result<serde_json::Value, String>> + Send + 'static,
    {
        // We use this approach to convert a Future into a properly boxed, Unpin future
        struct BoxedFuture<T>(Pin<Box<dyn Future<Output = T> + Send>>);

        impl<T> Future for BoxedFuture<T> {
            type Output = T;
            fn poll(
                mut self: Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                self.0.as_mut().poll(cx)
            }
        }

        impl<T> Unpin for BoxedFuture<T> {}

        Box::new(BoxedFuture(Box::pin(f)))
    }

    /// Box an async function for compensation
    pub fn box_compensation_handler<F>(
        f: F,
    ) -> Box<dyn Future<Output = Result<(), String>> + Send + Unpin>
    where
        F: Future<Output = Result<(), String>> + Send + 'static,
    {
        // Same approach as above but for a different return type
        struct BoxedFuture<T>(Pin<Box<dyn Future<Output = T> + Send>>);

        impl<T> Future for BoxedFuture<T> {
            type Output = T;
            fn poll(
                mut self: Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                self.0.as_mut().poll(cx)
            }
        }

        impl<T> Unpin for BoxedFuture<T> {}

        Box::new(BoxedFuture(Box::pin(f)))
    }
}
