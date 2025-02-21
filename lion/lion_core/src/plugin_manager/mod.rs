pub mod error;
pub mod loader;
pub mod manager;
pub mod manifest;
pub mod registry;

pub use error::{PluginError, Result};
pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use manifest::{LanguageCapabilities, PluginDependency, PluginManifest, SecuritySettings};
pub use registry::{PluginMetadata, PluginRegistry};

// Constants
pub const DEFAULT_STORAGE_PATH: &str = "plugins/data";
