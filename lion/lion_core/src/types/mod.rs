//! Core data types for the Lion microkernel.
//! 
//! This module defines the fundamental data structures used throughout
//! the system.

pub mod plugin;
pub mod workflow;
pub mod memory;
pub mod access;

pub use plugin::{PluginConfig, PluginMetadata, PluginState, PluginType, ResourceUsage};
pub use workflow::{Workflow, WorkflowNode, NodeType, ErrorPolicy, ExecutionStatus, NodeStatus, ExecutionOptions};
pub use memory::{MemoryRegion, MemoryRegionType};
pub use access::{AccessRequest, AccessRequestType};