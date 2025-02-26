//! Core capability trait.
//! 
//! This module defines the core `Capability` trait that is implemented by
//! all specific capability types.

use std::fmt::Debug;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;

/// Core capability trait.
///
/// A capability is an unforgeable token of authority that grants specific
/// permissions.
pub trait Capability: Debug + Send + Sync {
    /// Returns the type of this capability.
    fn capability_type(&self) -> &str;
    
    /// Checks if this capability permits the given access request.
    ///
    /// # Arguments
    ///
    /// * `request` - The access request to check.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the access is permitted.
    /// * `Err(CapabilityError)` if the access is denied.
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError>;
    
    /// Constrains this capability with the given constraints.
    ///
    /// This is used to create a derived capability with reduced permissions.
    ///
    /// # Arguments
    ///
    /// * `constraints` - The constraints to apply.
    ///
    /// # Returns
    ///
    /// * `Ok(Box<dyn Capability>)` - A new capability with the constraints applied.
    /// * `Err(CapabilityError)` - If the constraints cannot be applied.
    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        Err(CapabilityError::ConstraintError("Constraint not supported".into()).into())
    }
    
    /// Splits this capability into constituent parts.
    ///
    /// This is used for partial revocation.
    ///
    /// # Returns
    ///
    /// A vector of capabilities that, when combined, are equivalent to this capability.
    fn split(&self) -> Vec<Box<dyn Capability>> {
        vec![self.clone_box()]
    }
    
    /// Checks if this capability can be joined with another.
    ///
    /// # Arguments
    ///
    /// * `other` - The other capability to join with.
    ///
    /// # Returns
    ///
    /// `true` if the capabilities can be joined, `false` otherwise.
    fn can_join_with(&self, other: &dyn Capability) -> bool {
        self.capability_type() == other.capability_type()
    }
    
    /// Joins this capability with another compatible one.
    ///
    /// # Arguments
    ///
    /// * `other` - The other capability to join with.
    ///
    /// # Returns
    ///
    /// * `Ok(Box<dyn Capability>)` - A new capability that combines the permissions of both.
    /// * `Err(CapabilityError)` - If the capabilities cannot be joined.
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        Err(CapabilityError::CompositionError("Join not supported".into()).into())
    }
    
    /// Clone this capability as a boxed trait object.
    fn clone_box(&self) -> Box<dyn Capability>;
}

/// A constraint that can be applied to a capability.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Constrain file paths.
    FilePath(String),
    
    /// Constrain file operations.
    FileOperation {
        /// Whether read operations are allowed.
        read: bool,
        
        /// Whether write operations are allowed.
        write: bool,
        
        /// Whether execute operations are allowed.
        execute: bool,
    },
    
    /// Constrain network hosts.
    NetworkHost(String),
    
    /// Constrain network ports.
    NetworkPort(u16),
    
    /// Constrain network operations.
    NetworkOperation {
        /// Whether outbound connections are allowed.
        connect: bool,
        
        /// Whether listening for inbound connections is allowed.
        listen: bool,
    },
    
    /// Constrain plugin calls.
    PluginCall {
        /// Plugin ID.
        plugin_id: String,
        
        /// Function name.
        function: String,
    },
    
    /// Constrain memory regions.
    MemoryRegion {
        /// Region ID.
        region_id: String,
        
        /// Whether read operations are allowed.
        read: bool,
        
        /// Whether write operations are allowed.
        write: bool,
    },
    
    /// Constrain message sending.
    Message {
        /// Recipient plugin ID.
        recipient: String,
        
        /// Topic.
        topic: String,
    },
    
    /// Custom constraint.
    Custom {
        /// The constraint type.
        constraint_type: String,
        
        /// The constraint value.
        value: String,
    },
}

impl Constraint {
    /// Get the type of this constraint.
    pub fn constraint_type(&self) -> &str {
        match self {
            Self::FilePath(_) => "file_path",
            Self::FileOperation { .. } => "file_operation",
            Self::NetworkHost(_) => "network_host",
            Self::NetworkPort(_) => "network_port",
            Self::NetworkOperation { .. } => "network_operation",
            Self::PluginCall { .. } => "plugin_call",
            Self::MemoryRegion { .. } => "memory_region",
            Self::Message { .. } => "message",
            Self::Custom { constraint_type, .. } => constraint_type,
        }
    }
}

/// A custom capability type.
#[derive(Debug, Clone)]
pub struct CustomCapability {
    /// The capability type.
    pub capability_type: String,
    
    /// The capability value.
    pub value: String,
    
    /// Custom permission check function.
    pub check_fn: fn(&AccessRequest) -> Result<(), CapabilityError>,
}

impl Capability for CustomCapability {
    fn capability_type(&self) -> &str {
        &self.capability_type
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        (self.check_fn)(request)
    }
    
    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }
}