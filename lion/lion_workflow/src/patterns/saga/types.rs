use crate::model::Priority;
use crate::patterns::event::types::EventError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Error types for saga transactions
#[derive(Error, Debug)]
pub enum SagaError {
    #[error("Step execution failed: {0}")]
    StepFailed(String),

    #[error("Compensation failed: {0}")]
    CompensationFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Saga definition error: {0}")]
    DefinitionError(String),

    #[error("Saga already exists: {0}")]
    AlreadyExists(String),

    #[error("Saga not found: {0}")]
    NotFound(String),

    #[error("Event error: {0}")]
    EventError(#[from] EventError),

    #[error("Step not found: {0}")]
    StepNotFound(String),

    #[error("Saga aborted")]
    Aborted,

    #[error("Other error: {0}")]
    Other(String),
}

/// Saga status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaStatus {
    /// Saga created but not yet started
    Created,

    /// Saga is running
    Running,

    /// Saga completed successfully
    Completed,

    /// Saga failed and compensations were triggered
    Failed,

    /// Saga is in the process of compensating
    Compensating,

    /// Saga compensation completed
    Compensated,

    /// Saga failed and compensation also failed
    FailedWithErrors,

    /// Saga was aborted
    Aborted,
}

/// Step status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Step created but not yet started
    Pending,

    /// Step is running
    Running,

    /// Step completed successfully
    Completed,

    /// Step failed
    Failed,

    /// Step compensation is running
    Compensating,

    /// Step compensation completed
    Compensated,

    /// Step compensation failed
    CompensationFailed,

    /// Step was skipped
    Skipped,
}

/// Saga coordination strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaStrategy {
    /// Orchestration: central coordinator issues commands
    Orchestration,

    /// Choreography: events trigger next steps
    Choreography,
}

impl Default for SagaStrategy {
    fn default() -> Self {
        SagaStrategy::Orchestration
    }
}

/// Result of a saga step execution
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Step ID
    pub step_id: String,

    /// Step status
    pub status: StepStatus,

    /// Step result data
    pub data: Option<serde_json::Value>,

    /// Step error
    pub error: Option<String>,

    /// Saga instance ID
    pub saga_id: String,
}

/// Handler function type for saga steps
pub type StepHandler = Arc<
    dyn Fn(
            &crate::patterns::saga::step::SagaStep,
        ) -> Box<dyn Future<Output = Result<serde_json::Value, String>> + Send + Unpin>
        + Send
        + Sync,
>;

/// Handler function type for saga step compensations
pub type CompensationHandler = Arc<
    dyn Fn(
            &crate::patterns::saga::step::SagaStep,
        ) -> Box<dyn Future<Output = Result<(), String>> + Send + Unpin>
        + Send
        + Sync,
>;

/// Queued saga compensation task
#[derive(Debug, Clone)]
pub struct CompensationTask {
    pub saga_id: String,
}

/// Queued saga abort task
#[derive(Debug, Clone)]
pub struct AbortTask {
    pub saga_id: String,
    pub reason: String,
}

/// Configuration for saga orchestrator
#[derive(Debug, Clone)]
pub struct SagaOrchestratorConfig {
    /// Maximum number of concurrent sagas
    pub max_concurrent_sagas: usize,

    /// Default saga timeout
    pub default_timeout_ms: u64,

    /// Check interval for saga timeouts
    pub check_interval_ms: u64,

    /// Whether to use idempotent operations by default
    pub use_idempotent_operations: bool,

    /// Whether to clean up completed sagas automatically
    pub auto_cleanup: bool,

    /// How long to keep completed sagas (in ms)
    pub cleanup_after_ms: u64,

    /// Default number of retries
    pub default_retries: u32,

    /// Default retry delay
    pub default_retry_delay_ms: u64,

    /// Channel buffer size
    pub channel_buffer_size: usize,
}

impl Default for SagaOrchestratorConfig {
    fn default() -> Self {
        SagaOrchestratorConfig {
            max_concurrent_sagas: 100,
            default_timeout_ms: 300000, // 5 minutes
            check_interval_ms: 1000,    // 1 second
            use_idempotent_operations: true,
            auto_cleanup: true,
            cleanup_after_ms: 3600000, // 1 hour
            default_retries: 3,
            default_retry_delay_ms: 1000, // 1 second
            channel_buffer_size: 1000,
        }
    }
}
