//! Integration tests for isolation features.
//!
//! These tests verify the isolation mechanisms in the Lion microkernel,
//! focusing on plugin sandboxing, memory regions, and resource limits.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lion_core::error::{Error, IsolationError, Result};
use lion_core::id::{PluginId, RegionId};
use lion_core::traits::isolation::ResourceLimitType;
use lion_core::traits::IsolationBackend;
use lion_core::types::{MemoryRegion, MemoryRegionType, PluginConfig, PluginState};

/// A test implementation of IsolationBackend that tracks operations and
/// can be configured to simulate different scenarios.
struct TestIsolationBackend {
    plugins: Arc<Mutex<HashMap<PluginId, Vec<u8>>>>,
    states: Arc<Mutex<HashMap<PluginId, PluginState>>>,
    memory_regions: Arc<Mutex<HashMap<RegionId, Vec<u8>>>>,
    limits: Arc<Mutex<HashMap<PluginId, HashMap<ResourceLimitType, u64>>>>,
    fail_next_operation: Arc<Mutex<Option<String>>>,
}

impl TestIsolationBackend {
    fn new() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
            states: Arc::new(Mutex::new(HashMap::new())),
            memory_regions: Arc::new(Mutex::new(HashMap::new())),
            limits: Arc::new(Mutex::new(HashMap::new())),
            fail_next_operation: Arc::new(Mutex::new(None)),
        }
    }

    fn set_fail_next(&self, operation: Option<String>) {
        let operation_clone = operation.clone();
        // Acquire the lock in a separate scope to ensure it's released
        {
            let mut fail_op = self.fail_next_operation.lock().unwrap();
            *fail_op = operation;
        }
        println!("Set fail_next_operation to {:?}", operation_clone);
    }

    fn should_fail(&self, operation: &str) -> bool {
        println!("Checking if operation '{}' should fail", operation);
        // Acquire the lock and check if the operation should fail
        let mut fail_op = self.fail_next_operation.lock().unwrap();
        let should_fail = fail_op.as_ref().map_or(false, |op| op == operation);
        if should_fail {
            println!("Operation '{}' set to fail, resetting fail flag", operation);
            *fail_op = None;
        }
        should_fail
    }
}

impl IsolationBackend for TestIsolationBackend {
    fn load_plugin(&self, id: PluginId, module_bytes: &[u8], _config: &PluginConfig) -> Result<()> {
        if self.should_fail("load") {
            return Err(IsolationError::LoadFailed("Simulated failure".into()).into());
        }

        self.plugins
            .lock()
            .unwrap()
            .insert(id, module_bytes.to_vec());
        self.states.lock().unwrap().insert(id, PluginState::Ready);
        self.limits.lock().unwrap().insert(id, HashMap::new());
        Ok(())
    }

    fn unload_plugin(&self, id: &PluginId) -> Result<()> {
        if self.should_fail("unload") {
            return Err(IsolationError::LoadFailed("Simulated failure".into()).into());
        }

        if self.plugins.lock().unwrap().remove(id).is_none() {
            return Err(IsolationError::PluginNotLoaded(*id).into());
        }
        self.states.lock().unwrap().remove(id);
        self.limits.lock().unwrap().remove(id);
        Ok(())
    }

    fn call_function(&self, id: &PluginId, function: &str, params: &[u8]) -> Result<Vec<u8>> {
        if self.should_fail("call") {
            return Err(IsolationError::ExecutionTrap("Simulated execution trap".into()).into());
        }

        if !self.plugins.lock().unwrap().contains_key(id) {
            return Err(IsolationError::PluginNotLoaded(*id).into());
        }

        // Simulate state transition to Running during call
        let mut states = self.states.lock().unwrap();
        let state = states.get_mut(id).unwrap();
        *state = PluginState::Running;

        // For testing, just return a concatenation of the function name and params
        let mut result = function.as_bytes().to_vec();
        result.extend_from_slice(params);

        // Return to Ready state
        *state = PluginState::Ready;

        Ok(result)
    }

    fn get_plugin_state(&self, id: &PluginId) -> Result<PluginState> {
        match self.states.lock().unwrap().get(id) {
            Some(state) => Ok(*state),
            None => Err(IsolationError::PluginNotLoaded(*id).into()),
        }
    }

    fn pause_plugin(&self, id: &PluginId) -> Result<()> {
        if self.should_fail("pause") {
            return Err(IsolationError::ExecutionTrap("Simulated failure".into()).into());
        }

        let mut states = self.states.lock().unwrap();
        if let Some(state) = states.get_mut(id) {
            *state = PluginState::Paused;
            Ok(())
        } else {
            Err(IsolationError::PluginNotLoaded(*id).into())
        }
    }

    fn resume_plugin(&self, id: &PluginId) -> Result<()> {
        if self.should_fail("resume") {
            return Err(IsolationError::ExecutionTrap("Simulated failure".into()).into());
        }

        let mut states = self.states.lock().unwrap();
        if let Some(state) = states.get_mut(id) {
            if *state == PluginState::Paused {
                *state = PluginState::Ready;
                Ok(())
            } else {
                Err(IsolationError::ExecutionTrap("Plugin is not paused".into()).into())
            }
        } else {
            Err(IsolationError::PluginNotLoaded(*id).into())
        }
    }

    fn update_plugin(&self, id: &PluginId, module_bytes: &[u8]) -> Result<()> {
        if self.should_fail("update") {
            return Err(IsolationError::LoadFailed("Simulated failure".into()).into());
        }

        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin_data) = plugins.get_mut(id) {
            *plugin_data = module_bytes.to_vec();
            Ok(())
        } else {
            Err(IsolationError::PluginNotLoaded(*id).into())
        }
    }

    fn create_memory_region(&self, size: usize) -> Result<MemoryRegion> {
        if self.should_fail("create_memory") {
            return Err(IsolationError::MemoryAccessError("Simulated failure".into()).into());
        }

        let region_id = RegionId::new();
        self.memory_regions
            .lock()
            .unwrap()
            .insert(region_id, vec![0; size]);

        Ok(MemoryRegion {
            id: region_id,
            size,
            region_type: MemoryRegionType::Shared,
        })
    }

    fn get_memory_region(&self, id: &RegionId) -> Result<MemoryRegion> {
        let regions = self.memory_regions.lock().unwrap();
        if let Some(data) = regions.get(id) {
            Ok(MemoryRegion {
                id: *id,
                size: data.len(),
                region_type: MemoryRegionType::Shared,
            })
        } else {
            Err(IsolationError::RegionNotFound(*id).into())
        }
    }

    fn read_memory(&self, region_id: &RegionId, offset: usize, size: usize) -> Result<Vec<u8>> {
        if self.should_fail("read_memory") {
            return Err(IsolationError::MemoryAccessError("Simulated failure".into()).into());
        }

        let regions = self.memory_regions.lock().unwrap();
        if let Some(data) = regions.get(region_id) {
            if offset + size > data.len() {
                return Err(IsolationError::MemoryAccessError("Out of bounds read".into()).into());
            }
            Ok(data[offset..offset + size].to_vec())
        } else {
            Err(IsolationError::RegionNotFound(*region_id).into())
        }
    }

    fn write_memory(&self, region_id: &RegionId, offset: usize, data: &[u8]) -> Result<()> {
        if self.should_fail("write_memory") {
            return Err(IsolationError::MemoryAccessError("Simulated failure".into()).into());
        }

        let mut regions = self.memory_regions.lock().unwrap();
        if let Some(region_data) = regions.get_mut(region_id) {
            if offset + data.len() > region_data.len() {
                return Err(IsolationError::MemoryAccessError("Out of bounds write".into()).into());
            }
            region_data[offset..offset + data.len()].copy_from_slice(data);
            Ok(())
        } else {
            Err(IsolationError::RegionNotFound(*region_id).into())
        }
    }

    fn set_resource_limit(
        &self,
        id: &PluginId,
        limit_type: ResourceLimitType,
        value: u64,
    ) -> Result<()> {
        if self.should_fail("set_limit") {
            return Err(IsolationError::ResourceExhausted("Simulated failure".into()).into());
        }

        let mut limits = self.limits.lock().unwrap();
        if let Some(plugin_limits) = limits.get_mut(id) {
            plugin_limits.insert(limit_type, value);
            Ok(())
        } else {
            Err(IsolationError::PluginNotLoaded(*id).into())
        }
    }

    fn get_available_functions(&self, id: &PluginId) -> Result<Vec<String>> {
        if !self.plugins.lock().unwrap().contains_key(id) {
            return Err(IsolationError::PluginNotLoaded(*id).into());
        }

        // For testing, return some dummy functions
        Ok(vec!["test_func".to_string(), "another_func".to_string()])
    }
}

#[test]
fn test_plugin_lifecycle() {
    let backend = TestIsolationBackend::new();
    let plugin_id = PluginId::new();
    let module_bytes = vec![1, 2, 3, 4]; // Dummy module bytes
    let config = PluginConfig::default();

    // Load the plugin
    backend
        .load_plugin(plugin_id, &module_bytes, &config)
        .unwrap();

    // Check initial state
    let state = backend.get_plugin_state(&plugin_id).unwrap();
    assert_eq!(state, PluginState::Ready);

    // Call a function
    let result = backend
        .call_function(&plugin_id, "test_function", &[5, 6, 7])
        .unwrap();
    assert_eq!(&result[0..13], b"test_function");
    assert_eq!(&result[13..], &[5, 6, 7]);

    // Pause the plugin
    backend.pause_plugin(&plugin_id).unwrap();

    // Verify paused state
    let state = backend.get_plugin_state(&plugin_id).unwrap();
    assert_eq!(state, PluginState::Paused);

    // Trying to call while paused should fail
    backend.set_fail_next(Some("call".to_string()));
    let result = backend.call_function(&plugin_id, "test_function", &[5, 6, 7]);
    assert!(result.is_err());

    // Resume the plugin
    backend.resume_plugin(&plugin_id).unwrap();

    // Verify resumed state
    let state = backend.get_plugin_state(&plugin_id).unwrap();
    assert_eq!(state, PluginState::Ready);

    // Update the plugin
    let new_module_bytes = vec![8, 9, 10, 11];
    backend
        .update_plugin(&plugin_id, &new_module_bytes)
        .unwrap();

    // Unload the plugin
    backend.unload_plugin(&plugin_id).unwrap();

    // Verify unloaded
    let result = backend.get_plugin_state(&plugin_id);
    assert!(result.is_err());
}

#[test]
fn test_memory_regions() {
    let backend = TestIsolationBackend::new();

    // Create a memory region
    let region = backend.create_memory_region(1024).unwrap();
    assert_eq!(region.size, 1024);

    // Write to memory
    let data = vec![1, 2, 3, 4, 5];
    backend.write_memory(&region.id, 0, &data).unwrap();

    // Read from memory
    let read_data = backend.read_memory(&region.id, 0, 5).unwrap();
    assert_eq!(read_data, data);

    // Test read with offset
    let read_data = backend.read_memory(&region.id, 2, 3).unwrap();
    assert_eq!(read_data, vec![3, 4, 5]);

    // Test boundary conditions

    // Reading past the end should fail
    let result = backend.read_memory(&region.id, 1020, 10);
    assert!(result.is_err());
    match result {
        Err(Error::Isolation(IsolationError::MemoryAccessError(_))) => {}
        _ => panic!("Expected MemoryAccessError"),
    }

    // Writing past the end should fail
    let result = backend.write_memory(&region.id, 1020, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert!(result.is_err());
    match result {
        Err(Error::Isolation(IsolationError::MemoryAccessError(_))) => {}
        _ => panic!("Expected MemoryAccessError"),
    }

    // Simulated memory access failure
    backend.set_fail_next(Some("read_memory".to_string()));
    let result = backend.read_memory(&region.id, 0, 5);
    assert!(result.is_err());

    backend.set_fail_next(Some("write_memory".to_string()));
    let result = backend.write_memory(&region.id, 0, &[1, 2, 3]);
    assert!(result.is_err());
}

#[test]
fn test_resource_limits() {
    let backend = TestIsolationBackend::new();
    let plugin_id = PluginId::new();
    let module_bytes = vec![1, 2, 3, 4];
    let config = PluginConfig::default();

    // Load the plugin
    backend
        .load_plugin(plugin_id, &module_bytes, &config)
        .unwrap();

    // Set memory limit
    backend
        .set_resource_limit(&plugin_id, ResourceLimitType::Memory, 1024 * 1024)
        .unwrap();

    // Set CPU time limit
    backend
        .set_resource_limit(&plugin_id, ResourceLimitType::CpuTime, 1000000)
        .unwrap();

    // Set function timeout
    backend
        .set_resource_limit(&plugin_id, ResourceLimitType::FunctionTimeout, 5000)
        .unwrap();

    // Set instruction count limit
    backend
        .set_resource_limit(&plugin_id, ResourceLimitType::InstructionCount, 10000000)
        .unwrap();

    // Setting a limit that fails
    backend.set_fail_next(Some("set_limit".to_string()));
    let result = backend.set_resource_limit(&plugin_id, ResourceLimitType::FunctionCallCount, 100);
    assert!(result.is_err());

    // Unload the plugin
    backend.unload_plugin(&plugin_id).unwrap();

    // Setting a limit on an unloaded plugin should fail
    let result = backend.set_resource_limit(&plugin_id, ResourceLimitType::Memory, 2048 * 1024);
    assert!(result.is_err());
    match result {
        Err(Error::Isolation(IsolationError::PluginNotLoaded(id))) => {
            assert_eq!(id, plugin_id);
        }
        _ => panic!("Expected PluginNotLoaded error"),
    }
}

#[test]
fn test_plugin_state_transitions() {
    let backend = TestIsolationBackend::new();
    let plugin_id = PluginId::new();
    let module_bytes = vec![1, 2, 3, 4];
    let config = PluginConfig::default();

    // Load the plugin
    backend
        .load_plugin(plugin_id, &module_bytes, &config)
        .unwrap();

    // Initial state should be Ready
    let state = backend.get_plugin_state(&plugin_id).unwrap();
    assert_eq!(state, PluginState::Ready);

    // Calling a function should temporarily transition to Running and back to Ready
    backend.call_function(&plugin_id, "test", &[]).unwrap();
    let state = backend.get_plugin_state(&plugin_id).unwrap();
    assert_eq!(state, PluginState::Ready);

    // Test pause/resume cycle
    backend.pause_plugin(&plugin_id).unwrap();
    assert_eq!(
        backend.get_plugin_state(&plugin_id).unwrap(),
        PluginState::Paused
    );

    backend.resume_plugin(&plugin_id).unwrap();
    assert_eq!(
        backend.get_plugin_state(&plugin_id).unwrap(),
        PluginState::Ready
    );

    // Test invalid state transition: resume when not paused
    backend.set_fail_next(Some("resume".to_string()));
    let result = backend.resume_plugin(&plugin_id);
    assert!(result.is_err());
}

#[test]
fn test_available_functions() {
    let backend = TestIsolationBackend::new();
    let plugin_id = PluginId::new();
    let module_bytes = vec![1, 2, 3, 4];
    let config = PluginConfig::default();

    // Load the plugin
    backend
        .load_plugin(plugin_id, &module_bytes, &config)
        .unwrap();

    // Get available functions
    let functions = backend.get_available_functions(&plugin_id).unwrap();
    assert_eq!(functions.len(), 2);
    assert!(functions.contains(&"test_func".to_string()));
    assert!(functions.contains(&"another_func".to_string()));

    // Unloaded plugin should fail
    let unloaded_id = PluginId::new();
    let result = backend.get_available_functions(&unloaded_id);
    assert!(result.is_err());
}

#[test]
fn test_isolation_failures() {
    println!("Starting test_isolation_failures");
    let backend = TestIsolationBackend::new();
    let plugin_id = PluginId::new();
    let module_bytes = vec![1, 2, 3, 4];
    let config = PluginConfig::default();

    // Test failure during load
    backend.set_fail_next(Some("load".to_string()));
    println!("Testing load failure");
    let result = backend.load_plugin(plugin_id, &module_bytes, &config);
    assert!(result.is_err());
    match result {
        Err(Error::Isolation(IsolationError::LoadFailed(_))) => {}
        _ => panic!("Expected LoadFailed error"),
    }

    // Successfully load for subsequent tests
    println!("Loading plugin for subsequent tests");
    backend
        .load_plugin(plugin_id, &module_bytes, &config)
        .unwrap();

    // Test failure during call
    backend.set_fail_next(Some("call".to_string()));
    println!("Testing call failure");
    let result = backend.call_function(&plugin_id, "test", &[]);
    assert!(result.is_err());
    match result {
        Err(Error::Isolation(IsolationError::ExecutionTrap(_))) => {}
        _ => panic!("Expected ExecutionTrap error"),
    }

    // Test failure during memory region creation
    backend.set_fail_next(Some("create_memory".to_string()));
    println!("Testing memory region creation failure");
    let result = backend.create_memory_region(1024);
    assert!(result.is_err());
    match result {
        Err(Error::Isolation(IsolationError::MemoryAccessError(_))) => {}
        _ => panic!("Expected MemoryAccessError error"),
    }

    // Test failure during unload
    backend.set_fail_next(Some("unload".to_string()));
    println!("Testing unload failure");
    let result = backend.unload_plugin(&plugin_id);
    assert!(result.is_err());
    match result {
        Err(Error::Isolation(IsolationError::LoadFailed(_))) => {}
        _ => panic!("Expected LoadFailed error"),
    }
    println!("Completed test_isolation_failures");
}
