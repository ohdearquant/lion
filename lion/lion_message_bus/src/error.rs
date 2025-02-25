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
    
    #[error("Subscriber already exists for topic")]
    SubscriberAlreadyExists,
    
    #[error("Subscriber not found for topic")]
    SubscriberNotFound,
    
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
    
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
            MessageBusError::SubscriberAlreadyExists => {
                Self::DeliveryFailed("Subscriber already exists".to_string())
            }
            MessageBusError::SubscriberNotFound => {
                Self::DeliveryFailed("Subscriber not found".to_string())
            }
            MessageBusError::InvalidFormat(msg) => Self::FormatError(msg),
            MessageBusError::Internal(msg) => Self::DeliveryFailed(msg),
        }
    }
}