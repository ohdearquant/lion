use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur in plugin operations
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Plugin not found: {0}")]
    NotFound(Uuid),

    #[error("Failed to load plugin: {0}")]
    LoadError(String),

    #[error("Failed to invoke plugin: {0}")]
    InvokeError(String),

    #[error("Plugin initialization failed: {0}")]
    InitializationError(String),

    #[error("Plugin validation failed: {0}")]
    ValidationError(String),

    #[error("Plugin security error: {0}")]
    SecurityError(String),

    #[error("Plugin dependency error: {0}")]
    DependencyError(String),

    #[error("Plugin state error: {0}")]
    StateError(String),
}

/// Result type for plugin operations
pub type Result<T> = std::result::Result<T, PluginError>;
