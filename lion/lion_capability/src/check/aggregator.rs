//! Capability aggregation.
//! 
//! This module provides capability aggregation functionality.

use std::collections::HashMap;
use lion_core::id::{PluginId, CapabilityId};

use crate::model::Capability;
use crate::store::CapabilityStore;

/// Capability aggregator.
///
/// This aggregator provides functionality for aggregating capabilities.
#[derive(Clone)]
pub struct CapabilityAggregator<S> {
    /// The capability store.
    store: S,
}

impl<S: CapabilityStore> CapabilityAggregator<S> {
    /// Create a new capability aggregator.
    ///
    /// # Arguments
    ///
    /// * `store` - The capability store.
    ///
    /// # Returns
    ///
    /// A new capability aggregator.
    pub fn new(store: S) -> Self {
        Self { store }
    }
    
    /// Get all capabilities by type.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to get capabilities for.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, Vec<(CapabilityId, Box<dyn Capability>)>>)` - The capabilities, grouped by type.
    /// * `Err` - If the capabilities could not be retrieved.
    pub fn capabilities_by_type(
        &self,
        plugin_id: &PluginId,
    ) -> lion_core::error::Result<HashMap<String, Vec<(CapabilityId, Box<dyn Capability>)>>> {
        // Get the capabilities for the plugin
        let capabilities = self.store.list_capabilities(plugin_id)?;
        
        // Group the capabilities by type
        let mut result = HashMap::new();
        for (id, capability) in capabilities {
            let capability_type = capability.capability_type().to_string();
            result.entry(capability_type)
                .or_insert_with(Vec::new)
                .push((id, capability));
        }
        
        Ok(result)
    }
    
    /// Merge capabilities of the same type.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to merge capabilities for.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the capabilities were successfully merged.
    /// * `Err` - If the capabilities could not be merged.
    pub fn merge_capabilities_by_type(&self, plugin_id: &PluginId) -> lion_core::error::Result<()> {
        // Get the capabilities by type
        let capabilities_by_type = self.capabilities_by_type(plugin_id)?;
        
        // For each type, merge the capabilities
        for (type_name, capabilities) in capabilities_by_type {
            // Skip if there's only one capability of this type
            if capabilities.len() <= 1 {
                continue;
            }
            
            // Merge the capabilities
            let (first_id, first_capability) = capabilities[0].clone();
            let mut merged = first_capability;
            
            // Remove all capabilities of this type
            for (id, _) in &capabilities {
                self.store.remove_capability(plugin_id, id)?;
            }
            
            // Merge the remaining capabilities
            for (_, capability) in capabilities.into_iter().skip(1) {
                merged = merged.join(&*capability)?;
            }
            
            // Add the merged capability
            self.store.add_capability(plugin_id.clone(), merged)?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::FileCapability;
    use crate::store::InMemoryCapabilityStore;
    use std::path::PathBuf;
    
    #[test]
    fn test_capabilities_by_type() {
        let store = InMemoryCapabilityStore::new();
        let aggregator = CapabilityAggregator::new(store.clone());
        let plugin_id = PluginId::new();
        
        // Add capabilities
        let capability1 = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        let capability2 = Box::new(FileCapability::write_only(vec![PathBuf::from("/var")]));
        store.add_capability(plugin_id.clone(), capability1).unwrap();
        store.add_capability(plugin_id.clone(), capability2).unwrap();
        
        // Get capabilities by type
        let capabilities_by_type = aggregator.capabilities_by_type(&plugin_id).unwrap();
        
        // Check that there's one type with two capabilities
        assert_eq!(capabilities_by_type.len(), 1);
        assert_eq!(capabilities_by_type.get("file").unwrap().len(), 2);
    }
    
    #[test]
    fn test_merge_capabilities_by_type() {
        let store = InMemoryCapabilityStore::new();
        let aggregator = CapabilityAggregator::new(store.clone());
        let plugin_id = PluginId::new();
        
        // Add capabilities
        let capability1 = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        let capability2 = Box::new(FileCapability::write_only(vec![PathBuf::from("/var")]));
        store.add_capability(plugin_id.clone(), capability1).unwrap();
        store.add_capability(plugin_id.clone(), capability2).unwrap();
        
        // Merge capabilities
        aggregator.merge_capabilities_by_type(&plugin_id).unwrap();
        
        // Check that there's now one capability
        let capabilities = store.list_capabilities(&plugin_id).unwrap();
        assert_eq!(capabilities.len(), 1);
        
        // Check that the merged capability permits both read access to /tmp and write access to /var
        let (_, merged) = &capabilities[0];
        
        // Read access to /tmp
        let request = lion_core::types::AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(merged.permits(&request).is_ok());
        
        // Write access to /var
        let request = lion_core::types::AccessRequest::File {
            path: PathBuf::from("/var/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(merged.permits(&request).is_ok());
    }
}