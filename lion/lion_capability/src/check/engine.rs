//! Capability checking engine.
//! 
//! This module provides the capability checker engine.

use lion_core::error::{Result, CapabilityError};
use lion_core::id::PluginId;
use lion_core::types::AccessRequest;

use crate::store::CapabilityStore;

/// Capability checker engine.
///
/// This engine checks if a plugin has the capability to perform a given access.
#[derive(Clone)]
pub struct CapabilityChecker<S> {
    /// The capability store.
    store: S,
}

impl<S: CapabilityStore> CapabilityChecker<S> {
    /// Create a new capability checker.
    ///
    /// # Arguments
    ///
    /// * `store` - The capability store.
    ///
    /// # Returns
    ///
    /// A new capability checker.
    pub fn new(store: S) -> Self {
        Self { store }
    }
    
    /// Check if a plugin has the capability to perform a given access.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to check.
    /// * `request` - The access request to check.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the plugin has the capability.
    /// * `Err` - If the plugin does not have the capability.
    pub fn check(&self, plugin_id: &PluginId, request: &AccessRequest) -> Result<()> {
        // Get the capabilities for the plugin
        let capabilities = self.store.list_capabilities(plugin_id)?;
        
        // Check if any capability permits the access
        for (_, capability) in capabilities {
            if capability.permits(request).is_ok() {
                return Ok(());
            }
        }
        
        // No capability permitted the access
        Err(CapabilityError::PermissionDenied(
            format!("No capability permits {:?}", request)
        ).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::FileCapability;
    use crate::store::InMemoryCapabilityStore;
    use std::path::PathBuf;
    
    #[test]
    fn test_capability_checker() {
        let store = InMemoryCapabilityStore::new();
        let checker = CapabilityChecker::new(store.clone());
        let plugin_id = PluginId::new();
        
        // Add a capability
        let capability = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        store.add_capability(plugin_id.clone(), capability).unwrap();
        
        // Check for read access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(checker.check(&plugin_id, &request).is_ok());
        
        // Check for write access to /tmp (should fail)
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(checker.check(&plugin_id, &request).is_err());
        
        // Check for read access to /etc (should fail)
        let request = AccessRequest::File {
            path: PathBuf::from("/etc/passwd"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(checker.check(&plugin_id, &request).is_err());
    }
    
    #[test]
    fn test_capability_checker_multiple_capabilities() {
        let store = InMemoryCapabilityStore::new();
        let checker = CapabilityChecker::new(store.clone());
        let plugin_id = PluginId::new();
        
        // Add capabilities
        let capability1 = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        let capability2 = Box::new(FileCapability::write_only(vec![PathBuf::from("/var")]));
        store.add_capability(plugin_id.clone(), capability1).unwrap();
        store.add_capability(plugin_id.clone(), capability2).unwrap();
        
        // Check for read access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(checker.check(&plugin_id, &request).is_ok());
        
        // Check for write access to /var
        let request = AccessRequest::File {
            path: PathBuf::from("/var/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(checker.check(&plugin_id, &request).is_ok());
        
        // Check for write access to /tmp (should fail)
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(checker.check(&plugin_id, &request).is_err());
        
        // Check for read access to /var (should fail)
        let request = AccessRequest::File {
            path: PathBuf::from("/var/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(checker.check(&plugin_id, &request).is_err());
    }
}