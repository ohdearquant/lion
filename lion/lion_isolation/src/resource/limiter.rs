//! Resource limiter.
//!
//! This module provides functionality for limiting the resources used by plugins.

use lion_core::error::{IsolationError, Result};

use crate::resource::usage::ResourceUsage;

/// A resource limiter.
///
/// A resource limiter enforces limits on the resources used by plugins.
pub trait ResourceLimiter: Send + Sync {
    /// Check if a plugin is within resource limits.
    ///
    /// # Arguments
    ///
    /// * `usage` - The current resource usage.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the plugin is within limits.
    /// * `Err` - If the plugin has exceeded its limits.
    fn check_limits(&self, usage: &ResourceUsage) -> Result<()>;

    /// The memory limit, in bytes.
    fn memory_limit(&self) -> Option<usize>;

    /// The CPU time limit, in microseconds.
    fn cpu_time_limit(&self) -> Option<u64>;

    /// The function timeout, in milliseconds.
    fn function_timeout(&self) -> Option<u64>;
}

/// A default resource limiter.
pub struct DefaultResourceLimiter {
    /// The memory limit, in bytes.
    pub memory_limit: Option<usize>,

    /// The CPU time limit, in microseconds.
    pub cpu_time_limit: Option<u64>,

    /// The function timeout, in milliseconds.
    pub function_timeout: Option<u64>,
}

impl DefaultResourceLimiter {
    /// Create a new default resource limiter.
    ///
    /// # Arguments
    ///
    /// * `memory_limit` - The memory limit, in bytes.
    /// * `cpu_time_limit` - The CPU time limit, in microseconds.
    /// * `function_timeout` - The function timeout, in milliseconds.
    ///
    /// # Returns
    ///
    /// A new default resource limiter.
    pub fn new(
        memory_limit: Option<usize>,
        cpu_time_limit: Option<u64>,
        function_timeout: Option<u64>,
    ) -> Self {
        Self {
            memory_limit,
            cpu_time_limit,
            function_timeout,
        }
    }
}

impl Default for DefaultResourceLimiter {
    fn default() -> Self {
        Self {
            memory_limit: Some(100 * 1024 * 1024),  // 100 MB
            cpu_time_limit: Some(10 * 1000 * 1000), // 10 seconds
            function_timeout: Some(5000),           // 5 seconds
        }
    }
}

impl ResourceLimiter for DefaultResourceLimiter {
    fn check_limits(&self, usage: &ResourceUsage) -> Result<()> {
        // Check memory usage
        if let Some(limit) = self.memory_limit {
            if usage.memory_bytes > limit {
                return Err(IsolationError::ResourceExhausted(format!(
                    "Memory usage ({} bytes) exceeds limit ({} bytes)",
                    usage.memory_bytes, limit
                ))
                .into());
            }
        }

        // Check CPU time
        if let Some(limit) = self.cpu_time_limit {
            if usage.cpu_time_us > limit {
                return Err(IsolationError::ResourceExhausted(format!(
                    "CPU time ({} us) exceeds limit ({} us)",
                    usage.cpu_time_us, limit
                ))
                .into());
            }
        }

        // Check function timeout
        if let Some(limit) = self.function_timeout {
            let elapsed = usage.function_start_time.elapsed();
            let elapsed_ms = elapsed.as_millis() as u64;

            if elapsed_ms > limit {
                return Err(IsolationError::ResourceExhausted(format!(
                    "Function timeout ({} ms) exceeded",
                    limit
                ))
                .into());
            }
        }

        Ok(())
    }

    fn memory_limit(&self) -> Option<usize> {
        self.memory_limit
    }

    fn cpu_time_limit(&self) -> Option<u64> {
        self.cpu_time_limit
    }

    fn function_timeout(&self) -> Option<u64> {
        self.function_timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::usage::ResourceUsage;
    use std::time::{Duration, Instant};

    #[test]
    fn test_default_resource_limiter() {
        let limiter = DefaultResourceLimiter::default();

        // Check default limits
        assert_eq!(limiter.memory_limit, Some(100 * 1024 * 1024));
        assert_eq!(limiter.cpu_time_limit, Some(10 * 1000 * 1000));
        assert_eq!(limiter.function_timeout, Some(5000));
    }

    #[test]
    fn test_check_limits() {
        let limiter = DefaultResourceLimiter::new(Some(100), Some(100), Some(100));

        // Create resource usage within limits
        let usage = ResourceUsage {
            memory_bytes: 50,
            cpu_time_us: 50,
            function_calls: 1,
            function_start_time: Instant::now(),
        };

        // Check limits
        assert!(limiter.check_limits(&usage).is_ok());

        // Create resource usage exceeding memory limit
        let usage = ResourceUsage {
            memory_bytes: 200,
            cpu_time_us: 50,
            function_calls: 1,
            function_start_time: Instant::now(),
        };

        // Check limits
        assert!(limiter.check_limits(&usage).is_err());

        // Create resource usage exceeding CPU time limit
        let usage = ResourceUsage {
            memory_bytes: 50,
            cpu_time_us: 200,
            function_calls: 1,
            function_start_time: Instant::now(),
        };

        // Check limits
        assert!(limiter.check_limits(&usage).is_err());

        // Create resource usage exceeding function timeout
        let usage = ResourceUsage {
            memory_bytes: 50,
            cpu_time_us: 50,
            function_calls: 1,
            function_start_time: Instant::now() - Duration::from_millis(200),
        };

        // Check limits
        assert!(limiter.check_limits(&usage).is_err());
    }
}
