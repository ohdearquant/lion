//! Strongly-typed identifiers for the Lion microkernel.
//! 
//! This module provides a set of identifier types that are used throughout
//! the system, ensuring type safety and clear semantics.

use std::fmt;
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// A type-safe identifier based on UUID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id<T> {
    uuid: Uuid,
    #[serde(skip)]
    _marker: std::marker::PhantomData<T>,
}

impl<T> Id<T> {
    /// Create a new random identifier.
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            _marker: std::marker::PhantomData,
        }
    }
    
    /// Create an identifier from a specific UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self {
            uuid,
            _marker: std::marker::PhantomData,
        }
    }
    
    /// Get the underlying UUID.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
    
    /// Create a nil (all zeros) identifier.
    pub fn nil() -> Self {
        Self {
            uuid: Uuid::nil(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

impl<T> FromStr for Id<T> {
    type Err = uuid::Error;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            uuid: Uuid::parse_str(s)?,
            _marker: std::marker::PhantomData,
        })
    }
}

/// Marker type for plugins.
pub struct PluginMarker;
/// Identifier for a plugin.
pub type PluginId = Id<PluginMarker>;

/// Marker type for capabilities.
pub struct CapabilityMarker;
/// Identifier for a capability.
pub type CapabilityId = Id<CapabilityMarker>;

/// Marker type for workflows.
pub struct WorkflowMarker;
/// Identifier for a workflow.
pub type WorkflowId = Id<WorkflowMarker>;

/// Marker type for workflow nodes.
pub struct NodeMarker;
/// Identifier for a workflow node.
pub type NodeId = Id<NodeMarker>;

/// Marker type for workflow executions.
pub struct ExecutionMarker;
/// Identifier for a workflow execution.
pub type ExecutionId = Id<ExecutionMarker>;

/// Marker type for memory regions.
pub struct RegionMarker;
/// Identifier for a memory region.
pub type RegionId = Id<RegionMarker>;

/// Marker type for messages.
pub struct MessageMarker;
/// Identifier for a message.
pub type MessageId = Id<MessageMarker>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_id_new() {
        let id1 = PluginId::new();
        let id2 = PluginId::new();
        assert_ne!(id1, id2, "Generated IDs should be unique");
    }
    
    #[test]
    fn test_id_display() {
        let id = PluginId::new();
        let display = id.to_string();
        assert_eq!(display.len(), 36, "UUID string should be 36 characters");
    }
    
    #[test]
    fn test_id_from_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id = PluginId::from_str(uuid_str).unwrap();
        assert_eq!(id.to_string(), uuid_str);
    }
    
    #[test]
    fn test_type_safety() {
        let plugin_id = PluginId::new();
        let capability_id = CapabilityId::new();
        
        // This would not compile if uncommented:
        // let _: PluginId = capability_id;
        
        // Different ID types are different types, even with the same UUID
        let same_uuid = Uuid::new_v4();
        let plugin_id = PluginId::from_uuid(same_uuid);
        let capability_id = CapabilityId::from_uuid(same_uuid);
        
        assert_eq!(plugin_id.uuid(), capability_id.uuid());
        // But they're still different types
        // This would not compile:
        // assert_eq!(plugin_id, capability_id);
    }
}