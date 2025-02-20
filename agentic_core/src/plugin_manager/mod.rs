mod error;
mod loader;
mod manager;
mod manifest;
mod registry;

pub use error::PluginError;
pub use manager::PluginManager;
// Use the new PluginManifest from types
pub use crate::types::plugin::{Plugin, PluginManifest, PluginState};
pub use registry::PluginMetadata;

// Re-export common types that consumers will need
pub type Result<T> = std::result::Result<T, PluginError>;

// Constants
pub const DEFAULT_STORAGE_PATH: &str = "plugins/data";
