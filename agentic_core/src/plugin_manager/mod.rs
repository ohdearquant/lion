mod error;
mod loader;
mod manager;
mod manifest;
mod registry;

pub use error::PluginError;
pub use manager::PluginManager;
pub use manifest::PluginManifest;
pub use registry::{PluginMetadata, PluginRegistry, PluginState};

// Re-export common types that consumers will need
pub type Result<T> = std::result::Result<T, PluginError>;

// Constants
pub const DEFAULT_STORAGE_PATH: &str = "plugins/data";
