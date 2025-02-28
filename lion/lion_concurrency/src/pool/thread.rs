//! Thread pool implementation for parallel execution.
//!
//! Provides a generic thread pool for executing tasks in parallel.

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use log::{debug, error, info, trace};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Error when submitting a task to the thread pool
#[derive(Error, Debug)]
pub enum ThreadPoolError {
    /// The thread pool is shutting down
    #[error("thread pool is shutting down")]
    ShuttingDown,

    /// The task queue is full
    #[error("thread pool queue is full")]
    QueueFull,
}

/// Statistics about the thread pool
#[derive(Debug, Default, Clone)]
pub struct ThreadPoolStats {
    /// Number of tasks queued
    pub tasks_queued: usize,

    /// Number of tasks completed
    pub tasks_completed: usize,

    /// Number of tasks that panicked
    pub tasks_panicked: usize,

    /// Total task execution time (microseconds)
    pub total_execution_time_us: u64,

    /// Queue wait time (microseconds)
    pub total_queue_time_us: u64,

    /// Maximum task execution time (microseconds)
    pub max_execution_time_us: u64,
}

/// Configuration for the thread pool
#[derive(Debug, Clone)]
pub struct ThreadPoolConfig {
    /// Maximum size of the task queue
    pub queue_size: usize,

    /// Maximum number of worker threads
    pub max_threads: usize,

    /// Name prefix for worker threads
    pub thread_name_prefix: String,

    /// Whether to collect performance statistics
    pub collect_stats: bool,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        Self {
            queue_size: 1000,
            max_threads: num_cpus::get(),
            thread_name_prefix: "lion-worker".to_string(),
            collect_stats: true,
        }
    }
}

/// Task with metadata for tracking
struct Task {
    /// The closure to execute
    func: Box<dyn FnOnce() + Send + 'static>,

    /// When the task was enqueued
    enqueued_at: Instant,
}

impl Task {
    /// Create a new task
    fn new<F>(f: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            func: Box::new(f),
            enqueued_at: Instant::now(),
        }
    }
}

/// Worker context holding shared state for the worker loop
struct WorkerContext {
    receiver: Receiver<Task>,
    shutdown_flag: Arc<AtomicBool>,
    collect_stats: bool,
    tasks_completed: Arc<AtomicUsize>,
    tasks_panicked: Arc<AtomicUsize>,
    total_execution_time_us: Arc<AtomicUsize>,
    total_queue_time_us: Arc<AtomicUsize>,
    max_execution_time_us: Arc<AtomicUsize>,
}

/// A generic thread pool for executing tasks
#[allow(dead_code)]
pub struct ThreadPool {
    /// Channel for sending tasks to worker threads
    task_sender: Sender<Task>,

    /// Worker threads
    workers: Vec<JoinHandle<()>>,

    /// Flag indicating if the pool is shutting down
    is_shutting_down: Arc<AtomicBool>,

    /// Statistics counters
    stats: Arc<ThreadPoolStats>,

    /// Configuration
    config: ThreadPoolConfig,

    /// Tasks tracking for statistics
    tasks_queued: Arc<AtomicUsize>,

    /// Tasks completed tracking for statistics
    tasks_completed: Arc<AtomicUsize>,

    /// Tasks panicked tracking for statistics
    tasks_panicked: Arc<AtomicUsize>,

    /// Total execution time tracking for statistics
    total_execution_time_us: Arc<AtomicUsize>,

    /// Total queue time tracking for statistics
    total_queue_time_us: Arc<AtomicUsize>,

    /// Maximum execution time tracking for statistics
    max_execution_time_us: Arc<AtomicUsize>,
}

impl ThreadPool {
    /// Create a new thread pool with the default configuration
    pub fn new(threads: usize) -> Self {
        let config = ThreadPoolConfig {
            max_threads: threads,
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Create a new thread pool with the specified configuration
    pub fn with_config(config: ThreadPoolConfig) -> Self {
        let (task_sender, task_receiver) = bounded(config.queue_size);
        let is_shutting_down = Arc::new(AtomicBool::new(false));

        let tasks_queued = Arc::new(AtomicUsize::new(0));
        let tasks_completed = Arc::new(AtomicUsize::new(0));
        let tasks_panicked = Arc::new(AtomicUsize::new(0));
        let total_execution_time_us = Arc::new(AtomicUsize::new(0));
        let total_queue_time_us = Arc::new(AtomicUsize::new(0));
        let max_execution_time_us = Arc::new(AtomicUsize::new(0));

        let stats = Arc::new(ThreadPoolStats::default());

        info!(
            "Creating thread pool with {} workers and queue size {}",
            config.max_threads, config.queue_size
        );

        let mut workers = Vec::with_capacity(config.max_threads);

        for id in 0..config.max_threads {
            let thread_name = format!("{}-{}", config.thread_name_prefix, id);
            let receiver = task_receiver.clone();
            let shutdown_flag = Arc::clone(&is_shutting_down);

            let tasks_completed = Arc::clone(&tasks_completed);
            let tasks_panicked = Arc::clone(&tasks_panicked);
            let total_execution_time_us = Arc::clone(&total_execution_time_us);
            let total_queue_time_us = Arc::clone(&total_queue_time_us);
            let max_execution_time_us = Arc::clone(&max_execution_time_us);
            let collect_stats = config.collect_stats;

            let builder = thread::Builder::new().name(thread_name.clone());

            let handle = builder
                .spawn(move || {
                    let ctx = WorkerContext {
                        receiver,
                        shutdown_flag,
                        collect_stats,
                        tasks_completed,
                        tasks_panicked,
                        total_execution_time_us,
                        total_queue_time_us,
                        max_execution_time_us,
                    };

                    Self::worker_loop(id, ctx);
                })
                .expect("Failed to spawn worker thread");

            workers.push(handle);
        }

        Self {
            task_sender,
            workers,
            is_shutting_down,
            stats,
            config,
            tasks_queued,
            tasks_completed,
            tasks_panicked,
            total_execution_time_us,
            total_queue_time_us,
            max_execution_time_us,
        }
    }

    /// Worker thread main loop
    fn worker_loop(id: usize, ctx: WorkerContext) {
        debug!("Worker {}: Starting", id);

        while !ctx.shutdown_flag.load(Ordering::Relaxed) {
            // Wait for a task or check shutdown flag every 100ms
            match ctx.receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(task) => {
                    // Calculate queue time
                    let queue_time = task.enqueued_at.elapsed();

                    if ctx.collect_stats {
                        ctx.total_queue_time_us
                            .fetch_add(queue_time.as_micros() as usize, Ordering::Relaxed);
                    }

                    trace!(
                        "Worker {}: Executing task (queue time: {:.2}ms)",
                        id,
                        queue_time.as_micros() as f64 / 1000.0
                    );

                    let exec_start = Instant::now();

                    // Execute the task and catch any panics
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (task.func)();
                    }));

                    let exec_time = exec_start.elapsed();

                    if ctx.collect_stats {
                        // Update execution time stats
                        ctx.total_execution_time_us
                            .fetch_add(exec_time.as_micros() as usize, Ordering::Relaxed);

                        // Update max execution time using compare-and-swap
                        let mut current_max = ctx.max_execution_time_us.load(Ordering::Relaxed);
                        let exec_time_us = exec_time.as_micros() as usize;

                        while exec_time_us > current_max {
                            match ctx.max_execution_time_us.compare_exchange(
                                current_max,
                                exec_time_us,
                                Ordering::SeqCst,
                                Ordering::Relaxed,
                            ) {
                                Ok(_) => break,
                                Err(actual) => current_max = actual,
                            }
                        }
                    }

                    match result {
                        Ok(_) => {
                            trace!(
                                "Worker {}: Task completed in {:.2}ms",
                                id,
                                exec_time.as_micros() as f64 / 1000.0
                            );

                            if ctx.collect_stats {
                                ctx.tasks_completed.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        Err(e) => {
                            error!(
                                "Worker {}: Task panicked: {:?}",
                                id,
                                e.downcast_ref::<&str>().unwrap_or(&"<unknown panic>")
                            );

                            if ctx.collect_stats {
                                ctx.tasks_panicked.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
                Err(_) => {
                    // Timeout occurred, check shutdown flag
                    if ctx.shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        }

        debug!("Worker {}: Shutting down", id);
    }

    /// Submit a task to be executed by the thread pool
    pub fn execute<F>(&self, f: F) -> Result<(), ThreadPoolError>
    where
        F: FnOnce() + Send + 'static,
    {
        if self.is_shutting_down.load(Ordering::Relaxed) {
            return Err(ThreadPoolError::ShuttingDown);
        }

        let task = Task::new(f);

        match self.task_sender.try_send(task) {
            Ok(_) => {
                if self.config.collect_stats {
                    self.tasks_queued.fetch_add(1, Ordering::Relaxed);
                }
                Ok(())
            }
            Err(TrySendError::Full(_)) => Err(ThreadPoolError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(ThreadPoolError::ShuttingDown),
        }
    }

    /// Submit a task and block until it's accepted
    pub fn execute_blocking<F>(&self, f: F) -> Result<(), ThreadPoolError>
    where
        F: FnOnce() + Send + 'static,
    {
        if self.is_shutting_down.load(Ordering::Relaxed) {
            return Err(ThreadPoolError::ShuttingDown);
        }

        let task = Task::new(f);

        match self.task_sender.send(task) {
            Ok(_) => {
                if self.config.collect_stats {
                    self.tasks_queued.fetch_add(1, Ordering::Relaxed);
                }
                Ok(())
            }
            Err(_) => Err(ThreadPoolError::ShuttingDown),
        }
    }

    /// Get current statistics for the thread pool
    pub fn get_stats(&self) -> ThreadPoolStats {
        if self.config.collect_stats {
            ThreadPoolStats {
                tasks_queued: self.tasks_queued.load(Ordering::Relaxed),
                tasks_completed: self.tasks_completed.load(Ordering::Relaxed),
                tasks_panicked: self.tasks_panicked.load(Ordering::Relaxed),
                total_execution_time_us: self.total_execution_time_us.load(Ordering::Relaxed)
                    as u64,
                total_queue_time_us: self.total_queue_time_us.load(Ordering::Relaxed) as u64,
                max_execution_time_us: self.max_execution_time_us.load(Ordering::Relaxed) as u64,
            }
        } else {
            ThreadPoolStats::default()
        }
    }

    /// Gracefully shut down the thread pool
    pub fn shutdown(&self) {
        info!("Shutting down thread pool");
        self.is_shutting_down.store(true, Ordering::Relaxed);

        // Drop the sender to close the channel
        // (would need to clone the ThreadPool struct to do this)
        // Instead, workers check the shutdown flag regularly
    }

    /// Shut down the thread pool and wait for workers to finish
    pub fn shutdown_and_join(mut self) {
        self.shutdown();

        // Wait for all workers to finish
        for worker in self.workers.drain(..) {
            worker.join().unwrap_or_else(|e| {
                error!("Worker thread panicked during shutdown: {:?}", e);
            });
        }

        info!("Thread pool shutdown complete");
    }

    /// Get the number of worker threads
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Check if the thread pool is shutting down
    pub fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::Relaxed)
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        if !self.is_shutting_down.load(Ordering::Relaxed) {
            self.shutdown();
        }

        // We can't join the threads here because it would require moving out of self
        // But we set the shutdown flag so threads should exit soon
        debug!("Thread pool dropped - workers will exit when they next check the shutdown flag");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_thread_pool_basic() {
        let pool = ThreadPool::new(4);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        pool.execute(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();

        // Give the task time to complete
        thread::sleep(Duration::from_millis(50));

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_thread_pool_multiple_tasks() {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..10 {
            let counter = counter.clone();
            pool.execute(move || {
                counter.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(10));
            })
            .unwrap();
        }

        // Give tasks time to complete
        thread::sleep(Duration::from_millis(200));

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_thread_pool_panic_handling() {
        let pool = ThreadPool::new(1);

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        // Task 1: Panics
        pool.execute(|| {
            panic!("This task should panic");
        })
        .unwrap();

        // Task 2: Should still run
        pool.execute(move || {
            flag_clone.store(true, Ordering::SeqCst);
        })
        .unwrap();

        // Give tasks time to complete
        thread::sleep(Duration::from_millis(100));

        assert!(flag.load(Ordering::SeqCst));

        // Check panic was recorded in stats
        let stats = pool.get_stats();
        assert_eq!(stats.tasks_panicked, 1);
    }

    #[test]
    fn test_thread_pool_shutdown() {
        let pool = ThreadPool::new(2);
        let barrier = Arc::new(Mutex::new(()));
        let lock = barrier.lock().unwrap();

        let barrier_clone = barrier.clone();
        pool.execute(move || {
            // This task will block until we release the barrier
            let _lock = barrier_clone.lock().unwrap();
        })
        .unwrap();

        // Task is now waiting on the barrier
        pool.shutdown();

        // Try to submit another task after shutdown
        let result = pool.execute(|| {});
        assert!(matches!(result, Err(ThreadPoolError::ShuttingDown)));

        // Release the barrier and allow the first task to complete
        drop(lock);
    }

    #[test]
    fn test_thread_pool_stats() {
        let pool = ThreadPool::new(1);

        // Submit tasks
        for _ in 0..5 {
            pool.execute(|| {
                thread::sleep(Duration::from_millis(10));
            })
            .unwrap();
        }

        // Submit a task that panics
        pool.execute(|| {
            panic!("This task should panic");
        })
        .unwrap();

        // Give tasks time to complete
        thread::sleep(Duration::from_millis(100));

        // Check stats
        let stats = pool.get_stats();
        assert_eq!(stats.tasks_queued, 6);
        assert_eq!(stats.tasks_completed, 5);
        assert_eq!(stats.tasks_panicked, 1);
        assert!(stats.total_execution_time_us > 0);
        assert!(stats.max_execution_time_us > 0);
    }

    #[test]
    fn test_thread_pool_queue_full() {
        let config = ThreadPoolConfig {
            queue_size: 1,
            max_threads: 1,
            thread_name_prefix: "test".to_string(),
            collect_stats: true,
        };

        let pool = ThreadPool::with_config(config);

        // Create a barrier to block the worker
        let barrier = Arc::new(Mutex::new(()));
        let lock = barrier.lock().unwrap();

        // Submit a task that blocks
        let barrier_clone = barrier.clone();
        pool.execute(move || {
            // This task will block until we release the barrier
            let _lock = barrier_clone.lock().unwrap();
        })
        .unwrap();

        // Wait for the task to start
        thread::sleep(Duration::from_millis(10));

        // Fill the queue
        pool.execute(|| {}).unwrap();

        // Try to submit one more task - should fail with QueueFull
        let result = pool.execute(|| {});
        assert!(matches!(result, Err(ThreadPoolError::QueueFull)));

        // Release the barrier
        drop(lock);
    }
}
