//! Partial capability revocation.
//! 
//! This module provides functionality for partially revoking capabilities.

use lion_core::error::{Result, CapabilityError};
use lion_core::id::{PluginId, CapabilityId};
use lion_core::types::AccessRequest;

use crate::model::Capability;
use crate::store::CapabilityStore;

/// Trait for partial capability revocation.
pub trait PartialRevocation {
    /// Partially revoke a capability.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin that owns the capability.
    /// * `capability_id` - The ID of the capability to revoke.
    /// * `revocation` - The access request that should no longer be permitted.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the capability was successfully revoked.
    /// * `Err` - If the capability could not be revoked.
    fn partial_revoke(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
        revocation: &AccessRequest,
    ) -> Result<()>;
}

impl<T: CapabilityStore> PartialRevocation for T {
    fn partial_revoke(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
        revocation: &AccessRequest,
    ) -> Result<()> {
        // Get the original capability
        let old_capability = self.get_capability(plugin_id, capability_id)?;
        
        // Check if the capability permits the revocation request
        if old_capability.permits(revocation).is_err() {
            // The capability already doesn't permit this request
            return Ok(());
        }
        
        // Split the capability into constituent parts
        let mut parts = old_capability.split();
        
        // Filter out parts that permit the revocation request
        parts.retain(|part| part.permits(revocation).is_err());
        
        // If no parts remain, the capability would be completely revoked
        if parts.is_empty() {
            return Err(CapabilityError::RevocationFailed(
                "Cannot completely revoke a capability with partial revocation".into()
            ).into());
        }
        
        // Create a new composite capability from the remaining parts
        let mut new_capability = parts.remove(0);
        for part in parts {
            new_capability = new_capability.join(&*part)?;
        }
        
        // Replace the old capability with the new one
        self.replace_capability(plugin_id, capability_id, new_capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::FileCapability;
    use crate::store::InMemoryCapabilityStore;
    use std::path::PathBuf;
    
    #[test]
    fn test_partial_revoke() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = PluginId::new();
        let capability = Box::new(FileCapability::new(
            vec![PathBuf::from("/tmp"), PathBuf::from("/var")],
            true,
            true,
            false,
        ));
        
        // Add the capability
        let capability_id = store.add_capability(plugin_id.clone(), capability).unwrap();
        
        // Revoke write access to /tmp
        let revocation = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        store.partial_revoke(&plugin_id, &capability_id, &revocation).unwrap();
        
        // Get the capability
        let retrieved = store.get_capability(&plugin_id, &capability_id).unwrap();
        
        // Check that it still permits read access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(retrieved.permits(&request).is_ok());
        
        // Check that it no longer permits write access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(retrieved.permits(&request).is_err());
        
        // Check that it still permits read and write access to /var
        let request = AccessRequest::File {
            path: PathBuf::from("/var/file"),
            read: true,
            write: true,
            execute: false,
        };
        assert!(retrieved.permits(&request).is_ok());
    }
    
    #[test]
    fn test_partial_revoke_already_revoked() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = PluginId::new();
        let capability = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        
        // Add the capability
        let capability_id = store.add_capability(plugin_id.clone(), capability).unwrap();
        
        // Try to revoke write access to /tmp (which is already not permitted)
        let revocation = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        store.partial_revoke(&plugin_id, &capability_id, &revocation).unwrap();
        
        // Get the capability
        let retrieved = store.get_capability(&plugin_id, &capability_id).unwrap();
        
        // Check that it still permits read access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(retrieved.permits(&request).is_ok());
    }
    
    #[test]
    fn test_partial_revoke_complete_revocation() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = PluginId::new();
        let capability = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        
        // Add the capability
        let capability_id = store.add_capability(plugin_id.clone(), capability).unwrap();
        
        // Try to revoke read access to /tmp (which would completely revoke the capability)
        let revocation = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        
        // This should fail
        let result = store.partial_revoke(&plugin_id, &capability_id, &revocation);
        assert!(result.is_err());
    }
}