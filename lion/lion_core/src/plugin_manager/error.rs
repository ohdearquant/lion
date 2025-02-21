use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(Uuid),
    #[error("Failed to load plugin: {0}")]
    LoadError(String),
    #[error("Failed to invoke plugin: {0}")]
    InvokeError(String),
    #[error("Plugin process error: {0}")]
    ProcessError(String),
    #[error("Failed to read manifest: {0}")]
    ManifestError(String),
}
