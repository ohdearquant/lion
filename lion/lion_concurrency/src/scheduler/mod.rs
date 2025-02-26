//! Task scheduling and execution strategies.
//!
//! This module provides mechanisms for scheduling and executing tasks with
//! various execution strategies and policies:
//!
//! - Task execution with timeouts and retries
//! - Priority-based scheduling
//! - Capability-based access control for tasks

pub mod executor;

// Re-export key types from executor
pub use executor::{
    ExecutionError, ExecutionOptions, Executor, ExecutorConfig, TaskHandle, TaskStatus,
};

#[cfg(test)]
mod tests {
    // Integration tests for the scheduler module can be added here
}
