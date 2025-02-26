//! Core traits that define the Lion architecture.
//!
//! This module contains the fundamental interfaces that each component
//! must implement. These interfaces are designed for stability and
//! clear separation of concerns.

use std::sync::Arc;

use crate::error::{Result, PluginError};
use crate::types::{PluginId, PluginConfig, PluginMetadata, PluginState, ResourceUsage};

/// Core trait for plugin lifecycle management.
pub trait PluginManager: Send + Sync {
    /// Load a plugin from binary code.
    fn load_plugin(
        &self,
        name: &str,
        version: &str,
        description: &str, 
        plugin_type: crate::types::PluginType,
        code: Vec<u8>,
        config: PluginConfig,
    ) -> Result<PluginId>;
    
    /// Call a function in a plugin.
    fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>>;
    
    /// Get metadata for a loaded plugin.
    fn get_metadata(&self, plugin_id: &PluginId) -> Option<PluginMetadata>;
    
    /// Unload a plugin.
    fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()>;
    
    /// Pause a plugin.
    fn pause_plugin(&self, plugin_id: &PluginId) -> Result<()>;
    
    /// Resume a plugin.
    fn resume_plugin(&self, plugin_id: &PluginId) -> Result<()>;
    
    /// Get a list of all loaded plugins.
    fn list_plugins(&self) -> Vec<PluginMetadata>;
    
    /// Get resource usage for a plugin.
    fn get_resource_usage(&self, plugin_id: &PluginId) -> Result<ResourceUsage>;
    
    /// Get the current state of a plugin.
    fn get_plugin_state(&self, plugin_id: &PluginId) -> Result<PluginState>;
}

/// Core trait for plugin isolation.
pub trait IsolationBackend: Send + Sync {
    /// Load a plugin.
    fn load_plugin(
        &self,
        plugin_id: PluginId,
        code: Vec<u8>,
        config: PluginConfig,
    ) -> Result<()>;
    
    /// Call a function in a plugin.
    fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>>;
    
    /// Unload a plugin.
    fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()>;
    
    /// List available functions for a plugin.
    fn list_functions(&self, plugin_id: &PluginId) -> Result<Vec<String>>;
    
    /// Get resource usage for a plugin.
    fn get_resource_usage(&self, plugin_id: &PluginId) -> Result<ResourceUsage>;
}

/// Factory for creating isolation backends.
pub trait IsolationBackendFactory: Send + Sync {
    /// Create a new isolation backend.
    fn create_backend(&self) -> Result<Arc<dyn IsolationBackend>>;
}

/// Basic capability interface.
pub trait Capability: Send + Sync {
    /// Get the capability type as a string.
    fn get_type(&self) -> &str;
    
    /// Check if this capability includes another.
    fn includes(&self, other: &dyn Capability) -> bool;
}

/// Message bus for inter-plugin communication.
pub trait MessageBus: Send + Sync {
    /// Send a message.
    fn send_message(&self, message: crate::types::Message) -> Result<()>;
    
    /// Receive a message for a plugin.
    fn receive_message(&self, plugin_id: &PluginId) -> Result<Option<crate::types::Message>>;
    
    /// Get the number of pending messages for a plugin.
    fn pending_message_count(&self, plugin_id: &PluginId) -> Result<usize>;
}