//! Concurrency trait definitions.
//! 
//! This module defines the core traits for the concurrency system, which
//! is based on an actor model with message passing. The concurrency system
//! implements the concepts described in "Concurrent Plugin Lifecycle Management
//! in Capability-Secured Microkernels" research.
//! 
//! # Concurrency Model
//! 
//! The Lion microkernel uses a hybrid actor-based approach where:
//! 
//! - Each plugin is an isolated actor with its own state
//! - Actors communicate through message passing, not shared memory
//! - The actor system provides supervision for fault tolerance
//! - Instance pooling is used for efficient resource utilization

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::error::{Result, ConcurrencyError};
use crate::id::PluginId;

/// Core trait for concurrency management.
///
/// This trait provides an interface for scheduling tasks and managing
/// concurrent execution through an actor-based model.
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
    /// This method schedules a task to be executed by the concurrency system.
    /// The task is executed asynchronously, and this method returns immediately.
    ///
    /// # Arguments
    ///
    /// * `task` - The task to execute, represented as a boxed closure.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the task was scheduled successfully.
    /// * `Err(ConcurrencyError)` if the task could not be scheduled.
    fn schedule_task(&self, task: Box<dyn FnOnce() + Send + 'static>) -> Result<(), ConcurrencyError>;
    
    /// Call a function in a plugin.
    ///
    /// This is a high-level interface that handles acquiring an instance,
    /// executing the function, and releasing the instance. It encapsulates
    /// the entire lifecycle of a function call.
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
    
    /// Call a function in a plugin with a timeout.
    ///
    /// This is similar to `call_function`, but with a timeout. If the function
    /// does not complete within the timeout, it is aborted and an error is returned.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to call.
    /// * `function` - The name of the function to call.
    /// * `params` - The parameters to pass to the function.
    /// * `timeout` - The maximum time to wait for the function to complete.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The result of the function call.
    /// * `Err(ConcurrencyError)` - If the function call failed or timed out.
    fn call_function_with_timeout(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        // Default implementation just calls the regular function and ignores the timeout
        self.call_function(plugin_id, function, params)
    }
    
    /// Configure concurrency settings for a plugin.
    ///
    /// This configures the concurrency settings for a plugin, such as the
    /// number of instances to keep in the pool and the maximum number of
    /// instances allowed.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to configure.
    /// * `min_instances` - The minimum number of instances to keep ready in the pool.
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
    /// This returns the number of instances currently in the pool for the
    /// given plugin, both idle and in use.
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
    /// This cleans up idle instances that have been in the pool for too long,
    /// freeing up resources.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of instances cleaned up.
    /// * `Err(ConcurrencyError)` if the cleanup failed.
    fn cleanup_idle(&self) -> Result<usize> {
        Err(ConcurrencyError::InstanceCreationFailed("Not implemented".into()).into())
    }
    
    /// Create a new instance of a plugin.
    ///
    /// This creates a new instance of the plugin and adds it to the pool.
    /// This is useful for warming up the pool before handling requests.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to create an instance for.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the instance was created successfully.
    /// * `Err(ConcurrencyError)` if the instance could not be created.
    fn create_instance(&self, plugin_id: &PluginId) -> Result<(), ConcurrencyError> {
        Err(ConcurrencyError::InstanceCreationFailed("Not implemented".into()).into())
    }
    
    /// Send a message to a plugin actor.
    ///
    /// This sends a message to a plugin's actor, which will be processed
    /// asynchronously by the plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to send the message to.
    /// * `message` - The message to send, serialized as bytes.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the message was sent successfully.
    /// * `Err(ConcurrencyError)` if the message could not be sent.
    fn send_message(&self, plugin_id: &PluginId, message: Vec<u8>) -> Result<(), ConcurrencyError> {
        Err(ConcurrencyError::MessageDeliveryFailed("Not implemented".into()).into())
    }
}

/// Trait for asynchronous concurrency management.
///
/// This trait extends the `ConcurrencyManager` trait with asynchronous versions
/// of its methods. It is only available when the "async" feature is enabled.
#[cfg(feature = "async")]
pub trait AsyncConcurrencyManager: Send + Sync {
    /// Schedule an asynchronous task for execution.
    ///
    /// This method schedules an asynchronous task (a future) to be executed
    /// by the concurrency system.
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
    /// This is similar to `call_function`, but returns a future that resolves
    /// to the result of the function call.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to call.
    /// * `function` - The name of the function to call.
    /// * `params` - The parameters to pass to the function.
    ///
    /// # Returns
    ///
    /// A future that resolves to:
    /// * `Ok(Vec<u8>)` - The result of the function call.
    /// * `Err(ConcurrencyError)` - If the function call failed.
    fn call_function_async<'a>(
        &'a self,
        plugin_id: &'a PluginId,
        function: &'a str,
        params: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>>> + Send + 'a>>;
    
    /// Call a function in a plugin asynchronously with a timeout.
    ///
    /// This is similar to `call_function_async`, but with a timeout. If the
    /// function does not complete within the timeout, it is aborted and an
    /// error is returned.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to call.
    /// * `function` - The name of the function to call.
    /// * `params` - The parameters to pass to the function.
    /// * `timeout` - The maximum time to wait for the function to complete.
    ///
    /// # Returns
    ///
    /// A future that resolves to:
    /// * `Ok(Vec<u8>)` - The result of the function call.
    /// * `Err(ConcurrencyError)` - If the function call failed or timed out.
    fn call_function_async_with_timeout<'a>(
        &'a self,
        plugin_id: &'a PluginId,
        function: &'a str,
        params: &'a [u8],
        timeout: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>>> + Send + 'a>>;
    
    /// Send a message to a plugin actor asynchronously.
    ///
    /// This sends a message to a plugin's actor, which will be processed
    /// asynchronously by the plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to send the message to.
    /// * `message` - The message to send, serialized as bytes.
    ///
    /// # Returns
    ///
    /// A future that resolves to:
    /// * `Ok(())` if the message was sent successfully.
    /// * `Err(ConcurrencyError)` if the message could not be sent.
    fn send_message_async<'a>(
        &'a self,
        plugin_id: &'a PluginId,
        message: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ConcurrencyError>> + Send + 'a>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    
    // A simple concurrency manager for testing
    struct TestConcurrencyManager {
        tasks_executed: Arc<Mutex<Vec<String>>>,
    }
    
    impl TestConcurrencyManager {
        fn new() -> Self {
            Self {
                tasks_executed: Arc::new(Mutex::new(Vec::new())),
            }
        }
        
        fn tasks_executed(&self) -> Vec<String> {
            self.tasks_executed.lock().unwrap().clone()
        }
    }
    
    impl ConcurrencyManager for TestConcurrencyManager {
        fn schedule_task(&self, task: Box<dyn FnOnce() + Send + 'static>) -> Result<(), ConcurrencyError> {
            // Execute the task immediately in the current thread for testing
            task();
            Ok(())
        }
        
        fn call_function(&self, plugin_id: &PluginId, function: &str, params: &[u8]) -> Result<Vec<u8>> {
            // Record the function call
            let call = format!("{}:{}:{:?}", plugin_id, function, params);
            self.tasks_executed.lock().unwrap().push(call);
            
            // Return some dummy data
            Ok(vec![1, 2, 3, 4])
        }
        
        fn send_message(&self, plugin_id: &PluginId, message: Vec<u8>) -> Result<(), ConcurrencyError> {
            // Record the message
            let msg = format!("Message to {}: {:?}", plugin_id, message);
            self.tasks_executed.lock().unwrap().push(msg);
            Ok(())
        }
    }
    
    #[test]
    fn test_schedule_task() {
        let manager = TestConcurrencyManager::new();
        let tasks_executed = manager.tasks_executed.clone();
        
        manager.schedule_task(Box::new(move || {
            tasks_executed.lock().unwrap().push("Task executed".to_string());
        })).unwrap();
        
        assert_eq!(manager.tasks_executed(), vec!["Task executed"]);
    }
    
    #[test]
    fn test_call_function() {
        let manager = TestConcurrencyManager::new();
        let plugin_id = PluginId::new();
        let params = vec![5, 6, 7, 8];
        
        let result = manager.call_function(&plugin_id, "test_function", &params).unwrap();
        
        // Check that the function call was recorded
        let tasks = manager.tasks_executed();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].contains(&plugin_id.to_string()));
        assert!(tasks[0].contains("test_function"));
        
        // Check the dummy result
        assert_eq!(result, vec![1, 2, 3, 4]);
    }
    
    #[test]
    fn test_send_message() {
        let manager = TestConcurrencyManager::new();
        let plugin_id = PluginId::new();
        let message = vec![9, 10, 11, 12];
        
        manager.send_message(&plugin_id, message.clone()).unwrap();
        
        // Check that the message was recorded
        let tasks = manager.tasks_executed();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].contains(&plugin_id.to_string()));
        assert!(tasks[0].contains(&format!("{:?}", message)));
    }
}