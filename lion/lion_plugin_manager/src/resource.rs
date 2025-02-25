//! Resource monitoring for plugins.

use dashmap::DashMap;
use lion_core::error::ResourceError;
use lion_core::plugin::PluginId;
use lion_core::resource::{ResourceLimits, ResourceMonitor, ResourceUsage};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Implementation of resource monitoring for plugins
pub struct ResourceMonitorImpl {
    /// Resource usage statistics for each plugin
    stats: DashMap<PluginId, Mutex<PluginStats>>,
    
    /// Resource limits for each plugin
    limits: DashMap<PluginId, ResourceLimits>,
}

/// Statistics for a plugin
struct PluginStats {
    /// Memory usage in bytes
    memory_bytes: usize,
    
    /// Peak memory usage in bytes
    peak_memory_bytes: usize,
    
    /// Total execution time
    execution_time: Duration,
    
    /// Last execution start time
    last_execution_start: Option<Instant>,
    
    /// Number of messages processed
    messages_processed: u64,
}

impl Default for PluginStats {
    fn default() -> Self {
        Self {
            memory_bytes: 0,
            peak_memory_bytes: 0,
            execution_time: Duration::from_secs(0),
            last_execution_start: None,
            messages_processed: 0,
        }
    }
}

impl ResourceMonitorImpl {
    /// Create a new resource monitor
    pub fn new() -> Self {
        Self {
            stats: DashMap::new(),
            limits: DashMap::new(),
        }
    }
    
    /// Register a plugin for monitoring
    pub fn register_plugin(&self, plugin_id: PluginId) {
        // Create default stats
        self.stats.insert(plugin_id, Mutex::new(PluginStats::default()));
        
        // Create default limits
        self.limits.insert(plugin_id, ResourceLimits::default());
    }
    
    /// Unregister a plugin
    pub fn unregister_plugin(&self, plugin_id: PluginId) {
        self.stats.remove(&plugin_id);
        self.limits.remove(&plugin_id);
    }
    
    /// Update memory usage for a plugin
    pub fn update_memory_usage(&self, plugin_id: PluginId, bytes: usize) -> Result<(), ResourceError> {
        if let Some(stats) = self.stats.get(&plugin_id) {
            let mut stats = stats.lock().unwrap();
            stats.memory_bytes = bytes;
            stats.peak_memory_bytes = stats.peak_memory_bytes.max(bytes);
            
            // Check if we're over the limit
            if let Some(limits) = self.limits.get(&plugin_id) {
                if bytes > limits.max_memory_bytes {
                    return Err(ResourceError::LimitExceeded(format!(
                        "Memory usage ({} bytes) exceeds limit ({} bytes)",
                        bytes, limits.max_memory_bytes
                    )));
                }
            }
            
            Ok(())
        } else {
            Err(ResourceError::NotAvailable)
        }
    }
    
    /// Start execution for a plugin
    pub fn start_execution(&self, plugin_id: PluginId) {
        if let Some(stats) = self.stats.get(&plugin_id) {
            let mut stats = stats.lock().unwrap();
            stats.last_execution_start = Some(Instant::now());
        }
    }
    
    /// End execution for a plugin
    pub fn end_execution(&self, plugin_id: PluginId) {
        if let Some(stats) = self.stats.get(&plugin_id) {
            let mut stats = stats.lock().unwrap();
            if let Some(start) = stats.last_execution_start {
                let duration = start.elapsed();
                stats.execution_time += duration;
                stats.last_execution_start = None;
                stats.messages_processed += 1;
            }
        }
    }
}

impl ResourceMonitor for ResourceMonitorImpl {
    fn get_usage(&self, plugin_id: PluginId) -> Result<ResourceUsage, ResourceError> {
        if let Some(stats) = self.stats.get(&plugin_id) {
            let stats = stats.lock().unwrap();
            Ok(ResourceUsage {
                memory_bytes: stats.memory_bytes,
                cpu_usage: 0.0, // Not implemented in MVP
                execution_time: stats.execution_time,
                peak_memory_bytes: stats.peak_memory_bytes,
                messages_processed: stats.messages_processed,
            })
        } else {
            Err(ResourceError::NotAvailable)
        }
    }
    
    fn set_limits(&self, plugin_id: PluginId, limits: ResourceLimits) -> Result<(), ResourceError> {
        if self.stats.contains_key(&plugin_id) {
            self.limits.insert(plugin_id, limits);
            Ok(())
        } else {
            Err(ResourceError::NotAvailable)
        }
    }
    
    fn get_limits(&self, plugin_id: PluginId) -> Result<ResourceLimits, ResourceError> {
        if let Some(limits) = self.limits.get(&plugin_id) {
            Ok(limits.clone())
        } else {
            Err(ResourceError::NotAvailable)
        }
    }
    
    fn is_exceeding_limits(&self, plugin_id: PluginId) -> Result<bool, ResourceError> {
        if let (Some(stats), Some(limits)) = (self.stats.get(&plugin_id), self.limits.get(&plugin_id)) {
            let stats = stats.lock().unwrap();
            
            // Check memory limit
            if stats.memory_bytes > limits.max_memory_bytes {
                return Ok(true);
            }
            
            // Check execution time limit (if we're executing)
            if let Some(start) = stats.last_execution_start {
                let duration = start.elapsed();
                if duration > limits.max_execution_time {
                    return Ok(true);
                }
            }
            
            // Check message rate limit
            if let Some(max_rate) = limits.max_messages_per_second {
                // Not implemented in MVP
                // Would need to track message times
            }
            
            Ok(false)
        } else {
            Err(ResourceError::NotAvailable)
        }
    }
    
    fn reset_stats(&self, plugin_id: PluginId) -> Result<(), ResourceError> {
        if let Some(stats) = self.stats.get(&plugin_id) {
            let mut stats = stats.lock().unwrap();
            *stats = PluginStats::default();
            Ok(())
        } else {
            Err(ResourceError::NotAvailable)
        }
    }
}
