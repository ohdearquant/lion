//! Core traits that define the Lion microkernel interfaces.
//! 
//! This module contains the fundamental interfaces that each component
//! must implement. These interfaces are designed for stability and
//! clear separation of concerns.

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