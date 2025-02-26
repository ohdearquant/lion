//! Capability storage.
//!
//! This module provides storage for capability assignments,
//! mapping plugins to their granted capabilities.

use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;

use lion_core::error::CapabilityError;
use lion_core::types::PluginId;
use crate::capability::{Capability, CapabilitySet};

/// Interface for capability storage.
pub trait CapabilityStore: Send + Sync {
    /// Add a capability to a plugin.
    fn add_capability(
        &self,
        plugin_id: &PluginId,
        capability: Arc<dyn Capability>,
    ) -> Result<(), CapabilityError>;
    
    /// Remove a capability from a plugin.
    fn remove_capability(
        &self,
        plugin_id: &PluginId,
        capability_type: &str,
    ) -> Result<(), CapabilityError>;
    
    /// Get all capabilities for a plugin.
    fn get_capabilities(&self, plugin_id: &PluginId) -> Result<CapabilitySet, CapabilityError>;
}

/// In-memory capability store.
pub struct MemoryCapabilityStore {
    /// Storage for plugin capabilities.
    capabilities: DashMap<PluginId, RwLock<CapabilitySet>>,
}

impl MemoryCapabilityStore {
    /// Create a new in-memory store.
    pub fn new() -> Self {
        Self {
            capabilities: DashMap::new(),
        }
    }
}

impl CapabilityStore for MemoryCapabilityStore {
    fn add_capability(
        &self,
        plugin_id: &PluginId,
        capability: Arc<dyn Capability>,
    ) -> Result<(), CapabilityError> {
        // Get or create the plugin's capability set
        let entry = self.capabilities
            .entry(plugin_id.clone())
            .or_insert_with(|| RwLock::new(CapabilitySet::new()));
        
        // Add the capability
        let mut set = entry.write();
        set.add(capability);
        
        Ok(())
    }
    
    fn remove_capability(
        &self,
        plugin_id: &PluginId,
        capability_type: &str,
    ) -> Result<(), CapabilityError> {
        // Check if the plugin exists
        if let Some(entry) = self.capabilities.get(plugin_id) {
            // Get the current capabilities
            let mut set = entry.write();
            let old_capabilities = set.get_all().to_vec();
            
            // Create a new set without the specified capability type
            let mut new_set = CapabilitySet::new();
            for cap in old_capabilities {
                if cap.get_type() != capability_type {
                    new_set.add(cap);
                }
            }
            
            // Replace the set
            *set = new_set;
            
            Ok(())
        } else {
            Err(CapabilityError::NotGranted(format!(
                "Plugin {} has no capabilities",
                plugin_id
            )))
        }
    }
    
    fn get_capabilities(&self, plugin_id: &PluginId) -> Result<CapabilitySet, CapabilityError> {
        // Get the plugin's capabilities or return an empty set
        if let Some(entry) = self.capabilities.get(plugin_id) {
            let set = entry.read();
            Ok(set.clone())
        } else {
            Ok(CapabilitySet::new())
        }
    }
}