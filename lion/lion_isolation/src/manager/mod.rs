//! Plugin isolation management.
//!
//! This module provides management of isolated plugins, including lifecycle,
//! resource consumption, and capabilities.

mod backend;
mod lifecycle;
mod pool;

pub use backend::{DefaultIsolationBackend, IsolationBackend};
pub use lifecycle::{PluginLifecycle, PluginState};
pub use pool::{InstancePool, PooledInstance};

use crate::interface::CapabilityChecker;
use crate::resource::ResourceLimiter;
use crate::wasm::{WasmEngine, WasmModule};
use lion_core::error::{IsolationError, Result};
use lion_core::PluginId;
use std::sync::Arc;

/// An isolation manager.
///
/// An isolation manager provides methods for managing isolated plugins.
pub struct IsolationManager {
    /// The backend.
    backend: Box<dyn IsolationBackend>,
}

impl IsolationManager {
    /// Create a new isolation manager.
    ///
    /// # Arguments
    ///
    /// * `backend` - The backend.
    ///
    /// # Returns
    ///
    /// A new isolation manager.
    pub fn new(backend: Box<dyn IsolationBackend>) -> Self {
        Self { backend }
    }

    /// Create a new isolation manager with a default backend.
    ///
    /// # Arguments
    ///
    /// * `engine` - The WebAssembly engine.
    /// * `resource_limiter` - The resource limiter.
    ///
    /// # Returns
    ///
    /// A new isolation manager with a default backend.
    pub fn with_default_backend(
        engine: Arc<WasmEngine>,
        resource_limiter: Arc<dyn ResourceLimiter>,
    ) -> Result<Self> {
        let backend = DefaultIsolationBackend::new(engine, resource_limiter)?;
        Ok(Self::new(Box::new(backend)))
    }

    /// Get a mutable reference to the backend.
    pub fn backend_mut(&mut self) -> &mut Box<dyn IsolationBackend> {
        &mut self.backend
    }

    /// Load a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `wasm_bytes` - The WebAssembly binary.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the plugin was successfully loaded.
    /// * `Err` - If the plugin could not be loaded.
    pub fn load_plugin(&mut self, plugin_id: &PluginId, wasm_bytes: &[u8]) -> Result<()> {
        self.backend.load_plugin(plugin_id, wasm_bytes)
    }

    /// Unload a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the plugin was successfully unloaded.
    /// * `Err` - If the plugin could not be unloaded.
    pub fn unload_plugin(&mut self, plugin_id: &PluginId) -> Result<()> {
        self.backend.unload_plugin(plugin_id)
    }

    /// Call a function in a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `function_name` - The name of the function.
    /// * `params` - The parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The result of the function call.
    /// * `Err` - If the function could not be called.
    pub fn call_function(
        &self,
        plugin_id: &PluginId,
        function_name: &str,
        params: &[u8],
    ) -> Result<Vec<u8>> {
        self.backend.call_function(plugin_id, function_name, params)
    }

    /// Get the state of a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    ///
    /// # Returns
    ///
    /// * `Ok(PluginState)` - The state of the plugin.
    /// * `Err` - If the plugin does not exist.
    pub fn get_plugin_state(&self, plugin_id: &PluginId) -> Result<PluginState> {
        self.backend.get_plugin_state(plugin_id)
    }

    /// Get the resource usage of a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    ///
    /// # Returns
    ///
    /// * `Ok(ResourceUsage)` - The resource usage of the plugin.
    /// * `Err` - If the plugin does not exist.
    pub fn get_resource_usage(
        &self,
        plugin_id: &PluginId,
    ) -> Result<lion_core::types::ResourceUsage> {
        self.backend.get_resource_usage(plugin_id)
    }

    /// Set the capability checker.
    ///
    /// # Arguments
    ///
    /// * `checker` - The capability checker.
    pub fn set_capability_checker(&mut self, checker: Box<dyn CapabilityChecker>) {
        self.backend.set_capability_checker(checker);
    }
}
