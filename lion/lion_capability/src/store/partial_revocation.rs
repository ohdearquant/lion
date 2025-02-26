// Use the re-exported FilterCapability from the crate root
use crate::{AccessRequest, Capability, CapabilityError, FilterCapability};
use std::sync::Arc;

/// Apply partial revocation to a capability, removing specific access
///
/// This function takes a capability and an access request, and returns a new
/// capability that no longer permits that specific access request, but still
/// permits all other access that the original capability permitted.
pub fn apply_partial_revocation(
    capability: Box<dyn Capability>,
    request: &AccessRequest,
) -> Result<Box<dyn Capability>, CapabilityError> {
    // First check if the capability permits the request
    match capability.permits(request) {
        // If it doesn't permit, no need to revoke
        Err(_) => return Ok(capability),
        Ok(_) => {
            // Store the specific values we want to revoke
            let filter: Arc<dyn Fn(&AccessRequest) -> bool + Send + Sync + 'static> = match request
            {
                AccessRequest::File {
                    path,
                    read,
                    write,
                    execute,
                } => {
                    let path_clone = path.clone();
                    let is_read = *read;
                    let is_write = *write;
                    let is_execute = *execute;

                    Arc::new(move |req: &AccessRequest| {
                        if let AccessRequest::File {
                            path: req_path,
                            read: req_read,
                            write: req_write,
                            execute: req_execute,
                        } = req
                        {
                            // This is an exact match with the revoked request
                            if req_path == &path_clone
                                && *req_read == is_read
                                && *req_write == is_write
                                && *req_execute == is_execute
                            {
                                // Deny this specific request
                                return false;
                            }
                        }
                        // Allow all other requests
                        true
                    })
                }
                // For other request types, use a direct clone for comparison
                _ => {
                    let request_clone = request.clone();
                    Arc::new(move |req: &AccessRequest| req != &request_clone)
                }
            };

            // Create a FilterCapability to enforce this
            let description = format!("Partial revocation: {:?}", request);
            Ok(Box::new(FilterCapability::with_description(
                capability,
                filter,
                description,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::{FileCapability, FileOperations};

    #[test]
    fn test_partial_revocation_file() {
        // Create a file capability with multiple paths
        let paths = ["/tmp/file1.txt".to_string(), "/tmp/file2.txt".to_string()]
            .into_iter()
            .collect();

        let file_cap = FileCapability::new(paths, FileOperations::READ | FileOperations::WRITE);

        // Create a request for a specific access
        let revoke_request = AccessRequest::File {
            path: "/tmp/file1.txt".to_string(),
            read: true,
            write: false,
            execute: false,
        };

        // Apply partial revocation
        let reduced = apply_partial_revocation(Box::new(file_cap), &revoke_request).unwrap();

        // The reduced capability should no longer permit the revoke_request
        assert!(reduced.permits(&revoke_request).is_err());

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
