//! # Lion Core
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

pub mod agent;
pub mod collections;
pub mod event_log;
pub mod events;
pub mod orchestrator;
pub mod plugin_manager;
pub mod storage;
pub mod types;

// Re-export commonly used types
pub use collections::{Pile, Progression};
pub use event_log::{EventLog, EventRecord, EventStats, EventSummary};
pub use events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent};
pub use orchestrator::{Orchestrator, OrchestratorConfig};
pub use plugin_manager::{PluginManager, PluginManifest};
pub use storage::Store;
pub use types::{
    Error,
    Metadata,
    ParticipantState,
    Result,
    agent::{
        AgentInfo,
        AgentState,
        AgentStatus,
    },
    plugin::PluginState,
    traits::{
        Describable,
        Identifiable,
        Initializable,
        JsonSerializable,
        LanguageMessage,
        LanguageMessageType,
        LanguageParticipant,
        MetricsProvider,
        Stateful,
        TaskProcessor,
        Toggleable,
        Validatable,
        Versionable,
    },
};

#[cfg(feature = "wasm_sandbox")]
pub mod wasm;

#[cfg(feature = "multi_agent")]
pub mod scheduling;
