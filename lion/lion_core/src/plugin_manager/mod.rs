pub mod error;
pub mod loader;
pub mod manager;
pub mod manifest;
pub mod registry;

pub use error::{PluginError, Result};
pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use manifest::{PluginManifest, LanguageCapabilities, SecuritySettings, PluginDependency};
pub use registry::{PluginRegistry, PluginMetadata};

// Constants
pub const DEFAULT_STORAGE_PATH: &str = "plugins/data";
