//! Saga orchestration pattern for distributed transactions

pub mod definition;
pub mod execution;
pub mod orchestrator;
pub mod step;
pub mod types;

// Re-exports
pub use definition::SagaDefinition;
pub use orchestrator::SagaOrchestrator;
pub use step::SagaStep;
pub use step::SagaStepDefinition;
pub use types::{
    AbortTask, CompensationTask, SagaError, SagaStatus, SagaStrategy, StepResult, StepStatus,
};
pub use types::{CompensationHandler, SagaOrchestratorConfig, StepHandler};

#[cfg(test)]
mod tests {
    // Import necessary items for testing
    // Add test modules here as needed
}
