//! File capability model.
//! 
//! This module defines capabilities for file access.

use std::path::{Path, PathBuf};
use std::collections::HashSet;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;

use super::capability::{Capability, Constraint};

/// A capability that grants permission to access files.
#[derive(Debug, Clone)]
pub struct FileCapability {
    /// The paths that are allowed.
    paths: HashSet<PathBuf>,
    
    /// Whether read operations are allowed.
    read: bool,
    
    /// Whether write operations are allowed.
    write: bool,
    
    /// Whether execute operations are allowed.
    execute: bool,
}

impl FileCapability {
    /// Create a new file capability.
    ///
    /// # Arguments
    ///
    /// * `paths` - The paths that are allowed.
    /// * `read` - Whether read operations are allowed.
    /// * `write` - Whether write operations are allowed.
    /// * `execute` - Whether execute operations are allowed.
    ///
    /// # Returns
    ///
    /// A new file capability.
    pub fn new<P: AsRef<Path>>(
        paths: impl IntoIterator<Item = P>,
        read: bool,
        write: bool,
        execute: bool,
    ) -> Self {
        Self {
            paths: paths.into_iter().map(|p| p.as_ref().to_path_buf()).collect(),
            read,
            write,
            execute,
        }
    }
    
    /// Create a new read-only file capability.
    ///
    /// # Arguments
    ///
    /// * `paths` - The paths that are allowed.
    ///
    /// # Returns
    ///
    /// A new read-only file capability.
    pub fn read_only<P: AsRef<Path>>(paths: impl IntoIterator<Item = P>) -> Self {
        Self::new(paths, true, false, false)
    }
    
    /// Create a new write-only file capability.
    ///
    /// # Arguments
    ///
    /// * `paths` - The paths that are allowed.
    ///
    /// # Returns
    ///
    /// A new write-only file capability.
    pub fn write_only<P: AsRef<Path>>(paths: impl IntoIterator<Item = P>) -> Self {
        Self::new(paths, false, true, false)
    }
    
    /// Create a new read-write file capability.
    ///
    /// # Arguments
    ///
    /// * `paths` - The paths that are allowed.
    ///
    /// # Returns
    ///
    /// A new read-write file capability.
    pub fn read_write<P: AsRef<Path>>(paths: impl IntoIterator<Item = P>) -> Self {
        Self::new(paths, true, true, false)
    }
    
    /// Check if a path is allowed.
    ///
    /// A path is allowed if it is a direct prefix of one of the allowed paths.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to check.
    ///
    /// # Returns
    ///
    /// `true` if the path is allowed, `false` otherwise.
    fn is_path_allowed(&self, path: &Path) -> bool {
        // Canonicalize the path to avoid path traversal attacks
        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };
        
        for allowed_path in &self.paths {
            // Canonicalize the allowed path as well
            let canonical_allowed_path = match allowed_path.canonicalize() {
                Ok(p) => p,
                Err(_) => continue,
            };
            
            // Check if the canonical path starts with the canonical allowed path
            if canonical_path.starts_with(&canonical_allowed_path) {
                return true;
            }
        }
        
        false
    }
    
    /// Get the allowed paths.
    pub fn paths(&self) -> &HashSet<PathBuf> {
        &self.paths
    }
    
    /// Check if read operations are allowed.
    pub fn can_read(&self) -> bool {
        self.read
    }
    
    /// Check if write operations are allowed.
    pub fn can_write(&self) -> bool {
        self.write
    }
    
    /// Check if execute operations are allowed.
    pub fn can_execute(&self) -> bool {
        self.execute
    }
}

impl Capability for FileCapability {
    fn capability_type(&self) -> &str {
        "file"
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::File { path, read, write, execute } => {
                // Check if the path is allowed
                if !self.is_path_allowed(path) {
                    return Err(CapabilityError::PermissionDenied(
                        format!("Access to path {} is not allowed", path.display())
                    ).into());
                }
                
                // Check if the operations are allowed
                if *read && !self.read {
                    return Err(CapabilityError::PermissionDenied(
                        "Read access is not allowed".into()
                    ).into());
                }
                
                if *write && !self.write {
                    return Err(CapabilityError::PermissionDenied(
                        "Write access is not allowed".into()
                    ).into());
                }
                
                if *execute && !self.execute {
                    return Err(CapabilityError::PermissionDenied(
                        "Execute access is not allowed".into()
                    ).into());
                }
                
                Ok(())
            },
            _ => Err(CapabilityError::PermissionDenied(
                "Only file access is allowed".into()
            ).into()),
        }
    }
    
    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        let mut paths = self.paths.clone();
        let mut read = self.read;
        let mut write = self.write;
        let mut execute = self.execute;
        
        for constraint in constraints {
            match constraint {
                Constraint::FilePath(path) => {
                    // Filter paths that start with the constraint path
                    paths.retain(|p| p.starts_with(path));
                    
                    // If no paths remain, return an error
                    if paths.is_empty() {
                        return Err(CapabilityError::ConstraintError(
                            format!("No paths remain after applying constraint {}", path)
                        ).into());
                    }
                },
                Constraint::FileOperation { read: r, write: w, execute: e } => {
                    // Can only remove permissions, not add them
                    read = read && *r;
                    write = write && *w;
                    execute = execute && *e;
                    
                    // If all operations are disallowed, return an error
                    if !read && !write && !execute {
                        return Err(CapabilityError::ConstraintError(
                            "No operations allowed after applying constraint".into()
                        ).into());
                    }
                },
                _ => return Err(CapabilityError::ConstraintError(
                    format!("Constraint type {} not supported for file capability", constraint.constraint_type())
                ).into()),
            }
        }
        
        Ok(Box::new(Self { paths, read, write, execute }))
    }
    
    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut capabilities = Vec::new();
        
        // Split by operation
        if self.read {
            capabilities.push(Box::new(Self::new(
                self.paths.iter().cloned(),
                true,
                false,
                false,
            )) as Box<dyn Capability>);
        }
        
        if self.write {
            capabilities.push(Box::new(Self::new(
                self.paths.iter().cloned(),
                false,
                true,
                false,
            )) as Box<dyn Capability>);
        }
        
        if self.execute {
            capabilities.push(Box::new(Self::new(
                self.paths.iter().cloned(),
                false,
                false,
                true,
            )) as Box<dyn Capability>);
        }
        
        // If we didn't split by operation, just clone
        if capabilities.is_empty() {
            capabilities.push(Box::new(self.clone()));
        }
        
        capabilities
    }
    
    fn can_join_with(&self, other: &dyn Capability) -> bool {
        other.capability_type() == "file"
    }
    
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        if !self.can_join_with(other) {
            return Err(CapabilityError::CompositionError(
                format!("Cannot join file capability with {}", other.capability_type())
            ).into());
        }
        
        // Downcast the other capability to a FileCapability
        let other = match other.permits(&AccessRequest::File {
            path: PathBuf::from("/"),
            read: true,
            write: true,
            execute: true,
        }) {
            Ok(()) => {
                // If it permits everything, it's probably a super-capability
                return Ok(Box::new(Self {
                    paths: self.paths.union(&self.paths).cloned().collect(),
                    read: true,
                    write: true,
                    execute: true,
                }));
            },
            Err(_) => {
                // Try to get more precise information
                let mut paths = self.paths.clone();
                let mut read = self.read;
                let mut write = self.write;
                let mut execute = self.execute;
                
                // Check if it permits read
                if other.permits(&AccessRequest::File {
                    path: PathBuf::from("/"),
                    read: true,
                    write: false,
                    execute: false,
                }).is_ok() {
                    read = true;
                }
                
                // Check if it permits write
                if other.permits(&AccessRequest::File {
                    path: PathBuf::from("/"),
                    read: false,
                    write: true,
                    execute: false,
                }).is_ok() {
                    write = true;
                }
                
                // Check if it permits execute
                if other.permits(&AccessRequest::File {
                    path: PathBuf::from("/"),
                    read: false,
                    write: false,
                    execute: true,
                }).is_ok() {
                    execute = true;
                }
                
                // TODO: More precise path information
                
                Self {
                    paths,
                    read,
                    write,
                    execute,
                }
            }
        };
        
        // Join the capabilities
        let joined = Self {
            paths: self.paths.union(&other.paths).cloned().collect(),
            read: self.read || other.read,
            write: self.write || other.write,
            execute: self.execute || other.execute,
        };
        
        Ok(Box::new(joined))
    }
    
    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_file_capability_permits() {
        let capability = FileCapability::new(
            vec![PathBuf::from("/tmp")],
            true,
            false,
            false,
        );
        
        // Test read access to allowed path
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_ok());
        
        // Test write access to allowed path (should fail)
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
        
        // Test read access to disallowed path
        let request = AccessRequest::File {
            path: PathBuf::from("/etc/passwd"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
        
        // Test non-file access
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(capability.permits(&request).is_err());
    }
    
    #[test]
    fn test_file_capability_constrain() {
        let capability = FileCapability::new(
            vec![PathBuf::from("/tmp"), PathBuf::from("/var/log")],
            true,
            true,
            false,
        );
        
        // Constrain to a subset of paths
        let constraints = vec![Constraint::FilePath("/tmp".to_string())];
        let constrained = capability.constrain(&constraints).unwrap();
        
        // Should allow access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Should deny access to /var/log
        let request = AccessRequest::File {
            path: PathBuf::from("/var/log/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(constrained.permits(&request).is_err());
        
        // Constrain to read-only
        let constraints = vec![Constraint::FileOperation {
            read: true,
            write: false,
            execute: false,
        }];
        let constrained = capability.constrain(&constraints).unwrap();
        
        // Should allow read access
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Should deny write access
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(constrained.permits(&request).is_err());
    }
    
    #[test]
    fn test_file_capability_split() {
        let capability = FileCapability::new(
            vec![PathBuf::from("/tmp")],
            true,
            true,
            false,
        );
        
        let split = capability.split();
        assert_eq!(split.len(), 2);
        
        // Check that the first capability allows read but not write
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(split[0].permits(&request).is_ok());
        
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(split[0].permits(&request).is_err());
        
        // Check that the second capability allows write but not read
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(split[1].permits(&request).is_err());
        
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(split[1].permits(&request).is_ok());
    }
    
    #[test]
    fn test_file_capability_join() {
        let capability1 = FileCapability::new(
            vec![PathBuf::from("/tmp")],
            true,
            false,
            false,
        );
        
        let capability2 = FileCapability::new(
            vec![PathBuf::from("/var/log")],
            false,
            true,
            false,
        );
        
        let joined = capability1.join(&capability2).unwrap();
        
        // Should allow read access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should allow write access to /var/log
        let request = AccessRequest::File {
            path: PathBuf::from("/var/log/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should deny write access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(joined.permits(&request).is_err());
        
        // Should deny read access to /var/log
        let request = AccessRequest::File {
            path: PathBuf::from("/var/log/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(joined.permits(&request).is_err());
    }
}