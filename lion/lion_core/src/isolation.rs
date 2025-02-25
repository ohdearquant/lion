//! Isolation backend abstraction for plugin execution.
//!
//! This module defines the `IsolationBackend` trait that provides a common
//! interface for different isolation strategies (e.g., WebAssembly, process, container).
//! This abstraction allows the plugin manager to remain agnostic about the specific
//! isolation technology used.

use crate::error::IsolationError;
use crate::plugin::{Plugin, PluginId, PluginManifest};
use std::sync::Arc;

/// Interface for isolation backends that execute plugin code
pub trait IsolationBackend: Send + Sync {
    /// Load a plugin from a manifest
    fn load_plugin(&self, manifest: &PluginManifest) -> Result<PluginId, IsolationError>;
    
    /// Unload a plugin
    fn unload_plugin(&self, plugin_id: PluginId) -> Result<(), IsolationError>;
    
    /// Initialize a plugin
    fn initialize_plugin(&self, plugin_id: PluginId) -> Result<(), IsolationError>;
    
    /// Handle a message for a plugin
    fn handle_message(
        &self,
        plugin_id: PluginId,
        message: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, IsolationError>;
    
    /// Get a plugin instance
    fn get_plugin(&self, plugin_id: PluginId) -> Option<Arc<dyn Plugin>>;
    
    /// Get a plugin's manifest
    fn get_manifest(&self, plugin_id: PluginId) -> Option<PluginManifest>;
    
    /// List all loaded plugins
    fn list_plugins(&self) -> Vec<PluginId>;
    
    /// Get memory usage for a plugin in bytes
    fn get_memory_usage(&self, plugin_id: PluginId) -> Result<usize, IsolationError>;
    
    /// Get execution time for a plugin
    fn get_execution_time(&self, plugin_id: PluginId) -> Result<std::time::Duration, IsolationError>;
    
    /// Returns self as Any for downcasting in advanced scenarios
    fn as_any(&self) -> &dyn std::any::Any;
}