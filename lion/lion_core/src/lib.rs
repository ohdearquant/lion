pub mod agent;
pub mod element;
pub mod event_log;
pub mod orchestrator;
pub mod pile;
pub mod plugin_manager;
pub mod progression;
pub mod storage;

// Re-export commonly used types
pub use agent::{AgentEvent, AgentProtocol, MockStreamingAgent};
pub use element::ElementData;
pub use event_log::{EventLog, EventRecord};
pub use orchestrator::{Orchestrator, SystemEvent};
pub use pile::Pile;
pub use plugin_manager::{PluginError, PluginFunction, PluginManager, PluginManifest};
pub use progression::Progression;
pub use storage::InMemoryStore;
