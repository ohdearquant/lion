pub mod agents;
pub mod events;
pub mod plugins;

// Re-export commonly used types
pub use agents::{list_agents, spawn_agent};
pub use events::AppState;
pub use plugins::{invoke_plugin_handler, list_plugins_handler, load_plugin_handler};

pub use crate::events::sse_handler;

#[cfg(test)]
mod tests;
