mod in_memory;
mod partial_revocation;

use lion_core::id::{CapabilityId, PluginId};

use crate::model::{AccessRequest, Capability, CapabilityError};

/// A store of capabilities, used to track which capabilities are assigned to each plugin
pub trait CapabilityStore: Send + Sync {
    /// Add a capability to a plugin, returning the generated capability ID
    fn add_capability(
        &self,
        plugin_id: PluginId,
        capability: Box<dyn Capability>,
    ) -> Result<CapabilityId, CapabilityError>;

    /// Get a capability by plugin ID and capability ID
    fn get_capability(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
    ) -> Result<Box<dyn Capability>, CapabilityError>;

    /// Remove a capability from a plugin
    fn remove_capability(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
    ) -> Result<(), CapabilityError>;

    /// Replace a capability with a new one
    fn replace_capability(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
        new_cap: Box<dyn Capability>,
    ) -> Result<(), CapabilityError>;

    /// List all capabilities for a plugin
    fn list_capabilities(
        &self,
        plugin_id: &PluginId,
    ) -> Result<Vec<(CapabilityId, Box<dyn Capability>)>, CapabilityError>;

    /// Clear all capabilities for a plugin
    fn clear_plugin_capabilities(&self, plugin_id: &PluginId) -> Result<(), CapabilityError>;

    /// Check if a plugin has permission for a specific access request
    ///
    /// This is a convenience method that checks all capabilities for the plugin
    /// and returns Ok if any of them permit the request
    fn check_permission(
        &self,
        plugin_id: &PluginId,
        request: &AccessRequest,
    ) -> Result<(), CapabilityError> {
        // Get all capabilities for this plugin
        let capabilities = self.list_capabilities(plugin_id)?;

        // Check if any of them permit this request
        for (_, cap) in capabilities {
            if cap.permits(request).is_ok() {
                return Ok(());
            }
        }

        // None of the capabilities permitted this request
        Err(CapabilityError::AccessDenied(format!(
            "Plugin {} does not have permission for {:?}",
            plugin_id, request
        )))
    }

    /// Apply partial revocation for a capability
    ///
    /// This revokes specific access from a capability by:
    /// 1. Splitting the capability into sub-capabilities
    /// 2. Removing the parts that would permit the specified access
    /// 3. Joining the remaining parts back together
    /// 4. Replacing the original capability with this reduced version
    fn partial_revoke(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
        request: &AccessRequest,
    ) -> Result<(), CapabilityError>;
}

/// A capability reference, used to identify a capability in the store
/// with generational index for safer revocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityRef {
    /// ID of the capability
    pub id: CapabilityId,
    /// Generation of the capability
    pub generation: u64,
}

pub use in_memory::InMemoryCapabilityStore;
pub use partial_revocation::apply_partial_revocation;
