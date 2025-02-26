use std::any::Any;

use crate::model::{AccessRequest, Capability, CapabilityError};

/// Apply partial revocation to a capability, removing specific access
///
/// This function takes a capability and an access request, and returns a new
/// capability that no longer permits that specific access request, but still
/// permits all other access that the original capability permitted.
///
/// The implementation uses the capability's `split` and `permits` methods to
/// identify and remove only the parts that would permit the specified access.
pub fn apply_partial_revocation(
    capability: Box<dyn Capability>,
    request: &AccessRequest,
) -> Result<Box<dyn Capability>, CapabilityError> {
    // Check if this capability even permits the request
    if !capability.permits(request).is_ok() {
        // Already doesn't permit this request, so no need to revoke
        return Ok(capability);
    }

    // Strategy 1: If the capability supports splitting, split it and
    // recombine all parts that don't permit the request
    let parts = capability.split();

    if parts.len() > 1 {
        // Filter out parts that permit the request
        let remaining_parts: Vec<Box<dyn Capability>> = parts
            .into_iter()
            .filter(|part| !part.permits(request).is_ok())
            .collect();

        if remaining_parts.is_empty() {
            return Err(CapabilityError::InvalidState(
                "Partial revocation would remove all permissions".to_string(),
            ));
        }

        // Join all remaining parts
        let mut result = remaining_parts[0].clone_box();

        for part in &remaining_parts[1..] {
            result = result.join(part.as_ref())?;
        }

        return Ok(result);
    }

    // Strategy 2: For capabilities that don't split well, try to apply constraints
    // derived from the request to further constrain the capability
    match request {
        AccessRequest::File {
            path,
            read,
            write,
            execute,
        } => {
            // For file capabilities, we can apply constraints to remove specific paths
            // or operations without removing everything

            use crate::model::Constraint;

            // If it's a specific path request, create a constraint that excludes just that path
            // by permitting only operations other than the requested ones
            let constraints = vec![Constraint::FileOperation {
                read: *read,
                write: *write,
                execute: *execute,
            }];

            // Apply the constraints to get a reduced capability
            let reduced = capability.constrain(&constraints)?;

            // Verify that the reduced capability no longer permits the request
            if reduced.permits(request).is_ok() {
                return Err(CapabilityError::InvalidState(
                    "Failed to revoke specific access through constraints".to_string(),
                ));
            }

            Ok(reduced)
        }
        // Handle other request types similarly
        // For now we'll just return an error for other types
        _ => Err(CapabilityError::UnsupportedOperation(
            "Partial revocation not implemented for this request type".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::{FileCapability, FileOperations};
    use std::collections::HashSet;

    #[test]
    fn test_partial_revocation_file() {
        // Create a file capability with multiple paths
        let paths = ["/tmp/file1.txt".to_string(), "/tmp/file2.txt".to_string()]
            .into_iter()
            .collect();

        let file_cap = FileCapability::new(paths, FileOperations::READ | FileOperations::WRITE);

        // Create a request for a specific access
        let request = AccessRequest::File {
            path: "/tmp/file1.txt".to_string(),
            read: true,
            write: false,
            execute: false,
        };

        // Apply partial revocation
        let reduced = apply_partial_revocation(Box::new(file_cap), &request).unwrap();

        // The reduced capability should no longer permit the request
        assert!(reduced.permits(&request).is_err());

        // But it should still permit other access
        assert!(reduced
            .permits(&AccessRequest::File {
                path: "/tmp/file2.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        assert!(reduced
            .permits(&AccessRequest::File {
                path: "/tmp/file1.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_ok());
    }

    #[test]
    fn test_partial_revocation_no_change() {
        // Create a file capability
        let paths = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        // Create a request that the capability does not permit
        let request = AccessRequest::File {
            path: "/tmp/file.txt".to_string(),
            read: false,
            write: true,
            execute: false,
        };

        // Apply partial revocation
        let reduced = apply_partial_revocation(Box::new(file_cap), &request).unwrap();

        // The reduced capability should be unchanged
        assert!(reduced
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());
    }
}
