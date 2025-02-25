//! Error types for the capability manager.

use thiserror::Error;

/// Errors that can occur in the capability manager
#[derive(Error, Debug)]
pub enum CapabilityManagerError {
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
    
    #[error("Capability already granted")]
    CapabilityAlreadyGranted,
    
    #[error("Capability not granted")]
    CapabilityNotGranted,
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Capability policy error: {0}")]
    PolicyError(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<CapabilityManagerError> for lion_core::error::CapabilityError {
    fn from(err: CapabilityManagerError) -> Self {
        match err {
            CapabilityManagerError::PluginNotFound(plugin_id) => {
                Self::UnknownPlugin(plugin_id)
            }
            CapabilityManagerError::CapabilityAlreadyGranted => Self::AlreadyGranted,
            CapabilityManagerError::CapabilityNotGranted => Self::NotGranted,
            CapabilityManagerError::PermissionDenied(msg) => Self::PermissionDenied,
            CapabilityManagerError::PolicyError(msg) => {
                Self::OperationFailed(format!("Policy error: {}", msg))
            }
            CapabilityManagerError::Internal(msg) => {
                Self::OperationFailed(format!("Internal error: {}", msg))
            }
        }
    }
}