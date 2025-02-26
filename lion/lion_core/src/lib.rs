//! # Lion Core
//! 
//! `lion_core` provides the fundamental building blocks for the Lion microkernel system.
//! This includes error types, ID definitions, traits, and common data structures used
//! throughout the system.
//! 
//! The core principles of Lion include:
//! 
//! 1. **Capability-Based Security**: Access to resources is controlled through
//!    unforgeable capability tokens following the principle of least privilege.
//! 
//! 2. **Unified Capability-Policy Model**: Capabilities and policies are integrated
//!    in a cohesive security model that requires both capability possession and
//!    policy compliance for resource access.
//! 
//! 3. **Actor-Based Concurrency**: Stateful components operate as isolated actors
//!    that communicate solely through message passing.
//! 
//! 4. **WebAssembly Isolation**: Plugins are isolated in WebAssembly sandboxes for
//!    security and resource control.
//! 
//! 5. **Workflow Orchestration**: Complex multi-step processes can be orchestrated
//!    with parallel execution and error handling.

pub mod error;
pub mod id;
pub mod traits;
pub mod types;
pub mod utils;

// Re-export key types and traits for convenience
pub use error::{Error, Result};
pub use id::{CapabilityId, PluginId, WorkflowId, NodeId, ExecutionId, RegionId, MessageId};
pub use traits::{Capability, PluginManager, IsolationBackend, ConcurrencyManager, WorkflowEngine};