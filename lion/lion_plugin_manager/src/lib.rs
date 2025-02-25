//! # Lion Plugin Manager
//!
//! This crate provides the plugin management system for the Lion WebAssembly Plugin System.
//! It handles loading, initializing, and executing plugins, as well as coordinating
//! inter-plugin communication through plugin chains.
//!
//! ## Features
//!
//! - Plugin lifecycle management (loading, initializing, executing, unloading)
//! - Plugin manifest parsing and validation
//! - Resource usage monitoring
//! - Plugin workflow/chain orchestration
//! - Integration with the capability system and isolation backends

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

use lion_core::capability::CapabilityManager;
use lion_core::isolation::IsolationBackend;
use lion_core::message::MessageBus;
use lion_core::resource::ResourceMonitor;
use std::sync::Arc;

/// Create a new plugin manager with default configuration
pub fn create_plugin_manager(
    capability_manager: Arc<dyn CapabilityManager>,
    message_bus: Arc<dyn MessageBus>,
    isolation_backend: Arc<dyn IsolationBackend>,
    resource_monitor: Arc<dyn ResourceMonitor>,
) -> Arc<PluginManager> {
    Arc::new(PluginManager::new(
        capability_manager,
        message_bus,
        isolation_backend,
        resource_monitor,
        PluginManagerConfig::default(),
    ))
}