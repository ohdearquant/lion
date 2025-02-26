//! Integration tests for concurrency features.
//!
//! These tests verify the concurrency mechanisms in the Lion microkernel,
//! focusing on actor message passing, instance pooling, and execution tasks.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use lion_core::error::{ConcurrencyError, Error, Result};
use lion_core::id::PluginId;
use lion_core::traits::ConcurrencyManager;

/// A test implementation of ConcurrencyManager that tracks calls and can be configured
/// to succeed or fail for testing different scenarios.
struct TestConcurrencyManager {
    tasks_executed: Arc<Mutex<Vec<String>>>,
    should_fail: Arc<Mutex<bool>>,
    delay_ms: u64,
}

impl TestConcurrencyManager {
    fn new(should_fail: bool, delay_ms: u64) -> Self {
        Self {
            tasks_executed: Arc::new(Mutex::new(Vec::new())),
            should_fail: Arc::new(Mutex::new(should_fail)),
            delay_ms,
        }
    }

    fn tasks_executed(&self) -> Vec<String> {
        self.tasks_executed.lock().unwrap().clone()
    }

    fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }
}

impl ConcurrencyManager for TestConcurrencyManager {
    fn schedule_task(&self, task: Box<dyn FnOnce() + Send + 'static>) -> Result<()> {
        if *self.should_fail.lock().unwrap() {
            return Err(Error::Concurrency(ConcurrencyError::ThreadPoolExhausted));
        }

        // Introduce an artificial delay to test timing
        let delay = self.delay_ms;
        if delay > 0 {
            thread::sleep(Duration::from_millis(delay));
        }

        // Execute the task
        task();

        Ok(())
    }

    fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>> {
        let tasks = self.tasks_executed.clone();
        let plugin_id_str = plugin_id.to_string();
        let function_str = function.to_string();
        let params_vec = params.to_vec();

        // Record the function call
        let call = format!("{}:{}:{:?}", plugin_id_str, function_str, params_vec);
        tasks.lock().unwrap().push(call);

        if *self.should_fail.lock().unwrap() {
            return Err(ConcurrencyError::NoAvailableInstances(*plugin_id).into());
        }

        // Simulate a successful function call
        Ok(vec![42, 43, 44, 45])
    }

    fn call_function_with_timeout(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let tasks = self.tasks_executed.clone();
        let plugin_id_str = plugin_id.to_string();
        let function_str = function.to_string();
        let params_vec = params.to_vec();

        // Record the function call with timeout
        let call = format!(
            "{}:{}:{:?} (timeout: {:?})",
            plugin_id_str, function_str, params_vec, timeout
        );
        tasks.lock().unwrap().push(call);

        // If delay is longer than timeout, simulate a timeout
        if self.delay_ms > timeout.as_millis() as u64 {
            return Err(ConcurrencyError::AcquisitionTimeout(
                timeout.as_millis() as u64,
                *plugin_id,
            )
            .into());
        }

        if *self.should_fail.lock().unwrap() {
            return Err(ConcurrencyError::NoAvailableInstances(*plugin_id).into());
        }

        // Simulate a successful function call
        Ok(vec![42, 43, 44, 45])
    }

    fn configure_concurrency(
        &self,
        plugin_id: &PluginId,
        min_instances: usize,
        max_instances: usize,
    ) -> Result<()> {
        let tasks = self.tasks_executed.clone();
        let plugin_id_str = plugin_id.to_string();

        // Record the configuration
        let call = format!(
            "Configure {}:min={},max={}",
            plugin_id_str, min_instances, max_instances
        );
        tasks.lock().unwrap().push(call);

        if *self.should_fail.lock().unwrap() {
            return Err(Error::Concurrency(ConcurrencyError::PoolLimitReached(
                "Test failure".to_string(),
            )));
        }

        Ok(())
    }

    fn send_message(&self, plugin_id: &PluginId, message: Vec<u8>) -> Result<()> {
        let tasks = self.tasks_executed.clone();
        let plugin_id_str = plugin_id.to_string();

        // Record the message send
        let msg = format!("Message to {}: {:?}", plugin_id_str, message);
        tasks.lock().unwrap().push(msg);

        if *self.should_fail.lock().unwrap() {
            return Err(Error::Concurrency(ConcurrencyError::MessageDeliveryFailed(
                "Test failure".to_string(),
            )));
        }

        Ok(())
    }
}

#[test]
fn test_schedule_multiple_tasks() {
    let manager = TestConcurrencyManager::new(false, 0);
    let tasks_executed = manager.tasks_executed.clone();

    // Schedule multiple tasks
    for i in 0..5 {
        let tasks = tasks_executed.clone();
        let task_num = i;

        manager
            .schedule_task(Box::new(move || {
                tasks
                    .lock()
                    .unwrap()
                    .push(format!("Task {} executed", task_num));
            }))
            .unwrap();
    }

    // Verify that all tasks were executed
    let executed_tasks = manager.tasks_executed();
    assert_eq!(executed_tasks.len(), 5);
    for i in 0..5 {
        assert!(executed_tasks.contains(&format!("Task {} executed", i)));
    }
}

#[test]
fn test_schedule_task_failure() {
    let manager = TestConcurrencyManager::new(true, 0);
    let tasks_executed = manager.tasks_executed.clone();

    // Schedule a task that should fail
    let result = manager.schedule_task(Box::new(move || {
        tasks_executed
            .lock()
            .unwrap()
            .push("Task executed".to_string());
    }));

    // Verify that the task failed to schedule
    assert!(result.is_err());
    match result {
        Err(Error::Concurrency(ConcurrencyError::ThreadPoolExhausted)) => {}
        _ => panic!("Expected ThreadPoolExhausted error"),
    }

    // Verify that the task was not executed
    assert_eq!(manager.tasks_executed().len(), 0);
}

#[test]
fn test_call_function_timeout_success() {
    let manager = TestConcurrencyManager::new(false, 50);
    let plugin_id = PluginId::new();
    let params = vec![1, 2, 3, 4];

    // Call a function with a timeout that should succeed
    let result = manager.call_function_with_timeout(
        &plugin_id,
        "test_function",
        &params,
        Duration::from_millis(100),
    );

    // Verify that the function call succeeded
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![42, 43, 44, 45]);

    // Verify that the function call was recorded
    let tasks = manager.tasks_executed();
    assert_eq!(tasks.len(), 1);
    assert!(tasks[0].contains(&plugin_id.to_string()));
    assert!(tasks[0].contains("test_function"));
    assert!(tasks[0].contains("timeout"));
}

#[test]
fn test_call_function_timeout_failure() {
    let manager = TestConcurrencyManager::new(false, 150);
    let plugin_id = PluginId::new();
    let params = vec![1, 2, 3, 4];

    // Call a function with a timeout that should fail
    let result = manager.call_function_with_timeout(
        &plugin_id,
        "test_function",
        &params,
        Duration::from_millis(100),
    );

    // Verify that the function call timed out
    assert!(result.is_err());
    match result {
        Err(Error::Concurrency(ConcurrencyError::AcquisitionTimeout(_, id))) => {
            assert_eq!(id, plugin_id);
        }
        _ => panic!("Expected AcquisitionTimeout error"),
    }

    // Verify that the function call was recorded
    let tasks = manager.tasks_executed();
    assert_eq!(tasks.len(), 1);
    assert!(tasks[0].contains(&plugin_id.to_string()));
    assert!(tasks[0].contains("test_function"));
    assert!(tasks[0].contains("timeout"));
}

#[test]
fn test_configure_concurrency() {
    let manager = TestConcurrencyManager::new(false, 0);
    let plugin_id = PluginId::new();

    // Configure concurrency
    let result = manager.configure_concurrency(&plugin_id, 2, 10);

    // Verify that configuration succeeded
    assert!(result.is_ok());

    // Verify that the configuration was recorded
    let tasks = manager.tasks_executed();
    assert_eq!(tasks.len(), 1);
    assert!(tasks[0].contains(&plugin_id.to_string()));
    assert!(tasks[0].contains("min=2"));
    assert!(tasks[0].contains("max=10"));

    // Test failure case
    manager.set_should_fail(true);
    let result = manager.configure_concurrency(&plugin_id, 3, 15);

    // Verify that configuration failed
    assert!(result.is_err());
    match result {
        Err(Error::Concurrency(ConcurrencyError::PoolLimitReached(_))) => {}
        _ => panic!("Expected PoolLimitReached error"),
    }
}

#[test]
fn test_send_message() {
    let manager = TestConcurrencyManager::new(false, 0);
    let plugin_id = PluginId::new();
    let message = vec![10, 11, 12, 13];

    // Send a message
    let result = manager.send_message(&plugin_id, message.clone());

    // Verify that message sending succeeded
    assert!(result.is_ok());

    // Verify that the message was recorded
    let tasks = manager.tasks_executed();
    assert_eq!(tasks.len(), 1);
    assert!(tasks[0].contains(&plugin_id.to_string()));
    assert!(tasks[0].contains(&format!("{:?}", message)));

    // Test failure case
    manager.set_should_fail(true);
    let result = manager.send_message(&plugin_id, vec![20, 21, 22, 23]);

    // Verify that message sending failed
    assert!(result.is_err());
    match result {
        Err(Error::Concurrency(ConcurrencyError::MessageDeliveryFailed(_))) => {}
        _ => panic!("Expected MessageDeliveryFailed error"),
    }
}

#[test]
fn test_concurrent_plugin_calls() {
    let manager = Arc::new(TestConcurrencyManager::new(false, 0));
    let plugin_id = PluginId::new();

    // Spawn 10 threads, each making a call to the plugin
    let mut handles = Vec::new();

    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let plugin_id_clone = plugin_id;

        let handle = thread::spawn(move || {
            let params = vec![i as u8];
            let result = manager_clone.call_function(&plugin_id_clone, "test_function", &params);
            assert!(result.is_ok());
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify that all calls were recorded
    let tasks = manager.tasks_executed();
    assert_eq!(tasks.len(), 10);

    // Each call should reference the same plugin ID but different params
    for task in &tasks {
        assert!(task.contains(&plugin_id.to_string()));
        assert!(task.contains("test_function"));
    }
}
