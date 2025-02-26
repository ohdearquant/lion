//! Isolation trait definitions.
//!
//! This module defines the core traits for the isolation system, which provides
//! secure sandboxing for plugins. The isolation system is designed around the
//! WebAssembly Security Model described in the WebAssembly research documents.
//!
//! # Isolation Model
//!
//! The Lion microkernel provides strong isolation through WebAssembly:
//!
//! - Each plugin runs in its own WebAssembly sandbox
//! - Memory is isolated by default with no shared access
//! - Resources are accessed only through capabilities
//! - The host controls all external interactions
//! - Resource limits are enforced (memory, CPU time, etc.)

use crate::error::{IsolationError, Result};
use crate::id::{PluginId, RegionId};
use crate::types::{MemoryRegion, PluginConfig, PluginState, ResourceUsage};

/// Core trait for isolation backends.
///
/// This trait provides an interface for loading, unloading, and
/// executing plugins in isolated environments. The primary isolation
/// mechanism is WebAssembly, but other backends could be implemented.
///
/// # Examples
///
/// ```
/// use lion_core::traits::IsolationBackend;
/// use lion_core::id::{PluginId, RegionId};
/// use lion_core::types::{PluginConfig, PluginState, ResourceUsage, MemoryRegion};
/// use lion_core::error::Result;
///
/// struct DummyIsolationBackend;
///
/// impl IsolationBackend for DummyIsolationBackend {
///     fn load_plugin(&self, _id: PluginId, _module_bytes: &[u8], _config: &PluginConfig) -> Result<()> {
///         // In a real implementation, we would compile and instantiate the WebAssembly module
///         unimplemented!("Not implemented in this example")
///     }
///     
///     fn unload_plugin(&self, _id: &PluginId) -> Result<()> {
///         unimplemented!("Not implemented in this example")
///     }
///     
///     fn call_function(&self, _id: &PluginId, _function: &str, _params: &[u8]) -> Result<Vec<u8>> {
///         unimplemented!("Not implemented in this example")
///     }
///     
///     fn get_plugin_state(&self, _id: &PluginId) -> Result<PluginState> {
///         unimplemented!("Not implemented in this example")
///     }
/// }
/// ```
pub trait IsolationBackend: Send + Sync {
    /// Load a plugin from WebAssembly module bytes.
    ///
    /// This compiles the WebAssembly module, instantiates it, and
    /// registers it with the backend.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to assign to the plugin.
    /// * `module_bytes` - The WebAssembly module bytes.
    /// * `config` - Configuration options for the plugin.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was loaded successfully.
    /// * `Err(IsolationError)` if the plugin could not be loaded.
    fn load_plugin(&self, id: PluginId, module_bytes: &[u8], config: &PluginConfig) -> Result<()>;

    /// Unload a plugin.
    ///
    /// This removes the plugin from the backend and frees any resources
    /// associated with it.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to unload.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was unloaded successfully.
    /// * `Err(IsolationError)` if the plugin could not be unloaded.
    fn unload_plugin(&self, id: &PluginId) -> Result<()>;

    /// Call a function in a plugin.
    ///
    /// This calls a function in the plugin, passing the provided parameters
    /// and returning the result.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to call.
    /// * `function` - The name of the function to call.
    /// * `params` - The parameters to pass to the function.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The result of the function call.
    /// * `Err(IsolationError)` - If the function call failed.
    fn call_function(&self, id: &PluginId, function: &str, params: &[u8]) -> Result<Vec<u8>>;

    /// Get the current state of a plugin.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to check.
    ///
    /// # Returns
    ///
    /// * `Ok(PluginState)` - The current state of the plugin.
    /// * `Err(IsolationError)` if the state could not be retrieved.
    fn get_plugin_state(&self, id: &PluginId) -> Result<PluginState>;

    /// Get resource usage statistics for a plugin.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to check.
    ///
    /// # Returns
    ///
    /// * `Ok(ResourceUsage)` - The resource usage statistics.
    /// * `Err(IsolationError)` if the statistics could not be retrieved.
    fn get_resource_usage(&self, id: &PluginId) -> Result<ResourceUsage> {
        Err(IsolationError::PluginNotLoaded(*id).into())
    }

    /// Pause a plugin.
    ///
    /// This temporarily suspends execution of the plugin.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to pause.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was paused successfully.
    /// * `Err(IsolationError)` if the plugin could not be paused.
    fn pause_plugin(&self, id: &PluginId) -> Result<()> {
        Err(IsolationError::PluginNotLoaded(*id).into())
    }

    /// Resume a paused plugin.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to resume.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was resumed successfully.
    /// * `Err(IsolationError)` if the plugin could not be resumed.
    fn resume_plugin(&self, id: &PluginId) -> Result<()> {
        Err(IsolationError::PluginNotLoaded(*id).into())
    }

    /// Update a plugin with a new WebAssembly module.
    ///
    /// This performs a hot reload of the plugin, replacing the
    /// WebAssembly module while preserving state.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to update.
    /// * `module_bytes` - The new WebAssembly module bytes.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was updated successfully.
    /// * `Err(IsolationError)` if the plugin could not be updated.
    fn update_plugin(&self, id: &PluginId, _module_bytes: &[u8]) -> Result<()> {
        Err(IsolationError::PluginNotLoaded(*id).into())
    }

    /// Create a memory region that can be shared with plugins.
    ///
    /// # Arguments
    ///
    /// * `size` - The size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// * `Ok(MemoryRegion)` - The created memory region.
    /// * `Err(IsolationError)` if the memory region could not be created.
    fn create_memory_region(&self, _size: usize) -> Result<MemoryRegion> {
        Err(IsolationError::MemoryAccessError("Memory regions not supported".into()).into())
    }

    /// Get a memory region.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the memory region to get.
    ///
    /// # Returns
    ///
    /// * `Ok(MemoryRegion)` - The memory region.
    /// * `Err(IsolationError)` if the memory region could not be found.
    fn get_memory_region(&self, id: &RegionId) -> Result<MemoryRegion> {
        Err(IsolationError::RegionNotFound(*id).into())
    }

    /// Share a memory region with a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to share with.
    /// * `region_id` - The ID of the memory region to share.
    /// * `read_only` - Whether the plugin should have read-only access.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the memory region was shared successfully.
    /// * `Err(IsolationError)` if the memory region could not be shared.
    fn share_memory_region(
        &self,
        _plugin_id: &PluginId,
        _region_id: &RegionId,
        _read_only: bool,
    ) -> Result<()> {
        Err(IsolationError::MemoryAccessError("Memory sharing not supported".into()).into())
    }

    /// Read data from a memory region.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The ID of the memory region to read from.
    /// * `offset` - The offset within the memory region to start reading.
    /// * `size` - The number of bytes to read.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The data read from the memory region.
    /// * `Err(IsolationError)` if the data could not be read.
    fn read_memory(&self, _region_id: &RegionId, _offset: usize, _size: usize) -> Result<Vec<u8>> {
        Err(IsolationError::MemoryAccessError("Memory reading not supported".into()).into())
    }

    /// Write data to a memory region.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The ID of the memory region to write to.
    /// * `offset` - The offset within the memory region to start writing.
    /// * `data` - The data to write.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the data was written successfully.
    /// * `Err(IsolationError)` if the data could not be written.
    fn write_memory(&self, _region_id: &RegionId, _offset: usize, _data: &[u8]) -> Result<()> {
        Err(IsolationError::MemoryAccessError("Memory writing not supported".into()).into())
    }

    /// Set a resource limit for a plugin.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to set the limit for.
    /// * `limit_type` - The type of limit to set.
    /// * `value` - The limit value.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the limit was set successfully.
    /// * `Err(IsolationError)` if the limit could not be set.
    fn set_resource_limit(
        &self,
        id: &PluginId,
        _limit_type: ResourceLimitType,
        _value: u64,
    ) -> Result<()> {
        Err(IsolationError::PluginNotLoaded(*id).into())
    }

    /// Get a list of available functions in a plugin.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to check.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - The list of available functions.
    /// * `Err(IsolationError)` if the functions could not be retrieved.
    fn get_available_functions(&self, id: &PluginId) -> Result<Vec<String>> {
        Err(IsolationError::PluginNotLoaded(*id).into())
    }
}

/// Type of resource limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceLimitType {
    /// Maximum memory usage in bytes.
    Memory,

    /// Maximum CPU time in microseconds.
    CpuTime,

    /// Function call timeout in milliseconds.
    FunctionTimeout,

    /// Maximum number of instructions per function call.
    InstructionCount,

    /// Maximum number of function calls.
    FunctionCallCount,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // A simple isolation backend for testing
    struct TestIsolationBackend {
        plugins: Arc<Mutex<HashMap<PluginId, Vec<u8>>>>,
        states: Arc<Mutex<HashMap<PluginId, PluginState>>>,
        memory_regions: Arc<Mutex<HashMap<RegionId, Vec<u8>>>>,
    }

    impl TestIsolationBackend {
        fn new() -> Self {
            Self {
                plugins: Arc::new(Mutex::new(HashMap::new())),
                states: Arc::new(Mutex::new(HashMap::new())),
                memory_regions: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    impl IsolationBackend for TestIsolationBackend {
        fn load_plugin(
            &self,
            id: PluginId,
            module_bytes: &[u8],
            _config: &PluginConfig,
        ) -> Result<()> {
            self.plugins
                .lock()
                .unwrap()
                .insert(id, module_bytes.to_vec());
            self.states.lock().unwrap().insert(id, PluginState::Ready);
            Ok(())
        }

        fn unload_plugin(&self, id: &PluginId) -> Result<()> {
            if self.plugins.lock().unwrap().remove(id).is_none() {
                return Err(IsolationError::PluginNotLoaded(*id).into());
            }
            self.states.lock().unwrap().remove(id);
            Ok(())
        }

        fn call_function(&self, id: &PluginId, function: &str, params: &[u8]) -> Result<Vec<u8>> {
            if !self.plugins.lock().unwrap().contains_key(id) {
                return Err(IsolationError::PluginNotLoaded(*id).into());
            }

            // For testing, just return a concatenation of the function name and params
            let mut result = function.as_bytes().to_vec();
            result.extend_from_slice(params);
            Ok(result)
        }

        fn get_plugin_state(&self, id: &PluginId) -> Result<PluginState> {
            match self.states.lock().unwrap().get(id) {
                Some(state) => Ok(*state),
                None => Err(IsolationError::PluginNotLoaded(*id).into()),
            }
        }

        fn pause_plugin(&self, id: &PluginId) -> Result<()> {
            let mut states = self.states.lock().unwrap();
            if let Some(state) = states.get_mut(id) {
                *state = PluginState::Paused;
                Ok(())
            } else {
                Err(IsolationError::PluginNotLoaded(*id).into())
            }
        }

        fn resume_plugin(&self, id: &PluginId) -> Result<()> {
            let mut states = self.states.lock().unwrap();
            if let Some(state) = states.get_mut(id) {
                if *state == PluginState::Paused {
                    *state = PluginState::Running;
                    Ok(())
                } else {
                    Err(IsolationError::ExecutionTrap("Plugin is not paused".into()).into())
                }
            } else {
                Err(IsolationError::PluginNotLoaded(*id).into())
            }
        }

        fn create_memory_region(&self, size: usize) -> Result<MemoryRegion> {
            let region_id = RegionId::new();
            self.memory_regions
                .lock()
                .unwrap()
                .insert(region_id, vec![0; size]);

            Ok(MemoryRegion {
                id: region_id,
                size,
                region_type: crate::types::MemoryRegionType::Shared,
            })
        }

        fn get_memory_region(&self, id: &RegionId) -> Result<MemoryRegion> {
            let regions = self.memory_regions.lock().unwrap();
            if let Some(data) = regions.get(id) {
                Ok(MemoryRegion {
                    id: *id,
                    size: data.len(),
                    region_type: crate::types::MemoryRegionType::Shared,
                })
            } else {
                Err(IsolationError::RegionNotFound(*id).into())
            }
        }

        fn read_memory(&self, region_id: &RegionId, offset: usize, size: usize) -> Result<Vec<u8>> {
            let regions = self.memory_regions.lock().unwrap();
            if let Some(data) = regions.get(region_id) {
                if offset + size > data.len() {
                    return Err(
                        IsolationError::MemoryAccessError("Out of bounds read".into()).into(),
                    );
                }
                Ok(data[offset..offset + size].to_vec())
            } else {
                Err(IsolationError::RegionNotFound(*region_id).into())
            }
        }

        fn write_memory(&self, region_id: &RegionId, offset: usize, data: &[u8]) -> Result<()> {
            let mut regions = self.memory_regions.lock().unwrap();
            if let Some(region_data) = regions.get_mut(region_id) {
                if offset + data.len() > region_data.len() {
                    return Err(
                        IsolationError::MemoryAccessError("Out of bounds write".into()).into(),
                    );
                }
                region_data[offset..offset + data.len()].copy_from_slice(data);
                Ok(())
            } else {
                Err(IsolationError::RegionNotFound(*region_id).into())
            }
        }
    }

    #[test]
    fn test_load_unload_plugin() {
        let backend = TestIsolationBackend::new();
        let plugin_id = PluginId::new();
        let module_bytes = vec![1, 2, 3, 4];
        let config = PluginConfig::default();

        // Load the plugin
        backend
            .load_plugin(plugin_id, &module_bytes, &config)
            .unwrap();

        // Check that the plugin is loaded
        let state = backend.get_plugin_state(&plugin_id).unwrap();
        assert_eq!(state, PluginState::Ready);

        // Unload the plugin
        backend.unload_plugin(&plugin_id).unwrap();

        // Check that the plugin is unloaded
        let result = backend.get_plugin_state(&plugin_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_call_function() {
        let backend = TestIsolationBackend::new();
        let plugin_id = PluginId::new();
        let module_bytes = vec![1, 2, 3, 4];
        let config = PluginConfig::default();

        // Load the plugin
        backend
            .load_plugin(plugin_id, &module_bytes, &config)
            .unwrap();

        // Call a function
        let params = vec![5, 6, 7, 8];
        let result = backend
            .call_function(&plugin_id, "test_function", &params)
            .unwrap();

        // Check the result
        let expected = {
            let mut expected = "test_function".as_bytes().to_vec();
            expected.extend_from_slice(&params);
            expected
        };
        assert_eq!(result, expected);

        // Call a function on an unloaded plugin
        let unloaded_id = PluginId::new();
        let result = backend.call_function(&unloaded_id, "test_function", &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_pause_resume_plugin() {
        let backend = TestIsolationBackend::new();
        let plugin_id = PluginId::new();
        let module_bytes = vec![1, 2, 3, 4];
        let config = PluginConfig::default();

        // Load the plugin
        backend
            .load_plugin(plugin_id, &module_bytes, &config)
            .unwrap();

        // Pause the plugin
        backend.pause_plugin(&plugin_id).unwrap();

        // Check that the plugin is paused
        let state = backend.get_plugin_state(&plugin_id).unwrap();
        assert_eq!(state, PluginState::Paused);

        // Resume the plugin
        backend.resume_plugin(&plugin_id).unwrap();

        // Check that the plugin is running
        let state = backend.get_plugin_state(&plugin_id).unwrap();
        assert_eq!(state, PluginState::Running);
    }

    #[test]
    fn test_memory_regions() {
        let backend = TestIsolationBackend::new();

        // Create a memory region
        let region = backend.create_memory_region(1024).unwrap();

        // Write to the memory region
        let data = vec![9, 10, 11, 12];
        backend.write_memory(&region.id, 0, &data).unwrap();

        // Read from the memory region
        let read_data = backend.read_memory(&region.id, 0, data.len()).unwrap();
        assert_eq!(read_data, data);

        // Attempt to read beyond the end of the region
        let result = backend.read_memory(&region.id, 1020, 8);
        assert!(result.is_err());

        // Attempt to write beyond the end of the region
        let result = backend.write_memory(&region.id, 1020, &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(result.is_err());

        // Attempt to read from a non-existent region
        let result = backend.read_memory(&RegionId::new(), 0, 4);
        assert!(result.is_err());
    }
}
