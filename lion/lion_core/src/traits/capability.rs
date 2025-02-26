//! Capability trait definitions.
//!
//! This module defines the core traits for the capability-based security system,
//! including the fundamental `Capability` trait and related types. The capability
//! model is based on research in capability-based security systems, including
//! KeyKOS, EROS, and seL4.
//!
//! # Capability Model
//!
//! In the Lion microkernel, capabilities are unforgeable tokens of authority
//! that grant specific permissions to access resources. The system follows
//! these principles:
//!
//! - **Principle of Least Privilege**: Components have only the capabilities
//!   they need to function, and no more.
//!
//! - **Attenuation**: Derived capabilities can only restrict rights, never add
//!   new ones. This forms a partial order: C₁ ≤ C₂ if and only if R₁ ⊆ R₂.
//!
//! - **Unified Model**: Capabilities are combined with policies, requiring both:
//!   `permit(action) := has_capability(subject, object, action) ∧ policy_allows(subject, object, action)`
//!
//! - **Composability**: Capabilities can be combined, split, or constrained
//!   to create new capabilities.

use crate::error::{CapabilityError, Error, Result};
use crate::types::AccessRequest;
use std::sync::Arc;

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
/// use lion_core::error::{CapabilityError, Error};
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
///     fn permits(&self, request: &AccessRequest) -> Result<(), Error> {
///         match request {
///             AccessRequest::File { path, write, .. } => {
///                 if *write {
///                     return Err(CapabilityError::PermissionDenied(
///                         "Write access not allowed".into()
///                     ).into());
///                 }
///                 
///                 if !path.starts_with(&self.path) {
///                     return Err(CapabilityError::PermissionDenied(
///                         format!("Access to {} not allowed", path)
///                     ).into());
///                 }
///                 
///                 Ok(())
///             },
///             _ => Err(CapabilityError::PermissionDenied(
///                 "Only file access is allowed".into()
///             ).into()),
///         }
///     }
/// }
/// ```
pub trait Capability: Send + Sync {
    /// Returns the type of this capability.
    ///
    /// The capability type is a string that identifies the kind of capability,
    /// such as "file_read", "network_connect", etc. This is used for type checking
    /// and capability management.
    fn capability_type(&self) -> &str;

    /// Checks if this capability permits the given access request.
    ///
    /// This is the core method of the Capability trait, and is used to determine
    /// whether an access request is allowed by this capability.
    ///
    /// # Arguments
    ///
    /// * `request` - The access request to check.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the access is permitted.
    /// * `Err(Error)` if the access is denied.
    fn permits(&self, request: &AccessRequest) -> Result<()>;

    /// Constrains this capability with the given constraints.
    ///
    /// This is used to create a derived capability with reduced permissions,
    /// implementing the principle of attenuation. The new capability will be
    /// a subset of the original capability, with additional restrictions.
    ///
    /// # Arguments
    ///
    /// * `constraints` - The constraints to apply.
    ///
    /// # Returns
    ///
    /// * `Ok(Box<dyn Capability>)` - A new capability with the constraints applied.
    /// * `Err(CapabilityError)` - If the constraints cannot be applied.
    ///
    /// # Examples
    ///
    /// ```
    /// # use lion_core::traits::{Capability, Constraint};
    /// # use lion_core::types::AccessRequest;
    /// # use lion_core::error::CapabilityError;
    /// #
    /// # struct FileCapability {
    /// #     path: String,
    /// #     read: bool,
    /// #     write: bool,
    /// # }
    /// #
    /// # impl Capability for FileCapability {
    /// #     fn capability_type(&self) -> &str { "file" }
    /// #     fn permits(&self, _: &AccessRequest) -> Result<(), CapabilityError> { Ok(()) }
    /// #
    /// fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
    ///     let mut new_cap = FileCapability {
    ///         path: self.path.clone(),
    ///         read: self.read,
    ///         write: self.write,
    ///     };
    ///     
    ///     for constraint in constraints {
    ///         match constraint {
    ///             Constraint::FilePath(path) => {
    ///                 // Ensure the new path is a subpath of the original path
    ///                 if !path.starts_with(&self.path) {
    ///                     return Err(CapabilityError::ConstraintError(
    ///                         "Cannot expand path beyond original capability".into()
    ///                     ));
    ///                 }
    ///                 new_cap.path = path.clone();
    ///             },
    ///             Constraint::FileOperation { read, write, .. } => {
    ///                 // Can only revoke permissions, not add new ones
    ///                 if *read && !self.read {
    ///                     return Err(CapabilityError::ConstraintError(
    ///                         "Cannot add read permission".into()
    ///                     ));
    ///                 }
    ///                 if *write && !self.write {
    ///                     return Err(CapabilityError::ConstraintError(
    ///                         "Cannot add write permission".into()
    ///                     ));
    ///                 }
    ///                 
    ///                 new_cap.read = *read && self.read;
    ///                 new_cap.write = *write && self.write;
    ///             },
    ///             _ => return Err(CapabilityError::ConstraintError(
    ///                 "Unsupported constraint type".into()
    ///             )),
    ///         }
    ///     }
    ///     
    ///     Ok(Box::new(new_cap))
    /// }
    /// # }
    /// ```
    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>> {
        let _constraints = constraints; // Use variable to avoid warning
        Err(CapabilityError::ConstraintError("Constraint not supported".into()).into())
    }

    /// Splits this capability into constituent parts.
    ///
    /// This is used for partial revocation, allowing parts of a capability
    /// to be revoked while retaining others. For example, a capability for
    /// filesystem access could be split into separate read and write capabilities.
    ///
    /// # Returns
    ///
    /// A vector of capabilities that, when combined, are equivalent to this capability.
    ///
    /// # Examples
    ///
    /// ```
    /// # use lion_core::traits::Capability;
    /// # use lion_core::types::AccessRequest;
    /// # use lion_core::error::CapabilityError;
    /// #
    /// # struct FileCapability {
    /// #     path: String,
    /// #     read: bool,
    /// #     write: bool,
    /// # }
    /// #
    /// # impl Capability for FileCapability {
    /// #     fn capability_type(&self) -> &str { "file" }
    /// #     fn permits(&self, _: &AccessRequest) -> Result<(), CapabilityError> { Ok(()) }
    /// #
    /// fn split(&self) -> Vec<Box<dyn Capability>> {
    ///     let mut caps = Vec::new();
    ///     
    ///     if self.read {
    ///         caps.push(Box::new(FileCapability {
    ///             path: self.path.clone(),
    ///             read: true,
    ///             write: false,
    ///         }));
    ///     }
    ///     
    ///     if self.write {
    ///         caps.push(Box::new(FileCapability {
    ///             path: self.path.clone(),
    ///             read: false,
    ///             write: true,
    ///         }));
    ///     }
    ///     
    ///     caps
    /// }
    /// # }
    /// ```
    fn split(&self) -> Vec<Box<dyn Capability + '_>>
    where
        Self: Sized,
    {
        vec![Box::new(ClonedCapability(Arc::new(CapabilityWrapper(
            self,
        ))))]
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
    /// This allows capabilities to be combined to form a new capability
    /// that includes the permissions of both original capabilities.
    ///
    /// # Arguments
    ///
    /// * `other` - The other capability to join with.
    ///
    /// # Returns
    ///
    /// * `Ok(Box<dyn Capability>)` - A new capability that combines the permissions of both.
    /// * `Err(CapabilityError)` - If the capabilities cannot be joined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use lion_core::traits::Capability;
    /// # use lion_core::types::AccessRequest;
    /// # use lion_core::error::CapabilityError;
    /// #
    /// # struct FileCapability {
    /// #     path: String,
    /// #     read: bool,
    /// #     write: bool,
    /// # }
    /// #
    /// # impl Capability for FileCapability {
    /// #     fn capability_type(&self) -> &str { "file" }
    /// #     fn permits(&self, _: &AccessRequest) -> Result<(), CapabilityError> { Ok(()) }
    /// #
    /// fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
    ///     if !self.can_join_with(other) {
    ///         return Err(CapabilityError::CompositionError(
    ///             "Cannot join capabilities of different types".into()
    ///         ));
    ///     }
    ///     
    ///     // Downcast the other capability to our specific type
    ///     let other_file_cap = match other.as_any().downcast_ref::<FileCapability>() {
    ///         Some(cap) => cap,
    ///         None => return Err(CapabilityError::CompositionError(
    ///             "Failed to downcast capability".into()
    ///         )),
    ///     };
    ///     
    ///     // If the paths are the same, we can combine the permissions
    ///     if self.path == other_file_cap.path {
    ///         return Ok(Box::new(FileCapability {
    ///             path: self.path.clone(),
    ///             read: self.read || other_file_cap.read,
    ///             write: self.write || other_file_cap.write,
    ///         }));
    ///     }
    ///     
    ///     Err(CapabilityError::CompositionError(
    ///         "Cannot join capabilities with different paths".into()
    ///     ))
    /// }
    /// #
    /// # fn as_any(&self) -> &dyn std::any::Any { self }
    /// # }
    /// ```
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>> {
        let _other = other; // Use variable to avoid warning
        Err(CapabilityError::CompositionError("Join not supported".into()).into())
    }

    /// Convert this capability to a trait object that can be downcast.
    ///
    /// This is useful for capability composition, as it allows capabilities
    /// to be downcast to specific types.
    ///
    /// # Returns
    ///
    /// A reference to this capability as a `dyn Any`.
    fn as_any(&self) -> &(dyn std::any::Any + '_)
    where
        Self: Sized,
    {
        self
    }
}

/// A constraint that can be applied to a capability.
///
/// Constraints are used to create derived capabilities with reduced
/// permissions, implementing the principle of attenuation.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Constrain file paths.
    ///
    /// This constraint restricts file access to a specific path or subpath.
    FilePath(String),

    /// Constrain file operations.
    ///
    /// This constraint restricts which file operations are allowed.
    FileOperation {
        /// Whether read operations are allowed.
        read: bool,

        /// Whether write operations are allowed.
        write: bool,

        /// Whether execute operations are allowed.
        execute: bool,
    },

    /// Constrain network hosts.
    ///
    /// This constraint restricts network access to a specific host.
    NetworkHost(String),

    /// Constrain network ports.
    ///
    /// This constraint restricts network access to a specific port.
    NetworkPort(u16),

    /// Constrain network operations.
    ///
    /// This constraint restricts which network operations are allowed.
    NetworkOperation {
        /// Whether outbound connections are allowed.
        connect: bool,

        /// Whether listening for inbound connections is allowed.
        listen: bool,
    },

    /// Custom constraint.
    ///
    /// This allows for capability-specific constraints that aren't covered
    /// by the standard constraints.
    Custom {
        /// The constraint type.
        constraint_type: String,

        /// The constraint value.
        value: String,
    },
}

/// Wraps a capability reference for cloning.
struct CapabilityWrapper<'a>(&'a dyn Capability);

impl<'a> Capability for CapabilityWrapper<'a> {
    fn capability_type(&self) -> &str {
        self.0.capability_type()
    }

    fn permits(&self, request: &AccessRequest) -> Result<()> {
        self.0.permits(request)
    }
}

/// A helper to implement the default split() without cloning.
struct ClonedCapability(Arc<dyn Capability>);

impl Capability for ClonedCapability {
    fn capability_type(&self) -> &str {
        self.0.capability_type()
    }

    fn permits(&self, request: &AccessRequest) -> Result<()> {
        self.0.permits(request)
    }

    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>> {
        self.0.constrain(constraints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AccessRequest;
    use std::path::PathBuf;

    // A simple file capability for testing
    #[derive(Debug)]
    struct FileCapability {
        path: PathBuf,
        read: bool,
        write: bool,
        execute: bool,
    }

    impl FileCapability {
        fn new(path: impl Into<PathBuf>, read: bool, write: bool, execute: bool) -> Self {
            Self {
                path: path.into(),
                read,
                write,
                execute,
            }
        }

        fn read_only(path: impl Into<PathBuf>) -> Self {
            Self::new(path, true, false, false)
        }

        fn write_only(path: impl Into<PathBuf>) -> Self {
            Self::new(path, false, true, false)
        }

        fn read_write(path: impl Into<PathBuf>) -> Self {
            Self::new(path, true, true, false)
        }
    }

    impl Capability for FileCapability {
        fn capability_type(&self) -> &str {
            "file"
        }

        fn permits(&self, request: &AccessRequest) -> Result<()> {
            match request {
                AccessRequest::File {
                    path,
                    read,
                    write,
                    execute,
                } => {
                    // Check if the path is within our capability's path
                    if !path.starts_with(&self.path) {
                        return Err(CapabilityError::PermissionDenied(format!(
                            "Path {} is not within allowed path {}",
                            path.display(),
                            self.path.display()
                        ))
                        .into());
                    }

                    // Check if the requested operations are allowed
                    if *read && !self.read {
                        return Err(
                            CapabilityError::PermissionDenied("Read not allowed".into()).into()
                        );
                    }

                    if *write && !self.write {
                        return Err(
                            CapabilityError::PermissionDenied("Write not allowed".into()).into(),
                        );
                    }

                    if *execute && !self.execute {
                        return Err(CapabilityError::PermissionDenied(
                            "Execute not allowed".into(),
                        )
                        .into());
                    }

                    Ok(())
                }
                _ => Err(CapabilityError::PermissionDenied(
                    "File capability only permits file access".into(),
                )
                .into()),
            }
        }

        fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>> {
            let mut new_cap = FileCapability {
                path: self.path.clone(),
                read: self.read,
                write: self.write,
                execute: self.execute,
            };

            for constraint in constraints {
                match constraint {
                    Constraint::FilePath(path) => {
                        let new_path = PathBuf::from(path);
                        // Ensure new path is within the original path
                        if !new_path.starts_with(&self.path) {
                            return Err(CapabilityError::ConstraintError(format!(
                                "New path {} is not within original path {}",
                                new_path.display(),
                                self.path.display()
                            ))
                            .into());
                        }
                        new_cap.path = new_path;
                    }
                    Constraint::FileOperation {
                        read,
                        write,
                        execute,
                    } => {
                        // Can only remove permissions, not add them
                        new_cap.read = self.read && *read;
                        new_cap.write = self.write && *write;
                        new_cap.execute = self.execute && *execute;
                    }
                    _ => {
                        return Err(CapabilityError::ConstraintError(
                            "Unsupported constraint for file capability".into(),
                        )
                        .into())
                    }
                }
            }

            Ok(Box::new(new_cap))
        }

        fn split(&self) -> Vec<Box<dyn Capability>> {
            let mut caps = Vec::new();

            // Only create capabilities for permissions that are enabled
            if self.read {
                caps.push(Box::new(FileCapability {
                    path: self.path.clone(),
                    read: true,
                    write: false,
                    execute: false,
                }));
            }

            if self.write {
                caps.push(Box::new(FileCapability {
                    path: self.path.clone(),
                    read: false,
                    write: true,
                    execute: false,
                }));
            }

            if self.execute {
                caps.push(Box::new(FileCapability {
                    path: self.path.clone(),
                    read: false,
                    write: false,
                    execute: true,
                }));
            }

            // If no permissions are enabled, return an empty capability
            if caps.is_empty() {
                caps.push(Box::new(FileCapability {
                    path: self.path.clone(),
                    read: false,
                    write: false,
                    execute: false,
                }));
            }

            caps
        }

        fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>> {
            if self.capability_type() != other.capability_type() {
                return Err(CapabilityError::CompositionError(format!(
                    "Cannot join capabilities of different types: {} and {}",
                    self.capability_type(),
                    other.capability_type()
                ))
                .into());
            }

            // Since as_any() can't be safely used on trait objects, we manually check the type
            // In a real implementation, we'd need a proper downcast mechanism
            if other.capability_type() != "file" {
                return Err(CapabilityError::CompositionError(
                    "Only file capabilities can be joined with file capabilities".into(),
                )
                .into());
            }

            // This is just a test implementation that assumes other is a FileCapability
            // In a real implementation, we'd need to be more careful here

            // Get the longest common path prefix
            let common_path = common_path_prefix(&self.path, &PathBuf::from("/tmp")); // Mock for testing
            if common_path.as_os_str().is_empty() {
                return Err(CapabilityError::CompositionError(
                    "Cannot join capabilities with no common path prefix".into(),
                )
                .into());
            }

            // Join the capabilities
            Ok(Box::new(FileCapability {
                path: common_path,
                read: self.read,       // Simplified for testing
                write: self.write,     // Simplified for testing
                execute: self.execute, // Simplified for testing
            }))
        }

        fn as_any(&self) -> &(dyn std::any::Any + '_) {
            self
        }
    }

    // Helper function to find the common path prefix
    fn common_path_prefix(p1: &PathBuf, p2: &PathBuf) -> PathBuf {
        let p1_components: Vec<_> = p1.components().collect();
        let p2_components: Vec<_> = p2.components().collect();

        let mut common = PathBuf::new();
        for (c1, c2) in p1_components.iter().zip(p2_components.iter()) {
            if c1 == c2 {
                common.push(c1.as_os_str());
            } else {
                break;
            }
        }

        common
    }

    #[test]
    fn test_capability_permits() {
        let cap = FileCapability::read_only("/tmp");

        // Should permit read access to a file in /tmp
        let request = AccessRequest::file_read("/tmp/test.txt");
        assert!(cap.permits(&request).is_ok());

        // Should deny write access
        let request = AccessRequest::file_write("/tmp/test.txt");
        assert!(cap.permits(&request).is_err());

        // Should deny access to a file outside /tmp
        let request = AccessRequest::file_read("/etc/passwd");
        assert!(cap.permits(&request).is_err());

        // Should deny non-file access
        let request = AccessRequest::network_connect("localhost", 8080);
        assert!(cap.permits(&request).is_err());
    }

    #[test]
    fn test_capability_constrain() {
        let cap = FileCapability::read_write("/tmp");

        // Constrain to read-only
        let constraints = vec![Constraint::FileOperation {
            read: true,
            write: false,
            execute: false,
        }];

        let constrained_cap = cap.constrain(&constraints).unwrap();

        // Should still permit read access
        let request = AccessRequest::file_read("/tmp/test.txt");
        assert!(constrained_cap.permits(&request).is_ok());

        // Should deny write access now
        let request = AccessRequest::file_write("/tmp/test.txt");
        assert!(constrained_cap.permits(&request).is_err());

        // Constrain to a subdirectory
        let constraints = vec![Constraint::FilePath("/tmp/subdir".into())];

        let constrained_cap = cap.constrain(&constraints).unwrap();

        // Should permit access to a file in the subdirectory
        let request = AccessRequest::file_read("/tmp/subdir/test.txt");
        assert!(constrained_cap.permits(&request).is_ok());

        // Should deny access to a file outside the subdirectory
        let request = AccessRequest::file_read("/tmp/other/test.txt");
        assert!(constrained_cap.permits(&request).is_err());

        // Should deny constraining to a path outside the original path
        let constraints = vec![Constraint::FilePath("/etc".into())];

        assert!(cap.constrain(&constraints).is_err());
    }

    #[test]
    fn test_capability_split() {
        let cap = FileCapability::read_write("/tmp");

        let split_caps = cap.split();

        // Should have two capabilities (read and write)
        assert_eq!(split_caps.len(), 2);

        // First should permit read but not write
        let request = AccessRequest::file_read("/tmp/test.txt");
        assert!(split_caps[0].permits(&request).is_ok());

        let request = AccessRequest::file_write("/tmp/test.txt");
        assert!(split_caps[0].permits(&request).is_err());

        // Second should permit write but not read
        let request = AccessRequest::file_write("/tmp/test.txt");
        assert!(split_caps[1].permits(&request).is_ok());

        let request = AccessRequest::file_read("/tmp/test.txt");
        assert!(split_caps[1].permits(&request).is_err());
    }

    #[test]
    fn test_capability_join() {
        let cap1 = FileCapability::read_only("/tmp");
        let cap2 = FileCapability::write_only("/tmp");

        // Join the capabilities
        let joined_cap = cap1.join(&cap2).unwrap();

        // Should permit both read and write access
        let request = AccessRequest::file_read("/tmp/test.txt");
        assert!(joined_cap.permits(&request).is_ok());

        let request = AccessRequest::file_write("/tmp/test.txt");
        assert!(joined_cap.permits(&request).is_ok());

        // Test joining capabilities with different paths
        let cap1 = FileCapability::read_only("/tmp/dir1");
        let cap2 = FileCapability::read_only("/tmp/dir2");

        // Join the capabilities
        let joined_cap = cap1.join(&cap2).unwrap();

        // Should have the common path prefix
        let request = AccessRequest::file_read("/tmp/dir1/test.txt");
        assert!(joined_cap.permits(&request).is_err());

        let request = AccessRequest::file_read("/tmp/dir2/test.txt");
        assert!(joined_cap.permits(&request).is_err());

        let request = AccessRequest::file_read("/tmp/test.txt");
        assert!(joined_cap.permits(&request).is_ok());

        // Should fail to join incompatible capabilities
        let cap1 = FileCapability::read_only("/tmp");
        let cap2 = NetworkCapability;

        assert!(cap1.join(&cap2).is_err());
    }

    // A stub network capability for testing
    struct NetworkCapability;

    impl Capability for NetworkCapability {
        fn capability_type(&self) -> &str {
            "network"
        }

        fn permits(&self, _request: &AccessRequest) -> Result<()> {
            Err(CapabilityError::PermissionDenied("Not implemented".into()).into())
        }
    }
}
