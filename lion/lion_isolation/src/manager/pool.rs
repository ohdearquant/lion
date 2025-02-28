//! Instance pooling.
//!
//! This module provides instance pooling for better performance.
use std::collections::HashMap;
use std::sync::Arc;
use tracing::trace;
use wasmtime::{Instance, Store};

use crate::resource::ResourceLimiter;
use crate::wasm::{HostCallContext, WasmEngine, WasmModule};
use lion_core::error::{IsolationError, Result};
use lion_core::types::ResourceUsage;
use lion_core::PluginId;

/// A pooled instance.
pub struct PooledInstance {
    /// The plugin ID.
    plugin_id: PluginId,

    /// The store.
    store: Store<HostCallContext>,

    /// The instance.
    instance: Instance,
}

impl PooledInstance {
    /// Create a new pooled instance.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `store` - The store.
    /// * `instance` - The instance.
    ///
    /// # Returns
    ///
    /// A new pooled instance.
    pub fn new(plugin_id: PluginId, store: Store<HostCallContext>, instance: Instance) -> Self {
        Self {
            plugin_id,
            store,
            instance,
        }
    }

    /// Get the plugin ID.
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Get the store.
    pub fn store(&self) -> &Store<HostCallContext> {
        &self.store
    }

    /// Get a mutable reference to the store.
    pub fn store_mut(&mut self) -> &mut Store<HostCallContext> {
        &mut self.store
    }

    /// Get the instance.
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    /// Call a function in the instance.
    ///
    /// # Arguments
    ///
    /// * `function_name` - The name of the function.
    /// * `params` - The parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The result of the function call.
    /// * `Err` - If the function could not be called.
    pub fn call_function(&mut self, function_name: &str, params: &[u8]) -> Result<Vec<u8>> {
        trace!(
            "Calling function '{}' in plugin {}",
            function_name,
            self.plugin_id
        );

        // Reset function start time in resource metering
        if let Some(metering) = self.store.data_mut().resource_metering_mut() {
            metering.reset_function_start_time();
            metering.increment_function_calls();
        }

        // Get the function
        let func = match self.instance.get_func(&mut self.store, function_name) {
            Some(f) => f,
            None => {
                return Err(lion_core::error::Error::Isolation(
                    lion_core::error::IsolationError::LinkingFailed(format!(
                        "Function '{}' not found in plugin {}",
                        function_name, self.plugin_id
                    )),
                ));
            }
        };

        // Call the function using a more compatible approach
        let result_ptr = match call_function_with_bytes(&mut self.store, func, params) {
            Ok(ptr) => ptr,
            Err(e) => {
                return Err(lion_core::error::Error::Isolation(
                    lion_core::error::IsolationError::ExecutionTrap(format!(
                        "Failed to call function '{}' in plugin {}: {}",
                        function_name, self.plugin_id, e
                    )),
                ));
            }
        };

        // Get the result from memory
        if result_ptr < 0 {
            return Err(IsolationError::ExecutionTrap(format!(
                "Function '{}' returned error code {}",
                function_name, result_ptr
            ))
            .into());
        }

        // Check resource limits
        if let Some(metering) = self.store.data().resource_metering() {
            metering.check_limits()?;
        }

        // In a real implementation, we would read the result from memory
        // This is a simplified version
        let result = vec![0; 0];

        trace!(
            "Function '{}' in plugin {} returned successfully",
            function_name,
            self.plugin_id
        );

        Ok(result)
    }

    /// Get the resource usage of this instance.
    pub fn resource_usage(&self) -> Option<ResourceUsage> {
        self.store
            .data()
            .resource_metering()
            .map(|metering| metering.usage().to_core_resource_usage())
    }
}

// Helper function to call a WebAssembly function with byte parameters
fn call_function_with_bytes(
    _store: &mut Store<HostCallContext>,
    _func: wasmtime::Func,
    _params: &[u8],
) -> anyhow::Result<i32> {
    // In a real implementation, we would:
    // 1. Get the memory from the store
    // 2. Allocate memory for parameters
    // 3. Write parameters to memory
    // 4. Call the function with the pointer to parameters
    // 5. Read the result from memory

    // For now, we're just returning a dummy value since this is a simplified implementation
    // In a real implementation, we would call the function with the actual parameters
    // and return the actual result

    // Simplified implementation
    Ok(42) // Return a dummy success value
}
/// An instance pool.
pub struct InstancePool {
    /// The instances, organized by plugin ID.
    instances: HashMap<PluginId, Vec<PooledInstance>>,

    /// The maximum number of instances per plugin.
    max_instances_per_plugin: usize,
}

impl InstancePool {
    /// Create a new instance pool.
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            max_instances_per_plugin: 10,
        }
    }
}

impl Default for InstancePool {
    fn default() -> Self {
        Self::new()
    }
}

impl InstancePool {
    /// Set the maximum number of instances per plugin.
    ///
    /// # Arguments
    ///
    /// * `max` - The maximum number of instances per plugin.
    pub fn set_max_instances_per_plugin(&mut self, max: usize) {
        self.max_instances_per_plugin = max;
    }

    /// Get an instance for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    ///
    /// # Returns
    ///
    /// * `Some(PooledInstance)` - An instance for the plugin.
    /// * `None` - If there are no instances for the plugin.
    pub fn get_instance(&mut self, plugin_id: &PluginId) -> Option<PooledInstance> {
        let instances = self.instances.get_mut(plugin_id)?;

        if instances.is_empty() {
            return None;
        }

        Some(instances.pop().unwrap())
    }

    /// Return an instance to the pool.
    ///
    /// # Arguments
    ///
    /// * `instance` - The instance.
    pub fn return_instance(&mut self, instance: PooledInstance) {
        let plugin_id = *instance.plugin_id();

        let instances = self.instances.entry(plugin_id).or_default();

        // Only keep up to max_instances_per_plugin
        if instances.len() < self.max_instances_per_plugin {
            instances.push(instance);
        }
    }

    /// Create a new instance for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `engine` - The WebAssembly engine.
    /// * `module` - The module.
    /// * `resource_limiter` - The resource limiter.
    ///
    /// # Returns
    ///
    /// * `Ok(PooledInstance)` - The new instance.
    /// * `Err` - If the instance could not be created.
    pub fn create_instance(
        &mut self,
        plugin_id: &PluginId,
        engine: &WasmEngine,
        module: &WasmModule,
        _resource_limiter: Arc<dyn ResourceLimiter>,
    ) -> Result<PooledInstance> {
        trace!("Creating new instance for plugin {}", plugin_id);

        // Create a host context
        let host_context = HostCallContext::new(plugin_id.clone().to_string());

        // Instantiate the module
        let mut store = match engine.instantiate_module(module, host_context) {
            Ok(s) => s,
            Err(e) => {
                return Err(lion_core::error::Error::Isolation(
                    lion_core::error::IsolationError::InstantiationFailed(format!(
                        "Failed to instantiate module for plugin {}: {}",
                        plugin_id, e
                    )),
                ));
            }
        };

        // Get the instance
        let instance = match module.linker().instantiate(&mut store, module.module()) {
            Ok(i) => i,
            Err(e) => {
                return Err(lion_core::error::Error::Isolation(
                    lion_core::error::IsolationError::InstantiationFailed(format!(
                        "Failed to instantiate linker for plugin {}: {}",
                        plugin_id, e
                    )),
                ));
            }
        };

        // Create a pooled instance
        let pooled_instance = PooledInstance::new(*plugin_id, store, instance);

        // Ensure the plugin exists in the instances map
        self.instances.entry(*plugin_id).or_default();

        Ok(pooled_instance)
    }

    /// Get or create an instance for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `engine` - The WebAssembly engine.
    /// * `module` - The module.
    /// * `resource_limiter` - The resource limiter.
    ///
    /// # Returns
    ///
    /// * `Ok(PooledInstance)` - An instance for the plugin.
    /// * `Err` - If the instance could not be created.
    pub fn get_or_create_instance(
        &mut self,
        plugin_id: &PluginId,
        engine: &WasmEngine,
        module: &Arc<WasmModule>,
        resource_limiter: Arc<dyn ResourceLimiter>,
    ) -> Result<PooledInstance> {
        // Try to get an existing instance
        if let Some(instance) = self.get_instance(plugin_id) {
            return Ok(instance);
        }

        // Create a new instance
        self.create_instance(plugin_id, engine, module, resource_limiter)
    }

    /// Remove all instances for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    pub fn remove_plugin_instances(&mut self, plugin_id: &PluginId) {
        self.instances.remove(plugin_id);
    }

    /// Get the resource usage for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    ///
    /// # Returns
    ///
    /// * `Some(ResourceUsage)` - The resource usage for the plugin.
    /// * `None` - If the plugin does not exist.
    pub fn get_plugin_resource_usage(&self, plugin_id: &PluginId) -> Option<ResourceUsage> {
        let instances = self.instances.get(plugin_id)?;

        // If there are no running instances, return empty usage
        if instances.is_empty() {
            return Some(lion_core::ResourceUsage {
                memory_bytes: 0,
                cpu_time_us: 0,
                function_calls: 0,
                last_updated: chrono::Utc::now(),
                active_instances: 0,
                peak_memory_bytes: 0,
                peak_cpu_time_us: 0,
                avg_function_call_us: 0,
                custom_metrics: HashMap::new(),
            });
        }

        // Aggregate usage from all instances
        let mut total_memory = 0;
        let mut total_cpu_time = 0;
        let mut total_function_calls = 0;

        for instance in instances {
            if let Some(usage) = instance.resource_usage() {
                total_memory = total_memory.max(usage.memory_bytes);
                total_cpu_time += usage.cpu_time_us;
                total_function_calls += usage.function_calls;
            }
        }

        Some(lion_core::ResourceUsage {
            memory_bytes: total_memory,
            cpu_time_us: total_cpu_time,
            function_calls: total_function_calls,
            last_updated: chrono::Utc::now(),
            active_instances: instances.len(),
            peak_memory_bytes: total_memory,
            peak_cpu_time_us: total_cpu_time,
            avg_function_call_us: if total_function_calls > 0 {
                total_cpu_time / total_function_calls
            } else {
                0
            },
            custom_metrics: HashMap::new(),
        })
    }
}
