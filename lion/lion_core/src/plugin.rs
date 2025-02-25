//! Plugin system definitions for Lion.
//!
//! This module defines the plugin interfaces and management systems that
//! enable loading, executing, and interacting with WebAssembly plugins.

use crate::capability::CoreCapability;
use crate::error::PluginError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Unique identifier for a plugin instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub Uuid);

impl PluginId {
    /// Create a new random plugin ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PluginId {
    fn default() -> Self {
        Self::new()
    }
}

/// The source of a plugin's code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginSource {
    /// A file path to a .wasm or source file
    FilePath(PathBuf),
    
    /// In-memory WebAssembly bytecode
    InMemory(#[serde(with = "serde_bytes")] Vec<u8>),
    
    /// A URL to a WebAssembly module
    Url(String),
}

/// The manifest for a plugin, describing its metadata and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// The name of the plugin
    pub name: String,
    
    /// The version of the plugin
    pub version: String,
    
    /// Optional description
    pub description: Option<String>,
    
    /// Optional author information
    pub author: Option<String>,
    
    /// The source of the plugin code
    pub source: PluginSource,
    
    /// The capabilities requested by this plugin
    pub requested_capabilities: Vec<CoreCapability>,
}

/// The state of a plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// The plugin is created but not initialized
    Created,
    
    /// The plugin is being initialized
    Initializing,
    
    /// The plugin is ready to receive messages
    Ready,
    
    /// The plugin is actively processing a message
    Processing,
    
    /// The plugin is paused
    Paused,
    
    /// The plugin has failed
    Failed,
    
    /// The plugin has been terminated
    Terminated,
}

/// Interface for a plugin
pub trait Plugin: Send + Sync {
    /// Get the ID of this plugin
    fn id(&self) -> PluginId;
    
    /// Get the name of this plugin
    fn name(&self) -> &str;
    
    /// Get the current state of this plugin
    fn state(&self) -> PluginState;
    
    /// Initialize the plugin
    fn initialize(&mut self) -> Result<(), PluginError>;
    
    /// Handle a message
    fn handle_message(&mut self, message: serde_json::Value) -> Result<Option<serde_json::Value>, PluginError>;
    
    /// Shutdown the plugin
    fn shutdown(&mut self) -> Result<(), PluginError>;
    
    /// Clone this plugin as a trait object
    fn clone_box(&self) -> Box<dyn Plugin>;
}

impl Clone for Box<dyn Plugin> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Interface for managing plugins
pub trait PluginManager: Send + Sync {
    /// Load a plugin from a manifest
    fn load_plugin(&self, manifest: &PluginManifest) -> Result<PluginId, PluginError>;
    
    /// Unload a plugin
    fn unload_plugin(&self, plugin_id: PluginId) -> Result<(), PluginError>;
    
    /// Get a reference to a plugin
    fn get_plugin(&self, plugin_id: PluginId) -> Option<Arc<dyn Plugin>>;
    
    /// List all loaded plugins
    fn list_plugins(&self) -> Vec<PluginId>;
    
    /// Get a plugin's manifest
    fn get_manifest(&self, plugin_id: PluginId) -> Option<PluginManifest>;
    
    /// Returns self as Any for downcasting in advanced scenarios
    fn as_any(&self) -> &dyn std::any::Any;
}