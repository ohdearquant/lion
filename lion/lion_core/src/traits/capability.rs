//! Capability trait definitions.
//! 
//! This module defines the core traits for the capability-based security system.

use crate::error::{Result, CapabilityError};
use crate::id::CapabilityId;
use crate::types::AccessRequest;

/// Core trait for capabilities.
///
/// A capability is an unforgeable token of authority that grants specific
/// permissions to access resources. The capability model follows the
/// principle of least privilege.
///
/// # Examples
///
/// ```
/// use lion_core::traits::Capability;
/// use lion_core::types::AccessRequest;
/// use lion_core::error::CapabilityError;
///
/// struct FileReadCapability {
///     path: String,
/// }
///
/// impl Capability for FileReadCapability {
///     fn capability_type(&self) -> &str {
///         "file_read"
///     }
///
///     fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
///         match request {
///             AccessRequest::File { path, write, .. } => {
///                 if *write {
///                     return Err(CapabilityError::PermissionDenied(
///                         "Write access not allowed".into()
///                     ));
///                 }
///                 
///                 if !path.starts_with(&self.path) {
///                     return Err(CapabilityError::PermissionDenied(
///                         format!("Access to {} not allowed", path)
///                     ));
///                 }
///                 
///                 Ok(())
///             },
///             _ => Err(CapabilityError::PermissionDenied(
///                 "Only file access is allowed".into()
///             )),
///         }
///     }
/// }
/// ```
pub trait Capability: Send + Sync {
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
        vec![Box::new(ClonedCapability(self))]
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
    
    /// Custom constraint.
    Custom {
        /// The constraint type.
        constraint_type: String,
        
        /// The constraint value.
        value: String,
    },
}

/// A helper to implement the default split() without cloning.
struct ClonedCapability<'a>(&'a dyn Capability);

impl<'a> Capability for ClonedCapability<'a> {
    fn capability_type(&self) -> &str {
        self.0.capability_type()
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        self.0.permits(request)
    }
}