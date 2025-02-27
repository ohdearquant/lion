use crate::engine::context::{ExecutionContext, ContextError};
use crate::engine::NodeResult;
use crate::model::{WorkflowDefinition, WorkflowId, NodeId, NodeStatus, Priority};
use crate::patterns::event::{Event, EventBroker, EventError, EventStatus, DeliverySemantic};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn new(id: &str, name: &str, service: &str, action: &str, command: serde_json::Value) -> Self {
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

/// Saga definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaDefinition {
    /// Saga ID
    pub id: String,
    
    /// Saga name
    pub name: String,
    
    /// Saga steps
    pub steps: Vec<SagaStepDefinition>,
    
    /// Saga coordination strategy
    #[serde(default)]
    pub strategy: SagaStrategy,
    
    /// Maximum execution time for the entire saga
    pub timeout_ms: u64,
    
    /// Number of retries for failed steps
    pub max_retries: u32,
    
    /// Duration between retries
    pub retry_delay_ms: u64,
    
    /// Whether to use idempotent operations
    pub use_idempotent_operations: bool,
    
    /// Custom metadata
    pub metadata: serde_json::Value,
}

impl SagaDefinition {
    /// Create a new saga definition
    pub fn new(id: &str, name: &str) -> Self {
        SagaDefinition {
            id: id.to_string(),
            name: name.to_string(),
            steps: Vec::new(),
            strategy: SagaStrategy::Orchestration,
            timeout_ms: 300000, // 5 minutes
            max_retries: 3,
            retry_delay_ms: 1000, // 1 second
            use_idempotent_operations: true,
            metadata: serde_json::Value::Null,
        }
    }
    
    /// Add a step to the saga
    pub fn add_step(&mut self, step: SagaStepDefinition) -> Result<(), SagaError> {
        // Check if step ID is unique
        if self.steps.iter().any(|s| s.id == step.id) {
            return Err(SagaError::DefinitionError(format!("Step ID already exists: {}", step.id)));
        }
        
        // Validate step dependencies
        for dep_id in &step.dependencies {
            if !self.steps.iter().any(|s| &s.id == dep_id) && !self.steps.iter().any(|s| s.id == *dep_id) {
                return Err(SagaError::DefinitionError(format!("Dependency not found: {}", dep_id)));
            }
        }
        
        // Add the step
        self.steps.push(step);
        
        Ok(())
    }
    
    /// Set the saga strategy
    pub fn with_strategy(mut self, strategy: SagaStrategy) -> Self {
        self.strategy = strategy;
        self
    }
    
    /// Set the saga timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
    
    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32, retry_delay_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_ms = retry_delay_ms;
        self
    }
    
    /// Set whether to use idempotent operations
    pub fn with_idempotence(mut self, use_idempotent: bool) -> Self {
        self.use_idempotent_operations = use_idempotent;
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
    
    /// Validate the saga definition
    pub fn validate(&self) -> Result<(), SagaError> {
        // Check for empty steps
        if self.steps.is_empty() {
            return Err(SagaError::DefinitionError("Saga has no steps".to_string()));
        }
        
        // Check for cycles in dependencies
        let mut visited = HashSet::new();
        let mut path = HashSet::new();
        
        // Create an adjacency list representation of the dependency graph
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for step in &self.steps {
            let deps = step.dependencies.clone();
            graph.insert(step.id.clone(), deps);
        }
        
        // Check for cycles using DFS
        for step in &self.steps {
            if !visited.contains(&step.id) {
                if self.has_cycle_dfs(&step.id, &graph, &mut visited, &mut path) {
                    return Err(SagaError::DefinitionError("Dependency cycle detected".to_string()));
                }
            }
        }
        
        Ok(())
    }
    
    // Helper for cycle detection
    fn has_cycle_dfs(
        &self,
        step_id: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        path: &mut HashSet<String>,
    ) -> bool {
        visited.insert(step_id.to_string());
        path.insert(step_id.to_string());
        
        if let Some(deps) = graph.get(step_id) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_cycle_dfs(dep, graph, visited, path) {
                        return true;
                    }
                } else if path.contains(dep) {
                    return true; // Cycle detected
                }
            }
        }
        
        path.remove(step_id);
        false
    }
    
    /// Get steps in execution order
    pub fn get_execution_order(&self) -> Result<Vec<String>, SagaError> {
        // Validate first
        self.validate()?;
        
        // Topological sort
        let mut result = Vec::new();
        let mut in_degree = HashMap::new();
        let mut zero_degree = Vec::new();
        
        // Create an adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut reverse_graph: HashMap<String, Vec<String>> = HashMap::new();
        
        // Initialize adjacency list and in-degree
        for step in &self.steps {
            graph.insert(step.id.clone(), Vec::new());
            reverse_graph.insert(step.id.clone(), step.dependencies.clone());
            in_degree.insert(step.id.clone(), step.dependencies.len());
            
            if step.dependencies.is_empty() {
                zero_degree.push(step.id.clone());
            }
        }
        
        // Add edges (reversed because dependencies)
        for step in &self.steps {
            for dep in &step.dependencies {
                graph.entry(dep.clone())
                    .or_insert_with(Vec::new)
                    .push(step.id.clone());
            }
        }
        
        // Topological sort
        while !zero_degree.is_empty() {
            let step_id = zero_degree.pop().unwrap();
            result.push(step_id.clone());
            
            if let Some(dependents) = graph.get(&step_id) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            zero_degree.push(dependent.clone());
                        }
                    }
                }
            }
        }
        
        // Check if all steps were included (no cycles)
        if result.len() != self.steps.len() {
            return Err(SagaError::DefinitionError("Dependency cycle detected".to_string()));
        }
        
        Ok(result)
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

/// Saga instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saga {
    /// Saga definition
    pub definition: SagaDefinition,
    
    /// Saga instance ID
    pub instance_id: String,
    
    /// Saga status
    pub status: SagaStatus,
    
    /// Saga steps
    pub steps: HashMap<String, SagaStep>,
    
    /// Step execution order
    pub execution_order: Vec<String>,
    
    /// Creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Start time
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    
    /// End time
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Correlation ID
    pub correlation_id: Option<String>,
    
    /// Initiator ID
    pub initiator: Option<String>,
    
    /// Overall result
    pub result: Option<serde_json::Value>,
    
    /// Overall error
    pub error: Option<String>,
}

impl Saga {
    /// Create a new saga from a definition
    pub fn new(definition: SagaDefinition) -> Result<Self, SagaError> {
        // Validate the definition
        definition.validate()?;
        
        // Get execution order
        let execution_order = definition.get_execution_order()?;
        
        // Create steps
        let mut steps = HashMap::new();
        for step_def in &definition.steps {
            steps.insert(step_def.id.clone(), SagaStep::new(step_def.clone()));
        }
        
        Ok(Saga {
            definition: definition.clone(),
            instance_id: format!("saga-{}", Uuid::new_v4()),
            status: SagaStatus::Created,
            steps,
            execution_order,
            created_at: chrono::Utc::now(),
            start_time: None,
            end_time: None,
            correlation_id: None,
            initiator: None,
            result: None,
            error: None,
        })
    }
    
    /// Set a correlation ID
    pub fn with_correlation_id(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }
    
    /// Set an initiator
    pub fn with_initiator(mut self, initiator: &str) -> Self {
        self.initiator = Some(initiator.to_string());
        self
    }
    
    /// Mark the saga as running
    pub fn mark_running(&mut self) {
        self.status = SagaStatus::Running;
        self.start_time = Some(chrono::Utc::now());
    }
    
    /// Mark the saga as completed
    pub fn mark_completed(&mut self, result: Option<serde_json::Value>) {
        self.status = SagaStatus::Completed;
        self.result = result;
        self.end_time = Some(chrono::Utc::now());
    }
    
    /// Mark the saga as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.status = SagaStatus::Failed;
        self.error = Some(error.to_string());
        self.end_time = Some(chrono::Utc::now());
    }
    
    /// Mark the saga as compensating
    pub fn mark_compensating(&mut self) {
        self.status = SagaStatus::Compensating;
    }
    
    /// Mark the saga as compensated
    pub fn mark_compensated(&mut self) {
        self.status = SagaStatus::Compensated;
        self.end_time = Some(chrono::Utc::now());
    }
    
    /// Mark the saga as failed with compensation errors
    pub fn mark_failed_with_errors(&mut self, error: &str) {
        self.status = SagaStatus::FailedWithErrors;
        self.error = Some(error.to_string());
        self.end_time = Some(chrono::Utc::now());
    }
    
    /// Mark the saga as aborted
    pub fn mark_aborted(&mut self, reason: &str) {
        self.status = SagaStatus::Aborted;
        self.error = Some(reason.to_string());
        self.end_time = Some(chrono::Utc::now());
    }
    
    /// Get all steps that are ready to execute
    pub fn get_ready_steps(&self) -> Vec<String> {
        let mut ready_steps = Vec::new();
        
        for step_id in &self.execution_order {
            let step = &self.steps[step_id];
            
            // Skip if not pending
            if step.status != StepStatus::Pending {
                continue;
            }
            
            // Check if all dependencies are completed
            let mut dependencies_met = true;
            for dep_id in &step.definition.dependencies {
                if let Some(dep_step) = self.steps.get(dep_id) {
                    if dep_step.status != StepStatus::Completed && dep_step.status != StepStatus::Skipped {
                        dependencies_met = false;
                        break;
                    }
                } else {
                    dependencies_met = false;
                    break;
                }
            }
            
            if dependencies_met {
                ready_steps.push(step_id.clone());
            }
        }
        
        ready_steps
    }
    
    /// Get steps that need compensation
    pub fn get_compensation_steps(&self) -> Vec<String> {
        // Compensation is in reverse order of execution
        let mut compensation_steps = Vec::new();
        
        for step_id in self.execution_order.iter().rev() {
            let step = &self.steps[step_id];
            
            // Only compensate completed steps with compensation actions
            if step.status == StepStatus::Completed && step.has_compensation() {
                compensation_steps.push(step_id.clone());
            }
        }
        
        compensation_steps
    }
    
    /// Check if the saga is complete (all steps in terminal state)
    pub fn is_complete(&self) -> bool {
        self.steps.values().all(|step| step.is_terminal())
    }
    
    /// Get overall saga progress (0-100%)
    pub fn get_progress(&self) -> f64 {
        let total_steps = self.steps.len() as f64;
        if total_steps == 0.0 {
            return 100.0;
        }
        
        let completed_steps = self.steps.values()
            .filter(|step| step.is_terminal())
            .count() as f64;
        
        (completed_steps / total_steps) * 100.0
    }
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
            cleanup_after_ms: 3600000,  // 1 hour
            default_retries: 3,
            default_retry_delay_ms: 1000, // 1 second
            channel_buffer_size: 1000,
        }
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
pub type StepHandler = Arc<dyn Fn(&SagaStep) -> 
    Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + Unpin> + Send + Sync>;

/// Handler function type for saga step compensations
pub type CompensationHandler = Arc<dyn Fn(&SagaStep) -> 
    Box<dyn std::future::Future<Output = Result<(), String>> + Send + Unpin> + Send + Sync>;

/// Saga orchestrator
pub struct SagaOrchestrator {
    /// Orchestrator configuration
    config: RwLock<SagaOrchestratorConfig>,
    
    /// Active sagas
    sagas: RwLock<HashMap<String, Arc<RwLock<Saga>>>>,
    
    /// Step handlers by service and action
    step_handlers: RwLock<HashMap<String, HashMap<String, StepHandler>>>,
    
    /// Compensation handlers by service and action
    compensation_handlers: RwLock<HashMap<String, HashMap<String, CompensationHandler>>>,
    
    /// Event broker for choreography
    event_broker: Option<Arc<EventBroker>>,
    
    /// Running flag
    is_running: RwLock<bool>,
    
    /// Cancellation channel
    cancel_tx: mpsc::Sender<()>,
    
    /// Cancellation receiver
    cancel_rx: Mutex<mpsc::Receiver<()>>,
}

impl SagaOrchestrator {
    /// Create a new saga orchestrator
    pub fn new(config: SagaOrchestratorConfig) -> Self {
        let (tx, rx) = mpsc::channel(1);
        
        SagaOrchestrator {
            config: RwLock::new(config),
            sagas: RwLock::new(HashMap::new()),
            step_handlers: RwLock::new(HashMap::new()),
            compensation_handlers: RwLock::new(HashMap::new()),
            event_broker: None,
            is_running: RwLock::new(false),
            cancel_tx: tx,
            cancel_rx: Mutex::new(rx),
        }
    }
    
    /// Set an event broker for choreography
    pub fn with_event_broker(mut self, broker: Arc<EventBroker>) -> Self {
        self.event_broker = Some(broker);
        self
    }
    
    /// Register a step handler
    pub async fn register_step_handler(
        &self,
        service: &str,
        action: &str,
        handler: StepHandler,
    ) {
        let mut handlers = self.step_handlers.write().await;
        
        handlers.entry(service.to_string())
            .or_insert_with(HashMap::new)
            .insert(action.to_string(), handler);
    }
    
    /// Register a compensation handler
    pub async fn register_compensation_handler(
        &self,
        service: &str,
        action: &str,
        handler: CompensationHandler,
    ) {
        let mut handlers = self.compensation_handlers.write().await;
        
        handlers.entry(service.to_string())
            .or_insert_with(HashMap::new)
            .insert(action.to_string(), handler);
    }
    
    /// Start the orchestrator
    pub async fn start(&self) -> Result<(), SagaError> {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Start the timeout monitor
        self.start_timeout_monitor().await?;
        
        // Start the cleanup task if auto cleanup is enabled
        let config = self.config.read().await;
        if config.auto_cleanup {
            self.start_cleanup_task().await?;
        }
        
        Ok(())
    }
    
    /// Start the timeout monitor
    async fn start_timeout_monitor(&self) -> Result<(), SagaError> {
        let orch = self.clone();
        let mut cancel_rx = self.cancel_rx.lock().await.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(
                orch.config.read().await.check_interval_ms,
            ));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        orch.check_timeouts().await;
                    }
                    _ = cancel_rx.recv() => {
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Start the cleanup task
    async fn start_cleanup_task(&self) -> Result<(), SagaError> {
        let orch = self.clone();
        let mut cancel_rx = self.cancel_rx.lock().await.clone();
        
        tokio::spawn(async move {
            let interval_ms = orch.config.read().await.check_interval_ms * 10; // Less frequent
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        orch.cleanup_completed_sagas().await;
                    }
                    _ = cancel_rx.recv() => {
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Create a new saga instance
    pub async fn create_saga(&self, definition: SagaDefinition) -> Result<String, SagaError> {
        // Create the saga
        let saga = Saga::new(definition)?;
        let instance_id = saga.instance_id.clone();
        
        // Store it
        let saga = Arc::new(RwLock::new(saga));
        
        let mut sagas = self.sagas.write().await;
        sagas.insert(instance_id.clone(), saga);
        
        Ok(instance_id)
    }
    
    /// Start a saga execution
    pub async fn start_saga(&self, saga_id: &str) -> Result<(), SagaError> {
        let saga_lock = {
            let sagas = self.sagas.read().await;
            sagas.get(saga_id)
                .cloned()
                .ok_or_else(|| SagaError::NotFound(saga_id.to_string()))?
        };
        
        // Mark saga as running
        {
            let mut saga = saga_lock.write().await;
            saga.mark_running();
        }
        
        // Start executing ready steps
        self.execute_ready_steps(saga_id).await?;
        
        Ok(())
    }
    
    /// Execute ready steps
    async fn execute_ready_steps(&self, saga_id: &str) -> Result<(), SagaError> {
        let saga_lock = {
            let sagas = self.sagas.read().await;
            sagas.get(saga_id)
                .cloned()
                .ok_or_else(|| SagaError::NotFound(saga_id.to_string()))?
        };
        
        // Get ready steps
        let ready_steps = {
            let saga = saga_lock.read().await;
            saga.get_ready_steps()
        };
        
        // Execute each ready step
        for step_id in ready_steps {
            self.execute_step(saga_id, &step_id).await?;
        }
        
        Ok(())
    }
    
    /// Execute a specific step
    async fn execute_step(&self, saga_id: &str, step_id: &str) -> Result<StepResult, SagaError> {
        let saga_lock = {
            let sagas = self.sagas.read().await;
            sagas.get(saga_id)
                .cloned()
                .ok_or_else(|| SagaError::NotFound(saga_id.to_string()))?
        };
        
        // Get the step
        let step = {
            let saga = saga_lock.read().await;
            saga.steps.get(step_id)
                .cloned()
                .ok_or_else(|| SagaError::StepNotFound(step_id.to_string()))?
        };
        
        // Mark step as running
        {
            let mut saga = saga_lock.write().await;
            let step = saga.steps.get_mut(step_id)
                .ok_or_else(|| SagaError::StepNotFound(step_id.to_string()))?;
            
            step.mark_running();
        }
        
        // Find handler for this step
        let handler = {
            let handlers = self.step_handlers.read().await;
            
            if let Some(service_handlers) = handlers.get(&step.definition.service) {
                if let Some(action_handler) = service_handlers.get(&step.definition.action) {
                    action_handler.clone()
                } else {
                    return Err(SagaError::Other(format!(
                        "No handler for action: {}", step.definition.action
                    )));
                }
            } else {
                return Err(SagaError::Other(format!(
                    "No handlers for service: {}", step.definition.service
                )));
            }
        };
        
        // Execute the step with timeout
        let timeout_duration = Duration::from_millis(step.definition.timeout_ms);
        let step_execution = (handler)(&step);
        
        let execution_result = match timeout(timeout_duration, step_execution).await {
            Ok(result) => result,
            Err(_) => Err(format!("Step timed out after {}ms", step.definition.timeout_ms)),
        };
        
        // Process result
        let result = match execution_result {
            Ok(data) => {
                // Step succeeded
                let mut saga = saga_lock.write().await;
                let step = saga.steps.get_mut(step_id)
                    .ok_or_else(|| SagaError::StepNotFound(step_id.to_string()))?;
                
                step.mark_completed(data.clone());
                
                // Check if saga is complete
                if saga.is_complete() {
                    saga.mark_completed(Some(serde_json::json!({
                        "steps": saga.steps,
                    })));
                }
                
                StepResult {
                    step_id: step_id.to_string(),
                    status: StepStatus::Completed,
                    data: Some(data),
                    error: None,
                    saga_id: saga_id.to_string(),
                }
            },
            Err(error) => {
                // Step failed
                let mut saga = saga_lock.write().await;
                let step = saga.steps.get_mut(step_id)
                    .ok_or_else(|| SagaError::StepNotFound(step_id.to_string()))?;
                
                step.mark_failed(&error);
                
                // Check if this failure triggers compensation
                if step.definition.triggers_compensation {
                    saga.mark_compensating();
                    
                    // Launch compensation asynchronously
                    let self_clone = self.clone();
                    let saga_id_clone = saga_id.to_string();
                    
                    tokio::spawn(async move {
                        if let Err(e) = self_clone.compensate_saga(&saga_id_clone).await {
                            log::error!("Failed to compensate saga {}: {:?}", saga_id_clone, e);
                        }
                    });
                } else if step.definition.continue_on_failure {
                    // Continue despite failure
                    // This might enable next steps that don't depend on this one
                } else {
                    // Non-compensating, non-continuing failure = aborted saga
                    saga.mark_failed(&error);
                }
                
                StepResult {
                    step_id: step_id.to_string(),
                    status: StepStatus::Failed,
                    data: None,
                    error: Some(error),
                    saga_id: saga_id.to_string(),
                }
            }
        };
        
        // Check and execute any newly ready steps
        tokio::spawn({
            let self_clone = self.clone();
            let saga_id = saga_id.to_string();
            
            async move {
                if let Err(e) = self_clone.execute_ready_steps(&saga_id).await {
                    log::error!("Failed to execute ready steps for saga {}: {:?}", saga_id, e);
                }
            }
        });
        
        Ok(result)
    }
    
    /// Compensate a saga (undo completed steps)
    async fn compensate_saga(&self, saga_id: &str) -> Result<(), SagaError> {
        let saga_lock = {
            let sagas = self.sagas.read().await;
            sagas.get(saga_id)
                .cloned()
                .ok_or_else(|| SagaError::NotFound(saga_id.to_string()))?
        };
        
        // Get compensation steps
        let compensation_steps = {
            let saga = saga_lock.read().await;
            
            if saga.status != SagaStatus::Compensating && saga.status != SagaStatus::Failed {
                return Err(SagaError::Other(format!(
                    "Cannot compensate saga in state: {:?}", saga.status
                )));
            }
            
            saga.get_compensation_steps()
        };
        
        // Execute compensation for each step in reverse order
        let mut compensation_errors = Vec::new();
        
        for step_id in compensation_steps {
            if let Err(e) = self.compensate_step(saga_id, &step_id).await {
                compensation_errors.push(format!("Step {}: {}", step_id, e));
            }
        }
        
        // Update saga status
        {
            let mut saga = saga_lock.write().await;
            
            if compensation_errors.is_empty() {
                saga.mark_compensated();
            } else {
                saga.mark_failed_with_errors(&compensation_errors.join("; "));
            }
        }
        
        Ok(())
    }
    
    /// Compensate a specific step
    async fn compensate_step(&self, saga_id: &str, step_id: &str) -> Result<(), SagaError> {
        let saga_lock = {
            let sagas = self.sagas.read().await;
            sagas.get(saga_id)
                .cloned()
                .ok_or_else(|| SagaError::NotFound(saga_id.to_string()))?
        };
        
        // Get the step
        let step = {
            let saga = saga_lock.read().await;
            saga.steps.get(step_id)
                .cloned()
                .ok_or_else(|| SagaError::StepNotFound(step_id.to_string()))?
        };
        
        // Check if step needs compensation
        if step.status != StepStatus::Completed || !step.has_compensation() {
            return Ok(());
        }
        
        // Mark step as compensating
        {
            let mut saga = saga_lock.write().await;
            let step = saga.steps.get_mut(step_id)
                .ok_or_else(|| SagaError::StepNotFound(step_id.to_string()))?;
            
            step.mark_compensating();
        }
        
        // Find compensation handler
        let compensation_action = step.definition.compensation.as_ref()
            .ok_or_else(|| SagaError::Other(format!("No compensation for step: {}", step_id)))?;
        
        let handler = {
            let handlers = self.compensation_handlers.read().await;
            
            if let Some(service_handlers) = handlers.get(&step.definition.service) {
                if let Some(action_handler) = service_handlers.get(compensation_action) {
                    action_handler.clone()
                } else {
                    return Err(SagaError::Other(format!(
                        "No compensation handler for action: {}", compensation_action
                    )));
                }
            } else {
                return Err(SagaError::Other(format!(
                    "No compensation handlers for service: {}", step.definition.service
                )));
            }
        };
        
        // Execute compensation with timeout
        let timeout_duration = Duration::from_millis(step.definition.timeout_ms);
        let compensation_execution = (handler)(&step);
        
        let execution_result = match timeout(timeout_duration, compensation_execution).await {
            Ok(result) => result,
            Err(_) => Err(format!("Compensation timed out after {}ms", step.definition.timeout_ms)),
        };
        
        // Process result
        match execution_result {
            Ok(_) => {
                // Compensation succeeded
                let mut saga = saga_lock.write().await;
                let step = saga.steps.get_mut(step_id)
                    .ok_or_else(|| SagaError::StepNotFound(step_id.to_string()))?;
                
                step.mark_compensated();
                Ok(())
            },
            Err(error) => {
                // Compensation failed
                let mut saga = saga_lock.write().await;
                let step = saga.steps.get_mut(step_id)
                    .ok_or_else(|| SagaError::StepNotFound(step_id.to_string()))?;
                
                step.mark_compensation_failed(&error);
                Err(SagaError::CompensationFailed(error))
            }
        }
    }
    
    /// Abort a running saga
    pub async fn abort_saga(&self, saga_id: &str, reason: &str) -> Result<(), SagaError> {
        let saga_lock = {
            let sagas = self.sagas.read().await;
            sagas.get(saga_id)
                .cloned()
                .ok_or_else(|| SagaError::NotFound(saga_id.to_string()))?
        };
        
        // Update saga status
        {
            let mut saga = saga_lock.write().await;
            
            if saga.status == SagaStatus::Running || saga.status == SagaStatus::Created {
                saga.mark_aborted(reason);
                
                // Launch compensation
                let self_clone = self.clone();
                let saga_id_clone = saga_id.to_string();
                
                tokio::spawn(async move {
                    if let Err(e) = self_clone.compensate_saga(&saga_id_clone).await {
                        log::error!("Failed to compensate aborted saga {}: {:?}", saga_id_clone, e);
                    }
                });
                
                Ok(())
            } else {
                Err(SagaError::Other(format!(
                    "Cannot abort saga in state: {:?}", saga.status
                )))
            }
        }
    }
    
    /// Get a saga instance
    pub async fn get_saga(&self, saga_id: &str) -> Option<Arc<RwLock<Saga>>> {
        let sagas = self.sagas.read().await;
        sagas.get(saga_id).cloned()
    }
    
    /// Get all sagas
    pub async fn get_all_sagas(&self) -> Vec<Arc<RwLock<Saga>>> {
        let sagas = self.sagas.read().await;
        sagas.values().cloned().collect()
    }
    
    /// Get sagas by status
    pub async fn get_sagas_by_status(&self, status: SagaStatus) -> Vec<Arc<RwLock<Saga>>> {
        let sagas = self.sagas.read().await;
        let mut result = Vec::new();
        
        for saga_lock in sagas.values() {
            let saga = saga_lock.read().await;
            if saga.status == status {
                result.push(saga_lock.clone());
            }
        }
        
        result
    }
    
    /// Check for saga timeouts
    async fn check_timeouts(&self) {
        let sagas = self.sagas.read().await;
        let now = chrono::Utc::now();
        
        for (saga_id, saga_lock) in sagas.iter() {
            let should_timeout = {
                let saga = saga_lock.read().await;
                
                if saga.status != SagaStatus::Running {
                    continue;
                }
                
                if let Some(start_time) = saga.start_time {
                    let elapsed = now.signed_duration_since(start_time);
                    elapsed.num_milliseconds() > saga.definition.timeout_ms as i64
                } else {
                    false
                }
            };
            
            if should_timeout {
                // Abort the saga
                let self_clone = self.clone();
                let saga_id = saga_id.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = self_clone.abort_saga(&saga_id, "Saga timeout").await {
                        log::error!("Failed to abort timed out saga {}: {:?}", saga_id, e);
                    }
                });
            }
        }
    }
    
    /// Cleanup completed sagas
    async fn cleanup_completed_sagas(&self) {
        let config = self.config.read().await;
        let now = chrono::Utc::now();
        let mut sagas_to_remove = Vec::new();
        
        // Find sagas to cleanup
        {
            let sagas = self.sagas.read().await;
            
            for (saga_id, saga_lock) in sagas.iter() {
                let should_cleanup = {
                    let saga = saga_lock.read().await;
                    
                    if let Some(end_time) = saga.end_time {
                        if matches!(
                            saga.status,
                            SagaStatus::Completed | SagaStatus::Compensated | SagaStatus::FailedWithErrors | SagaStatus::Aborted
                        ) {
                            let elapsed = now.signed_duration_since(end_time);
                            elapsed.num_milliseconds() > config.cleanup_after_ms as i64
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };
                
                if should_cleanup {
                    sagas_to_remove.push(saga_id.clone());
                }
            }
        }
        
        // Remove sagas
        if !sagas_to_remove.is_empty() {
            let mut sagas = self.sagas.write().await;
            
            for saga_id in sagas_to_remove {
                sagas.remove(&saga_id);
            }
        }
    }
    
    /// Stop the orchestrator
    pub async fn stop(&self) -> Result<(), SagaError> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        // Send cancellation signal to background tasks
        let _ = self.cancel_tx.send(()).await;
        
        Ok(())
    }
}

impl Clone for SagaOrchestrator {
    fn clone(&self) -> Self {
        let (tx, rx) = mpsc::channel(1);
        
        SagaOrchestrator {
            config: self.config.clone(),
            sagas: self.sagas.clone(),
            step_handlers: self.step_handlers.clone(),
            compensation_handlers: self.compensation_handlers.clone(),
            event_broker: self.event_broker.clone(),
            is_running: self.is_running.clone(),
            cancel_tx: tx,
            cancel_rx: Mutex::new(rx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_saga_definition() {
        // Create a simple saga definition
        let mut saga_def = SagaDefinition::new("test-saga", "Test Saga");
        
        // Add steps
        let step1 = SagaStepDefinition::new(
            "step1",
            "Reserve Items",
            "inventory",
            "reserve",
            serde_json::json!({"items": ["item1", "item2"]}),
        ).with_compensation(
            "cancel_reservation",
            serde_json::json!({"reservation_id": "123"}),
        );
        
        let step2 = SagaStepDefinition::new(
            "step2",
            "Process Payment",
            "payment",
            "process",
            serde_json::json!({"amount": 100.0}),
        ).with_compensation(
            "refund",
            serde_json::json!({"payment_id": "456"}),
        ).with_dependency("step1");
        
        saga_def.add_step(step1).unwrap();
        saga_def.add_step(step2).unwrap();
        
        // Validate definition
        saga_def.validate().unwrap();
        
        // Check execution order
        let order = saga_def.get_execution_order().unwrap();
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], "step1");
        assert_eq!(order[1], "step2");
    }
    
    #[tokio::test]
    async fn test_saga_instance() {
        // Create a saga definition
        let mut saga_def = SagaDefinition::new("test-saga", "Test Saga");
        
        // Add steps
        let step1 = SagaStepDefinition::new(
            "step1",
            "Reserve Items",
            "inventory",
            "reserve",
            serde_json::json!({"items": ["item1", "item2"]}),
        ).with_compensation(
            "cancel_reservation",
            serde_json::json!({"reservation_id": "123"}),
        );
        
        let step2 = SagaStepDefinition::new(
            "step2",
            "Process Payment",
            "payment",
            "process",
            serde_json::json!({"amount": 100.0}),
        ).with_compensation(
            "refund",
            serde_json::json!({"payment_id": "456"}),
        ).with_dependency("step1");
        
        saga_def.add_step(step1).unwrap();
        saga_def.add_step(step2).unwrap();
        
        // Create a saga instance
        let mut saga = Saga::new(saga_def).unwrap();
        
        // Check initial state
        assert_eq!(saga.status, SagaStatus::Created);
        assert_eq!(saga.steps.len(), 2);
        
        // Check ready steps (should be only step1)
        let ready_steps = saga.get_ready_steps();
        assert_eq!(ready_steps.len(), 1);
        assert_eq!(ready_steps[0], "step1");
        
        // Mark step1 as running and completed
        let step1 = saga.steps.get_mut("step1").unwrap();
        step1.mark_running();
        step1.mark_completed(serde_json::json!({"reservation_id": "123"}));
        
        // Now step2 should be ready
        let ready_steps = saga.get_ready_steps();
        assert_eq!(ready_steps.len(), 1);
        assert_eq!(ready_steps[0], "step2");
        
        // Mark step2 as failed
        let step2 = saga.steps.get_mut("step2").unwrap();
        step2.mark_running();
        step2.mark_failed("Payment declined");
        
        // Mark saga as compensating
        saga.mark_compensating();
        
        // Get compensation steps
        let comp_steps = saga.get_compensation_steps();
        assert_eq!(comp_steps.len(), 1);
        assert_eq!(comp_steps[0], "step1"); // Only step1 was completed and has compensation
    }
    
    #[tokio::test]
    async fn test_saga_orchestrator() {
        // Create an orchestrator
        let orch = SagaOrchestrator::new(SagaOrchestratorConfig::default());
        
        // Start the orchestrator
        orch.start().await.unwrap();
        
        // Register handlers
        orch.register_step_handler("inventory", "reserve", Arc::new(|step| {
            Box::new(async move {
                // Simple mock handler that always succeeds
                Ok(serde_json::json!({"reservation_id": "123"}))
            })
        })).await;
        
        orch.register_step_handler("payment", "process", Arc::new(|step| {
            Box::new(async move {
                // Simple mock handler that always fails
                Err("Payment declined".to_string())
            })
        })).await;
        
        orch.register_compensation_handler("inventory", "cancel_reservation", Arc::new(|step| {
            Box::new(async move {
                // Simple mock compensation that always succeeds
                Ok(())
            })
        })).await;
        
        // Create a saga definition
        let mut saga_def = SagaDefinition::new("test-saga", "Test Saga");
        
        // Add steps
        let step1 = SagaStepDefinition::new(
            "step1",
            "Reserve Items",
            "inventory",
            "reserve",
            serde_json::json!({"items": ["item1", "item2"]}),
        ).with_compensation(
            "cancel_reservation",
            serde_json::json!({"reservation_id": "123"}),
        );
        
        let step2 = SagaStepDefinition::new(
            "step2",
            "Process Payment",
            "payment",
            "process",
            serde_json::json!({"amount": 100.0}),
        ).with_compensation(
            "refund",
            serde_json::json!({"payment_id": "456"}),
        ).with_dependency("step1");
        
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
                
                if saga.status == SagaStatus::Compensated || saga.status == SagaStatus::FailedWithErrors {
                    saga_completed = true;
                    break;
                }
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // Stop the orchestrator
        orch.stop().await.unwrap();
        
        // Check saga status
        assert!(saga_completed, "Saga did not complete in time");
        
        let saga_lock = orch.get_saga(&saga_id).await.unwrap();
        let saga = saga_lock.read().await;
        
        assert_eq!(saga.status, SagaStatus::Compensated);
        assert_eq!(saga.steps["step1"].status, StepStatus::Compensated);
        assert_eq!(saga.steps["step2"].status, StepStatus::Failed);
    }
}