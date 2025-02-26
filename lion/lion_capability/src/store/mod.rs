//! Capability storage.
//! 
//! This module provides storage for capabilities.

mod in_memory;
mod partial_revocation;

pub use in_memory::InMemoryCapabilityStore;
pub use partial_revocation::PartialRevocation;

use lion_core::error::Result;
use lion_core::id::{PluginId, CapabilityId};
use crate::model::Capability;

/// Trait for capability storage.
///
/// A capability store is responsible for storing and retrieving capabilities.
pub trait CapabilityStore: Send + Sync {
    /// Add a capability to the store.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin that owns the capability.
    /// * `capability` - The capability to add.
    ///
    /// # Returns
    ///
    /// * `Ok(CapabilityId)` - The ID of the added capability.
    /// * `Err` - If the capability could not be added.
    fn add_capability(&self, plugin_id: PluginId, capability: Box<dyn Capability>) -> Result<CapabilityId>;
    
    /// Get a capability from the store.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin that owns the capability.
    /// * `capability_id` - The ID of the capability to get.
    ///
    /// # Returns
    ///
    /// * `Ok(Box<dyn Capability>)` - The capability.
    /// * `Err` - If the capability could not be found.
    fn get_capability(&self, plugin_id: &PluginId, capability_id: &CapabilityId) -> Result<Box<dyn Capability>>;
    
    /// Remove a capability from the store.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin that owns the capability.
    /// * `capability_id` - The ID of the capability to remove.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the capability was successfully removed.
    /// * `Err` - If the capability could not be removed.
    fn remove_capability(&self, plugin_id: &PluginId, capability_id: &CapabilityId) -> Result<()>;
    
    /// Replace a capability in the store.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin that owns the capability.
    /// * `capability_id` - The ID of the capability to replace.
    /// * `new_capability` - The new capability.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the capability was successfully replaced.
    /// * `Err` - If the capability could not be replaced.
    fn replace_capability(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
        new_capability: Box<dyn Capability>,
    ) -> Result<()>;
    
    /// List all capabilities for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to list capabilities for.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(CapabilityId, Box<dyn Capability>)>)` - The capabilities.
    /// * `Err` - If the capabilities could not be listed.
    fn list_capabilities(&self, plugin_id: &PluginId) -> Result<Vec<(CapabilityId, Box<dyn Capability>)>>;
    
    /// Clear all capabilities for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to clear capabilities for.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the capabilities were successfully cleared.
    /// * `Err` - If the capabilities could not be cleared.
    fn clear_plugin_capabilities(&self, plugin_id: &PluginId) -> Result<()>;
}