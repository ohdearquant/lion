//! Error types for the message bus.

use thiserror::Error;

/// Errors that can occur in the message bus
#[derive(Error, Debug)]
pub enum MessageBusError {
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
    
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Message too large: {0} bytes")]
    MessageTooLarge(usize),
    
    #[error("Message bus is full")]
    BusFull,
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<MessageBusError> for lion_core::error::MessageError {
    fn from(err: MessageBusError) -> Self {
        match err {
            MessageBusError::PluginNotFound(_) => Self::NoSuchPlugin,
            MessageBusError::TopicNotFound(_) => Self::NoSuchTopic,
            MessageBusError::PermissionDenied(_) => Self::PermissionDenied,
            MessageBusError::MessageTooLarge(size) => {
                Self::FormatError(format!("Message too large: {} bytes", size))
            }
            MessageBusError::BusFull => Self::BusFull,
            MessageBusError::Internal(msg) => Self::DeliveryFailed(msg),
        }
    }
}
