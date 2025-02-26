//! Resource usage.
//!
//! This module provides a resource usage tracker.

use std::collections::HashMap;
use std::time::Instant;

/// Resource usage.
///
/// Resource usage tracks the resources used by a plugin.
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// The memory usage, in bytes.
    pub memory_bytes: usize,

    /// The CPU time, in microseconds.
    pub cpu_time_us: u64,

    /// The number of function calls.
    pub function_calls: u64,

    /// When the current function started.
    pub function_start_time: Instant,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            memory_bytes: 0,
            cpu_time_us: 0,
            function_calls: 0,
            function_start_time: Instant::now(),
        }
    }
}

impl ResourceUsage {
    /// Convert to a lion_core resource usage.
    pub fn to_core_resource_usage(&self) -> lion_core::types::ResourceUsage {
        lion_core::ResourceUsage {
            memory_bytes: self.memory_bytes,
            cpu_time_us: self.cpu_time_us,
            function_calls: self.function_calls,
            last_updated: chrono::Utc::now(),
            active_instances: 1,
            peak_memory_bytes: self.memory_bytes,
            peak_cpu_time_us: self.cpu_time_us,
            avg_function_call_us: if self.function_calls > 0 {
                self.cpu_time_us / self.function_calls
            } else {
                0
            },
            custom_metrics: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let usage = ResourceUsage::default();

        assert_eq!(usage.memory_bytes, 0);
        assert_eq!(usage.cpu_time_us, 0);
        assert_eq!(usage.function_calls, 0);
    }

    #[test]
    fn test_to_core_resource_usage() {
        let usage = ResourceUsage {
            memory_bytes: 100,
            cpu_time_us: 200,
            function_calls: 3,
            function_start_time: Instant::now(),
        };

        let core_usage = usage.to_core_resource_usage();

        assert_eq!(core_usage.memory_bytes, 100);
        assert_eq!(core_usage.cpu_time_us, 200);
        assert_eq!(core_usage.function_calls, 3);
    }
}
