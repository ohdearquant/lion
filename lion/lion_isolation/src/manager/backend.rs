//! Isolation backend.
//!
//! This module provides an isolation backend for managing plugin execution.

use anyhow::Context;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, trace};

use super::lifecycle::{PluginLifecycle, PluginState};
use super::pool::{InstancePool, PooledInstance};
use crate::interface::CapabilityInterface;
use crate::resource::ResourceLimiter;
use crate::wasm::{HostCallContext, WasmEngine, WasmModule};
use lion_core::error::{IsolationError, Result};
use lion_core::types::ResourceUsage;
use lion_core::PluginId;

/// An isolation backend.
///
/// An isolation backend manages the execution of isolated plugins.
pub trait IsolationBackend: Send + Sync {
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
    fn load_plugin(&mut self, plugin_id: &PluginId, wasm_bytes: &[u8]) -> Result<()>;

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
    fn unload_plugin(&mut self, plugin_id: &PluginId) -> Result<()>;

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
    fn call_function(
        &self,
        plugin_id: &PluginId,
        function_name: &str,
        params: &[u8],
    ) -> Result<Vec<u8>>;

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
    fn get_plugin_state(&self, plugin_id: &PluginId) -> Result<PluginState>;

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
    fn get_resource_usage(&self, plugin_id: &PluginId) -> Result<ResourceUsage>;

    /// Set the capability checker.
    ///
    /// # Arguments
    ///
    /// * `checker` - The capability checker.
    fn set_capability_checker(&mut self, checker: Box<dyn crate::interface::CapabilityChecker>);
}

/// A default isolation backend.
pub struct DefaultIsolationBackend {
    /// The WebAssembly engine.
    engine: Arc<WasmEngine>,

    /// The resource limiter.
    resource_limiter: Arc<dyn ResourceLimiter>,

    /// The plugin lifecycles.
    plugin_lifecycles: DashMap<PluginId, PluginLifecycle>,

    /// The compiled modules.
    modules: DashMap<PluginId, Arc<WasmModule>>,

    /// The instance pool.
    instance_pool: Arc<Mutex<InstancePool>>,

    /// The capability interface.
    capability_interface: Arc<Mutex<CapabilityInterface>>,
}

impl DefaultIsolationBackend {
    /// Create a new default isolation backend.
    ///
    /// # Arguments
    ///
    /// * `engine` - The WebAssembly engine.
    /// * `resource_limiter` - The resource limiter.
    ///
    /// # Returns
    ///
    /// A new default isolation backend.
    pub fn new(
        engine: Arc<WasmEngine>,
        resource_limiter: Arc<dyn ResourceLimiter>,
    ) -> Result<Self> {
        let instance_pool = Arc::new(Mutex::new(InstancePool::new()));
        let capability_interface = Arc::new(Mutex::new(CapabilityInterface::new()));

        Ok(Self {
            engine,
            resource_limiter,
            plugin_lifecycles: DashMap::new(),
            modules: DashMap::new(),
            instance_pool,
            capability_interface,
        })
    }

    /// Set the capability checker.
    ///
    /// # Arguments
    ///
    /// * `checker` - The capability checker.
    pub fn set_capability_checker(&self, checker: Box<dyn crate::interface::CapabilityChecker>) {
        let mut capability_interface = self.capability_interface.lock().unwrap();
        capability_interface.set_capability_checker(checker);
    }

    /// Get a plugin lifecycle.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    ///
    /// # Returns
    ///
    /// * `Some(PluginLifecycle)` - The plugin lifecycle.
    /// * `None` - If the plugin does not exist.
    fn get_lifecycle(&self, plugin_id: &PluginId) -> Option<PluginLifecycle> {
        self.plugin_lifecycles
            .get(plugin_id)
            .map(|lifecycle| lifecycle.clone())
    }

    /// Get a plugin module.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    ///
    /// # Returns
    ///
    /// * `Some(Arc<WasmModule>)` - The plugin module.
    /// * `None` - If the plugin does not exist.
    fn get_module(&self, plugin_id: &PluginId) -> Option<Arc<WasmModule>> {
        self.modules.get(plugin_id).map(|module| module.clone())
    }
}

impl IsolationBackend for DefaultIsolationBackend {
    fn load_plugin(&mut self, plugin_id: &PluginId, wasm_bytes: &[u8]) -> Result<()> {
        // Check if plugin already exists
        if self.plugin_lifecycles.contains_key(plugin_id) {
            return Err(
                IsolationError::LoadFailed(format!("Plugin {} already exists", plugin_id)).into(),
            );
        }

        info!("Loading plugin {}", plugin_id);

        // Compile the module
        let mut module = match self.engine.compile_module(wasm_bytes) {
            Ok(m) => m,
            Err(e) => {
                return Err(IsolationError::CompilationFailed(format!(
                    "Failed to compile plugin {}: {}",
                    plugin_id, e
                ))
                .into());
            }
        };

        // Set up capability interface
        {
            let capability_interface = self.capability_interface.lock().unwrap();
            if let Err(e) = capability_interface.add_to_module(&mut module) {
                return Err(IsolationError::LinkingFailed(format!(
                    "Failed to add capability interface to plugin {}: {}",
                    plugin_id, e
                ))
                .into());
            }
        }

        // Create a module Arc
        let module_arc = Arc::new(module);

        // Create a plugin lifecycle
        let lifecycle = PluginLifecycle::new(plugin_id.clone(), PluginState::Loaded);

        // Store the module and lifecycle
        self.modules.insert(plugin_id.clone(), module_arc);
        self.plugin_lifecycles.insert(plugin_id.clone(), lifecycle);

        info!("Plugin {} loaded successfully", plugin_id);

        Ok(())
    }

    fn unload_plugin(&mut self, plugin_id: &PluginId) -> Result<()> {
        // Check if plugin exists
        if !self.plugin_lifecycles.contains_key(plugin_id) {
            return Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into());
        }

        info!("Unloading plugin {}", plugin_id);

        // Remove instances from the pool
        {
            let mut instance_pool = self.instance_pool.lock().unwrap();
            instance_pool.remove_plugin_instances(plugin_id);
        }

        // Remove the module and lifecycle
        self.modules.remove(plugin_id);
        self.plugin_lifecycles.remove(plugin_id);

        info!("Plugin {} unloaded successfully", plugin_id);

        Ok(())
    }

    fn call_function(
        &self,
        plugin_id: &PluginId,
        function_name: &str,
        params: &[u8],
    ) -> Result<Vec<u8>> {
        // Get the module
        let module = self
            .get_module(plugin_id)
            .ok_or_else(|| IsolationError::PluginNotLoaded(plugin_id.clone()))?;

        // Get or create an instance
        let mut instance_pool = self.instance_pool.lock().unwrap();
        let mut pooled_instance = instance_pool.get_or_create_instance(
            plugin_id,
            &self.engine,
            &module,
            self.resource_limiter.clone(),
        )?;

        // Get a lifecycle
        let mut lifecycle = self
            .get_lifecycle(plugin_id)
            .ok_or_else(|| IsolationError::PluginNotLoaded(plugin_id.clone()))?;

        // Check if the plugin is in a state that allows function calls
        if !lifecycle.can_call_function() {
            return Err(IsolationError::ExecutionTrap(format!(
                "Plugin {} is not in a runnable state",
                plugin_id
            ))
            .into());
        }

        // Update lifecycle state to Running if it was Loaded
        if lifecycle.state() == PluginState::Loaded {
            lifecycle.transition_to(PluginState::Running);
            if let Some(mut entry) = self.plugin_lifecycles.get_mut(plugin_id) {
                *entry = lifecycle.clone();
            }
        }

        // Call the function
        let result = pooled_instance.call_function(function_name, params)?;

        // Return the instance to the pool
        instance_pool.return_instance(pooled_instance);

        Ok(result)
    }

    fn get_plugin_state(&self, plugin_id: &PluginId) -> Result<PluginState> {
        let lifecycle = self
            .get_lifecycle(plugin_id)
            .ok_or_else(|| IsolationError::PluginNotLoaded(plugin_id.clone()))?;

        Ok(lifecycle.state())
    }

    fn get_resource_usage(&self, plugin_id: &PluginId) -> Result<ResourceUsage> {
        // Get an instance from the pool
        let instance_pool = self.instance_pool.lock().unwrap();

        // Get aggregated resource usage for the plugin
        let usage = instance_pool
            .get_plugin_resource_usage(plugin_id)
            .ok_or_else(|| IsolationError::PluginNotLoaded(plugin_id.clone()))?;

        Ok(usage)
    }

    fn set_capability_checker(&mut self, checker: Box<dyn crate::interface::CapabilityChecker>) {
        let mut capability_interface = self.capability_interface.lock().unwrap();
        capability_interface.set_capability_checker(checker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::DefaultResourceLimiter;

    fn create_test_backend() -> DefaultIsolationBackend {
        let engine = Arc::new(WasmEngine::default().unwrap());
        let resource_limiter = Arc::new(DefaultResourceLimiter::default());

        DefaultIsolationBackend::new(engine, resource_limiter).unwrap()
    }

    #[test]
    fn test_load_unload_plugin() {
        // Create a simple WebAssembly module (using a dummy empty array for testing)
        const WASM: &[u8] = &[0, 97, 115, 109, 1, 0, 0, 0]; // Minimal valid WebAssembly module header

        // Create a backend
        let mut backend = create_test_backend();

        // Load the plugin
        let plugin_id = PluginId::new();
        backend.load_plugin(&plugin_id, WASM).unwrap();

        // Check that the plugin is loaded
        assert!(backend.plugin_lifecycles.contains_key(&plugin_id));
        assert!(backend.modules.contains_key(&plugin_id));

        // Check plugin state
        let state = backend.get_plugin_state(&plugin_id).unwrap();
        assert_eq!(state, PluginState::Loaded);

        // Unload the plugin
        backend.unload_plugin(&plugin_id).unwrap();

        // Check that the plugin is unloaded
        assert!(!backend.plugin_lifecycles.contains_key(&plugin_id));
        assert!(!backend.modules.contains_key(&plugin_id));
    }
}
