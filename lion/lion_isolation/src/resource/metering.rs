//! Resource metering.
//! 
//! This module provides functionality for metering the resources used by plugins.

use std::sync::Arc;
use std::time::Instant;
use lion_core::error::Result;

use crate::resource::{ResourceLimiter, ResourceUsage};

/// Resource metering.
///
/// Resource metering tracks the resources used by a plugin.
pub struct ResourceMetering {
    /// The resource limiter.
    limiter: Arc<dyn ResourceLimiter>,
    
    /// The current resource usage.
    usage: ResourceUsage,
}

impl ResourceMetering {
    /// Create a new resource metering.
    ///
    /// # Arguments
    ///
    /// * `limiter` - The resource limiter.
    ///
    /// # Returns
    ///
    /// A new resource metering.
    pub fn new(limiter: Arc<dyn ResourceLimiter>) -> Self {
        Self {
            limiter,
            usage: ResourceUsage::default(),
        }
    }
    
    /// Get the current resource usage.
    pub fn usage(&self) -> &ResourceUsage {
        &self.usage
    }
    
    /// Reset the function start time.
    pub fn reset_function_start_time(&mut self) {
        self.usage.function_start_time = Instant::now();
    }
    
    /// Record resource usage.
    ///
    /// # Arguments
    ///
    /// * `cpu_time_us` - The CPU time used, in microseconds.
    /// * `memory_bytes` - The memory used, in bytes.
    pub fn record_usage(&mut self, cpu_time_us: u64, memory_bytes: usize) {
        self.usage.cpu_time_us += cpu_time_us;
        self.usage.memory_bytes = self.usage.memory_bytes.max(memory_bytes);
    }
    
    /// Increment the function call count.
    pub fn increment_function_calls(&mut self) {
        self.usage.function_calls += 1;
    }
    
    /// Check if the resource usage is within limits.
    ///
    /// # Returns
    ///
    /// `true` if the resource usage is within limits, `false` otherwise.
    pub fn is_within_limits(&self) -> bool {
        self.limiter.check_limits(&self.usage).is_ok()
    }
    
    /// Check if the resource usage is within limits and return an error if not.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the resource usage is within limits.
    /// * `Err` - If the resource usage exceeds limits.
    pub fn check_limits(&self) -> Result<()> {
        self.limiter.check_limits(&self.usage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::DefaultResourceLimiter;
    
    #[test]
    fn test_record_usage() {
        let limiter = Arc::new(DefaultResourceLimiter::default());
        let mut metering = ResourceMetering::new(limiter);
        
        // Record usage
        metering.record_usage(100, 100);
        
        // Check usage
        assert_eq!(metering.usage().cpu_time_us, 100);
        assert_eq!(metering.usage().memory_bytes, 100);
        
        // Record more usage
        metering.record_usage(100, 200);
        
        // Check usage
        assert_eq!(metering.usage().cpu_time_us, 200);
        assert_eq!(metering.usage().memory_bytes, 200);
        
        // Record usage with smaller memory
        metering.record_usage(100, 150);
        
        // Check usage
        assert_eq!(metering.usage().cpu_time_us, 300);
        assert_eq!(metering.usage().memory_bytes, 200);
    }
    
    #[test]
    fn test_increment_function_calls() {
        let limiter = Arc::new(DefaultResourceLimiter::default());
        let mut metering = ResourceMetering::new(limiter);
        
        // Check initial function calls
        assert_eq!(metering.usage().function_calls, 0);
        
        // Increment function calls
        metering.increment_function_calls();
        
        // Check function calls
        assert_eq!(metering.usage().function_calls, 1);
    }
    
    #[test]
    fn test_reset_function_start_time() {
        let limiter = Arc::new(DefaultResourceLimiter::default());
        let mut metering = ResourceMetering::new(limiter);
        
        // Store the initial function start time
        let initial_time = metering.usage().function_start_time;
        
        // Sleep a bit
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        // Reset the function start time
        metering.reset_function_start_time();
        
        // Check that the function start time changed
        assert!(metering.usage().function_start_time > initial_time);
    }
    
    #[test]
    fn test_is_within_limits() {
        let limiter = Arc::new(DefaultResourceLimiter::new(
            Some(100),
            Some(100),
            Some(100),
        ));
        let mut metering = ResourceMetering::new(limiter);
        
        // Check that we're within limits
        assert!(metering.is_within_limits());
        
        // Record usage within limits
        metering.record_usage(50, 50);
        
        // Check that we're still within limits
        assert!(metering.is_within_limits());
        
        // Record usage exceeding memory limit
        metering.record_usage(0, 150);
        
        // Check that we're no longer within limits
        assert!(!metering.is_within_limits());
    }
}