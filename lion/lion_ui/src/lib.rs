pub mod agents;
pub mod events;
pub mod plugins;
pub mod state;

pub use agents::{AgentInfo, ApiResponse, SpawnAgentRequest};
pub use plugins::LoadPluginRequest;
pub use lion_core::types::plugin::PluginResponse;
pub use state::{AppState, PluginInfo};

// Re-export handlers for use in main.rs
pub use agents::{list_agents, spawn_agent};
pub use events::sse_handler;
pub use plugins::{invoke_plugin_handler, list_plugins_handler, load_plugin_handler};
