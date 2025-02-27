//! Workflow execution for the Lion runtime
//!
//! This module provides components for workflow execution and management,
//! including workflow instantiation, execution, and state management.

pub mod execution;
pub mod manager;

// Re-export key types for convenience
pub use manager::WorkflowManager;