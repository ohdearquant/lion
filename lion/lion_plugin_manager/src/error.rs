//! Error types for the plugin manager.

use thiserror::Error;

/// Errors that can occur in the plugin manager
#[derive(Error, Debug)]
pub enum PluginManagerError {
    #[error("Failed to load plugin: {0}")]
    LoadFailure(String),
    
    #[error("Failed to initialize plugin: {0}")]
    InitializationFailure(String),
    
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
    
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    
    #[error("Capability error: {0}")]
    CapabilityError(String),
    
    #[error("Messaging error: {0}")]
    MessagingError(String),
    
    #[error("I/O error: {0}")]
    IoError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Isolation error: {0}")]
    IsolationError(String),
    
    #[error("Resource error: {0}")]
    ResourceError(String),
    
    #[error("Chain execution error: {0}")]
    ChainError(String),
    
    #[error("Workflow error: {0}")]
    WorkflowError(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<PluginManagerError> for lion_core::error::PluginError {
    fn from(err: PluginManagerError) -> Self {
        match err {
            PluginManagerError::LoadFailure(msg) => Self::LoadFailure(msg),
            PluginManagerError::InitializationFailure(msg) => Self::InitializationFailure(msg),
            PluginManagerError::PluginNotFound(id) => Self::NotFound(id),
            PluginManagerError::InvalidManifest(msg) => Self::InvalidManifest(msg),
            PluginManagerError::CapabilityError(msg) => {
                Self::ExecutionError(format!("Capability error: {}", msg))
            }
            PluginManagerError::MessagingError(msg) => {
                Self::ExecutionError(format!("Messaging error: {}", msg))
            }
            PluginManagerError::IoError(msg) => Self::LoadFailure(format!("I/O error: {}", msg)),
            PluginManagerError::ParseError(msg) => {
                Self::InvalidManifest(format!("Parse error: {}", msg))
            }
            PluginManagerError::IsolationError(msg) => Self::ExecutionError(msg),
            PluginManagerError::ResourceError(msg) => Self::ExecutionError(msg),
            PluginManagerError::ChainError(msg) => Self::ExecutionError(msg),
            PluginManagerError::WorkflowError(msg) => Self::ExecutionError(msg),
            PluginManagerError::Internal(msg) => Self::ExecutionError(msg),
        }
    }
}

impl From<std::io::Error> for PluginManagerError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<toml::de::Error> for PluginManagerError {
    fn from(err: toml::de::Error) -> Self {
        Self::ParseError(err.to_string())
    }
}

impl From<lion_core::error::CapabilityError> for PluginManagerError {
    fn from(err: lion_core::error::CapabilityError) -> Self {
        Self::CapabilityError(err.to_string())
    }
}

impl From<lion_core::error::MessageError> for PluginManagerError {
    fn from(err: lion_core::error::MessageError) -> Self {
        Self::MessagingError(err.to_string())
    }
}

impl From<lion_core::error::IsolationError> for PluginManagerError {
    fn from(err: lion_core::error::IsolationError) -> Self {
        Self::IsolationError(err.to_string())
    }
}

impl From<lion_core::error::ResourceError> for PluginManagerError {
    fn from(err: lion_core::error::ResourceError) -> Self {
        Self::ResourceError(err.to_string())
    }
}