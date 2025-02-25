//! Configuration for the message bus.

/// Common configuration for all message bus implementations
#[derive(Debug, Clone)]
pub struct MessageBusConfig {
    /// Maximum message size in bytes
    pub max_message_size: usize,
    
    /// Whether to check capabilities before message delivery
    pub check_capabilities: bool,
    
    /// Whether to validate message format
    pub validate_messages: bool,
    
    /// Whether to retain message history
    pub retain_message_history: bool,
    
    /// Maximum number of retained messages per topic
    pub max_retained_messages: usize,
}

impl Default for MessageBusConfig {
    fn default() -> Self {
        Self {
            max_message_size: 1024 * 1024, // 1 MB
            check_capabilities: true,
            validate_messages: true,
            retain_message_history: false,
            max_retained_messages: 100,
        }
    }
}