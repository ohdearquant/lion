//! Isolation trait definitions.
//! 
//! This module defines the core traits for the isolation system.

use crate::error::{Result, IsolationError};
use crate::id::{PluginId, RegionId};
use crate::types::{PluginConfig, ResourceUsage};

/// Core trait for isolation backends.
///
/// An isolation backend is responsible for loading and executing plugins
/// in a secure sandbox.
///
/// # Examples
///
/// ```
/// use lion_core::traits::IsolationBackend;
/// use lion_core::error::{Result, IsolationError};
/// use lion_core::id::{PluginId, RegionId};
/// use lion_core::types::{PluginConfig, ResourceUsage};
///
/// struct DummyIsolationBackend;
///
/// impl IsolationBackend for DummyIsolationBackend {
///     fn load_plugin(
///         &self,
///         plugin_id: PluginId,
///         code: Vec<u8>,
///         config: PluginConfig,
///     ) -> Result<()> {
///         // In a real implementation, we would load the plugin in a sandbox
///         println!("Loaded plugin: {}", plugin_id);
///         Ok(())
///     }
///
///     fn call_function(
///         &self,
///         plugin_id: &PluginId,
///         function: &str,
///         params: &[u8],
///     ) -> Result<Vec<u8>> {
///         // In a real implementation, we would call the function in the sandbox
///         println!("Called function: {}", function);
///         Ok(vec![])
///     }
///
///     fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()> {
///         // In a real implementation, we would unload the plugin from the sandbox
///         println!("Unloaded plugin: {}", plugin_id);
///         Ok(())
///     }
/// }
/// ```
pub trait IsolationBackend: Send + Sync {
    /// Load a plugin into the sandbox.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID to assign to the plugin.
    /// * `code` - The plugin code (typically WebAssembly binary).
    /// * `config` - The plugin configuration.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was loaded successfully.
    /// * `Err(IsolationError)` if the plugin could not be loaded.
    fn load_plugin(
        &self,
        plugin_id: PluginId,
        code: Vec<u8>,
        config: PluginConfig,
    ) -> Result<()>;
    
    /// Call a function in a plugin.
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
    /// * `Err(IsolationError)` - If the function call failed.
    fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>>;
    
    /// Unload a plugin from the sandbox.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to unload.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was unloaded successfully.
    /// * `Err(IsolationError)` if the plugin could not be unloaded.
    fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()>;
    
    /// List the available functions in a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to check.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - The names of the available functions.
    /// * `Err(IsolationError)` if the functions could not be listed.
    fn list_functions(&self, plugin_id: &PluginId) -> Result<Vec<String>> {
        Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into())
    }
    
    /// Get resource usage statistics for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to check.
    ///
    /// # Returns
    ///
    /// * `Ok(ResourceUsage)` - The resource usage statistics.
    /// * `Err(IsolationError)` if the statistics could not be retrieved.
    fn get_resource_usage(&self, plugin_id: &PluginId) -> Result<ResourceUsage> {
        Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into())
    }
    
    /// Share memory with a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to share memory with.
    /// * `data` - The data to share.
    ///
    /// # Returns
    ///
    /// * `Ok(RegionId)` - The ID of the shared memory region.
    /// * `Err(IsolationError)` if the memory could not be shared.
    fn share_memory(&self, plugin_id: &PluginId, data: &[u8]) -> Result<RegionId> {
        Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into())
    }
    
    /// Read memory from a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to read memory from.
    /// * `region_id` - The ID of the memory region to read.
    /// * `offset` - The offset into the memory region.
    /// * `length` - The number of bytes to read.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The data read from memory.
    /// * `Err(IsolationError)` if the memory could not be read.
    fn read_memory(
        &self,
        plugin_id: &PluginId,
        region_id: &RegionId,
        offset: usize,
        length: usize,
    ) -> Result<Vec<u8>> {
        Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into())
    }
}

/// Factory for creating isolation backends.
pub trait IsolationBackendFactory: Send + Sync {
    /// Create a new isolation backend.
    ///
    /// # Returns
    ///
    /// * `Ok(Box<dyn IsolationBackend>)` - A new isolation backend.
    /// * `Err(IsolationError)` - If the backend could not be created.
    fn create_backend(&self) -> Result<Box<dyn IsolationBackend>>;
}