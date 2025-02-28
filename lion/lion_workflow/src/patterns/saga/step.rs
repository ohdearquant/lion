use crate::model::Priority;
use crate::patterns::saga::types::StepStatus;
use serde::{Deserialize, Serialize};

/// Saga step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStepDefinition {
    /// Step ID
    pub id: String,

    /// Step name
    pub name: String,

    /// Service to call
    pub service: String,

    /// Action to perform
    pub action: String,

    /// Command payload
    pub command: serde_json::Value,

    /// Compensation action
    pub compensation: Option<String>,

    /// Compensation payload
    pub compensation_command: Option<serde_json::Value>,

    /// Timeout for step execution
    pub timeout_ms: u64,

    /// Step priority
    pub priority: Priority,

    /// Step dependencies (IDs of steps that must complete before this one)
    pub dependencies: Vec<String>,

    /// Whether this step is optional
    pub is_optional: bool,

    /// Whether to continue on failure
    pub continue_on_failure: bool,

    /// Whether the failure of this step triggers compensation
    pub triggers_compensation: bool,

    /// Custom metadata
    pub metadata: serde_json::Value,
}

impl SagaStepDefinition {
    /// Create a new saga step definition
    pub fn new(
        id: &str,
        name: &str,
        service: &str,
        action: &str,
        command: serde_json::Value,
    ) -> Self {
        SagaStepDefinition {
            id: id.to_string(),
            name: name.to_string(),
            service: service.to_string(),
            action: action.to_string(),
            command,
            compensation: None,
            compensation_command: None,
            timeout_ms: 30000, // 30 seconds
            priority: Priority::Normal,
            dependencies: Vec::new(),
            is_optional: false,
            continue_on_failure: false,
            triggers_compensation: true,
            metadata: serde_json::Value::Null,
        }
    }

    /// Set compensation action
    pub fn with_compensation(mut self, action: &str, command: serde_json::Value) -> Self {
        self.compensation = Some(action.to_string());
        self.compensation_command = Some(command);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Add a dependency
    pub fn with_dependency(mut self, step_id: &str) -> Self {
        self.dependencies.push(step_id.to_string());
        self
    }

    /// Set as optional
    pub fn as_optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    /// Set to continue on failure
    pub fn continue_on_failure(mut self) -> Self {
        self.continue_on_failure = true;
        self
    }

    /// Set whether this step triggers compensation
    pub fn triggers_compensation(mut self, value: bool) -> Self {
        self.triggers_compensation = value;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Saga step instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    /// Step definition
    pub definition: SagaStepDefinition,

    /// Step status
    pub status: StepStatus,

    /// Step result
    pub result: Option<serde_json::Value>,

    /// Step error
    pub error: Option<String>,

    /// Number of retries
    pub retry_count: u32,

    /// Start time
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,

    /// End time
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Compensation start time
    pub compensation_start_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Compensation end time
    pub compensation_end_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl SagaStep {
    /// Create a new saga step from a definition
    pub fn new(definition: SagaStepDefinition) -> Self {
        SagaStep {
            definition,
            status: StepStatus::Pending,
            result: None,
            error: None,
            retry_count: 0,
            start_time: None,
            end_time: None,
            compensation_start_time: None,
            compensation_end_time: None,
        }
    }

    /// Mark the step as running
    pub fn mark_running(&mut self) {
        self.status = StepStatus::Running;
        self.start_time = Some(chrono::Utc::now());
    }

    /// Mark the step as completed
    pub fn mark_completed(&mut self, result: serde_json::Value) {
        self.status = StepStatus::Completed;
        self.result = Some(result);
        self.end_time = Some(chrono::Utc::now());
    }

    /// Mark the step as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.status = StepStatus::Failed;
        self.error = Some(error.to_string());
        self.end_time = Some(chrono::Utc::now());
    }

    /// Mark the step as compensating
    pub fn mark_compensating(&mut self) {
        self.status = StepStatus::Compensating;
        self.compensation_start_time = Some(chrono::Utc::now());
    }

    /// Mark the step as compensated
    pub fn mark_compensated(&mut self) {
        self.status = StepStatus::Compensated;
        self.compensation_end_time = Some(chrono::Utc::now());
    }

    /// Mark the step compensation as failed
    pub fn mark_compensation_failed(&mut self, error: &str) {
        self.status = StepStatus::CompensationFailed;
        self.error = Some(error.to_string());
        self.compensation_end_time = Some(chrono::Utc::now());
    }

    /// Mark the step as skipped
    pub fn mark_skipped(&mut self) {
        self.status = StepStatus::Skipped;
        self.end_time = Some(chrono::Utc::now());
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Check if the step has a compensation action
    pub fn has_compensation(&self) -> bool {
        self.definition.compensation.is_some()
    }

    /// Check if the step is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            StepStatus::Completed
                | StepStatus::Compensated
                | StepStatus::CompensationFailed
                | StepStatus::Skipped
        )
    }
}
