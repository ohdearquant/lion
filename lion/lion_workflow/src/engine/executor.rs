use crate::engine::context::{CapabilityChecker, ContextError, ExecutionContext, NodeResult};
use crate::engine::scheduler::{SchedulerError, Task, TaskId, TaskStatus, WorkflowScheduler};
use crate::model::{NodeId, WorkflowDefinition};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;

/// Error types for workflow executor
#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Node execution error: {0}")]
    NodeError(String),

    #[error("Scheduling error: {0}")]
    SchedulingError(#[from] SchedulerError),

    #[error("Context error: {0}")]
    ContextError(#[from] ContextError),

    #[error("State machine error: {0}")]
    StateMachineError(#[from] crate::state::StateMachineError),

    #[error("Task timeout: {0}")]
    TaskTimeout(TaskId),

    #[error("Task cancelled: {0}")]
    TaskCancelled(TaskId),

    #[error("Task preempted: {0}")]
    TaskPreempted(TaskId),

    #[error("Workflow error: {0}")]
    WorkflowError(#[from] crate::model::WorkflowError),

    #[error("Executor stopped")]
    ExecutorStopped,

    #[error("No node handler for type: {0}")]
    NoNodeHandler(String),

    #[error("Other executor error: {0}")]
    Other(String),
}

/// Result of task execution
#[derive(Debug)]
pub struct TaskExecutionResult {
    /// Task ID
    pub task_id: TaskId,

    /// Node ID
    pub node_id: NodeId,

    /// Execution status
    pub status: TaskStatus,

    /// Execution result
    pub result: Option<NodeResult>,

    /// Error (if any)
    pub error: Option<String>,

    /// Execution duration
    pub duration: Duration,

    /// CPU time used
    pub cpu_time: Option<Duration>,

    /// Memory used (in bytes)
    pub memory_usage: Option<usize>,
}

/// Type for node execution handlers
pub type NodeHandler = Arc<
    dyn Fn(
            ExecutionContext,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<NodeResult, ExecutorError>> + Send>,
        > + Send
        + Sync,
>;

/// Configuration for workflow executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum execution time for a task
    pub max_execution_time: Duration,

    /// Default timeout for task execution
    pub default_timeout: Duration,

    /// Maximum task retries
    pub max_retries: u32,

    /// Whether to use cooperative preemption
    pub use_cooperative_preemption: bool,

    /// Cooperative preemption quantum (yield after this duration)
    pub preemption_quantum: Duration,

    /// Whether to use work stealing
    pub use_work_stealing: bool,

    /// Whether to prioritize deadline-critical tasks
    pub prioritize_deadlines: bool,

    /// Number of worker threads
    pub worker_threads: usize,

    /// Timeout for yielding a task (seconds)
    pub yield_timeout_seconds: u64,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        ExecutorConfig {
            max_execution_time: Duration::from_secs(60),
            default_timeout: Duration::from_secs(30),
            max_retries: 3,
            use_cooperative_preemption: true,
            preemption_quantum: Duration::from_millis(100),
            use_work_stealing: true,
            prioritize_deadlines: true,
            worker_threads: num_cpus::get(),
            yield_timeout_seconds: 1,
        }
    }
}

/// Execution worker state
struct Worker {
    /// Worker ID
    id: usize,

    /// Task currently being executed
    current_task: Option<TaskId>,

    /// Whether the worker is busy
    is_busy: bool,

    /// Last task completion time
    last_completion: Option<chrono::DateTime<chrono::Utc>>,

    /// Statistics for this worker
    stats: WorkerStats,
}

/// Worker statistics
#[derive(Debug, Default, Clone)]
struct WorkerStats {
    /// Number of tasks completed
    tasks_completed: usize,

    /// Number of tasks failed
    tasks_failed: usize,

    /// Total execution time (seconds)
    total_execution_time: f64,

    /// Total wait time (seconds)
    total_wait_time: f64,
}

/// Workflow executor
pub struct WorkflowExecutor<S>
where
    S: crate::state::storage::StorageBackend,
{
    /// Scheduler for tasks
    scheduler: Arc<WorkflowScheduler>,

    /// State machine manager
    state_manager: Arc<crate::state::StateMachineManager<S>>,

    /// Node handlers by node type
    node_handlers: Arc<RwLock<HashMap<String, NodeHandler>>>,

    /// Capability checker for capability-based security
    capability_checker: Option<Arc<dyn CapabilityChecker + 'static>>,

    /// Worker states
    workers: Arc<RwLock<Vec<Worker>>>,

    /// Execution configuration
    config: RwLock<ExecutorConfig>,

    /// Whether the executor is running
    is_running: RwLock<bool>,

    /// Cancellation channel
    cancel_tx: mpsc::Sender<()>,

    /// Cancellation receiver
    cancel_rx: Mutex<mpsc::Receiver<()>>,
}

impl<S> WorkflowExecutor<S>
where
    S: crate::state::storage::StorageBackend,
{
    /// Create a new workflow executor
    pub fn new(
        scheduler: Arc<WorkflowScheduler>,
        state_manager: Arc<crate::state::StateMachineManager<S>>,
        config: ExecutorConfig,
    ) -> Self {
        let (tx, rx) = mpsc::channel(1);

        // Initialize workers
        let mut workers = Vec::with_capacity(config.worker_threads);
        for i in 0..config.worker_threads {
            workers.push(Worker {
                id: i,
                current_task: None,
                is_busy: false,
                last_completion: None,
                stats: WorkerStats::default(),
            });
        }

        WorkflowExecutor {
            scheduler,
            state_manager,
            node_handlers: Arc::new(RwLock::new(HashMap::new())),
            capability_checker: None,
            workers: Arc::new(RwLock::new(workers)),
            config: RwLock::new(config),
            is_running: RwLock::new(true),
            cancel_tx: tx,
            cancel_rx: Mutex::new(rx),
        }
    }

    /// Set the capability checker
    pub fn with_capability_checker(
        mut self,
        checker: Arc<dyn CapabilityChecker + 'static>,
    ) -> Self {
        self.capability_checker = Some(checker);
        self
    }

    /// Register a node handler for a specific node type
    pub async fn register_node_handler(&self, node_type: &str, handler: NodeHandler) {
        let mut handlers = self.node_handlers.write().await;
        handlers.insert(node_type.to_string(), handler);
    }

    /// Start the executor
    pub async fn start(&self) -> Result<(), ExecutorError> {
        // Set the executor as running
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Start worker threads
        let config = self.config.read().await;
        let worker_count = config.worker_threads;

        for worker_id in 0..worker_count {
            self.start_worker(worker_id).await?;
        }

        // Start task monitor for timeouts
        self.start_task_monitor().await?;

        Ok(())
    }

    /// Start a worker thread
    async fn start_worker(&self, worker_id: usize) -> Result<(), ExecutorError> {
        // Clone necessary references for the worker
        let scheduler_clone = self.scheduler.clone();
        let state_manager_clone = self.state_manager.clone();
        let node_handlers_clone = self.node_handlers.clone();
        let capability_checker_clone = self.capability_checker.clone();
        let workers_clone = self.workers.clone();

        // Get current values
        let is_running_val = *self.is_running.read().await;
        let config_val = self.config.read().await.clone();

        // Create a new channel for this worker
        let (_tx, mut cancel_rx) = mpsc::channel::<()>(1);

        // Spawn a worker task
        tokio::spawn(async move {
            let worker_id_copy = worker_id;
            let is_running_local = is_running_val;

            // Worker loop
            'worker_loop: loop {
                // Check if executor is still running
                if !is_running_local {
                    break;
                }

                // Update worker status
                {
                    let mut workers_guard = workers_clone.write().await;
                    workers_guard[worker_id].is_busy = false;
                    workers_guard[worker_id].current_task = None;
                }

                // Check cancellation
                if cancel_rx.try_recv().is_ok() {
                    break;
                }

                // Get next task from scheduler
                let next_task = scheduler_clone.next_task().await;

                // If no task, wait a bit and try again
                if next_task.is_none() {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                        _ = cancel_rx.recv() => {
                            break 'worker_loop;
                        }
                    }
                    continue;
                }

                let task = next_task.unwrap();
                let task_id = task.id;
                let node_id = task.node_id.clone();
                let instance_id = task.instance_id.clone();

                // Update worker status
                {
                    let mut workers_guard = workers_clone.write().await;
                    workers_guard[worker_id].is_busy = true;
                    workers_guard[worker_id].current_task = Some(task_id);
                }

                // Mark task as running
                if let Err(e) = scheduler_clone.mark_task_running(task_id).await {
                    log::error!("Failed to mark task as running: {:?}", e);
                    continue;
                }

                // Mark node as running in state machine
                if let Err(e) = state_manager_clone
                    .set_node_running(&instance_id, &node_id)
                    .await
                {
                    log::error!("Failed to mark node as running: {:?}", e);
                    continue;
                }

                // Get node type
                let node_type =
                    if let Some(state) = state_manager_clone.get_instance(&instance_id).await {
                        let state_guard = state.read().await;
                        if let Some(def) = &state_guard.definition {
                            if let Some(node) = def.get_node(&node_id) {
                                node.name.clone()
                            } else {
                                String::from("unknown")
                            }
                        } else {
                            String::from("unknown")
                        }
                    } else {
                        String::from("unknown")
                    };

                // Get node handler
                let handler = {
                    let handlers = node_handlers_clone.read().await;
                    handlers.get(&node_type).cloned()
                };

                // Execute task with timeout
                let start_time = std::time::Instant::now();

                let execution_result = if let Some(handler) = handler {
                    // Create execution context
                    let mut context = task.context.clone();
                    if let Some(checker) = &capability_checker_clone {
                        context = context.with_capability_checker(checker.clone());
                    }

                    // Execute with timeout
                    let execution_future = (handler)(context);
                    match timeout(config_val.default_timeout, execution_future).await {
                        Ok(result) => result,
                        Err(_) => Err(ExecutorError::TaskTimeout(task_id)),
                    }
                } else {
                    Err(ExecutorError::NoNodeHandler(node_type))
                };

                let execution_time = start_time.elapsed();

                // Update worker stats
                {
                    let mut workers_guard = workers_clone.write().await;
                    let worker = &mut workers_guard[worker_id];
                    worker.last_completion = Some(chrono::Utc::now());
                    worker.stats.total_execution_time += execution_time.as_secs_f64();

                    match &execution_result {
                        Ok(_) => {
                            worker.stats.tasks_completed += 1;
                        }
                        Err(_) => {
                            worker.stats.tasks_failed += 1;
                        }
                    }
                }

                // Handle execution result
                match execution_result {
                    Ok(node_result) => {
                        // Mark task as completed
                        if let Err(e) = scheduler_clone.mark_task_completed(task_id).await {
                            log::error!("Failed to mark task as completed: {:?}", e);
                        }

                        // Update state machine
                        if let Err(e) = state_manager_clone
                            .set_node_completed(&instance_id, &node_id, node_result.output.clone())
                            .await
                        {
                            log::error!("Failed to mark node as completed: {:?}", e);
                        }
                    }
                    Err(e) => {
                        // Mark task as failed
                        if let Err(mark_err) = scheduler_clone.mark_task_failed(task_id).await {
                            log::error!("Failed to mark task as failed: {:?}", mark_err);
                        }

                        // Update state machine
                        let error_json = match &e {
                            ExecutorError::NodeError(msg) => {
                                serde_json::json!({ "error": msg })
                            }
                            ExecutorError::TaskTimeout(_) => {
                                serde_json::json!({ "error": "Task timed out" })
                            }
                            _ => {
                                serde_json::json!({ "error": format!("{:?}", e) })
                            }
                        };

                        if let Err(state_err) = state_manager_clone
                            .set_node_failed(&instance_id, &node_id, error_json)
                            .await
                        {
                            log::error!("Failed to mark node as failed: {:?}", state_err);
                        }

                        log::error!("Task execution failed: {:?}", e);
                    }
                }
            }

            // Update worker status on exit
            {
                let mut workers_guard = workers_clone.write().await;
                workers_guard[worker_id_copy].is_busy = false;
                workers_guard[worker_id_copy].current_task = None;
            }

            log::info!("Worker {} exited", worker_id_copy);
        });

        Ok(())
    }

    /// Start the task monitor for timeouts and scheduling corrections
    async fn start_task_monitor(&self) -> Result<(), ExecutorError> {
        // Clone necessary references
        let scheduler_clone = self.scheduler.clone();

        // Get current values
        let is_running_val = *self.is_running.read().await;

        // Create a new channel for this monitor
        let (_tx, mut cancel_rx) = mpsc::channel::<()>(1);

        // Spawn monitor task
        tokio::spawn(async move {
            let is_running_local = is_running_val;
            let check_interval = Duration::from_secs(1);

            // Monitor loop
            'monitor_loop: loop {
                // Check if executor is still running
                if !is_running_local {
                    break;
                }

                // Check cancellation
                if cancel_rx.try_recv().is_ok() {
                    break;
                }

                // Check for timed out tasks
                let timed_out_tasks = scheduler_clone.check_timeouts().await;

                for task_id in timed_out_tasks {
                    // Cancel timed out tasks
                    if let Err(e) = scheduler_clone.cancel_task(task_id).await {
                        log::error!("Failed to cancel timed out task {}: {:?}", task_id, e);
                    } else {
                        log::warn!("Task {} timed out and was cancelled", task_id);
                    }
                }

                // Sleep before next check
                tokio::select! {
                    _ = tokio::time::sleep(check_interval) => {}
                    _ = cancel_rx.recv() => {
                        break 'monitor_loop;
                    }
                }
            }

            log::info!("Task monitor exited");
        });

        Ok(())
    }

    /// Schedule a node for execution
    pub async fn schedule_node(
        &self,
        workflow_instance_id: &str,
        node_id: NodeId,
    ) -> Result<TaskId, ExecutorError> {
        // Check if executor is running
        if !*self.is_running.read().await {
            return Err(ExecutorError::ExecutorStopped);
        }

        // Get the workflow instance
        let instance = self
            .state_manager
            .get_instance(workflow_instance_id)
            .await
            .ok_or_else(|| {
                ExecutorError::Other(format!(
                    "Workflow instance not found: {}",
                    workflow_instance_id
                ))
            })?;

        // Get the workflow definition
        let instance_guard = instance.read().await;
        let definition = instance_guard.definition.clone().ok_or_else(|| {
            ExecutorError::Other("Workflow instance has no definition".to_string())
        })?;

        // Create execution context
        let context =
            ExecutionContext::new(definition, Arc::new(instance_guard.clone())).with_node(&node_id);

        // Create task
        let task = Task::new(node_id, workflow_instance_id.to_string(), context);

        // Schedule task
        let task_id = self.scheduler.schedule_task(task).await?;

        Ok(task_id)
    }

    /// Schedule newly ready nodes for a workflow instance
    pub async fn schedule_ready_nodes(
        &self,
        workflow_instance_id: &str,
    ) -> Result<Vec<TaskId>, ExecutorError> {
        // Get ready nodes from state machine
        let ready_nodes = self
            .state_manager
            .get_ready_nodes(workflow_instance_id)
            .await?;

        // Schedule each ready node
        let mut task_ids = Vec::new();
        for node_id in ready_nodes {
            let task_id = self.schedule_node(workflow_instance_id, node_id).await?;
            task_ids.push(task_id);
        }

        Ok(task_ids)
    }

    /// Execute a workflow instance
    pub async fn execute_workflow(
        &self,
        definition: Arc<WorkflowDefinition>,
    ) -> Result<String, ExecutorError> {
        // Create a new workflow instance
        let instance = self.state_manager.create_instance(definition).await?;

        // Get the instance ID
        let instance_id = {
            let state = instance.read().await;
            state.instance_id.clone()
        };

        // Schedule all ready nodes
        self.schedule_ready_nodes(&instance_id).await?;

        Ok(instance_id)
    }

    /// Stop the executor
    pub async fn stop(&self) -> Result<(), ExecutorError> {
        // Set the executor as not running
        let mut is_running = self.is_running.write().await;
        *is_running = false;

        // Send cancellation signal to all workers
        let _ = self.cancel_tx.send(()).await;

        // Stop the scheduler
        self.scheduler.stop().await;

        Ok(())
    }

    /// Update executor configuration
    pub async fn update_config(&self, config: ExecutorConfig) {
        let mut current_config = self.config.write().await;
        *current_config = config;
    }

    /// Get worker statistics
    pub async fn get_worker_stats(&self) -> Vec<(usize, WorkerStats)> {
        let workers = self.workers.read().await;
        workers.iter().map(|w| (w.id, w.stats.clone())).collect()
    }

    /// Get the number of busy workers
    pub async fn get_busy_worker_count(&self) -> usize {
        let workers = self.workers.read().await;
        workers.iter().filter(|w| w.is_busy).count()
    }

    /// Check if a task is running
    pub async fn is_task_running(&self, task_id: TaskId) -> bool {
        let workers = self.workers.read().await;
        workers.iter().any(|w| w.current_task == Some(task_id))
    }

    /// Cancel a running task
    pub async fn cancel_task(&self, task_id: TaskId) -> Result<(), ExecutorError> {
        // Cancel in the scheduler
        self.scheduler.cancel_task(task_id).await?;

        // For now, tasks aren't forcibly cancelled if they're already running
        // They'll continue until completion or timeout
        // A full implementation would track running futures and cancel them

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::scheduler::SchedulerConfig;
    use crate::model::{Edge, Node};
    use crate::state::storage::MemoryStorage;

    // Helper to create a test workflow
    fn create_test_workflow() -> Arc<WorkflowDefinition> {
        let mut workflow =
            WorkflowDefinition::new(crate::model::WorkflowId::new(), "Test Workflow".to_string());

        // Create nodes
        let node1 = Node::new(NodeId::new(), "start".to_string());
        let node2 = Node::new(NodeId::new(), "process".to_string());
        let node3 = Node::new(NodeId::new(), "end".to_string());

        // Clone the IDs before adding nodes to workflow
        let node1_id = node1.id.clone();
        let node2_id = node2.id.clone();
        let node3_id = node3.id.clone();

        // Add nodes to workflow
        workflow.add_node(node1).unwrap();
        workflow.add_node(node2).unwrap();
        workflow.add_node(node3).unwrap();

        // Add edges
        workflow
            .add_edge(Edge::new(
                crate::model::EdgeId::new(),
                node1_id.clone(),
                node2_id.clone(),
            ))
            .unwrap();
        workflow
            .add_edge(Edge::new(
                crate::model::EdgeId::new(),
                node2_id.clone(),
                node3_id.clone(),
            ))
            .unwrap();

        Arc::new(workflow)
    }

    #[tokio::test]
    async fn test_executor_basic_workflow() {
        // Create dependencies
        let scheduler = Arc::new(WorkflowScheduler::new(SchedulerConfig::default()));
        let state_manager = Arc::new(crate::state::StateMachineManager::<MemoryStorage>::new());

        // Create executor
        let executor = WorkflowExecutor::new(scheduler, state_manager, ExecutorConfig::default());

        // Register node handlers
        executor
            .register_node_handler(
                "start",
                Arc::new(|ctx| {
                    Box::pin(async move {
                        // Simple start node handler that returns success
                        Ok(NodeResult::success(
                            ctx.current_node_id.unwrap(),
                            serde_json::json!({"message": "Start completed"}),
                        ))
                    })
                }),
            )
            .await;

        executor
            .register_node_handler(
                "process",
                Arc::new(|ctx| {
                    Box::pin(async move {
                        // Process node that uses input from start node
                        let inputs = ctx.get_inputs()?;

                        // Create a result based on inputs
                        Ok(NodeResult::success(
                            ctx.current_node_id.unwrap(),
                            serde_json::json!({
                                "message": "Process completed",
                                "received_input": inputs,
                            }),
                        ))
                    })
                }),
            )
            .await;

        executor
            .register_node_handler(
                "end",
                Arc::new(|ctx| {
                    Box::pin(async move {
                        // End node that just returns success
                        Ok(NodeResult::success(
                            ctx.current_node_id.unwrap(),
                            serde_json::json!({"message": "End completed"}),
                        ))
                    })
                }),
            )
            .await;

        // Start the executor
        executor.start().await.unwrap();

        // Execute a workflow
        let workflow = create_test_workflow();
        let instance_id = executor.execute_workflow(workflow).await.unwrap();

        // Wait for workflow to complete
        let mut completed = false;
        for _ in 0..10 {
            // Check if instance exists
            let instance = executor.state_manager.get_instance(&instance_id).await;
            if let Some(instance) = instance {
                let state = instance.read().await;
                if state.is_completed {
                    completed = true;
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Stop the executor
        executor.stop().await.unwrap();

        // Verify workflow completed
        assert!(completed, "Workflow did not complete in time");

        // Check workflow results
        let instance = executor
            .state_manager
            .get_instance(&instance_id)
            .await
            .unwrap();
        let state = instance.read().await;

        // Check all nodes completed
        let all_completed = state
            .node_status
            .values()
            .all(|status| *status == crate::model::NodeStatus::Completed);

        assert!(all_completed, "Not all nodes completed");
    }

    #[tokio::test]
    async fn test_executor_node_failure() {
        // Create dependencies
        let scheduler = Arc::new(WorkflowScheduler::new(SchedulerConfig::default()));
        let state_manager = Arc::new(crate::state::StateMachineManager::<MemoryStorage>::new());

        // Create executor
        let executor = WorkflowExecutor::new(scheduler, state_manager, ExecutorConfig::default());

        // Register node handlers
        executor
            .register_node_handler(
                "start",
                Arc::new(|ctx| {
                    Box::pin(async move {
                        // Start node that succeeds
                        Ok(NodeResult::success(
                            ctx.current_node_id.unwrap(),
                            serde_json::json!({"message": "Start completed"}),
                        ))
                    })
                }),
            )
            .await;

        executor
            .register_node_handler(
                "process",
                Arc::new(|_| {
                    Box::pin(async move {
                        // Process node that deliberately fails
                        Err(ExecutorError::NodeError("Deliberate failure".to_string()))
                    })
                }),
            )
            .await;

        executor
            .register_node_handler(
                "end",
                Arc::new(|ctx| {
                    Box::pin(async move {
                        // End node that won't be reached
                        Ok(NodeResult::success(
                            ctx.current_node_id.unwrap(),
                            serde_json::json!({"message": "End completed"}),
                        ))
                    })
                }),
            )
            .await;

        // Start the executor
        executor.start().await.unwrap();

        // Execute a workflow
        let workflow = create_test_workflow();
        let instance_id = executor.execute_workflow(workflow).await.unwrap();

        // Wait for workflow to fail
        let mut failed = false;
        for _ in 0..10 {
            // Check if instance exists
            let instance = executor.state_manager.get_instance(&instance_id).await;
            if let Some(instance) = instance {
                let state = instance.read().await;
                if state.has_failed {
                    failed = true;
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Stop the executor
        executor.stop().await.unwrap();

        // Verify workflow failed
        assert!(failed, "Workflow did not fail as expected");

        // Check workflow state
        let instance = executor
            .state_manager
            .get_instance(&instance_id)
            .await
            .unwrap();
        let state = instance.read().await;

        // Get node statuses
        let nodes: Vec<NodeId> = state.node_status.keys().cloned().collect();

        // Verify start completed, process failed, end not started
        assert_eq!(
            state.node_status[&nodes[0]],
            crate::model::NodeStatus::Completed
        ); // start
        assert_eq!(
            state.node_status[&nodes[1]],
            crate::model::NodeStatus::Failed
        ); // process
        assert_eq!(
            state.node_status[&nodes[2]],
            crate::model::NodeStatus::Pending
        ); // end (not reached)
    }
}
