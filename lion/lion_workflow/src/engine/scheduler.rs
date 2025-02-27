use crate::model::{NodeId, Priority};
use crate::engine::context::ExecutionContext;
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

/// Error types for workflow scheduler
#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Task not found: {0}")]
    TaskNotFound(TaskId),
    
    #[error("Scheduler full: {0}")]
    SchedulerFull(String),
    
    #[error("Scheduler stopped")]
    SchedulerStopped,
    
    #[error("Scheduling error: {0}")]
    SchedulingError(String),
    
    #[error("Capacity error: {0}")]
    CapacityError(String),
}

/// Unique identifier for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(uuid::Uuid);

impl TaskId {
    /// Create a new task ID
    pub fn new() -> Self {
        TaskId(uuid::Uuid::new_v4())
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is waiting to be scheduled
    Pending,
    /// Task is running
    Running,
    /// Task has completed successfully
    Completed,
    /// Task has failed
    Failed,
    /// Task execution has been cancelled
    Cancelled,
}

/// Scheduling policy types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// First in, first out
    FIFO,
    /// Strict priority-based (higher priority first)
    Priority,
    /// Multi-level feedback queue (favor short tasks)
    MLFQ,
    /// Deadline-based (earliest deadline first)
    EDF,
    /// Round-robin (equal time slices)
    RoundRobin,
    /// Fair scheduling (balanced CPU time)
    Fair,
}

impl Default for SchedulingPolicy {
    fn default() -> Self {
        SchedulingPolicy::Priority
    }
}

/// Task for execution
#[derive(Debug)]
pub struct Task {
    /// Unique task ID
    pub id: TaskId,
    
    /// Node to execute
    pub node_id: NodeId,
    
    /// Workflow instance ID
    pub instance_id: String,
    
    /// Execution context
    pub context: ExecutionContext,
    
    /// Task priority
    pub priority: Priority,
    
    /// Deadline (if any)
    pub deadline: Option<DateTime<Utc>>,
    
    /// Creation time
    pub created_at: DateTime<Utc>,
    
    /// Start time (if started)
    pub started_at: Option<DateTime<Utc>>,
    
    /// Completion time (if completed)
    pub completed_at: Option<DateTime<Utc>>,
    
    /// Current status
    pub status: TaskStatus,
    
    /// Execution attempt
    pub attempt: u32,
    
    /// Maximum execution time
    pub max_execution_time: Option<Duration>,
    
    /// Workflow-specific metadata
    pub metadata: serde_json::Value,
}

impl Task {
    /// Create a new task
    pub fn new(node_id: NodeId, instance_id: String, context: ExecutionContext) -> Self {
        Task {
            id: TaskId::new(),
            node_id,
            instance_id,
            context: context.clone(),
            priority: match context.get_current_node() {
                Ok(node) => node.priority,
                Err(_) => Priority::Normal,
            },
            deadline: context.deadline,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            status: TaskStatus::Pending,
            attempt: context.attempt,
            max_execution_time: None,
            metadata: serde_json::Value::Null,
        }
    }
    
    /// Set the task priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set the task deadline
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }
    
    /// Set the maximum execution time
    pub fn with_max_execution_time(mut self, duration: Duration) -> Self {
        self.max_execution_time = Some(duration);
        self
    }
    
    /// Check if the task has a deadline
    pub fn has_deadline(&self) -> bool {
        self.deadline.is_some()
    }
    
    /// Check if the task's deadline has passed
    pub fn is_deadline_passed(&self) -> bool {
        if let Some(deadline) = self.deadline {
            Utc::now() > deadline
        } else {
            false
        }
    }
    
    /// Mark the task as running
    pub fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
    }
    
    /// Mark the task as completed
    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
    }
    
    /// Mark the task as failed
    pub fn mark_failed(&mut self) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
    }
    
    /// Mark the task as cancelled
    pub fn mark_cancelled(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }
    
    /// Check if the task has exceeded its maximum execution time
    pub fn has_exceeded_max_time(&self) -> bool {
        if let (Some(max_time), Some(started)) = (self.max_execution_time, self.started_at) {
            let elapsed = Utc::now().signed_duration_since(started);
            elapsed > chrono::Duration::from_std(max_time).unwrap()
        } else {
            false
        }
    }
}

/// Task wrapper for priority queue ordering
#[derive(Debug)]
struct PriorityTask(Arc<Task>);

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for PriorityTask {}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first
        let priority_ord = self.0.priority.cmp(&other.0.priority).reverse();
        
        // If priorities equal, use deadlines (earlier first)
        if priority_ord == Ordering::Equal && self.0.deadline.is_some() && other.0.deadline.is_some() {
            return self.0.deadline.cmp(&other.0.deadline);
        }
        
        // If one has deadline and other doesn't, prioritize the one with deadline
        if self.0.deadline.is_some() && other.0.deadline.is_none() {
            return Ordering::Greater;
        }
        if self.0.deadline.is_none() && other.0.deadline.is_some() {
            return Ordering::Less;
        }
        
        // If still equal, use creation time (earlier first)
        if priority_ord == Ordering::Equal {
            return self.0.created_at.cmp(&other.0.created_at);
        }
        
        priority_ord
    }
}

/// Task wrapper for deadline queue ordering
#[derive(Debug)]
struct DeadlineTask(Arc<Task>);

impl PartialEq for DeadlineTask {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for DeadlineTask {}

impl PartialOrd for DeadlineTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DeadlineTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Earlier deadline first
        match (self.0.deadline, other.0.deadline) {
            (Some(a), Some(b)) => b.cmp(&a), // Reverse order for max-heap (BinaryHeap)
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => self.0.priority.cmp(&other.0.priority).reverse(),
        }
    }
}

/// Configuration for workflow scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum number of tasks in queue
    pub max_queued_tasks: usize,
    
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: usize,
    
    /// Default task priority
    pub default_priority: Priority,
    
    /// Default task timeout
    pub default_timeout: Option<Duration>,
    
    /// Scheduling policy
    pub policy: SchedulingPolicy,
    
    /// Task quantum (time slice) for round-robin or MLFQ
    pub task_quantum: Duration,
    
    /// Number of priority levels for MLFQ
    pub mlfq_levels: usize,
    
    /// Enable work stealing between worker threads
    pub enable_work_stealing: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        SchedulerConfig {
            max_queued_tasks: 1000,
            max_concurrent_tasks: 10,
            default_priority: Priority::Normal,
            default_timeout: Some(Duration::from_secs(60)),
            policy: SchedulingPolicy::Priority,
            task_quantum: Duration::from_millis(100),
            mlfq_levels: 4,
            enable_work_stealing: true,
        }
    }
}

/// Workflow scheduler
pub struct WorkflowScheduler {
    /// Priority queue for tasks (used for Priority policy)
    priority_queue: Mutex<BinaryHeap<PriorityTask>>,
    
    /// Deadline queue for tasks (used for EDF policy)
    deadline_queue: Mutex<BinaryHeap<DeadlineTask>>,
    
    /// FIFO queue for tasks (used for FIFO policy)
    fifo_queue: Mutex<VecDeque<Arc<Task>>>,
    
    /// Multi-level feedback queues (used for MLFQ policy)
    mlfq_queues: Vec<Mutex<VecDeque<Arc<Task>>>>,
    
    /// Fair scheduling queue
    fair_queue: Mutex<VecDeque<Arc<Task>>>,
    
    /// Running tasks
    running_tasks: RwLock<HashMap<TaskId, Arc<Task>>>,
    
    /// Task registry (all tasks)
    task_registry: RwLock<HashMap<TaskId, Arc<Task>>>,
    
    /// Scheduler configuration
    config: RwLock<SchedulerConfig>,
    
    /// Current scheduling policy
    current_policy: RwLock<SchedulingPolicy>,
    
    /// Whether the scheduler is running
    is_running: RwLock<bool>,
}

impl WorkflowScheduler {
    /// Create a new workflow scheduler
    pub fn new(config: SchedulerConfig) -> Self {
        let mlfq_levels = config.mlfq_levels;
        let policy = config.policy;
        
        let mut mlfq_queues = Vec::with_capacity(mlfq_levels);
        for _ in 0..mlfq_levels {
            mlfq_queues.push(Mutex::new(VecDeque::new()));
        }
        
        WorkflowScheduler {
            priority_queue: Mutex::new(BinaryHeap::new()),
            deadline_queue: Mutex::new(BinaryHeap::new()),
            fifo_queue: Mutex::new(VecDeque::new()),
            mlfq_queues,
            fair_queue: Mutex::new(VecDeque::new()),
            running_tasks: RwLock::new(HashMap::new()),
            task_registry: RwLock::new(HashMap::new()),
            config: RwLock::new(config),
            current_policy: RwLock::new(policy),
            is_running: RwLock::new(true),
        }
    }
    
    /// Schedule a task for execution
    pub async fn schedule_task(&self, task: Task) -> Result<TaskId, SchedulerError> {
        // Check if scheduler is running
        if !*self.is_running.read().await {
            return Err(SchedulerError::SchedulerStopped);
        }
        
        // Check if queue is full
        let config = self.config.read().await;
        let task_registry = self.task_registry.read().await;
        
        if task_registry.len() >= config.max_queued_tasks {
            return Err(SchedulerError::SchedulerFull(
                format!("Maximum queued tasks reached: {}", config.max_queued_tasks)
            ));
        }
        
        let task_id = task.id;
        let task = Arc::new(task);
        
        // Add to appropriate queue based on current policy
        let policy = *self.current_policy.read().await;
        
        match policy {
            SchedulingPolicy::Priority => {
                let mut queue = self.priority_queue.lock().await;
                queue.push(PriorityTask(task.clone()));
            },
            SchedulingPolicy::EDF => {
                let mut queue = self.deadline_queue.lock().await;
                queue.push(DeadlineTask(task.clone()));
            },
            SchedulingPolicy::FIFO => {
                let mut queue = self.fifo_queue.lock().await;
                queue.push_back(task.clone());
            },
            SchedulingPolicy::MLFQ => {
                // Start at highest priority queue
                let mut queue = self.mlfq_queues[0].lock().await;
                queue.push_back(task.clone());
            },
            SchedulingPolicy::RoundRobin | SchedulingPolicy::Fair => {
                let mut queue = self.fair_queue.lock().await;
                queue.push_back(task.clone());
            },
        }
        
        // Add to registry
        drop(task_registry);
        let mut task_registry = self.task_registry.write().await;
        task_registry.insert(task_id, task);
        
        Ok(task_id)
    }
    
    /// Get the next task to execute
    pub async fn next_task(&self) -> Option<Arc<Task>> {
        let policy = *self.current_policy.read().await;
        
        match policy {
            SchedulingPolicy::Priority => {
                let mut queue = self.priority_queue.lock().await;
                queue.pop().map(|pt| pt.0)
            },
            SchedulingPolicy::EDF => {
                let mut queue = self.deadline_queue.lock().await;
                queue.pop().map(|dt| dt.0)
            },
            SchedulingPolicy::FIFO => {
                let mut queue = self.fifo_queue.lock().await;
                queue.pop_front()
            },
            SchedulingPolicy::MLFQ => {
                // Try each queue in order of priority
                for level in 0..self.mlfq_queues.len() {
                    let mut queue = self.mlfq_queues[level].lock().await;
                    if let Some(task) = queue.pop_front() {
                        return Some(task);
                    }
                }
                None
            },
            SchedulingPolicy::RoundRobin | SchedulingPolicy::Fair => {
                let mut queue = self.fair_queue.lock().await;
                queue.pop_front()
            },
        }
    }
    
    /// Mark a task as running
    pub async fn mark_task_running(&self, task_id: TaskId) -> Result<(), SchedulerError> {
        let mut task_registry = self.task_registry.write().await;
        let task = task_registry.get_mut(&task_id)
            .ok_or_else(|| SchedulerError::TaskNotFound(task_id))?;
        
        // Create mutable version of the task for updating status
        let task_ref = Arc::get_mut(task)
            .ok_or_else(|| SchedulerError::SchedulingError("Could not get mutable reference to task".to_string()))?;
        
        task_ref.mark_running();
        
        // Add to running tasks
        let mut running_tasks = self.running_tasks.write().await;
        running_tasks.insert(task_id, task.clone());
        
        Ok(())
    }
    
    /// Mark a task as completed
    pub async fn mark_task_completed(&self, task_id: TaskId) -> Result<(), SchedulerError> {
        let mut task_registry = self.task_registry.write().await;
        let task = task_registry.get_mut(&task_id)
            .ok_or_else(|| SchedulerError::TaskNotFound(task_id))?;
        
        // Create mutable version of the task for updating status
        let task_ref = Arc::get_mut(task)
            .ok_or_else(|| SchedulerError::SchedulingError("Could not get mutable reference to task".to_string()))?;
        
        task_ref.mark_completed();
        
        // Remove from running tasks
        let mut running_tasks = self.running_tasks.write().await;
        running_tasks.remove(&task_id);
        
        Ok(())
    }
    
    /// Mark a task as failed
    pub async fn mark_task_failed(&self, task_id: TaskId) -> Result<(), SchedulerError> {
        let mut task_registry = self.task_registry.write().await;
        let task = task_registry.get_mut(&task_id)
            .ok_or_else(|| SchedulerError::TaskNotFound(task_id))?;
        
        // Create mutable version of the task for updating status
        let task_ref = Arc::get_mut(task)
            .ok_or_else(|| SchedulerError::SchedulingError("Could not get mutable reference to task".to_string()))?;
        
        task_ref.mark_failed();
        
        // Remove from running tasks
        let mut running_tasks = self.running_tasks.write().await;
        running_tasks.remove(&task_id);
        
        Ok(())
    }
    
    /// Cancel a task
    pub async fn cancel_task(&self, task_id: TaskId) -> Result<(), SchedulerError> {
        let mut task_registry = self.task_registry.write().await;
        let task = task_registry.get_mut(&task_id)
            .ok_or_else(|| SchedulerError::TaskNotFound(task_id))?;
        
        // Create mutable version of the task for updating status
        let task_ref = Arc::get_mut(task)
            .ok_or_else(|| SchedulerError::SchedulingError("Could not get mutable reference to task".to_string()))?;
        
        task_ref.mark_cancelled();
        
        // Remove from running tasks
        let mut running_tasks = self.running_tasks.write().await;
        running_tasks.remove(&task_id);
        
        Ok(())
    }
    
    /// Get the current count of queued tasks
    pub async fn get_queued_task_count(&self) -> usize {
        let policy = *self.current_policy.read().await;
        
        match policy {
            SchedulingPolicy::Priority => {
                let queue = self.priority_queue.lock().await;
                queue.len()
            },
            SchedulingPolicy::EDF => {
                let queue = self.deadline_queue.lock().await;
                queue.len()
            },
            SchedulingPolicy::FIFO => {
                let queue = self.fifo_queue.lock().await;
                queue.len()
            },
            SchedulingPolicy::MLFQ => {
                let mut count = 0;
                for level in 0..self.mlfq_queues.len() {
                    let queue = self.mlfq_queues[level].lock().await;
                    count += queue.len();
                }
                count
            },
            SchedulingPolicy::RoundRobin | SchedulingPolicy::Fair => {
                let queue = self.fair_queue.lock().await;
                queue.len()
            },
        }
    }
    
    /// Get the current count of running tasks
    pub async fn get_running_task_count(&self) -> usize {
        let running_tasks = self.running_tasks.read().await;
        running_tasks.len()
    }
    
    /// Check if scheduler can accept more tasks
    pub async fn can_accept_more_tasks(&self) -> bool {
        let config = self.config.read().await;
        let task_registry = self.task_registry.read().await;
        
        task_registry.len() < config.max_queued_tasks
    }
    
    /// Check if more tasks can be executed concurrently
    pub async fn can_execute_more_tasks(&self) -> bool {
        let config = self.config.read().await;
        let running_tasks = self.running_tasks.read().await;
        
        running_tasks.len() < config.max_concurrent_tasks
    }
    
    /// Change the scheduling policy
    pub async fn change_policy(&self, policy: SchedulingPolicy) -> Result<(), SchedulerError> {
        let mut current_policy = self.current_policy.write().await;
        *current_policy = policy;
        
        // TODO: Rebalance queues if needed
        
        Ok(())
    }
    
    /// Stop the scheduler (no more tasks will be accepted)
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
    }
    
    /// Restart the scheduler
    pub async fn restart(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
    }
    
    /// Clear all tasks
    pub async fn clear(&self) {
        let mut priority_queue = self.priority_queue.lock().await;
        let mut deadline_queue = self.deadline_queue.lock().await;
        let mut fifo_queue = self.fifo_queue.lock().await;
        let mut fair_queue = self.fair_queue.lock().await;
        let mut task_registry = self.task_registry.write().await;
        let mut running_tasks = self.running_tasks.write().await;
        
        priority_queue.clear();
        deadline_queue.clear();
        fifo_queue.clear();
        fair_queue.clear();
        
        for level in 0..self.mlfq_queues.len() {
            let mut queue = self.mlfq_queues[level].lock().await;
            queue.clear();
        }
        
        task_registry.clear();
        running_tasks.clear();
    }
    
    /// Get task by ID
    pub async fn get_task(&self, task_id: TaskId) -> Option<Arc<Task>> {
        let task_registry = self.task_registry.read().await;
        task_registry.get(&task_id).cloned()
    }
    
    /// Get all tasks
    pub async fn get_all_tasks(&self) -> Vec<Arc<Task>> {
        let task_registry = self.task_registry.read().await;
        task_registry.values().cloned().collect()
    }
    
    /// Get all running tasks
    pub async fn get_running_tasks(&self) -> Vec<Arc<Task>> {
        let running_tasks = self.running_tasks.read().await;
        running_tasks.values().cloned().collect()
    }
    
    /// Update scheduler configuration
    pub async fn update_config(&self, config: SchedulerConfig) {
        let mut current_config = self.config.write().await;
        *current_config = config;
    }
    
    /// Demote a task in MLFQ (move to next lower priority queue)
    pub async fn demote_task(&self, task_id: TaskId) -> Result<(), SchedulerError> {
        let policy = *self.current_policy.read().await;
        
        if policy != SchedulingPolicy::MLFQ {
            return Ok(());
        }
        
        let task = {
            let task_registry = self.task_registry.read().await;
            task_registry.get(&task_id).cloned()
        };
        
        if let Some(task) = task {
            // Find the current level of the task
            for level in 0..self.mlfq_queues.len() - 1 {
                let mut queue = self.mlfq_queues[level].lock().await;
                let position = queue.iter().position(|t| t.id == task_id);
                
                if let Some(index) = position {
                    queue.remove(index);
                    drop(queue);
                    
                    // Add to next lower priority queue
                    let mut next_queue = self.mlfq_queues[level + 1].lock().await;
                    next_queue.push_back(task);
                    return Ok(());
                }
            }
        }
        
        Err(SchedulerError::TaskNotFound(task_id))
    }
    
    /// Check for task timeouts
    pub async fn check_timeouts(&self) -> Vec<TaskId> {
        let running_tasks = self.running_tasks.read().await;
        let mut timed_out = Vec::new();
        
        for (id, task) in running_tasks.iter() {
            if task.has_exceeded_max_time() {
                timed_out.push(*id);
            }
        }
        
        timed_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::NodeId;
    use crate::engine::context::ExecutionContext;
    use crate::model::WorkflowDefinition;
    use crate::state::WorkflowState;
    
    // Helper to create a test task
    fn create_test_task(priority: Priority) -> Task {
        let definition = Arc::new(WorkflowDefinition::new(crate::model::WorkflowId::new(), "Test".to_string()));
        let state = Arc::new(WorkflowState::new(definition.clone()));
        let context = ExecutionContext::new(definition, state);
        
        Task::new(NodeId::new(), "test-instance".to_string(), context)
            .with_priority(priority)
    }
    
    #[tokio::test]
    async fn test_priority_scheduling() {
        let config = SchedulerConfig {
            policy: SchedulingPolicy::Priority,
            ..Default::default()
        };
        
        let scheduler = WorkflowScheduler::new(config);
        
        // Create tasks with different priorities
        let low_task = create_test_task(Priority::Low);
        let normal_task = create_test_task(Priority::Normal);
        let high_task = create_test_task(Priority::High);
        
        // Schedule tasks in order of increasing priority
        let low_id = scheduler.schedule_task(low_task).await.unwrap();
        let normal_id = scheduler.schedule_task(normal_task).await.unwrap();
        let high_id = scheduler.schedule_task(high_task).await.unwrap();
        
        // Tasks should be dequeued in order of decreasing priority
        let task1 = scheduler.next_task().await.unwrap();
        let task2 = scheduler.next_task().await.unwrap();
        let task3 = scheduler.next_task().await.unwrap();
        
        assert_eq!(task1.id, high_id);
        assert_eq!(task2.id, normal_id);
        assert_eq!(task3.id, low_id);
    }
    
    #[tokio::test]
    async fn test_fifo_scheduling() {
        let config = SchedulerConfig {
            policy: SchedulingPolicy::FIFO,
            ..Default::default()
        };
        
        let scheduler = WorkflowScheduler::new(config);
        
        // Create tasks with the same priority
        let task1 = create_test_task(Priority::Normal);
        let task2 = create_test_task(Priority::Normal);
        let task3 = create_test_task(Priority::Normal);
        
        let id1 = task1.id;
        let id2 = task2.id;
        let id3 = task3.id;
        
        // Schedule tasks
        scheduler.schedule_task(task1).await.unwrap();
        scheduler.schedule_task(task2).await.unwrap();
        scheduler.schedule_task(task3).await.unwrap();
        
        // Tasks should be dequeued in order of insertion
        let task1 = scheduler.next_task().await.unwrap();
        let task2 = scheduler.next_task().await.unwrap();
        let task3 = scheduler.next_task().await.unwrap();
        
        assert_eq!(task1.id, id1);
        assert_eq!(task2.id, id2);
        assert_eq!(task3.id, id3);
    }
    
    #[tokio::test]
    async fn test_edf_scheduling() {
        let config = SchedulerConfig {
            policy: SchedulingPolicy::EDF,
            ..Default::default()
        };
        
        let scheduler = WorkflowScheduler::new(config);
        
        // Create tasks with different deadlines
        let mut task1 = create_test_task(Priority::Normal);
        let mut task2 = create_test_task(Priority::Normal);
        let mut task3 = create_test_task(Priority::Normal);
        
        // Set deadlines (further to closer)
        let now = Utc::now();
        let deadline1 = now + chrono::Duration::seconds(30);
        let deadline2 = now + chrono::Duration::seconds(20);
        let deadline3 = now + chrono::Duration::seconds(10);
        
        task1 = task1.with_deadline(deadline1);
        task2 = task2.with_deadline(deadline2);
        task3 = task3.with_deadline(deadline3);
        
        let id1 = task1.id;
        let id2 = task2.id;
        let id3 = task3.id;
        
        // Schedule tasks
        scheduler.schedule_task(task1).await.unwrap();
        scheduler.schedule_task(task2).await.unwrap();
        scheduler.schedule_task(task3).await.unwrap();
        
        // Tasks should be dequeued in order of deadline (closest first)
        let task1 = scheduler.next_task().await.unwrap();
        let task2 = scheduler.next_task().await.unwrap();
        let task3 = scheduler.next_task().await.unwrap();
        
        assert_eq!(task1.id, id3); // Closest deadline
        assert_eq!(task2.id, id2); // Middle deadline
        assert_eq!(task3.id, id1); // Furthest deadline
    }
    
    #[tokio::test]
    async fn test_task_lifecycle() {
        let config = SchedulerConfig::default();
        let scheduler = WorkflowScheduler::new(config);
        
        // Create and schedule a task
        let task = create_test_task(Priority::Normal);
        let task_id = scheduler.schedule_task(task).await.unwrap();
        
        // Get the task
        let task = scheduler.next_task().await.unwrap();
        assert_eq!(task.id, task_id);
        assert_eq!(task.status, TaskStatus::Pending);
        
        // Mark as running
        scheduler.mark_task_running(task_id).await.unwrap();
        let task = scheduler.get_task(task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Running);
        
        // Mark as completed
        scheduler.mark_task_completed(task_id).await.unwrap();
        let task = scheduler.get_task(task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
    }
    
    #[tokio::test]
    async fn test_scheduler_capacity() {
        let config = SchedulerConfig {
            max_queued_tasks: 2,
            ..Default::default()
        };
        
        let scheduler = WorkflowScheduler::new(config);
        
        // Schedule two tasks (should succeed)
        let task1 = create_test_task(Priority::Normal);
        let task2 = create_test_task(Priority::Normal);
        
        scheduler.schedule_task(task1).await.unwrap();
        scheduler.schedule_task(task2).await.unwrap();
        
        // Third task should fail due to capacity
        let task3 = create_test_task(Priority::Normal);
        let result = scheduler.schedule_task(task3).await;
        
        assert!(result.is_err());
        match result {
            Err(SchedulerError::SchedulerFull(_)) => (),
            _ => panic!("Expected SchedulerFull error"),
        }
    }
}