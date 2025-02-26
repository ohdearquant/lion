//! Resource pooling and efficient reuse of expensive resources.
//!
//! This module provides pooling mechanisms for various types of resources:
//!
//! - Thread pools for parallel task execution
//! - Instance pools for reusable object instances
//! - Resource pools for expensive resources like connections

pub mod instance;
pub mod resource;
pub mod thread;

// Re-export key types from instance
pub use instance::{InstanceHandle, InstancePool, InstancePoolConfig, PoolStats, Poolable};

// Re-export key types from resource
pub use resource::{Resource, ResourceHandle, ResourcePool, ResourcePoolConfig, ResourcePoolError};

// Re-export key types from thread
pub use thread::{ThreadPool, ThreadPoolConfig, ThreadPoolError, ThreadPoolStats};

#[cfg(test)]
mod tests {
    // Integration tests for the pool module can be added here
}
