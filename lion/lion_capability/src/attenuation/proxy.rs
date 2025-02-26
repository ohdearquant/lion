use std::any::Any;
use std::fmt;
use std::sync::Arc;

use crate::model::{AccessRequest, Capability, CapabilityError, Constraint};

/// A transformation function type that maps requests to new requests
pub type TransformFn = Arc<dyn Fn(&AccessRequest) -> AccessRequest + Send + Sync + 'static>;

/// A capability that proxies requests through a transformation function
///
/// This wrapper transforms access requests before passing them to the inner
/// capability. It's useful for creating virtual views of resources, such as
/// remapping directories or hostnames.
#[derive(Clone)]
pub struct ProxyCapability {
    /// The inner capability that will handle transformed requests
    inner: Box<dyn Capability>,

    /// The transformation function that maps requests
    transform: TransformFn,

    /// Optional description of the transformation
    description: Option<String>,
}

impl ProxyCapability {
    /// Creates a new proxy capability
    pub fn new(inner: Box<dyn Capability>, transform: TransformFn) -> Self {
        Self {
            inner,
            transform,
            description: None,
        }
    }

    /// Creates a new proxy capability with a description
    pub fn with_description(
        inner: Box<dyn Capability>,
        transform: TransformFn,
        description: String,
    ) -> Self {
        Self {
            inner,
            transform,
            description: Some(description),
        }
    }

    /// Gets the inner capability
    pub fn inner(&self) -> &Box<dyn Capability> {
        &self.inner
    }

    /// Gets the transformation description, if any
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    /// Helper function to transform a request
    fn transform_request(&self, request: &AccessRequest) -> AccessRequest {
        (self.transform)(request)
    }
}

impl fmt::Debug for ProxyCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyCapability")
            .field("inner", &self.inner)
            .field("description", &self.description)
            .finish()
    }
}

impl Capability for ProxyCapability {
    fn capability_type(&self) -> &str {
        // Use the same type as the inner capability
        self.inner.capability_type()
    }

    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        // Transform the request
        let transformed = self.transform_request(request);

        // Delegate to the inner capability with the transformed request
        self.inner.permits(&transformed)
    }

    fn constrain(
        &self,
        constraints: &[Constraint],
    ) -> Result<Box<dyn Capability>, CapabilityError> {
        // Apply constraints to the inner capability
        let constrained_inner = self.inner.constrain(constraints)?;

        // Create a new proxy capability with the constrained inner
        Ok(Box::new(ProxyCapability {
            inner: constrained_inner,
            transform: self.transform.clone(),
            description: self.description.clone(),
        }))
    }

    fn split(&self) -> Vec<Box<dyn Capability>> {
        // Split the inner capability
        let inner_parts = self.inner.split();

        // Apply the transform to each part
        inner_parts
            .into_iter()
            .map(|part| {
                Box::new(ProxyCapability {
                    inner: part,
                    transform: self.transform.clone(),
                    description: self.description.clone(),
                }) as Box<dyn Capability>
            })
            .collect()
    }

    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to ProxyCapability
        if let Some(other_proxy) = other.as_any().downcast_ref::<ProxyCapability>() {
            // Join can be tricky with different transformations
            // For now, we'll just create a composite capability instead

            // Join the inner capabilities
            let joined_inner = self.inner.join(other_proxy.inner.as_ref())?;

            // Create a new proxy that transforms requests and forwards to the joined inner
            // This is an approximation, as we don't have a perfect way to combine
            // arbitrary transformation functions
            let combined_transform = Arc::new({
                let self_transform = self.transform.clone();

                move |request: &AccessRequest| (self_transform)(request)
            });

            let description = match (&self.description, &other_proxy.description) {
                (Some(d1), Some(d2)) => Some(format!("{} + {}", d1, d2)),
                (Some(d), None) | (None, Some(d)) => Some(d.clone()),
                (None, None) => None,
            };

            Ok(Box::new(ProxyCapability {
                inner: joined_inner,
                transform: combined_transform,
                description,
            }))
        } else {
            // Just join with the other capability directly
            let joined_inner = self.inner.join(other)?;

            Ok(Box::new(ProxyCapability {
                inner: joined_inner,
                transform: self.transform.clone(),
                description: self.description.clone(),
            }))
        }
    }

    fn leq(&self, other: &dyn Capability) -> bool {
        // Try to downcast the other capability to ProxyCapability
        if let Some(other_proxy) = other.as_any().downcast_ref::<ProxyCapability>() {
            // A proxy capability is <= another if:
            // 1. The inner capability is <= the other's inner
            // 2. The transformation functions are equivalent

            // Check inner capability
            if !self.inner.leq(other_proxy.inner.as_ref()) {
                return false;
            }

            // We can't generally check arbitrary functions for equivalence,
            // so we just return false unless they're the same function
            Arc::ptr_eq(&self.transform, &other_proxy.transform)
        } else {
            // If the other capability is not a proxy, we can't easily compare
            false
        }
    }

    fn meet(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to ProxyCapability
        if let Some(other_proxy) = other.as_any().downcast_ref::<ProxyCapability>() {
            // Meet can be tricky with different transformations
            // For now, we'll be conservative and meet the inner capabilities

            // Meet the inner capabilities
            let meet_inner = self.inner.meet(other_proxy.inner.as_ref())?;

            // Use this transformation function for the new proxy
            let description = match (&self.description, &other_proxy.description) {
                (Some(d1), Some(d2)) => Some(format!("{} ∩ {}", d1, d2)),
                (Some(d), None) | (None, Some(d)) => Some(d.clone()),
                (None, None) => None,
            };

            // Use this transformation function for the new proxy
            // This is conservative, as we can't properly combine two transformation functions
            Ok(Box::new(ProxyCapability {
                inner: meet_inner,
                transform: self.transform.clone(),
                description,
            }))
        } else {
            // Meet with the other capability directly
            let meet_inner = self.inner.meet(other)?;

            Ok(Box::new(ProxyCapability {
                inner: meet_inner,
                transform: self.transform.clone(),
                description: self.description.clone(),
            }))
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
    use crate::model::file::{FileCapability, FileOperations};
    use std::collections::HashSet;

    #[test]
    fn test_proxy_capability() {
        // Create a file capability for /home/user
        let paths = ["/home/user/*".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        // Create a proxy that transforms /tmp requests to /home/user requests
        let transform = Arc::new(|request: &AccessRequest| {
            match request {
                AccessRequest::File {
                    path,
                    read,
                    write,
                    execute,
                } => {
                    // Rewrite /tmp paths to /home/user paths
                    let transformed_path = if path.starts_with("/tmp/") {
                        path.replace("/tmp/", "/home/user/")
                    } else {
                        path.clone()
                    };

                    AccessRequest::File {
                        path: transformed_path,
                        read: *read,
                        write: *write,
                        execute: *execute,
                    }
                }
                _ => request.clone(),
            }
        });

        let proxy_cap = ProxyCapability::with_description(
            Box::new(file_cap),
            transform,
            "Map /tmp to /home/user".to_string(),
        );

        // Test that the proxy allows access to /tmp/file.txt, which gets transformed
        assert!(proxy_cap
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        // Test that the proxy denies access to other paths
        assert!(proxy_cap
            .permits(&AccessRequest::File {
                path: "/var/log/system.log".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());

        // Test that the proxy denies write access to transformed paths
        assert!(proxy_cap
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_err());
    }

    #[test]
    fn test_proxy_capability_join() {
        // Create a file capability for /home/user/docs
        let paths1 = ["/home/user/docs/*".to_string()].into_iter().collect();
        let file_cap1 = FileCapability::new(paths1, FileOperations::READ);

        // Create a proxy that transforms /tmp/docs requests to /home/user/docs requests
        let transform1 = Arc::new(|request: &AccessRequest| {
            match request {
                AccessRequest::File {
                    path,
                    read,
                    write,
                    execute,
                } => {
                    // Rewrite /tmp/docs paths to /home/user/docs paths
                    let transformed_path = if path.starts_with("/tmp/docs/") {
                        path.replace("/tmp/docs/", "/home/user/docs/")
                    } else {
                        path.clone()
                    };

                    AccessRequest::File {
                        path: transformed_path,
                        read: *read,
                        write: *write,
                        execute: *execute,
                    }
                }
                _ => request.clone(),
            }
        });

        let proxy_cap1 = ProxyCapability::new(Box::new(file_cap1), transform1);

        // Create a file capability for /home/user/pictures
        let paths2 = ["/home/user/pictures/*".to_string()].into_iter().collect();
        let file_cap2 = FileCapability::new(paths2, FileOperations::READ);

        // Create a proxy that transforms /tmp/pics requests to /home/user/pictures requests
        let transform2 = Arc::new(|request: &AccessRequest| {
            match request {
                AccessRequest::File {
                    path,
                    read,
                    write,
                    execute,
                } => {
                    // Rewrite /tmp/pics paths to /home/user/pictures paths
                    let transformed_path = if path.starts_with("/tmp/pics/") {
                        path.replace("/tmp/pics/", "/home/user/pictures/")
                    } else {
                        path.clone()
                    };

                    AccessRequest::File {
                        path: transformed_path,
                        read: *read,
                        write: *write,
                        execute: *execute,
                    }
                }
                _ => request.clone(),
            }
        });

        let proxy_cap2 = ProxyCapability::new(Box::new(file_cap2), transform2);

        // Join the two proxy capabilities
        let joined = proxy_cap1.join(&proxy_cap2).unwrap();

        // Test that the joined capability allows access to /tmp/docs
        assert!(joined
            .permits(&AccessRequest::File {
                path: "/tmp/docs/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        // but not to /tmp/pics because the join doesn't combine transformations perfectly
        // (this is a limitation of our current implementation)
        assert!(joined
            .permits(&AccessRequest::File {
                path: "/tmp/pics/image.jpg".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());
    }

    #[test]
    fn test_proxy_capability_constrain() {
        // Create a file capability for all of /home/user
        let paths = ["/home/user/*".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ | FileOperations::WRITE);

        // Create a proxy that transforms /tmp requests to /home/user requests
        let transform = Arc::new(|request: &AccessRequest| {
            match request {
                AccessRequest::File {
                    path,
                    read,
                    write,
                    execute,
                } => {
                    // Rewrite /tmp paths to /home/user paths
                    let transformed_path = if path.starts_with("/tmp/") {
                        path.replace("/tmp/", "/home/user/")
                    } else {
                        path.clone()
                    };

                    AccessRequest::File {
                        path: transformed_path,
                        read: *read,
                        write: *write,
                        execute: *execute,
                    }
                }
                _ => request.clone(),
            }
        });

        let proxy_cap = ProxyCapability::new(Box::new(file_cap), transform);

        // Constrain the proxy to read-only
        let constrained = proxy_cap
            .constrain(&[Constraint::FileOperation {
                read: true,
                write: false,
                execute: false,
            }])
            .unwrap();

        // Test that the constrained proxy allows read access
        assert!(constrained
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        // Test that the constrained proxy denies write access
        assert!(constrained
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_err());
    }
}
