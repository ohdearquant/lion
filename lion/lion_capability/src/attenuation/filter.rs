use std::any::Any;
use std::fmt;
use std::sync::Arc;

use crate::model::{AccessRequest, Capability, CapabilityError, Constraint};
use std::collections::HashSet;

/// A filter function type that determines if a request should be allowed
pub type FilterFn = Arc<dyn Fn(&AccessRequest) -> bool + Send + Sync + 'static>;

/// A capability that filters requests through a predicate function
///
/// This is a wrapper around another capability that only allows requests that
/// pass a filter function. It's useful for creating custom attenuations that
/// are more specific than what the constraint system allows.
#[derive(Clone)]
pub struct FilterCapability {
    /// The inner capability that will handle permitted requests
    inner: Box<dyn Capability>,

    /// The filter function that determines if a request is allowed
    filter: FilterFn,

    /// Optional description of the filter
    description: Option<String>,
}

impl FilterCapability {
    /// Creates a new filter capability
    pub fn new(inner: Box<dyn Capability>, filter: FilterFn) -> Self {
        Self {
            inner,
            filter,
            description: None,
        }
    }

    /// Creates a new filter capability with a description
    pub fn with_description(
        inner: Box<dyn Capability>,
        filter: FilterFn,
        description: String,
    ) -> Self {
        Self {
            inner,
            filter,
            description: Some(description),
        }
    }

    /// Gets the inner capability
    pub fn inner(&self) -> &Box<dyn Capability> {
        &self.inner
    }

    /// Gets the filter description, if any
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }
}

impl fmt::Debug for FilterCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterCapability")
            .field("inner", &self.inner)
            .field("description", &self.description)
            .finish()
    }
}

impl Capability for FilterCapability {
    fn capability_type(&self) -> &str {
        // Use the same type as the inner capability
        self.inner.capability_type()
    }

    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        // Check the filter first
        if !(self.filter)(request) {
            return Err(CapabilityError::AccessDenied(format!(
                "Request denied by filter: {:?}",
                request
            )));
        }

        // If it passes the filter, delegate to the inner capability
        self.inner.permits(request)
    }

    fn constrain(
        &self,
        constraints: &[Constraint],
    ) -> Result<Box<dyn Capability>, CapabilityError> {
        // Apply constraints to the inner capability
        let constrained_inner = self.inner.constrain(constraints)?;

        // Create a new filter capability with the constrained inner
        Ok(Box::new(FilterCapability {
            inner: constrained_inner,
            filter: self.filter.clone(),
            description: self.description.clone(),
        }))
    }

    fn split(&self) -> Vec<Box<dyn Capability>> {
        // Split the inner capability
        let inner_parts = self.inner.split();

        // Apply the filter to each part
        inner_parts
            .into_iter()
            .map(|part| {
                Box::new(FilterCapability {
                    inner: part,
                    filter: self.filter.clone(),
                    description: self.description.clone(),
                }) as Box<dyn Capability>
            })
            .collect()
    }

    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to FilterCapability
        if let Some(other_filter) = other.as_any().downcast_ref::<FilterCapability>() {
            // Create a new filter that combines both filters
            let combined_filter = Arc::new({
                let self_filter = self.filter.clone();
                let other_filter = other_filter.filter.clone();

                move |request: &AccessRequest| (self_filter)(request) || (other_filter)(request)
            });

            // Join the inner capabilities
            let joined_inner = self.inner.join(other_filter.inner.as_ref())?;

            // Create a new filter capability with the joined inner and combined filter
            let description = match (&self.description, &other_filter.description) {
                (Some(d1), Some(d2)) => Some(format!("{} OR {}", d1, d2)),
                (Some(d), None) | (None, Some(d)) => Some(d.clone()),
                (None, None) => None,
            };

            Ok(Box::new(FilterCapability {
                inner: joined_inner,
                filter: combined_filter,
                description,
            }))
        } else {
            // Just join with the other capability directly
            let joined_inner = self.inner.join(other)?;

            Ok(Box::new(FilterCapability {
                inner: joined_inner,
                filter: self.filter.clone(),
                description: self.description.clone(),
            }))
        }
    }

    fn leq(&self, other: &dyn Capability) -> bool {
        // Try to downcast the other capability to FilterCapability
        if let Some(other_filter) = other.as_any().downcast_ref::<FilterCapability>() {
            // A filter capability is <= another if:
            // 1. The inner capability is <= the other's inner
            // 2. Every request that passes our filter also passes the other's filter

            // Check inner capability
            if !self.inner.leq(other_filter.inner.as_ref()) {
                return false;
            }

            // We can't generally check arbitrary functions for subset relationships,
            // so we just return false unless they're the same filter
            Arc::ptr_eq(&self.filter, &other_filter.filter)
        } else {
            // If the other capability is not a filter, we just check against the inner
            self.inner.leq(other)
        }
    }

    fn meet(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // First, try the inner meet operation
        let meet_result = self.inner.meet(other);

        // If the inner meet fails (like when no common paths),
        // return an empty capability rather than propagating the error
        if meet_result.is_err() {
            // Create an appropriate empty capability
            match self.inner.capability_type() {
                "file" => {
                    use crate::model::file::{FileCapability, FileOperations};
                    return Ok(Box::new(FileCapability::new(
                        HashSet::new(),
                        FileOperations::empty(),
                    )));
                }
                _ => {} // Fall through to FilterCapability handling
            }
        }

        // Try to downcast the other capability to FilterCapability
        if let Some(other_filter) = other.as_any().downcast_ref::<FilterCapability>() {
            // Create a new filter that requires both filters to pass
            let combined_filter = Arc::new({
                let self_filter = self.filter.clone();
                let other_filter = other_filter.filter.clone();

                move |request: &AccessRequest| (self_filter)(request) && (other_filter)(request)
            });

            // Meet the inner capabilities or use the result we got earlier
            let meet_inner = meet_result.unwrap_or_else(|_| {
                // If the inner meet failed, just return an empty capability of the same type
                match self.inner.capability_type() {
                    "file" => {
                        use crate::model::file::{FileCapability, FileOperations};
                        Box::new(FileCapability::new(HashSet::new(), FileOperations::empty()))
                    }
                    _ => self.inner.clone_box(), // Just use a clone as fallback
                }
            });

            // Create a new filter capability with the meet inner and combined filter
            let description = match (&self.description, &other_filter.description) {
                (Some(d1), Some(d2)) => Some(format!("{} AND {}", d1, d2)),
                (Some(d), None) | (None, Some(d)) => Some(d.clone()),
                (None, None) => None,
            };

            Ok(Box::new(FilterCapability {
                inner: meet_inner,
                filter: combined_filter,
                description,
            }))
        } else {
            // Meet with the other capability directly
            let meet_inner = match meet_result {
                Ok(cap) => cap,
                Err(_) => {
                    // If the inner meet failed, just return an empty capability of the same type
                    match self.inner.capability_type() {
                        "file" => {
                            use crate::model::file::{FileCapability, FileOperations};
                            Box::new(FileCapability::new(HashSet::new(), FileOperations::empty()))
                        }
                        _ => return meet_result, // Propagate the error for non-file types
                    }
                }
            };

            Ok(Box::new(FilterCapability {
                inner: meet_inner,
                filter: self.filter.clone(),
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
    fn test_filter_capability() {
        // Create a file capability
        let paths = ["/tmp/*".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ | FileOperations::WRITE);

        // Create a filter that only allows read operations
        let filter = Arc::new(|request: &AccessRequest| match request {
            AccessRequest::File { read, write, .. } => *read && !*write,
            _ => false,
        });

        let filter_cap = FilterCapability::new(Box::new(file_cap), filter);

        // Test permitted request
        assert!(filter_cap
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        // Test denied by filter
        assert!(filter_cap
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: true,
                execute: false,
            })
            .is_err());

        // Test denied by inner capability
        assert!(filter_cap
            .permits(&AccessRequest::File {
                path: "/home/user/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());
    }

    #[test]
    fn test_filter_capability_join() {
        // Create two file capabilities
        let paths = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ | FileOperations::WRITE);

        // Create a filter that only allows read operations
        let read_filter = Arc::new(|request: &AccessRequest| match request {
            AccessRequest::File { read, write, .. } => *read && !*write,
            _ => false,
        });

        let read_cap = FilterCapability::with_description(
            Box::new(file_cap.clone()),
            read_filter,
            "read only".to_string(),
        );

        // Create a filter that only allows write operations
        let write_filter = Arc::new(|request: &AccessRequest| match request {
            AccessRequest::File { read, write, .. } => !*read && *write,
            _ => false,
        });

        let write_cap = FilterCapability::with_description(
            Box::new(file_cap),
            write_filter,
            "write only".to_string(),
        );

        // Join the two filter capabilities
        let joined = read_cap.join(&write_cap).unwrap();

        // The joined capability should allow either read or write
        assert!(joined
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        assert!(joined
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_ok());

        // But not both
        assert!(joined
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: true,
                execute: false,
            })
            .is_err());
    }

    #[test]
    fn test_filter_capability_meet() {
        // Create a file capability for all /tmp files
        let paths1 = ["/tmp/*".to_string()].into_iter().collect();
        let file_cap1 = FileCapability::new(paths1, FileOperations::READ);

        // Create a file capability for specifically /tmp/file.txt
        let paths2 = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap2 = FileCapability::new(paths2, FileOperations::READ | FileOperations::WRITE);

        // Create filter capabilities
        let filter_cap1 = FilterCapability::new(
            Box::new(file_cap1),
            Arc::new(|_| true), // Allow all requests
        );

        let filter_cap2 = FilterCapability::new(
            Box::new(file_cap2),
            Arc::new(|_| true), // Allow all requests
        );

        // Meet the two filter capabilities
        let meet = filter_cap1.meet(&filter_cap2).unwrap();

        // The meet should allow read access to /tmp/file.txt
        assert!(meet
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        // But not write access
        assert!(meet
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_err());

        // And not access to other files
        assert!(meet
            .permits(&AccessRequest::File {
                path: "/tmp/other.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());
    }
}
