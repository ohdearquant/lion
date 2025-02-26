//! In-memory capability store.
//! 
//! This module provides an in-memory implementation of the capability store.

use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use lion_core::error::{Result, CapabilityError};
use lion_core::id::{PluginId, CapabilityId};

use crate::model::Capability;
use super::CapabilityStore;

/// An in-memory capability store.
#[derive(Clone)]
pub struct InMemoryCapabilityStore {
    /// The capabilities, indexed by (plugin_id, capability_id).
    capabilities: Arc<DashMap<(PluginId, CapabilityId), Box<dyn Capability>>>,
    
    /// Capability IDs by plugin, for faster listing.
    plugin_capabilities: Arc<DashMap<PluginId, Vec<CapabilityId>>>,
}

impl InMemoryCapabilityStore {
    /// Create a new in-memory capability store.
    pub fn new() -> Self {
        Self {
            capabilities: Arc::new(DashMap::new()),
            plugin_capabilities: Arc::new(DashMap::new()),
        }
    }
}

impl Default for InMemoryCapabilityStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityStore for InMemoryCapabilityStore {
    fn add_capability(&self, plugin_id: PluginId, capability: Box<dyn Capability>) -> Result<CapabilityId> {
        // Generate a new capability ID
        let capability_id = CapabilityId::new();
        
        // Add the capability to the store
        self.capabilities.insert((plugin_id.clone(), capability_id.clone()), capability);
        
        // Add the capability ID to the plugin's list
        self.plugin_capabilities
            .entry(plugin_id)
            .or_insert_with(Vec::new)
            .push(capability_id.clone());
        
        Ok(capability_id)
    }
    
    fn get_capability(&self, plugin_id: &PluginId, capability_id: &CapabilityId) -> Result<Box<dyn Capability>> {
        // Look up the capability in the store
        let key = (plugin_id.clone(), capability_id.clone());
        let capability = self.capabilities.get(&key)
            .ok_or_else(|| CapabilityError::NotFound(capability_id.clone()))?
            .clone_box();
        
        Ok(capability)
    }
    
    fn remove_capability(&self, plugin_id: &PluginId, capability_id: &CapabilityId) -> Result<()> {
        // Remove the capability from the store
        let key = (plugin_id.clone(), capability_id.clone());
        if self.capabilities.remove(&key).is_none() {
            return Err(CapabilityError::NotFound(capability_id.clone()).into());
        }
        
        // Remove the capability ID from the plugin's list
        if let Some(mut entry) = self.plugin_capabilities.entry(plugin_id.clone()) {
            entry.value_mut().retain(|id| id != capability_id);
        }
        
        Ok(())
    }
    
    fn replace_capability(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
        new_capability: Box<dyn Capability>,
    ) -> Result<()> {
        // Check if the capability exists
        let key = (plugin_id.clone(), capability_id.clone());
        if !self.capabilities.contains_key(&key) {
            return Err(CapabilityError::NotFound(capability_id.clone()).into());
        }
        
        // Replace the capability
        self.capabilities.insert(key, new_capability);
        
        Ok(())
    }
    
    fn list_capabilities(&self, plugin_id: &PluginId) -> Result<Vec<(CapabilityId, Box<dyn Capability>)>> {
        // Get the capability IDs for the plugin
        let capability_ids = match self.plugin_capabilities.get(plugin_id) {
            Some(ids) => ids.clone(),
            None => Vec::new(),
        };
        
        // Get the capabilities
        let mut capabilities = Vec::new();
        for capability_id in capability_ids {
            let key = (plugin_id.clone(), capability_id.clone());
            if let Some(capability) = self.capabilities.get(&key) {
                capabilities.push((capability_id.clone(), capability.clone_box()));
            }
        }
        
        Ok(capabilities)
    }
    
    fn clear_plugin_capabilities(&self, plugin_id: &PluginId) -> Result<()> {
        // Get the capability IDs for the plugin
        let capability_ids = match self.plugin_capabilities.get(plugin_id) {
            Some(ids) => ids.clone(),
            None => Vec::new(),
        };
        
        // Remove the capabilities
        for capability_id in capability_ids {
            let key = (plugin_id.clone(), capability_id.clone());
            self.capabilities.remove(&key);
        }
        
        // Clear the plugin's list
        self.plugin_capabilities.remove(plugin_id);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::FileCapability;
    use std::path::PathBuf;
    
    #[test]
    fn test_add_and_get_capability() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = PluginId::new();
        let capability = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        
        // Add the capability
        let capability_id = store.add_capability(plugin_id.clone(), capability).unwrap();
        
        // Get the capability
        let retrieved = store.get_capability(&plugin_id, &capability_id).unwrap();
        
        // Check that it's the same type
        assert_eq!(retrieved.capability_type(), "file");
    }
    
    #[test]
    fn test_remove_capability() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = PluginId::new();
        let capability = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        
        // Add the capability
        let capability_id = store.add_capability(plugin_id.clone(), capability).unwrap();
        
        // Remove the capability
        store.remove_capability(&plugin_id, &capability_id).unwrap();
        
        // Try to get the capability
        let result = store.get_capability(&plugin_id, &capability_id);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_replace_capability() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = PluginId::new();
        let capability1 = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        let capability2 = Box::new(FileCapability::write_only(vec![PathBuf::from("/var")]));
        
        // Add the first capability
        let capability_id = store.add_capability(plugin_id.clone(), capability1).unwrap();
        
        // Replace the capability
        store.replace_capability(&plugin_id, &capability_id, capability2).unwrap();
        
        // Get the capability
        let retrieved = store.get_capability(&plugin_id, &capability_id).unwrap();
        
        // Check that it's the new capability
        let request = lion_core::types::AccessRequest::File {
            path: PathBuf::from("/var/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(retrieved.permits(&request).is_ok());
    }
    
    #[test]
    fn test_list_capabilities() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = PluginId::new();
        let capability1 = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        let capability2 = Box::new(FileCapability::write_only(vec![PathBuf::from("/var")]));
        
        // Add the capabilities
        store.add_capability(plugin_id.clone(), capability1).unwrap();
        store.add_capability(plugin_id.clone(), capability2).unwrap();
        
        // List the capabilities
        let capabilities = store.list_capabilities(&plugin_id).unwrap();
        
        // Check that there are two capabilities
        assert_eq!(capabilities.len(), 2);
    }
    
    #[test]
    fn test_clear_plugin_capabilities() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = PluginId::new();
        let capability1 = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        let capability2 = Box::new(FileCapability::write_only(vec![PathBuf::from("/var")]));
        
        // Add the capabilities
        store.add_capability(plugin_id.clone(), capability1).unwrap();
        store.add_capability(plugin_id.clone(), capability2).unwrap();
        
        // Clear the capabilities
        store.clear_plugin_capabilities(&plugin_id).unwrap();
        
        // List the capabilities
        let capabilities = store.list_capabilities(&plugin_id).unwrap();
        
        // Check that there are no capabilities
        assert_eq!(capabilities.len(), 0);
    }
}