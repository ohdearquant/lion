//! # Agentic Core
//! 
//! A microkernel-based framework for multi-agent AI orchestration, powered by Rust.
//! 
//! This library provides an event-driven microkernel that implements a "language network protocol",
//! enabling secure multi-agent concurrency with plugin expansions. The architecture follows
//! microkernel principles:
//! 
//! - **Strong Isolation**: Each agent/language node operates as a separate plugin with strict boundaries
//! - **Event-Driven**: The kernel manages tasks and agent communications through a message-passing system
//! - **Minimal Core**: Core functionality is limited to scheduling and event routing
//! - **Maximum Extensibility**: Additional capabilities are added through a plugin system
//! 
//! ## Architecture
//! 
//! The system consists of several key components:
//! 
//! - **Orchestrator**: The microkernel heart that schedules tasks and manages events
//! - **Plugin Manager**: Handles dynamic loading of agent code and plugins
//! - **Event System**: Captures language-based messages and system state changes
//! - **Collections**: Thread-safe data structures for concurrent operations
//! - **Storage**: Persistent storage for agent transcripts and system state
//! 
//! ## Features
//! 
//! - `wasm_sandbox`: Enables WebAssembly-based plugin sandboxing
//! - `multi_agent`: Enables advanced multi-agent scheduling and coordination
//! - `language_protocol`: Enables specialized language network protocol features

#[cfg(feature = "wasm_sandbox")]
pub mod wasm;

#[cfg(feature = "multi_agent")]
pub mod scheduling;

pub mod agent;
pub mod collections;
pub mod event_log;
pub mod orchestrator;
pub mod plugin_manager;
pub mod storage;
pub mod store;
pub mod types;

// Re-export commonly used types
pub use collections::{Pile, Progression};
pub use event_log::{EventLog, EventRecord, EventStats, EventSummary};
pub use orchestrator::{events, metadata, Orchestrator};
pub use plugin_manager::{PluginManager, PluginManifest};
pub use types::{element::ElementData, traits};

/// Core error types for the agentic system
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Orchestration error: {0}")]
    Orchestration(String),
    #[error("Plugin error: {0}")]
    Plugin(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Agent error: {0}")]
    Agent(String),
}

/// Result type alias for agentic operations
pub type Result<T> = std::result::Result<T, Error>;
