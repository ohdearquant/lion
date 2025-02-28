//! Core data types for the Lion microkernel.
//!
//! This module defines the fundamental data structures used throughout
//! the system, including plugin configuration, workflow definitions,
//! memory management, and access control.

pub mod access;
pub mod memory;
pub mod plugin;
pub mod workflow;

pub use access::{AccessRequest, AccessRequestType};
pub use memory::{MemoryRegion, MemoryRegionType};
pub use plugin::{PluginConfig, PluginMetadata, PluginState, PluginType, ResourceUsage};
pub use workflow::{
    ErrorPolicy, ExecutionOptions, ExecutionStatus, NodeStatus, NodeType, Workflow, WorkflowNode,
};
