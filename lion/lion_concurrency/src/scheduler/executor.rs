//! Task execution and scheduling.
//!
//! Provides mechanisms for executing and scheduling tasks with
//! capability-based access control.

use crate::actor::system::ActorSystem;
use crate::pool::thread::ThreadPool;
use lion_core::error::{Error as LionError, Result as LionResult};
use log::{debug, error, info};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Error types for execution failures
#[derive(Error, Debug)]
pub enum ExecutionError {
    /// The task could not be scheduled due to resource constraints
    #[error("scheduling rejected: {0}")]
    SchedulingRejected(String),

    /// The task timed out during execution
    #[error("execution timed out after {0:?}")]
    Timeout(Duration),

    /// The task was cancelled
    #[error("execution cancelled: {0}")]
    Cancelled(String),

    /// The task faulted during execution
    #[error("execution faulted: {0}")]
    Fault(String),

    /// The task failed due to capability check failure
    #[error("capability check failed: {0}")]
    CapabilityDenied(String),
}

/// Status of a scheduled task
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    /// Task is waiting to be executed
    Pending,

    /// Task is currently running
    Running,

    /// Task completed successfully
    Completed,

    /// Task failed
    Failed(String),

    /// Task was cancelled
    Cancelled,

    /// Task timed out
    TimedOut,
}

/// Options for task execution
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    /// Priority of the task (higher numbers = higher priority)
    pub priority: u8,

    /// Maximum execution time for the task
    pub timeout: Option<Duration>,

    /// Whether to retry on failure
    pub retry_on_failure: bool,

    /// Maximum number of retry attempts
    pub max_retries: u8,

    /// Delay between retry attempts
    pub retry_delay: Duration,

    /// Capabilities required for execution
    pub required_capabilities: Vec<String>,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            priority: 0,
            timeout: None,
            retry_on_failure: false,
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            required_capabilities: Vec::new(),
        }
    }
}

/// A handle to a scheduled task
pub struct TaskHandle {
    /// Unique identifier for the task
    id: String,

    /// Status of the task
    status: Arc<Mutex<TaskStatus>>,

    /// Time when the task was created
    created_at: Instant,

    /// Execution options for the task
    options: ExecutionOptions,
}

impl TaskHandle {
    /// Create a new task handle
    fn new(id: String, options: ExecutionOptions) -> Self {
        Self {
            id,
            status: Arc::new(Mutex::new(TaskStatus::Pending)),
            created_at: Instant::now(),
            options,
        }
    }

    /// Get the task ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the current status of the task
    pub fn status(&self) -> TaskStatus {
        let status = self.status.lock().unwrap();
        status.clone()
    }

    /// Get the time since the task was created
    pub fn elapsed(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get the execution options for the task
    pub fn options(&self) -> &ExecutionOptions {
        &self.options
    }

    /// Wait for the task to complete with a timeout
    pub fn wait_with_timeout(&self, timeout: Duration) -> LionResult<TaskStatus> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            let status = self.status();

            match status {
                TaskStatus::Completed
                | TaskStatus::Failed(_)
                | TaskStatus::Cancelled
                | TaskStatus::TimedOut => {
                    return Ok(status);
                }
                _ => {
                    // Wait a bit before checking again
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }

        Err(LionError::Runtime(format!(
            "Task {} did not complete within timeout",
            self.id
        )))
    }

    /// Update the status of the task
    fn update_status(&self, new_status: TaskStatus) {
        let mut status = self.status.lock().unwrap();
        *status = new_status;
    }
}

/// Configuration for the executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Number of worker threads for the thread pool
    pub worker_threads: usize,

    /// Default timeout for tasks
    pub default_timeout: Option<Duration>,

    /// Whether to enforce capabilities
    pub enforce_capabilities: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            default_timeout: None,
            enforce_capabilities: true,
        }
    }
}

/// Task executor for Lion
pub struct Executor {
    /// Thread pool for executing tasks
    thread_pool: ThreadPool,

    /// Actor system for actor-based tasks
    actor_system: Option<Arc<ActorSystem>>,

    /// Configuration for this executor
    config: ExecutorConfig,

    /// Next task ID
    next_task_id: Mutex<u64>,
}

impl Executor {
    /// Create a new executor with the default configuration
    pub fn new() -> Self {
        Self::with_config(ExecutorConfig::default())
    }

    /// Create a new executor with the specified configuration
    pub fn with_config(config: ExecutorConfig) -> Self {
        let thread_pool = ThreadPool::new(config.worker_threads);

        info!(
            "Creating executor with {} worker threads",
            config.worker_threads
        );

        Self {
            thread_pool,
            actor_system: None,
            config,
            next_task_id: Mutex::new(1),
        }
    }

    /// Set the actor system for this executor
    pub fn set_actor_system(&mut self, system: Arc<ActorSystem>) {
        self.actor_system = Some(system);
    }

    /// Execute a task with the default options
    pub fn execute<F>(&self, task: F) -> LionResult<TaskHandle>
    where
        F: FnOnce() + Send + 'static,
    {
        self.execute_with_options(task, ExecutionOptions::default())
    }

    /// Execute a task with the specified options
    pub fn execute_with_options<F>(
        &self,
        task: F,
        options: ExecutionOptions,
    ) -> LionResult<TaskHandle>
    where
        F: FnOnce() + Send + 'static,
    {
        // Generate a task ID
        let task_id = {
            let mut id = self.next_task_id.lock().unwrap();
            let current_id = *id;
            *id += 1;
            format!("task-{}", current_id)
        };

        // Create the task handle
        let handle = TaskHandle::new(task_id.clone(), options.clone());
        let status = handle.status.clone();

        // Check for capabilities if enforcement is enabled
        if self.config.enforce_capabilities && !options.required_capabilities.is_empty() {
            // TODO: Implement capability checking
            // For now, we always allow
            debug!("Capability checking bypassed for task {}", task_id);
        }

        // Execute the task in the thread pool
        let task_timeout = options.timeout.or(self.config.default_timeout);

        let result = self.thread_pool.execute(move || {
            // Mark the task as running
            {
                let mut status_guard = status.lock().unwrap();
                *status_guard = TaskStatus::Running;
            }

            // Execute the task with timeout if specified
            let result = if let Some(timeout) = task_timeout {
                let start = Instant::now();

                // Create a thread to execute the task
                let task_thread = std::thread::spawn(move || {
                    task();
                });

                // Wait for the thread to complete or timeout
                match task_thread.join() {
                    Ok(_) => {
                        if start.elapsed() > timeout {
                            Err(ExecutionError::Timeout(timeout))
                        } else {
                            Ok(())
                        }
                    }
                    Err(_) => Err(ExecutionError::Fault("Task panicked".to_string())),
                }
            } else {
                // No timeout, just execute the task
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    task();
                }));

                match result {
                    Ok(_) => Ok(()),
                    Err(_) => Err(ExecutionError::Fault("Task panicked".to_string())),
                }
            };

            // Update the task status based on the result
            let mut status_guard = status.lock().unwrap();
            *status_guard = match result {
                Ok(_) => TaskStatus::Completed,
                Err(ExecutionError::Timeout(duration)) => TaskStatus::TimedOut,
                Err(ExecutionError::Fault(msg)) => TaskStatus::Failed(msg),
                Err(ExecutionError::Cancelled(msg)) => TaskStatus::Cancelled,
                Err(e) => TaskStatus::Failed(format!("{}", e)),
            };
        });

        match result {
            Ok(_) => Ok(handle),
            Err(e) => Err(LionError::Runtime(format!(
                "Failed to schedule task: {}",
                e
            ))),
        }
    }

    /// Execute a task and wait for it to complete
    pub fn execute_and_wait<F>(&self, task: F, timeout: Duration) -> LionResult<TaskStatus>
    where
        F: FnOnce() + Send + 'static,
    {
        let handle = self.execute(task)?;
        handle.wait_with_timeout(timeout)
    }

    /// Get the number of worker threads
    pub fn worker_count(&self) -> usize {
        self.thread_pool.worker_count()
    }

    /// Shut down the executor
    pub fn shutdown(&self) {
        info!("Shutting down executor");
        self.thread_pool.shutdown();

        if let Some(system) = &self.actor_system {
            system.shutdown();
        }
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_executor_basic() {
        let executor = Executor::new();

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        let handle = executor
            .execute(move || {
                thread::sleep(Duration::from_millis(10));
                flag_clone.store(true, Ordering::SeqCst);
            })
            .unwrap();

        // Wait for the task to complete
        thread::sleep(Duration::from_millis(50));

        assert!(flag.load(Ordering::SeqCst));
        assert_eq!(handle.status(), TaskStatus::Completed);
    }

    #[test]
    fn test_executor_timeout() {
        let executor = Executor::new();

        let options = ExecutionOptions {
            timeout: Some(Duration::from_millis(10)),
            ..Default::default()
        };

        let handle = executor
            .execute_with_options(
                || {
                    // This task will take longer than the timeout
                    thread::sleep(Duration::from_millis(50));
                },
                options,
            )
            .unwrap();

        // Wait for the task to be marked as timed out
        thread::sleep(Duration::from_millis(100));

        assert_eq!(handle.status(), TaskStatus::TimedOut);
    }

    #[test]
    fn test_executor_panic_handling() {
        let executor = Executor::new();

        let handle = executor
            .execute(|| {
                panic!("This task should panic");
            })
            .unwrap();

        // Wait for the task to fail
        thread::sleep(Duration::from_millis(50));

        match handle.status() {
            TaskStatus::Failed(msg) => {
                assert!(msg.contains("panicked"));
            }
            status => panic!("Expected Failed status, got {:?}", status),
        }
    }

    #[test]
    fn test_execute_and_wait() {
        let executor = Executor::new();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let status = executor
            .execute_and_wait(
                move || {
                    thread::sleep(Duration::from_millis(10));
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                },
                Duration::from_millis(100),
            )
            .unwrap();

        assert_eq!(status, TaskStatus::Completed);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_execute_and_wait_timeout() {
        let executor = Executor::new();

        let result = executor
            .execute_and_wait(
                || {
                    thread::sleep(Duration::from_millis(100));
                },
                Duration::from_millis(10),
            )
            .unwrap_err();

        // Should fail with a timeout error
        assert!(matches!(result, LionError::Timeout(_)));
    }

    #[test]
    fn test_multiple_tasks() {
        let executor = Executor::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();

        for _ in 0..10 {
            let counter = counter.clone();
            let handle = executor
                .execute(move || {
                    thread::sleep(Duration::from_millis(10));
                    counter.fetch_add(1, Ordering::SeqCst);
                })
                .unwrap();

            handles.push(handle);
        }

        // Wait for all tasks to complete
        thread::sleep(Duration::from_millis(100));

        // Check all tasks completed
        for handle in handles {
            assert_eq!(handle.status(), TaskStatus::Completed);
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_task_handle_wait() {
        let executor = Executor::new();

        let handle = executor
            .execute(|| {
                thread::sleep(Duration::from_millis(50));
            })
            .unwrap();

        // Wait with a longer timeout than the task
        let result = handle.wait_with_timeout(Duration::from_millis(100));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskStatus::Completed);

        // Create another task
        let handle = executor
            .execute(|| {
                thread::sleep(Duration::from_millis(100));
            })
            .unwrap();

        // Wait with a shorter timeout than the task
        let result = handle.wait_with_timeout(Duration::from_millis(50));
        assert!(result.is_err());
    }
}
