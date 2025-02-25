//! Plugin manager for the Lion WebAssembly Plugin System.
//!
//! This crate provides the primary interface for working with WebAssembly plugins.
//! It manages loading, initializing, and executing plugins, as well as
//! coordinating inter-plugin communication.

mod error;
mod loader;
mod manager;
mod manifest;
mod resource_monitor;
mod workflow;

pub use error::PluginManagerError;
pub use loader::PluginLoader;
pub use manager::{PluginManager, PluginManagerConfig};
pub use manifest::ManifestParser;
pub use resource_monitor::ResourceMonitorImpl;
pub use workflow::PluginChain;

use lion_core::capability::CapabilityManagerFactory;
use lion_core::message::MessageBusFactory;
use lion_core::plugin::PluginManagerFactory;
use std::sync::Arc;

/// Factory for creating plugin managers
pub struct PluginManagerFactoryImpl {
    /// Capability manager factory
    capability_factory: Arc<dyn CapabilityManagerFactory>,
    
    /// Message bus factory
    message_bus_factory: Arc<dyn MessageBusFactory>,
}

impl PluginManagerFactoryImpl {
    /// Create a new plugin manager factory
    pub fn new(
        capability_factory: Arc<dyn CapabilityManagerFactory>,
        message_bus_factory: Arc<dyn MessageBusFactory>,
    ) -> Self {
        Self {
            capability_factory,
            message_bus_factory,
        }
    }
}

impl PluginManagerFactory for PluginManagerFactoryImpl {
    fn create_plugin_manager(&self) -> Arc<dyn lion_core::plugin::PluginManager> {
        let capability_manager = self.capability_factory.create_capability_manager();
        let message_bus = self.message_bus_factory.create_message_bus();
        
        Arc::new(PluginManager::new(
            capability_manager,
            message_bus,
            PluginManagerConfig::default(),
        ))
    }
}
