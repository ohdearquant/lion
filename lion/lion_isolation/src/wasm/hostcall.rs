//! Host call context.
//! 
//! This module provides a context for host calls.

use std::sync::Arc;

use crate::resource::{ResourceMetering, ResourceLimiter};

/// A context for host calls.
pub struct HostCallContext {
    /// The plugin ID.
    pub plugin_id: String,
    
    /// The resource metering.
    resource_metering: Option<ResourceMetering>,
}

impl HostCallContext {
    /// Create a new host call context.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    ///
    /// # Returns
    ///
    /// A new host call context.
    pub fn new(plugin_id: String) -> Self {
        Self {
            plugin_id,
            resource_metering: None,
        }
    }
    
    /// Get the resource metering.
    pub fn resource_metering(&self) -> Option<&ResourceMetering> {
        self.resource_metering.as_ref()
    }
    
    /// Get a mutable reference to the resource metering.
    pub fn resource_metering_mut(&mut self) -> Option<&mut ResourceMetering> {
        self.resource_metering.as_mut()
    }
    
    /// Set the resource metering.
    pub fn set_resource_metering(&mut self, resource_metering: ResourceMetering) {
        self.resource_metering = Some(resource_metering);
    }
    
    /// Record resource usage.
    ///
    /// # Arguments
    ///
    /// * `cpu_time_us` - The CPU time used, in microseconds.
    /// * `memory_bytes` - The memory used, in bytes.
    pub fn record_resource_usage(&mut self, cpu_time_us: u64, memory_bytes: usize) {
        if let Some(metering) = &mut self.resource_metering {
            metering.record_usage(cpu_time_us, memory_bytes);
        }
    }
    
    /// Check if the resource usage is within limits.
    ///
    /// # Returns
    ///
    /// `true` if the resource usage is within limits, `false` otherwise.
    pub fn is_within_limits(&self) -> bool {
        if let Some(metering) = &self.resource_metering {
            metering.is_within_limits()
        } else {
            true
        }
    }
}