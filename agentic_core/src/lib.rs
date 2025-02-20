pub mod agent;
pub mod element;
pub mod event_log;
pub mod orchestrator;
pub mod pile;
pub mod plugin_manager;
pub mod progression;
pub mod storage;
pub mod store;

// Re-export commonly used types
pub use event_log::{EventLog, EventRecord, EventStats, EventSummary};
pub use orchestrator::{events, metadata, Orchestrator};
pub use plugin_manager::{PluginManager, PluginManifest};
