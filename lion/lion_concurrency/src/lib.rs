#![deny(warnings)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

//! # Lion Concurrency
//!
//! Concurrency primitives and execution models for the Lion microkernel.
//!
//! This crate provides the core concurrency infrastructure for Lion, including:
//!
//! - Actor-based messaging and supervision
//! - Resource and instance pooling
//! - Task scheduling and execution
//! - Efficient synchronization primitives
//!
//! ## Integration with Other Lion Crates
//!
//! - **lion_runtime**: Bootstrap and manage the actor system
//! - **lion_workflow**: Execute workflow steps as concurrent actors
//! - **lion_capability**: Enforce capability-based concurrency limits
//! - **lion_isolation**: Run isolated components safely within the concurrency framework

/// Actor-based concurrency system with message passing and supervision
pub mod actor;

/// Resource pooling and efficient reuse of expensive resources
pub mod pool;

/// Task scheduling and execution strategies
pub mod scheduler;

/// Synchronization primitives optimized for the Lion architecture
pub mod sync;

// Re-export key types for easier access
pub use actor::system::ActorSystem;
pub use pool::thread::ThreadPool;
pub use scheduler::executor::Executor;
