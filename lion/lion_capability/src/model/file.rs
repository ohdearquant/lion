use bitflags::bitflags;
use std::any::Any;
use std::collections::HashSet;

use super::capability::{path_matches, AccessRequest, Capability, CapabilityError, Constraint};

bitflags! {
    /// Represents file operation permissions as a bit field for efficient checking
    pub struct FileOperations: u8 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
        const EXECUTE = 0b00000100;
    }
}

/// Represents a capability to access files in the filesystem
#[derive(Debug, Clone)]
pub struct FileCapability {
    /// Set of path patterns that this capability grants access to
    paths: HashSet<String>,

    /// Operations permitted on those paths
    operations: FileOperations,
}

impl FileCapability {
    /// Creates a new file capability with the specified paths and operations
    pub fn new(paths: HashSet<String>, operations: FileOperations) -> Self {
        Self { paths, operations }
    }

    /// Creates a new file capability with a single path and operations
    pub fn with_path(path: String, operations: FileOperations) -> Self {
        let mut paths = HashSet::new();
        paths.insert(path);
        Self { paths, operations }
    }

    /// Creates a new file capability for read access
    pub fn read_only(paths: HashSet<String>) -> Self {
        Self {
            paths,
            operations: FileOperations::READ,
        }
    }

    /// Creates a new file capability for write access
    pub fn write_only(paths: HashSet<String>) -> Self {
        Self {
            paths,
            operations: FileOperations::WRITE,
        }
    }

    /// Creates a new file capability for read and write access
    pub fn read_write(paths: HashSet<String>) -> Self {
        Self {
            paths,
            operations: FileOperations::READ | FileOperations::WRITE,
        }
    }

    /// Gets paths this capability grants access to
    pub fn paths(&self) -> &HashSet<String> {
        &self.paths
    }

    /// Gets operations this capability permits
    pub fn operations(&self) -> FileOperations {
        self.operations
    }

    /// Checks if this capability allows access to the given path
    fn path_allowed(&self, path: &str) -> bool {
        self.paths.iter().any(|pattern| path_matches(pattern, path))
    }

    /// Applies a path constraint to this capability
    fn apply_path_constraint(&self, path: &str) -> Result<Self, CapabilityError> {
        // Check if the constrained path is allowed by any of our patterns
        if !self.path_allowed(path) {
            return Err(CapabilityError::InvalidConstraint(format!(
                "Path '{}' is not covered by this capability",
                path
            )));
        }

        // Create a new capability with just this path
        let mut paths = HashSet::new();
        paths.insert(path.to_string());

        Ok(Self {
            paths,
            operations: self.operations,
        })
    }

    /// Applies an operation constraint to this capability
    fn apply_operation_constraint(
        &self,
        read: bool,
        write: bool,
        execute: bool,
    ) -> Result<Self, CapabilityError> {
        let mut new_ops = FileOperations::empty();

        // Only allow operations that both the constraint and this capability permit
        if read && self.operations.contains(FileOperations::READ) {
            new_ops |= FileOperations::READ;
        }

        if write && self.operations.contains(FileOperations::WRITE) {
            new_ops |= FileOperations::WRITE;
        }

        if execute && self.operations.contains(FileOperations::EXECUTE) {
            new_ops |= FileOperations::EXECUTE;
        }

        // If no operations are allowed after applying the constraint, return an error
        if new_ops.is_empty() {
            return Err(CapabilityError::InvalidConstraint(
                "No operations would be allowed after applying this constraint".to_string(),
            ));
        }

        Ok(Self {
            paths: self.paths.clone(),
            operations: new_ops,
        })
    }
}

impl Capability for FileCapability {
    fn capability_type(&self) -> &str {
        "file"
    }

    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::File {
                path,
                read,
                write,
                execute,
            } => {
                // Check if the path is allowed
                if !self.path_allowed(path) {
                    return Err(CapabilityError::AccessDenied(format!(
                        "Path '{}' is not allowed by this capability",
                        path
                    )));
                }

                // Check if the requested operations are permitted
                let mut requested_ops = FileOperations::empty();
                if *read {
                    requested_ops |= FileOperations::READ;
                }
                if *write {
                    requested_ops |= FileOperations::WRITE;
                }
                if *execute {
                    requested_ops |= FileOperations::EXECUTE;
                }

                if !self.operations.contains(requested_ops) {
                    return Err(CapabilityError::AccessDenied(format!(
                        "Operation not permitted on path '{}'",
                        path
                    )));
                }

                Ok(())
            }
            _ => Err(CapabilityError::IncompatibleTypes(format!(
                "Expected File request, got {:?}",
                request
            ))),
        }
    }

    fn constrain(
        &self,
        constraints: &[Constraint],
    ) -> Result<Box<dyn Capability>, CapabilityError> {
        let mut result = self.clone();

        for constraint in constraints {
            match constraint {
                Constraint::FilePath(path) => {
                    result = result.apply_path_constraint(path)?;
                }
                Constraint::FileOperation {
                    read,
                    write,
                    execute,
                } => {
                    result = result.apply_operation_constraint(*read, *write, *execute)?;
                }
                _ => {
                    return Err(CapabilityError::InvalidConstraint(format!(
                        "Constraint {:?} not applicable to FileCapability",
                        constraint
                    )))
                }
            }
        }

        Ok(Box::new(result))
    }

    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut result = Vec::new();

        // Split by path
        for path in &self.paths {
            result.push(Box::new(FileCapability {
                paths: [path.clone()].into_iter().collect(),
                operations: self.operations,
            }) as Box<dyn Capability>);
        }

        result
    }

    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to FileCapability
        if let Some(other_file) = other.as_any().downcast_ref::<FileCapability>() {
            // Create a union of the paths
            let mut paths = self.paths.clone();
            paths.extend(other_file.paths.clone());

            // Take the union of operations
            let operations = self.operations | other_file.operations;

            Ok(Box::new(FileCapability { paths, operations }))
        } else {
            Err(CapabilityError::IncompatibleTypes(
                "Cannot join FileCapability with a different capability type".to_string(),
            ))
        }
    }

    fn leq(&self, other: &dyn Capability) -> bool {
        // Try to downcast the other capability to FileCapability
        if let Some(other_file) = other.as_any().downcast_ref::<FileCapability>() {
            // This capability is <= other if:
            // 1. All paths in self are allowed in other
            // 2. Operations in self are a subset of operations in other

            // Check operations first (faster)
            if !(self.operations & other_file.operations).bits() == self.operations.bits() {
                return false;
            }

            // Check paths
            for path in &self.paths {
                if !other_file.path_allowed(path) {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    fn meet(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to FileCapability
        if let Some(other_file) = other.as_any().downcast_ref::<FileCapability>() {
            // Find the intersection of paths
            let paths: HashSet<String> = self
                .paths
                .intersection(&other_file.paths)
                .cloned()
                .collect();

            if paths.is_empty() {
                return Err(CapabilityError::InvalidState(
                    "No paths in common between the capabilities".to_string(),
                ));
            }

            // Take the intersection of operations
            let operations = self.operations & other_file.operations;

            if operations.is_empty() {
                return Err(CapabilityError::InvalidState(
                    "No operations in common between the capabilities".to_string(),
                ));
            }

            Ok(Box::new(FileCapability { paths, operations }))
        } else {
            Err(CapabilityError::IncompatibleTypes(
                "Cannot compute meet with different capability types".to_string(),
            ))
        }
    }

    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_allowed() {
        let paths = ["/tmp/*".to_string(), "/home/user/file.txt".to_string()]
            .into_iter()
            .collect();
        let cap = FileCapability::new(paths, FileOperations::READ);

        assert!(cap.path_allowed("/tmp/test.txt"));
        assert!(cap.path_allowed("/home/user/file.txt"));
        assert!(!cap.path_allowed("/home/user/other.txt"));
        assert!(!cap.path_allowed("/var/log/system.log"));
    }

    #[test]
    fn test_permits() {
        let paths = ["/tmp/*".to_string(), "/home/user/file.txt".to_string()]
            .into_iter()
            .collect();
        let cap = FileCapability::new(paths, FileOperations::READ | FileOperations::WRITE);

        // Valid requests
        assert!(cap
            .permits(&AccessRequest::File {
                path: "/tmp/test.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        assert!(cap
            .permits(&AccessRequest::File {
                path: "/tmp/test.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_ok());

        // Invalid path
        assert!(cap
            .permits(&AccessRequest::File {
                path: "/var/log/system.log".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());

        // Invalid operation
        assert!(cap
            .permits(&AccessRequest::File {
                path: "/tmp/test.txt".to_string(),
                read: false,
                write: false,
                execute: true,
            })
            .is_err());

        // Invalid request type
        assert!(cap
            .permits(&AccessRequest::Network {
                host: Some("example.com".to_string()),
                port: Some(80),
                connect: true,
                listen: false,
                bind: false,
            })
            .is_err());
    }

    #[test]
    fn test_constrain() {
        let paths = ["/tmp/*".to_string(), "/home/user/file.txt".to_string()]
            .into_iter()
            .collect();
        let cap = FileCapability::new(paths, FileOperations::READ | FileOperations::WRITE);

        // Constrain by path
        let constrained = cap
            .constrain(&[Constraint::FilePath("/tmp/specific.txt".to_string())])
            .unwrap();
        assert!(constrained
            .permits(&AccessRequest::File {
                path: "/tmp/specific.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        assert!(constrained
            .permits(&AccessRequest::File {
                path: "/tmp/other.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());

        // Constrain by operation
        let constrained = cap
            .constrain(&[Constraint::FileOperation {
                read: true,
                write: false,
                execute: false,
            }])
            .unwrap();

        assert!(constrained
            .permits(&AccessRequest::File {
                path: "/tmp/test.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        assert!(constrained
            .permits(&AccessRequest::File {
                path: "/tmp/test.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_err());
    }

    #[test]
    fn test_leq() {
        let paths1 = ["/tmp/test.txt".to_string()].into_iter().collect();
        let cap1 = FileCapability::new(paths1, FileOperations::READ);

        let paths2 = ["/tmp/*".to_string()].into_iter().collect();
        let cap2 = FileCapability::new(paths2, FileOperations::READ | FileOperations::WRITE);

        // cap1 <= cap2 because cap1's paths are a subset and operations are a subset
        assert!(cap1.leq(&cap2));

        // cap2 !<= cap1 because cap2 has more operations
        assert!(!cap2.leq(&cap1));
    }

    #[test]
    fn test_join_and_meet() {
        let paths1 = ["/tmp/file1.txt".to_string(), "/tmp/file2.txt".to_string()]
            .into_iter()
            .collect();
        let cap1 = FileCapability::new(paths1, FileOperations::READ);

        let paths2 = ["/tmp/file2.txt".to_string(), "/tmp/file3.txt".to_string()]
            .into_iter()
            .collect();
        let cap2 = FileCapability::new(paths2, FileOperations::READ | FileOperations::WRITE);

        // Join
        let join = cap1.join(&cap2).unwrap();
        let join_file = join.as_any().downcast_ref::<FileCapability>().unwrap();

        assert_eq!(join_file.paths.len(), 3);
        assert!(join_file.operations.contains(FileOperations::READ));
        assert!(join_file.operations.contains(FileOperations::WRITE));

        // Meet
        let meet = cap1.meet(&cap2).unwrap();
        let meet_file = meet.as_any().downcast_ref::<FileCapability>().unwrap();

        assert_eq!(meet_file.paths.len(), 1);
        assert!(meet_file.paths.contains("/tmp/file2.txt"));
        assert!(meet_file.operations.contains(FileOperations::READ));
        assert!(!meet_file.operations.contains(FileOperations::WRITE));
    }
}
