//! Plugin trait definitions.
//! 
//! This module defines the core traits for the plugin system.

use crate::error::{Result, PluginError};
use crate::id::PluginId;
use crate::types::{PluginConfig, PluginMetadata, PluginState, PluginType, ResourceUsage};

/// Core trait for plugin management.
///
/// A plugin manager is responsible for the lifecycle of plugins, including
/// loading, calling functions, and unloading.
///
/// # Examples
///
/// ```
/// use lion_core::traits::PluginManager;
/// use lion_core::error::{Result, PluginError};
/// use lion_core::id::PluginId;
/// use lion_core::types::{PluginConfig, PluginMetadata, PluginState, PluginType, ResourceUsage};
///
/// struct DummyPluginManager;
///
/// impl PluginManager for DummyPluginManager {
///     fn load_plugin(
///         &self,
///         name: &str,
///         version: &str,
///         description: &str,
///         plugin_type: PluginType,
///         code: Vec<u8>,
///         config: PluginConfig,
///     ) -> Result<PluginId> {
///         // In a real implementation, we would load the plugin
///         let plugin_id = PluginId::new();
///         println!("Loaded plugin: {}", plugin_id);
///         Ok(plugin_id)
///     }
///
///     fn call_function(
///         &self,
///         plugin_id: &PluginId,
///         function: &str,
///         params: &[u8],
///     ) -> Result<Vec<u8>> {
///         // In a real implementation, we would call the function
///         println!("Called function: {}", function);
///         Ok(vec![])
///     }
///
///     fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()> {
///         // In a real implementation, we would unload the plugin
///         println!("Unloaded plugin: {}", plugin_id);
///         Ok(())
///     }
/// }
/// ```
pub trait PluginManager: Send + Sync {
    /// Load a plugin.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the plugin.
    /// * `version` - The version of the plugin.
    /// * `description` - A description of the plugin.
    /// * `plugin_type` - The type of the plugin (e.g., WebAssembly).
    /// * `code` - The plugin code (typically WebAssembly binary).
    /// * `config` - The plugin configuration.
    ///
    /// # Returns
    ///
    /// * `Ok(PluginId)` - The ID of the loaded plugin.
    /// * `Err(PluginError)` - If the plugin could not be loaded.
    fn load_plugin(
        &self,
        name: &str,
        version: &str,
        description: &str,
        plugin_type: PluginType,
        code: Vec<u8>,
        config: PluginConfig,
    ) -> Result<PluginId>;
    
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
    /// * `Err(PluginError)` - If the function call failed.
    fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>>;
    
    /// Get metadata for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to get metadata for.
    ///
    /// # Returns
    ///
    /// * `Some(PluginMetadata)` - The plugin metadata.
    /// * `None` - If the plugin was not found.
    fn get_metadata(&self, plugin_id: &PluginId) -> Option<PluginMetadata>;
    
    /// Unload a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to unload.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the plugin was successfully unloaded.
    /// * `Err(PluginError)` - If the plugin could not be unloaded.
    fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()>;
    
    /// Pause a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to pause.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the plugin was successfully paused.
    /// * `Err(PluginError)` - If the plugin could not be paused.
    fn pause_plugin(&self, plugin_id: &PluginId) -> Result<()>;
    
    /// Resume a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to resume.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the plugin was successfully resumed.
    /// * `Err(PluginError)` - If the plugin could not be resumed.
    fn resume_plugin(&self, plugin_id: &PluginId) -> Result<()>;
    
    /// List all loaded plugins.
    ///
    /// # Returns
    ///
    /// A vector of metadata for all loaded plugins.
    fn list_plugins(&self) -> Vec<PluginMetadata>;
    
    /// Get resource usage for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to get resource usage for.
    ///
    /// # Returns
    ///
    /// * `Ok(ResourceUsage)` - The resource usage statistics.
    /// * `Err(PluginError)` - If the resource usage could not be retrieved.
    fn get_resource_usage(&self, plugin_id: &PluginId) -> Result<ResourceUsage>;
    
    /// Get the current state of a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to get the state for.
    ///
    /// # Returns
    ///
    /// * `Ok(PluginState)` - The current state of the plugin.
    /// * `Err(PluginError)` - If the state could not be retrieved.
    fn get_plugin_state(&self, plugin_id: &PluginId) -> Result<PluginState>;
    
    /// Hot reload a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to reload.
    /// * `code` - The new plugin code.
    /// * `config` - The new plugin configuration.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the plugin was successfully reloaded.
    /// * `Err(PluginError)` - If the plugin could not be reloaded.
    fn hot_reload_plugin(
        &self,
        plugin_id: &PluginId,
        code: Vec<u8>,
        config: PluginConfig,
    ) -> Result<()> {
        Err(PluginError::ExecutionError("Hot reloading not supported".into()).into())
    }
}