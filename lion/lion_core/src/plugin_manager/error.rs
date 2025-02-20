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

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Plugin runtime error: {0}")]
    RuntimeError(String),

    #[error("Plugin initialization error: {0}")]
    InitError(String),
}
