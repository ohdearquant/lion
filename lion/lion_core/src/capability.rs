//! Capability-based security system for Lion.
//!
//! This module defines the capability system that controls what resources
//! plugins can access. Each plugin must explicitly request capabilities,
//! which are granted or denied based on system policy.

use crate::error::CapabilityError;
use crate::plugin::PluginId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Unique identifier for a capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub Uuid);

impl CapabilityId {
    /// Create a new random capability ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CapabilityId {
    fn default() -> Self {
        Self::new()
    }
}

/// Core capabilities supported by the Lion system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoreCapability {
    /// Read access to the file system (optionally with path restrictions)
    FileSystemRead { path: Option<String> },
    
    /// Write access to the file system (optionally with path restrictions)
    FileSystemWrite { path: Option<String> },
    
    /// Network client access (optionally with host/port restrictions)
    NetworkClient { hosts: Option<Vec<String>> },
    
    /// Inter-plugin communication capability
    InterPluginComm,
}

/// A capability that can be granted to a plugin
#[derive(Debug, Clone)]
pub struct Capability {
    /// Unique identifier for this capability instance
    pub id: CapabilityId,
    
    /// The actual capability type
    pub capability_type: CoreCapability,
    
    /// Optional description
    pub description: Option<String>,
}

/// Interface for checking and managing capabilities
pub trait CapabilityManager: Send + Sync {
    /// Check if a plugin has a specific capability
    fn has_capability(&self, plugin_id: PluginId, capability: &CoreCapability) -> bool;
    
    /// Grant a capability to a plugin
    fn grant_capability(
        &self, 
        plugin_id: PluginId, 
        capability: CoreCapability
    ) -> Result<CapabilityId, CapabilityError>;
    
    /// Revoke a capability from a plugin
    fn revoke_capability(
        &self, 
        plugin_id: PluginId, 
        capability_id: CapabilityId
    ) -> Result<(), CapabilityError>;
    
    /// List all capabilities granted to a plugin
    fn list_capabilities(&self, plugin_id: PluginId) -> Vec<Capability>;
    
    /// Returns self as Any for downcasting in advanced scenarios
    fn as_any(&self) -> &dyn std::any::Any;
}