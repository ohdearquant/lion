//! Implementation of the capability manager.

use crate::error::CapabilityManagerError;
use crate::policy::CapabilityPolicy;
use dashmap::DashMap;
use lion_core::capability::{Capability, CapabilityId, CapabilityManager, CoreCapability};
use lion_core::plugin::PluginId;
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for the capability manager implementation
#[derive(Clone)]
pub struct CapabilityManagerImplConfig {
    /// The capability policy to use
    pub policy: Arc<dyn CapabilityPolicy>,
}

impl Default for CapabilityManagerImplConfig {
    fn default() -> Self {
        Self {
            policy: Arc::new(crate::policy::DefaultCapabilityPolicy::default()),
        }
    }
}

/// Implementation of the capability manager
pub struct CapabilityManagerImpl {
    /// The capability policy to use
    policy: Arc<dyn CapabilityPolicy>,
    
    /// Map of plugin IDs to their granted capabilities
    plugin_capabilities: DashMap<PluginId, Vec<Capability>>,
}

impl CapabilityManagerImpl {
    /// Create a new capability manager implementation
    pub fn new(config: CapabilityManagerImplConfig) -> Self {
        Self {
            policy: config.policy,
            plugin_capabilities: DashMap::new(),
        }
    }
    
    /// Create a new capability manager with the default configuration
    pub fn default_manager() -> Arc<dyn CapabilityManager> {
        Arc::new(Self::new(CapabilityManagerImplConfig::default()))
    }
}

impl CapabilityManager for CapabilityManagerImpl {
    fn has_capability(&self, plugin_id: PluginId, capability: &CoreCapability) -> bool {
        // Check if the plugin has the capability
        if let Some(capabilities) = self.plugin_capabilities.get(&plugin_id) {
            for cap in capabilities.iter() {
                match (&cap.capability_type, capability) {
                    (
                        CoreCapability::FileSystemRead { path: cap_path },
                        CoreCapability::FileSystemRead { path: req_path },
                    ) => {
                        // Allow if the capability path is None (any path) or the requested path
                        // is under the capability path
                        match (cap_path, req_path) {
                            (None, _) => return true,
                            (Some(_), None) => return true,
                            (Some(cap), Some(req)) => {
                                let cap = std::path::PathBuf::from(cap);
                                let req = std::path::PathBuf::from(req);
                                if req.starts_with(cap) {
                                    return true;
                                }
                            }
                        }
                    }
                    (
                        CoreCapability::FileSystemWrite { path: cap_path },
                        CoreCapability::FileSystemWrite { path: req_path },
                    ) => {
                        // Allow if the capability path is None (any path) or the requested path
                        // is under the capability path
                        match (cap_path, req_path) {
                            (None, _) => return true,
                            (Some(_), None) => return true,
                            (Some(cap), Some(req)) => {
                                let cap = std::path::PathBuf::from(cap);
                                let req = std::path::PathBuf::from(req);
                                if req.starts_with(cap) {
                                    return true;
                                }
                            }
                        }
                    }
                    (
                        CoreCapability::NetworkClient { hosts: cap_hosts },
                        CoreCapability::NetworkClient { hosts: req_hosts },
                    ) => {
                        // Allow if the capability hosts is None (any host) or all requested hosts
                        // are in the capability hosts
                        match (cap_hosts, req_hosts) {
                            (None, _) => return true,
                            (Some(_), None) => return true,
                            (Some(cap), Some(req)) => {
                                let all_allowed = req.iter().all(|req_host| {
                                    cap.iter().any(|cap_host| {
                                        // Exact match or wildcard
                                        req_host == cap_host || 
                                        (cap_host.starts_with("*.") && req_host.ends_with(&cap_host[1..]))
                                    })
                                });
                                if all_allowed {
                                    return true;
                                }
                            }
                        }
                    }
                    (CoreCapability::InterPluginComm, CoreCapability::InterPluginComm) => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
        
        false
    }
    
    fn grant_capability(
        &self,
        plugin_id: PluginId,
        capability: CoreCapability,
    ) -> Result<CapabilityId, lion_core::error::CapabilityError> {
        // Check if the plugin is allowed to have this capability
        if !self.policy.can_grant_capability(plugin_id, &capability) {
            let reason = self.policy.get_denial_reason(plugin_id, &capability)
                .unwrap_or_else(|| "Capability not allowed by policy".to_string());
            return Err(lion_core::error::CapabilityError::PermissionDenied);
        }
        
        // Check if the plugin already has this capability
        if self.has_capability(plugin_id, &capability) {
            return Err(lion_core::error::CapabilityError::AlreadyGranted);
        }
        
        // Create a new capability
        let capability_id = CapabilityId::new();
        let capability = Capability {
            id: capability_id,
            capability_type: capability.clone(),
            description: None,
        };
        
        // Add the capability to the plugin
        self.plugin_capabilities
            .entry(plugin_id)
            .or_insert_with(Vec::new)
            .push(capability);
        
        Ok(capability_id)
    }
    
    fn revoke_capability(
        &self,
        plugin_id: PluginId,
        capability_id: CapabilityId,
    ) -> Result<(), lion_core::error::CapabilityError> {
        // Check if the plugin has the capability
        if let Some(mut capabilities) = self.plugin_capabilities.get_mut(&plugin_id) {
            let index = capabilities
                .iter()
                .position(|cap| cap.id == capability_id);
            
            if let Some(index) = index {
                capabilities.remove(index);
                return Ok(());
            }
        }
        
        Err(lion_core::error::CapabilityError::NotGranted)
    }
    
    fn list_capabilities(&self, plugin_id: PluginId) -> Vec<Capability> {
        self.plugin_capabilities
            .get(&plugin_id)
            .map(|capabilities| capabilities.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::DefaultCapabilityPolicy;
    
    #[test]
    fn test_has_capability() {
        // Create a policy that allows everything
        let policy = Arc::new(DefaultCapabilityPolicy::new(true));
        
        // Create a capability manager
        let config = CapabilityManagerImplConfig { policy };
        let manager = CapabilityManagerImpl::new(config);
        
        // Create a plugin ID
        let plugin_id = PluginId::new();
        
        // The plugin shouldn't have any capabilities yet
        assert!(!manager.has_capability(
            plugin_id,
            &CoreCapability::FileSystemRead { path: None }
        ));
        
        // Grant a capability
        let capability_id = manager
            .grant_capability(plugin_id, CoreCapability::FileSystemRead { path: None })
            .unwrap();
        
        // Now the plugin should have the capability
        assert!(manager.has_capability(
            plugin_id,
            &CoreCapability::FileSystemRead { path: None }
        ));
        
        // Revoke the capability
        manager.revoke_capability(plugin_id, capability_id).unwrap();
        
        // The plugin shouldn't have the capability anymore
        assert!(!manager.has_capability(
            plugin_id,
            &CoreCapability::FileSystemRead { path: None }
        ));
    }
    
    #[test]
    fn test_grant_capability() {
        // Create a policy that allows everything
        let policy = Arc::new(DefaultCapabilityPolicy::new(true));
        
        // Create a capability manager
        let config = CapabilityManagerImplConfig { policy };
        let manager = CapabilityManagerImpl::new(config);
        
        // Create a plugin ID
        let plugin_id = PluginId::new();
        
        // Grant a capability
        let capability_id = manager
            .grant_capability(plugin_id, CoreCapability::FileSystemRead { path: None })
            .unwrap();
        
        // Verify that the capability was granted
        let capabilities = manager.list_capabilities(plugin_id);
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].id, capability_id);
        
        // Try to grant the same capability again
        let result = manager.grant_capability(
            plugin_id,
            CoreCapability::FileSystemRead { path: None }
        );
        
        // Should fail because the capability is already granted
        assert!(result.is_err());
    }
    
    #[test]
    fn test_revoke_capability() {
        // Create a policy that allows everything
        let policy = Arc::new(DefaultCapabilityPolicy::new(true));
        
        // Create a capability manager
        let config = CapabilityManagerImplConfig { policy };
        let manager = CapabilityManagerImpl::new(config);
        
        // Create a plugin ID
        let plugin_id = PluginId::new();
        
        // Grant a capability
        let capability_id = manager
            .grant_capability(plugin_id, CoreCapability::FileSystemRead { path: None })
            .unwrap();
        
        // Revoke the capability
        let result = manager.revoke_capability(plugin_id, capability_id);
        
        // Should succeed
        assert!(result.is_ok());
        
        // Try to revoke the capability again
        let result = manager.revoke_capability(plugin_id, capability_id);
        
        // Should fail because the capability is not granted
        assert!(result.is_err());
    }
}