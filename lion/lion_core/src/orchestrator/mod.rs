//! # Orchestrator
//! 
//! The orchestrator is the core component of the Lion framework, responsible for managing
//! multi-agent concurrency, event routing, and system coordination.
//!
//! ## Architecture
//!
//! The orchestrator follows a message-passing architecture where different components
//! (agents, plugins, tasks) communicate through events. This ensures loose coupling and
//! enables scalable concurrent operations.
//!
//! ## Components
//!
//! - `events`: Defines the core event types (Agent, Plugin, System, Task) that flow
//!   through the system.
//!
//! - `metadata`: Provides event metadata tracking and correlation capabilities.
//!   Note: The `create_metadata` function is deprecated in favor of `EventMetadata::new`.
//!
//! - `processor`: Contains the main orchestrator implementation that processes events
//!   and manages system state.
//!
//! - `types`: Common type definitions used throughout the orchestrator system.
//!
//! ## Event Flow
//!
//! Events flow through the system in a predictable pattern:
//! 1. Events are created and tagged with metadata
//! 2. The orchestrator processes events based on type
//! 3. Results are emitted as completion events
//! 4. Subscribers receive and handle completion events

pub mod events;
pub mod metadata;
mod processor;
mod agent_manager;
mod metrics_manager;
mod types;

pub use events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent};
#[deprecated(note = "use EventMetadata::new instead")]
pub use metadata::create_metadata;
pub use metadata::EventMetadata;
pub use metrics_manager::MetricsManager;
pub use agent_manager::AgentManager;
pub use processor::{Orchestrator, OrchestratorConfig, OrchestratorError};
pub use types::*;

/// Result type for orchestrator operations
pub type Result<T> = std::result::Result<T, OrchestratorError>;
