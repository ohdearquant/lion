//! # Lion Message Bus
//!
//! This crate provides a message bus implementation for the Lion WebAssembly Plugin System,
//! enabling inter-plugin communication through direct messages and publish-subscribe patterns.
//!
//! ## Features
//!
//! - In-memory message passing between plugins
//! - Topic-based publish-subscribe messaging
//! - Direct plugin-to-plugin communication
//! - Integration with capability-based security
//! - Thread-safe message queues for concurrent access

mod error;
mod in_memory;
mod topic;
mod message_queue;
mod config;

pub use error::MessageBusError;
pub use in_memory::{InMemoryMessageBus, InMemoryMessageBusConfig};
pub use config::MessageBusConfig;

use lion_core::capability::CapabilityManager;
use std::sync::Arc;

/// Create a new in-memory message bus with the default configuration
pub fn create_default_message_bus() -> Arc<dyn lion_core::message::MessageBus> {
    Arc::new(in_memory::InMemoryMessageBus::new(
        in_memory::InMemoryMessageBusConfig::default(),
        None,
    ))
}

/// Create a new in-memory message bus with capability checking
pub fn create_message_bus_with_capability_manager(
    capability_manager: Arc<dyn CapabilityManager>,
) -> Arc<dyn lion_core::message::MessageBus> {
    Arc::new(in_memory::InMemoryMessageBus::new(
        in_memory::InMemoryMessageBusConfig::default(),
        Some(capability_manager),
    ))
}