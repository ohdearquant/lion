//! Message bus implementation for the Lion WebAssembly Plugin System.
//!
//! This crate provides a publish-subscribe message bus for inter-plugin communication.

mod error;
mod in_memory;

pub use error::MessageBusError;
pub use in_memory::{InMemoryMessageBus, InMemoryMessageBusConfig};

use lion_core::capability::CapabilityManager;
use lion_core::message::MessageBus;
use std::sync::Arc;

/// Factory for creating message buses
pub struct MessageBusFactory {
    /// Capability manager for checking capabilities
    capability_manager: Arc<dyn CapabilityManager>,
}

impl MessageBusFactory {
    /// Create a new message bus factory
    pub fn new(capability_manager: Arc<dyn CapabilityManager>) -> Self {
        Self { capability_manager }
    }
    
    /// Create a new in-memory message bus
    pub fn create_in_memory(&self, config: Option<InMemoryMessageBusConfig>) -> Arc<dyn MessageBus> {
        Arc::new(InMemoryMessageBus::new(
            self.capability_manager.clone(),
            config.unwrap_or_default(),
        ))
    }
}

impl lion_core::message::MessageBusFactory for MessageBusFactory {
    fn create_message_bus(&self) -> Arc<dyn MessageBus> {
        self.create_in_memory(None)
    }
}
