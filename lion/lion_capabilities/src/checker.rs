//! Centralized capability checking functions.
//!
//! This module provides helper functions for checking capabilities
//! that can be used by various subsystems (message bus, isolation backend, etc.)
//! to ensure consistent capability enforcement.

use lion_core::capability::{CapabilityManager, CoreCapability};
use lion_core::plugin::PluginId;
use std::path::{Path, PathBuf};

/// Check if a plugin has a specific capability
pub fn check_capability(
    plugin_id: PluginId,
    capability: &CoreCapability,
    capability_manager: &dyn CapabilityManager,
) -> Result<(), lion_core::error::CapabilityError> {
    if !capability_manager.has_capability(plugin_id, capability) {
        return Err(lion_core::error::CapabilityError::NotGranted);
    }
    Ok(())
}

/// Check if a plugin has file system read capability for a specific path
pub fn check_fs_read<P: AsRef<Path>>(
    plugin_id: PluginId,
    path: P,
    capability_manager: &dyn CapabilityManager,
) -> Result<(), lion_core::error::CapabilityError> {
    let path_str = path.as_ref().to_string_lossy().to_string();
    let capability = CoreCapability::FileSystemRead { path: Some(path_str) };
    check_capability(plugin_id, &capability, capability_manager)
}

/// Check if a plugin has file system write capability for a specific path
pub fn check_fs_write<P: AsRef<Path>>(
    plugin_id: PluginId,
    path: P,
    capability_manager: &dyn CapabilityManager,
) -> Result<(), lion_core::error::CapabilityError> {
    let path_str = path.as_ref().to_string_lossy().to_string();
    let capability = CoreCapability::FileSystemWrite { path: Some(path_str) };
    check_capability(plugin_id, &capability, capability_manager)
}

/// Check if a plugin has network capability for specific hosts
pub fn check_network(
    plugin_id: PluginId,
    hosts: &[String],
    capability_manager: &dyn CapabilityManager,
) -> Result<(), lion_core::error::CapabilityError> {
    let capability = CoreCapability::NetworkClient { hosts: Some(hosts.to_vec()) };
    check_capability(plugin_id, &capability, capability_manager)
}

/// Check if a plugin has inter-plugin communication capability
pub fn check_interplugin_comm(
    plugin_id: PluginId,
    capability_manager: &dyn CapabilityManager,
) -> Result<(), lion_core::error::CapabilityError> {
    let capability = CoreCapability::InterPluginComm;
    check_capability(plugin_id, &capability, capability_manager)
}

/// Normalize and validate a file system path
pub fn normalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, lion_core::error::CapabilityError> {
    let path = path.as_ref();
    
    // Check for path traversal attacks
    if path.components().any(|c| c.as_os_str() == "..") {
        return Err(lion_core::error::CapabilityError::PermissionDenied);
    }
    
    // Normalize path
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => return Err(lion_core::error::CapabilityError::OperationFailed(
                "Failed to get current directory".to_string()
            )),
        }
    };
    
    Ok(abs_path)
}

/// Validate a network host or URL
pub fn validate_host(host: &str) -> Result<(), lion_core::error::CapabilityError> {
    // Basic validation
    if host.is_empty() {
        return Err(lion_core::error::CapabilityError::OperationFailed(
            "Host cannot be empty".to_string()
        ));
    }
    
    // Additional validations could be added here
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::{CapabilityManagerImpl, CapabilityManagerImplConfig};
    use crate::policy::DefaultCapabilityPolicy;
    use std::sync::Arc;
    
    #[test]
    fn test_capability_checks() {
        // Create a capability manager with a permissive policy
        let policy = Arc::new(DefaultCapabilityPolicy::new(true));
        let config = CapabilityManagerImplConfig { policy };
        let manager = CapabilityManagerImpl::new(config);
        
        // Create a plugin ID
        let plugin_id = PluginId::new();
        
        // Grant file system read capability
        let read_cap = CoreCapability::FileSystemRead { path: Some("/tmp".to_string()) };
        manager.grant_capability(plugin_id, read_cap).unwrap();
        
        // Grant network capability
        let net_cap = CoreCapability::NetworkClient { 
            hosts: Some(vec!["api.example.com".to_string()]) 
        };
        manager.grant_capability(plugin_id, net_cap).unwrap();
        
        // Test file system read check
        assert!(check_fs_read(plugin_id, "/tmp/file.txt", &manager).is_ok());
        assert!(check_fs_read(plugin_id, "/etc/passwd", &manager).is_err());
        
        // Test network check
        assert!(check_network(
            plugin_id, 
            &["api.example.com".to_string()], 
            &manager
        ).is_ok());
        assert!(check_network(
            plugin_id, 
            &["malicious.com".to_string()], 
            &manager
        ).is_err());
    }
    
    #[test]
    fn test_path_normalization() {
        // Valid paths
        assert!(normalize_path("/tmp/file.txt").is_ok());
        assert!(normalize_path("file.txt").is_ok());
        
        // Path traversal attack
        assert!(normalize_path("/tmp/../etc/passwd").is_err());
    }
}