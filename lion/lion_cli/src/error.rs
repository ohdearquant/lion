//! Error types for the Lion CLI.

use thiserror::Error;

/// Errors that can occur in the CLI
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Plugin error: {0}")]
    Plugin(#[from] lion_core::error::PluginError),
    
    #[error("Capability error: {0}")]
    Capability(#[from] lion_core::error::CapabilityError),
    
    #[error("Message error: {0}")]
    Message(#[from] lion_core::error::MessageError),
    
    #[error("Resource error: {0}")]
    Resource(#[from] lion_core::error::ResourceError),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("Invalid plugin ID: {0}")]
    InvalidPluginId(String),
    
    #[error("Invalid command arguments: {0}")]
    InvalidArguments(String),
    
    #[error("System initialization error: {0}")]
    SystemInitialization(String),
    
    #[error("Demo error: {0}")]
    Demo(String),
    
    #[error("Chain error: {0}")]
    Chain(String),
    
    #[error("Unknown demo: {0}")]
    UnknownDemo(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}