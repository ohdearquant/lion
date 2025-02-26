//! Concurrency trait definitions.
//! 
//! This module defines the core traits for the concurrency system.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::error::{Result, ConcurrencyError};
use crate::id::PluginId;

/// Core trait for concurrency management.
///
/// This trait provides an interface for scheduling tasks and managing
/// concurrent execution.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use std::time::Duration;
/// use lion_core::traits::ConcurrencyManager;
/// use lion_core::error::{Result, ConcurrencyError};
/// use lion_core::id::PluginId;
///
/// struct SimpleConcurrencyManager;
///
/// impl ConcurrencyManager for SimpleConcurrencyManager {
///     fn schedule_task(&self, task: Box<dyn FnOnce() + Send + 'static>) -> Result<(), ConcurrencyError> {
///         // In a real implementation, we would use a thread pool or async runtime
///         std::thread::spawn(move || task());
///         Ok(())
///     }
///
///     fn call_function(&self, plugin_id: &PluginId, function: &str, params: &[u8]) -> Result<Vec<u8>> {
///         // This is a simplified implementation
///         Err(ConcurrencyError::InstanceCreationFailed("Not implemented".into()).into())
///     }
/// }
/// ```
pub trait ConcurrencyManager: Send + Sync {
    /// Schedule a task for execution.
    ///
    /// # Arguments
    ///
    /// * `task` - The task to execute.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the task was scheduled successfully.
    /// * `Err(ConcurrencyError)` if the task could not be scheduled.
    fn schedule_task(&self, task: Box<dyn FnOnce() + Send + 'static>) -> Result<(), ConcurrencyError>;
    
    /// Call a function in a plugin.
    ///
    /// This is a high-level interface that handles acquiring an instance
    /// and executing the function.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to call.
    /// * `function` - The name of the function to call.
    /// * `params` - The parameters to pass to the function.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The result of the function call.
    /// * `Err(ConcurrencyError)` - If the function call failed.
    fn call_function(&self, plugin_id: &PluginId, function: &str, params: &[u8]) -> Result<Vec<u8>>;
    
    /// Configure concurrency settings for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to configure.
    /// * `min_instances` - The minimum number of instances to keep ready.
    /// * `max_instances` - The maximum number of instances allowed.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the configuration was successful.
    /// * `Err(ConcurrencyError)` if the configuration failed.
    fn configure_concurrency(
        &self,
        plugin_id: &PluginId,
        min_instances: usize,
        max_instances: usize,
    ) -> Result<(), ConcurrencyError> {
        Err(ConcurrencyError::InstanceCreationFailed("Not implemented".into()).into())
    }
    
    /// Get the current instance count for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to check.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of instances.
    /// * `Err(ConcurrencyError)` if the count could not be retrieved.
    fn get_instance_count(&self, plugin_id: &PluginId) -> Result<usize> {
        Err(ConcurrencyError::InstanceCreationFailed("Not implemented".into()).into())
    }
    
    /// Clean up idle instances.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of instances cleaned up.
    /// * `Err(ConcurrencyError)` if the cleanup failed.
    fn cleanup_idle(&self) -> Result<usize> {
        Err(ConcurrencyError::InstanceCreationFailed("Not implemented".into()).into())
    }
}

/// Trait for asynchronous concurrency management.
#[cfg(feature = "async")]
pub trait AsyncConcurrencyManager: Send + Sync {
    /// Schedule an asynchronous task for execution.
    ///
    /// # Arguments
    ///
    /// * `task` - The future to execute.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the task was scheduled successfully.
    /// * `Err(ConcurrencyError)` if the task could not be scheduled.
    fn schedule_async_task<F, T>(&self, task: F) -> Result<(), ConcurrencyError>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static;
    
    /// Call a function in a plugin asynchronously.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to call.
    /// * `function` - The name of the function to call.
    /// * `params` - The parameters to pass to the function.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The result of the function call.
    /// * `Err(ConcurrencyError)` - If the function call failed.
    fn call_function_async<'a>(
        &'a self,
        plugin_id: &'a PluginId,
        function: &'a str,
        params: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>>> + Send + 'a>>;
}