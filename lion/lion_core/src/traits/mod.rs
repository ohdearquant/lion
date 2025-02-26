//! Core traits that define the Lion microkernel interfaces.
//!
//! This module contains the fundamental interfaces that each component
//! must implement. These interfaces are designed for stability and
//! clear separation of concerns.
//!
//! Key traits include:
//!
//! - `Capability`: The core interface for capability-based security
//! - `ConcurrencyManager`: Interface for concurrency and instance management
//! - `IsolationBackend`: Interface for plugin isolation
//! - `PluginManager`: Interface for plugin lifecycle management
//! - `WorkflowEngine`: Interface for workflow execution

pub mod capability;
pub mod concurrency;
pub mod isolation;
pub mod plugin;
pub mod workflow;

pub use capability::Capability;
pub use concurrency::ConcurrencyManager;
pub use isolation::IsolationBackend;
pub use plugin::PluginManager;
pub use workflow::WorkflowEngine;

#[cfg(feature = "async")]
pub use concurrency::AsyncConcurrencyManager;
