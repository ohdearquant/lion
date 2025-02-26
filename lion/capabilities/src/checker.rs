//! Capability checking logic.
//!
//! This module provides the enforcement mechanism for capabilities,
//! ensuring plugins only perform operations they're authorized for.

use std::sync::Arc;

use lion_core::error::CapabilityError;
use lion_core::types::PluginId;
use crate::capability::Capability;
use crate::store::CapabilityStore;

/// Interface for capability checking.
pub trait CapabilityChecker: Send + Sync {
    /// Check if a plugin has a capability.
    fn check_capability(
        &self,
        plugin_id: &PluginId,
        capability: &dyn Capability,
        store: &dyn CapabilityStore,
    ) -> Result<(), CapabilityError>;
}

/// Simple implementation of capability checking.
pub struct SimpleCapabilityChecker;

impl SimpleCapabilityChecker {
    /// Create a new capability checker.
    pub fn new() -> Self {
        Self
    }
}

impl CapabilityChecker for SimpleCapabilityChecker {
    fn check_capability(
        &self,
        plugin_id: &PluginId,
        capability: &dyn Capability,
        store: &dyn CapabilityStore,
    ) -> Result<(), CapabilityError> {
        // Get the plugin's capabilities
        let capabilities = store.get_capabilities(plugin_id)?;
        
        // Check if any capability includes the requested one
        if capabilities.includes(capability) {
            Ok(())
        } else {
            Err(CapabilityError::NotGranted(capability.to_string()))
        }
    }
}