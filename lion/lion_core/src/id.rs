//! Strongly-typed identifiers for the Lion microkernel.
//!
//! This module provides a set of identifier types that are used throughout
//! the system, ensuring type safety and clear semantics. Each identifier
//! type is a thin wrapper around a UUID with a phantom type parameter
//! to ensure type safety.
//!
//! # Examples
//!
//! ```
//! use lion_core::id::{PluginId, CapabilityId};
//! use std::str::FromStr;
//!
//! // Create new random IDs
//! let plugin_id = PluginId::new();
//! let capability_id = CapabilityId::new();
//!
//! // Different ID types are different types, even with the same underlying UUID
//! assert_ne!(plugin_id.to_string(), capability_id.to_string());
//!
//! // Create from string
//! let id_str = "550e8400-e29b-41d4-a716-446655440000";
//! let plugin_id = PluginId::from_str(id_str).unwrap();
//! assert_eq!(plugin_id.to_string(), id_str);
//! ```

use serde::{Deserialize, Serialize};
use std::cmp::{Ord, PartialOrd};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// A type-safe identifier based on UUID.
///
/// This is a generic identifier type that is specialized for different
/// entity types using the phantom type parameter `T`. This ensures that
/// identifiers for different entity types cannot be mixed up, even though
/// they share the same underlying UUID structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Id<T> {
    uuid: Uuid,
    #[serde(skip)]
    _marker: std::marker::PhantomData<T>,
}

impl<T> Id<T> {
    /// Create a new random identifier.
    ///
    /// This generates a new random UUID v4 and wraps it in the appropriate
    /// identifier type.
    ///
    /// # Examples
    ///
    /// ```
    /// use lion_core::id::PluginId;
    ///
    /// let id = PluginId::new();
    /// ```
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Create an identifier from a specific UUID.
    ///
    /// This is useful when you need to create an identifier with a known UUID,
    /// such as when deserializing from a database or message.
    ///
    /// # Examples
    ///
    /// ```
    /// use lion_core::id::PluginId;
    /// use uuid::Uuid;
    ///
    /// let uuid = Uuid::new_v4();
    /// let id = PluginId::from_uuid(uuid);
    /// assert_eq!(id.uuid(), uuid);
    /// ```
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self {
            uuid,
            _marker: std::marker::PhantomData,
        }
    }

    /// Get the underlying UUID.
    ///
    /// This is useful when you need to extract the raw UUID for serialization
    /// or other purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use lion_core::id::PluginId;
    ///
    /// let id = PluginId::new();
    /// let uuid = id.uuid();
    /// ```
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Create a nil (all zeros) identifier.
    ///
    /// This can be useful as a sentinel value or default value.
    ///
    /// # Examples
    ///
    /// ```
    /// use lion_core::id::PluginId;
    ///
    /// let nil_id = PluginId::nil();
    /// assert_eq!(nil_id.to_string(), "00000000-0000-0000-0000-000000000000");
    /// ```
    pub fn nil() -> Self {
        Self {
            uuid: Uuid::nil(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Check if this is a nil identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use lion_core::id::PluginId;
    ///
    /// let nil_id = PluginId::nil();
    /// assert!(nil_id.is_nil());
    ///
    /// let id = PluginId::new();
    /// assert!(!id.is_nil());
    /// ```
    pub fn is_nil(&self) -> bool {
        self.uuid == Uuid::nil()
    }

    /// Convert the ID to bytes for serialization
    pub fn to_bytes(&self) -> Vec<u8> {
        self.uuid.as_bytes().to_vec()
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new()
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginMarker;
/// Identifier for a plugin.
pub type PluginId = Id<PluginMarker>;

/// Marker type for capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityMarker;
/// Identifier for a capability.
pub type CapabilityId = Id<CapabilityMarker>;

/// Marker type for workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkflowMarker;
/// Identifier for a workflow.
pub type WorkflowId = Id<WorkflowMarker>;

/// Marker type for workflow nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeMarker;
/// Identifier for a workflow node.
pub type NodeId = Id<NodeMarker>;

/// Marker type for workflow executions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecutionMarker;
/// Identifier for a workflow execution.
pub type ExecutionId = Id<ExecutionMarker>;

/// Marker type for memory regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionMarker;
/// Identifier for a memory region.
pub type RegionId = Id<RegionMarker>;

/// Marker type for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    fn test_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = PluginId::from_uuid(uuid);
        assert_eq!(id.uuid(), uuid);
    }

    #[test]
    fn test_id_nil() {
        let nil_id = PluginId::nil();
        assert_eq!(nil_id.to_string(), "00000000-0000-0000-0000-000000000000");
        assert!(nil_id.is_nil());
    }

    #[test]
    fn test_type_safety() {
        // Use the variables to avoid warnings
        let _plugin_id = PluginId::new();
        let _capability_id = CapabilityId::new();

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

    #[test]
    fn test_id_serde() {
        let id = PluginId::new();
        let serialized = serde_json::to_string(&id).unwrap();
        let deserialized: PluginId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }
}
