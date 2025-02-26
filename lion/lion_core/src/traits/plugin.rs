//! Plugin management trait definitions.
//! 
//! This module defines the core traits for plugin lifecycle management.
//! The plugin system implements the concepts described in "Concurrent Plugin 
//! Lifecycle Management in Capability-Secured Microkernels" research.
//! 
//! # Plugin Lifecycle
//! 
//! The Lion microkernel manages plugins according to a well-defined lifecycle:
//! 
//! - **Created**: Plugin is registered but not yet initialized
//! - **Ready**: Plugin is initialized and ready to run
//! - **Running**: Plugin is actively processing requests
//! - **Paused**: Plugin is temporarily suspended
//! - **Failed**: Plugin has encountered an error
//! - **Terminated**: Plugin has been shut down
//! - **Upgrading**: Plugin is being hot-reloaded

use crate::error::Result;
use crate::id::{PluginId, CapabilityId};
use crate::types::{PluginConfig, PluginMetadata, PluginState, ResourceUsage};
use crate::traits::Capability;

/// Core trait for plugin management.
///
/// This trait provides an interface for managing the lifecycle of plugins,
/// including loading, unloading, starting, stopping, and upgrading.
///
/// # Examples
///
/// ```
/// use lion_core::traits::PluginManager;
/// use lion_core::id::{PluginId, CapabilityId};
/// use lion_core::types::{PluginConfig, PluginMetadata, PluginState};
/// use lion_core::error::Result;
///
/// struct DummyPluginManager;
///
/// impl PluginManager for DummyPluginManager {
///     fn load_plugin(&self, _path: &str, _config: Option<PluginConfig>) -> Result<PluginId> {
///         // In a real implementation, we would load and initialize the plugin
///         unimplemented!("Not implemented in this example")
///     }
///     
///     fn unload_plugin(&self, _id: &PluginId) -> Result<()> {
///         unimplemented!("Not implemented in this example")
///     }
///     
///     fn get_plugin_metadata(&self, _id: &PluginId) -> Result<PluginMetadata> {
///         unimplemented!("Not implemented in this example")
///     }
///     
///     fn get_plugin_state(&self, _id: &PluginId) -> Result<PluginState> {
///         unimplemented!("Not implemented in this example")
///     }
/// }
/// ```
pub trait PluginManager: Send + Sync {
    /// Load a plugin from a file path.
    ///
    /// This loads a plugin from the specified path, initializes it,
    /// and registers it with the system.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the plugin file (e.g., a WebAssembly module).
    /// * `config` - Optional configuration for the plugin.
    ///
    /// # Returns
    ///
    /// * `Ok(PluginId)` - The ID of the loaded plugin.
    /// * `Err` if the plugin could not be loaded.
    fn load_plugin(&self, path: &str, config: Option<PluginConfig>) -> Result<PluginId>;
    
    /// Load a plugin from memory.
    ///
    /// This is similar to `load_plugin`, but loads the plugin from
    /// a byte array in memory instead of a file.
    ///
    /// # Arguments
    ///
    /// * `name` - A name for the plugin, for identification purposes.
    /// * `bytes` - The plugin bytes (e.g., a WebAssembly module).
    /// * `config` - Optional configuration for the plugin.
    ///
    /// # Returns
    ///
    /// * `Ok(PluginId)` - The ID of the loaded plugin.
    /// * `Err` if the plugin could not be loaded.
    fn load_plugin_from_memory(
        &self,
        name: &str,
        bytes: &[u8],
        config: Option<PluginConfig>,
    ) -> Result<PluginId> {
        let _ = (name, bytes, config); // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Load from memory not implemented".into()))
    }
    
    /// Unload a plugin.
    ///
    /// This unloads a plugin, cleaning up any resources associated with it.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to unload.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was unloaded successfully.
    /// * `Err` if the plugin could not be unloaded.
    fn unload_plugin(&self, id: &PluginId) -> Result<()>;
    
    /// Get metadata for a plugin.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to get metadata for.
    ///
    /// # Returns
    ///
    /// * `Ok(PluginMetadata)` - The plugin metadata.
    /// * `Err` if the metadata could not be retrieved.
    fn get_plugin_metadata(&self, id: &PluginId) -> Result<PluginMetadata>;
    
    /// Get the current state of a plugin.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to get the state for.
    ///
    /// # Returns
    ///
    /// * `Ok(PluginState)` - The current state of the plugin.
    /// * `Err` if the state could not be retrieved.
    fn get_plugin_state(&self, id: &PluginId) -> Result<PluginState>;
    
    /// Start a plugin.
    ///
    /// This transitions a plugin from the Ready state to the Running state.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to start.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was started successfully.
    /// * `Err` if the plugin could not be started.
    fn start_plugin(&self, id: &PluginId) -> Result<()> {
        let _ = id; // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Start plugin not implemented".into()))
    }
    
    /// Stop a plugin.
    ///
    /// This transitions a plugin from the Running state to the Ready state.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to stop.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was stopped successfully.
    /// * `Err` if the plugin could not be stopped.
    fn stop_plugin(&self, id: &PluginId) -> Result<()> {
        let _ = id; // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Stop plugin not implemented".into()))
    }
    
    /// Pause a plugin.
    ///
    /// This temporarily suspends plugin execution, transitioning it to the Paused state.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to pause.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was paused successfully.
    /// * `Err` if the plugin could not be paused.
    fn pause_plugin(&self, id: &PluginId) -> Result<()> {
        let _ = id; // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Pause plugin not implemented".into()))
    }
    
    /// Resume a paused plugin.
    ///
    /// This resumes execution of a paused plugin, transitioning it back to the Running state.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to resume.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was resumed successfully.
    /// * `Err` if the plugin could not be resumed.
    fn resume_plugin(&self, id: &PluginId) -> Result<()> {
        let _ = id; // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Resume plugin not implemented".into()))
    }
    
    /// Update a plugin with a new version.
    ///
    /// This performs a hot reload of the plugin, replacing the code while
    /// preserving state where possible.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to update.
    /// * `path` - The path to the new plugin version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the plugin was updated successfully.
    /// * `Err` if the plugin could not be updated.
    fn update_plugin(&self, id: &PluginId, path: &str) -> Result<()> {
        let _ = (id, path); // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Update plugin not implemented".into()))
    }
    
    /// Get resource usage statistics for a plugin.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin to get resource usage for.
    ///
    /// # Returns
    ///
    /// * `Ok(ResourceUsage)` - The resource usage statistics.
    /// * `Err` if the statistics could not be retrieved.
    fn get_resource_usage(&self, id: &PluginId) -> Result<ResourceUsage> {
        let _ = id; // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Get resource usage not implemented".into()))
    }
    
    /// Call a function in a plugin.
    ///
    /// This is a convenience method that wraps the lower-level isolation backend
    /// call with plugin management concerns.
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
    /// * `Err` if the function call failed.
    fn call_function(&self, id: &PluginId, function: &str, params: &[u8]) -> Result<Vec<u8>> {
        let _ = (id, function, params); // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Call function not implemented".into()))
    }
    
    /// List all loaded plugins.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PluginId>)` - The IDs of all loaded plugins.
    /// * `Err` if the list could not be retrieved.
    fn list_plugins(&self) -> Result<Vec<PluginId>> {
        Err(crate::error::Error::NotImplemented("List plugins not implemented".into()))
    }
    
    /// Grant a capability to a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to grant the capability to.
    /// * `capability` - The capability to grant.
    ///
    /// # Returns
    ///
    /// * `Ok(CapabilityId)` - The ID of the granted capability.
    /// * `Err` if the capability could not be granted.
    fn grant_capability(
        &self,
        plugin_id: &PluginId,
        capability: Box<dyn Capability>,
    ) -> Result<CapabilityId> {
        let _ = (plugin_id, capability); // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Grant capability not implemented".into()))
    }
    
    /// Revoke a capability from a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to revoke the capability from.
    /// * `capability_id` - The ID of the capability to revoke.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the capability was revoked successfully.
    /// * `Err` if the capability could not be revoked.
    fn revoke_capability(&self, plugin_id: &PluginId, capability_id: &CapabilityId) -> Result<()> {
        let _ = (plugin_id, capability_id); // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("Revoke capability not implemented".into()))
    }
    
    /// List capabilities granted to a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to list capabilities for.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(CapabilityId, Box<dyn Capability>)>)` - The capabilities granted to the plugin.
    /// * `Err` if the capabilities could not be listed.
    fn list_capabilities(&self, plugin_id: &PluginId) -> Result<Vec<(CapabilityId, Box<dyn Capability>)>> {
        let _ = plugin_id; // Avoid unused variable warnings
        Err(crate::error::Error::NotImplemented("List capabilities not implemented".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use crate::types::{PluginType, AccessRequest};
    use chrono::Utc;
    
    // A simple capability for testing
    struct TestCapability;
    
    impl Capability for TestCapability {
        fn capability_type(&self) -> &str {
            "test"
        }
        
        fn permits(&self, _request: &AccessRequest) -> std::result::Result<(), crate::error::CapabilityError> {
            Ok(())
        }
    }
    
    // A simple plugin manager for testing
    struct TestPluginManager {
        plugins: Arc<Mutex<HashMap<PluginId, PluginMetadata>>>,
        states: Arc<Mutex<HashMap<PluginId, PluginState>>>,
        capabilities: Arc<Mutex<HashMap<PluginId, HashMap<CapabilityId, Box<dyn Capability>>>>>,
    }
    
    impl TestPluginManager {
        fn new() -> Self {
            Self {
                plugins: Arc::new(Mutex::new(HashMap::new())),
                states: Arc::new(Mutex::new(HashMap::new())),
                capabilities: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }
    
    impl PluginManager for TestPluginManager {
        fn load_plugin(&self, path: &str, config: Option<PluginConfig>) -> Result<PluginId> {
            let plugin_id = PluginId::new();
            let now = Utc::now();
            
            let metadata = PluginMetadata {
                id: plugin_id,
                name: path.to_string(),
                version: "1.0.0".to_string(),
                description: "Test plugin".to_string(),
                plugin_type: PluginType::Wasm,
                state: PluginState::Ready,
                created_at: now,
                updated_at: now,
                functions: vec!["test_function".to_string()],
            };
            
            self.plugins.lock().unwrap().insert(plugin_id, metadata);
            self.states.lock().unwrap().insert(plugin_id, PluginState::Ready);
            self.capabilities.lock().unwrap().insert(plugin_id, HashMap::new());
            
            Ok(plugin_id)
        }
        
        fn unload_plugin(&self, id: &PluginId) -> Result<()> {
            if self.plugins.lock().unwrap().remove(id).is_none() {
                return Err(crate::error::Error::Plugin(crate::error::PluginError::NotFound(*id)));
            }
            self.states.lock().unwrap().remove(id);
            self.capabilities.lock().unwrap().remove(id);
            Ok(())
        }
        
        fn get_plugin_metadata(&self, id: &PluginId) -> Result<PluginMetadata> {
            match self.plugins.lock().unwrap().get(id) {
                Some(metadata) => Ok(metadata.clone()),
                None => Err(crate::error::Error::Plugin(crate::error::PluginError::NotFound(*id))),
            }
        }
        
        fn get_plugin_state(&self, id: &PluginId) -> Result<PluginState> {
            match self.states.lock().unwrap().get(id) {
                Some(state) => Ok(*state),
                None => Err(crate::error::Error::Plugin(crate::error::PluginError::NotFound(*id))),
            }
        }
        
        fn start_plugin(&self, id: &PluginId) -> Result<()> {
            let mut states = self.states.lock().unwrap();
            if let Some(state) = states.get_mut(id) {
                if *state == PluginState::Ready {
                    *state = PluginState::Running;
                    Ok(())
                } else {
                    Err(crate::error::Error::Plugin(crate::error::PluginError::InvalidState(*state)))
                }
            } else {
                Err(crate::error::Error::Plugin(crate::error::PluginError::NotFound(*id)))
            }
        }
        
        fn stop_plugin(&self, id: &PluginId) -> Result<()> {
            let mut states = self.states.lock().unwrap();
            if let Some(state) = states.get_mut(id) {
                if *state == PluginState::Running {
                    *state = PluginState::Ready;
                    Ok(())
                } else {
                    Err(crate::error::Error::Plugin(crate::error::PluginError::InvalidState(*state)))
                }
            } else {
                Err(crate::error::Error::Plugin(crate::error::PluginError::NotFound(*id)))
            }
        }
        
        fn grant_capability(
            &self,
            plugin_id: &PluginId,
            capability: Box<dyn Capability>,
        ) -> Result<CapabilityId> {
            let capability_id = CapabilityId::new();
            let mut capabilities = self.capabilities.lock().unwrap();
            
            if let Some(plugin_capabilities) = capabilities.get_mut(plugin_id) {
                plugin_capabilities.insert(capability_id, capability);
                Ok(capability_id)
            } else {
                Err(crate::error::Error::Plugin(crate::error::PluginError::NotFound(*plugin_id)))
            }
        }
        
        fn revoke_capability(&self, plugin_id: &PluginId, capability_id: &CapabilityId) -> Result<()> {
            let mut capabilities = self.capabilities.lock().unwrap();
            
            if let Some(plugin_capabilities) = capabilities.get_mut(plugin_id) {
                if plugin_capabilities.remove(capability_id).is_none() {
                    return Err(crate::error::Error::Capability(crate::error::CapabilityError::NotFound(*capability_id)));
                }
                Ok(())
            } else {
                Err(crate::error::Error::Plugin(crate::error::PluginError::NotFound(*plugin_id)))
            }
        }
        
        fn list_capabilities(&self, plugin_id: &PluginId) -> Result<Vec<(CapabilityId, Box<dyn Capability>)>> {
            let capabilities = self.capabilities.lock().unwrap();
            
            if let Some(plugin_capabilities) = capabilities.get(plugin_id) {
                // We can't return references to the capabilities, so we need to clone them
                // This is just for testing purposes - real implementation would be different
                let result: Vec<(CapabilityId, Box<dyn Capability>)> = plugin_capabilities
                    .iter()
                    .map(|(id, _)| (*id, Box::new(TestCapability) as Box<dyn Capability>))
                    .collect();
                Ok(result)
            } else {
                Err(crate::error::Error::Plugin(crate::error::PluginError::NotFound(*plugin_id)))
            }
        }
    }
    
    #[test]
    fn test_load_unload_plugin() {
        let manager = TestPluginManager::new();
        let path = "/path/to/plugin.wasm";
        
        // Load a plugin
        let plugin_id = manager.load_plugin(path, None).unwrap();
        
        // Check that the plugin is loaded
        let metadata = manager.get_plugin_metadata(&plugin_id).unwrap();
        assert_eq!(metadata.name, path);
        assert_eq!(metadata.state, PluginState::Ready);
        
        // Unload the plugin
        manager.unload_plugin(&plugin_id).unwrap();
        
        // Check that the plugin is unloaded
        let result = manager.get_plugin_metadata(&plugin_id);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_plugin_state_transitions() {
        let manager = TestPluginManager::new();
        let path = "/path/to/plugin.wasm";
        
        // Load a plugin
        let plugin_id = manager.load_plugin(path, None).unwrap();
        
        // Check initial state
        let state = manager.get_plugin_state(&plugin_id).unwrap();
        assert_eq!(state, PluginState::Ready);
        
        // Start the plugin
        manager.start_plugin(&plugin_id).unwrap();
        
        // Check that the plugin is now running
        let state = manager.get_plugin_state(&plugin_id).unwrap();
        assert_eq!(state, PluginState::Running);
        
        // Stop the plugin
        manager.stop_plugin(&plugin_id).unwrap();
        
        // Check that the plugin is back to ready
        let state = manager.get_plugin_state(&plugin_id).unwrap();
        assert_eq!(state, PluginState::Ready);
    }
    
    #[test]
    fn test_capability_management() {
        let manager = TestPluginManager::new();
        let path = "/path/to/plugin.wasm";
        
        // Load a plugin
        let plugin_id = manager.load_plugin(path, None).unwrap();
        
        // Grant a capability
        let capability = Box::new(TestCapability);
        let capability_id = manager.grant_capability(&plugin_id, capability).unwrap();
        
        // List capabilities
        let capabilities = manager.list_capabilities(&plugin_id).unwrap();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].0, capability_id);
        
        // Revoke the capability
        manager.revoke_capability(&plugin_id, &capability_id).unwrap();
        
        // Check that the capability is revoked
        let capabilities = manager.list_capabilities(&plugin_id).unwrap();
        assert_eq!(capabilities.len(), 0);
    }
}