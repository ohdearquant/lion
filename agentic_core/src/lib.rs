pub mod element;
pub mod event_log;
pub mod orchestrator;
pub mod pile;
pub mod plugin_manager;
pub mod progression;
pub mod store;

// Re-export commonly used types
pub use element::ElementData;
pub use event_log::{EventLog, EventRecord};
pub use orchestrator::{Orchestrator, SystemEvent};
pub use pile::Pile;
pub use plugin_manager::{PluginError, PluginManager, PluginManifest};
pub use progression::Progression;
pub use store::InMemoryStore;
