//! # Lion Core
//!
//! `lion_core` provides the fundamental building blocks for the Lion microkernel system.
//! This includes error types, ID definitions, traits, and common data structures used
//! throughout the system.
//!
//! ## Core Principles
//!
//! The Lion microkernel is built upon several key architectural principles derived from
//! research in capability-based security, microkernel design, and WebAssembly isolation:
//!
//! 1. **Capability-Based Security**: Access to resources is controlled through
//!    unforgeable capability tokens following the principle of least privilege.
//!    Capabilities represent authority to access specific resources with specific
//!    permissions.
//!
//! 2. **Unified Capability-Policy Model**: Capabilities and policies are integrated
//!    in a cohesive security model that requires both capability possession and
//!    policy compliance for resource access:
//!    ```text
//!    permit(action) := has_capability(subject, object, action) ∧ policy_allows(subject, object, action)
//!    ```
//!
//! 3. **Partial Capability Relations**: Capabilities form a partial order based on
//!    privilege inclusion. This enables operations like capability attenuation,
//!    composition, and partial revocation.
//!
//! 4. **Actor-Based Concurrency**: Stateful components operate as isolated actors
//!    that communicate solely through message passing. Each plugin's lifecycle is
//!    managed by a dedicated task/thread following the single-writer principle.
//!
//! 5. **WebAssembly Isolation**: Plugins are isolated in WebAssembly sandboxes for
//!    security and resource control, with microsecond-level instantiation times and
//!    strong memory isolation.
//!
//! 6. **Workflow Orchestration**: Complex multi-step processes can be orchestrated
//!    with parallel execution and error handling through a declarative or programmatic
//!    workflow definition system.
//!
//! ## Crate Structure
//!
//! - **error**: Error types for all Lion components
//! - **id**: Strongly-typed identifier types
//! - **traits**: Core interfaces for the system
//! - **types**: Data structures used throughout the system
//! - **utils**: Utility functions and helpers
//! - **macros**: Convenience macros for logging, error handling, etc.

pub mod error;
pub mod id;
pub mod macros;
pub mod traits;
pub mod types;
pub mod utils;

// Re-export key types and traits for convenience
pub use error::{Error, Result};
pub use id::{CapabilityId, ExecutionId, MessageId, NodeId, PluginId, RegionId, WorkflowId};
// Macros are automatically exported at the crate root due to #[macro_export]
pub use traits::{Capability, ConcurrencyManager, IsolationBackend, PluginManager, WorkflowEngine};
pub use types::{
    AccessRequest, ErrorPolicy, ExecutionOptions, ExecutionStatus, MemoryRegion, MemoryRegionType,
    NodeStatus, NodeType, PluginConfig, PluginMetadata, PluginState, PluginType, ResourceUsage,
    Workflow, WorkflowNode,
};
pub use utils::{ConfigValue, LogLevel, Version};
