//! Resource monitoring for plugins.
//!
//! This module defines interfaces for monitoring and limiting resource usage
//! by plugins, such as memory and CPU time.

use crate::error::ResourceError;
use crate::plugin::PluginId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Resource usage statistics for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in bytes
    pub memory_bytes: usize,
    
    /// Estimated CPU usage (0.0 to 1.0)
    pub cpu_usage: f64,
    
    /// Total execution time
    pub execution_time: Duration,
    
    /// Peak memory usage in bytes
    pub peak_memory_bytes: usize,
    
    /// Number of messages processed
    pub messages_processed: u64,
}

/// Resource limits for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    
    /// Maximum execution time per message
    pub max_execution_time: Duration,
    
    /// Maximum messages per second
    pub max_messages_per_second: Option<u32>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 100 * 1024 * 1024, // 100 MB
            max_execution_time: Duration::from_secs(5),
            max_messages_per_second: None,
        }
    }
}

/// Interface for monitoring resource usage
pub trait ResourceMonitor: Send + Sync {
    /// Get the current resource usage for a plugin
    fn get_usage(&self, plugin_id: PluginId) -> Result<ResourceUsage, ResourceError>;
    
    /// Set resource limits for a plugin
    fn set_limits(&self, plugin_id: PluginId, limits: ResourceLimits) -> Result<(), ResourceError>;
    
    /// Get the current resource limits for a plugin
    fn get_limits(&self, plugin_id: PluginId) -> Result<ResourceLimits, ResourceError>;
    
    /// Check if a plugin is exceeding its resource limits
    fn is_exceeding_limits(&self, plugin_id: PluginId) -> Result<bool, ResourceError>;
    
    /// Reset the resource usage statistics for a plugin
    fn reset_stats(&self, plugin_id: PluginId) -> Result<(), ResourceError>;
    
    /// Register a new plugin for monitoring
    fn register_plugin(&self, plugin_id: PluginId) -> Result<(), ResourceError>;
    
    /// Unregister a plugin from monitoring
    fn unregister_plugin(&self, plugin_id: PluginId) -> Result<(), ResourceError>;
    
    /// Returns self as Any for downcasting in advanced scenarios
    fn as_any(&self) -> &dyn std::any::Any;
}