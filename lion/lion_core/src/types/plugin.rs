//! Plugin-related data types.
//! 
//! This module defines data structures for plugin configuration, metadata,
//! state, and resource usage.

use std::fmt;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::id::PluginId;

/// Plugin type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginType {
    /// WebAssembly plugin.
    Wasm,
    
    /// Native plugin (shared library).
    Native,
    
    /// JavaScript plugin.
    JavaScript,
    
    /// Remote plugin (in another process or node).
    Remote,
}

impl fmt::Display for PluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wasm => write!(f, "WebAssembly"),
            Self::Native => write!(f, "Native"),
            Self::JavaScript => write!(f, "JavaScript"),
            Self::Remote => write!(f, "Remote"),
        }
    }
}

/// Plugin state in the lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is created but not yet initialized.
    Created,
    
    /// Plugin is initialized and ready to run.
    Ready,
    
    /// Plugin is actively running.
    Running,
    
    /// Plugin is paused.
    Paused,
    
    /// Plugin has failed.
    Failed,
    
    /// Plugin has been terminated.
    Terminated,
    
    /// Plugin is upgrading (hot reload).
    Upgrading,
}

impl fmt::Display for PluginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Ready => write!(f, "Ready"),
            Self::Running => write!(f, "Running"),
            Self::Paused => write!(f, "Paused"),
            Self::Failed => write!(f, "Failed"),
            Self::Terminated => write!(f, "Terminated"),
            Self::Upgrading => write!(f, "Upgrading"),
        }
    }
}

/// Plugin configuration with resource limits.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Maximum memory usage in bytes.
    pub max_memory_bytes: Option<usize>,
    
    /// Maximum CPU time in microseconds.
    pub max_cpu_time_us: Option<u64>,
    
    /// Function call timeout in milliseconds.
    pub function_timeout_ms: Option<u64>,
    
    /// Additional configuration options.
    pub options: serde_json::Value,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(100 * 1024 * 1024), // 100 MB
            max_cpu_time_us: Some(10 * 1000 * 1000),   // 10 seconds
            function_timeout_ms: Some(5000),           // 5 seconds
            options: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

/// Basic plugin metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin ID.
    pub id: PluginId,
    
    /// Human-readable name.
    pub name: String,
    
    /// Version string.
    pub version: String,
    
    /// Description of the plugin.
    pub description: String,
    
    /// Plugin type.
    pub plugin_type: PluginType,
    
    /// Current state.
    pub state: PluginState,
    
    /// When the plugin was created.
    pub created_at: DateTime<Utc>,
    
    /// When the plugin was last updated.
    pub updated_at: DateTime<Utc>,
    
    /// Available functions.
    pub functions: Vec<String>,
}

/// Resource usage statistics for a plugin.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in bytes.
    pub memory_bytes: usize,
    
    /// CPU time used in microseconds.
    pub cpu_time_us: u64,
    
    /// Number of function calls executed.
    pub function_calls: u64,
    
    /// When resource usage was last updated.
    pub last_updated: DateTime<Utc>,
}