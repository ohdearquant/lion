//! Filtered capability attenuation.
//! 
//! This module provides a capability that filters access requests.

use std::fmt;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;

use crate::model::Capability;

/// A capability that filters access requests.
pub struct FilterCapability {
    /// The inner capability.
    inner: Box<dyn Capability>,
    
    /// The filter function.
    filter: Box<dyn Fn(&AccessRequest) -> bool + Send + Sync>,
}

impl FilterCapability {
    /// Create a new filter capability.
    ///
    /// # Arguments
    ///
    /// * `inner` - The inner capability.
    /// * `filter` - The filter function.
    ///
    /// # Returns
    ///
    /// A new filter capability.
    pub fn new<F>(inner: Box<dyn Capability>, filter: F) -> Self
    where
        F: Fn(&AccessRequest) -> bool + Send + Sync + 'static,
    {
        Self {
            inner,
            filter: Box::new(filter),
        }
    }
    
    /// Get the inner capability.
    pub fn inner(&self) -> &dyn Capability {
        &*self.inner
    }
}

impl Capability for FilterCapability {
    fn capability_type(&self) -> &str {
        self.inner.capability_type()
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        // Check if the request passes the filter
        if !(self.filter)(request) {
            return Err(CapabilityError::PermissionDenied(
                "Request blocked by filter".into()
            ).into());
        }
        
        // Check if the inner capability permits the request
        self.inner.permits(request)
    }
    
    fn constrain(&self, constraints: &[crate::model::capability::Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        // Constrain the inner capability
        let constrained = self.inner.constrain(constraints)?;
        
        // Return a new filter capability with the constrained inner capability
        Ok(Box::new(Self::new(constrained, self.filter.clone())))
    }
    
    fn split(&self) -> Vec<Box<dyn Capability>> {
        // Split the inner capability
        let parts = self.inner.split();
        
        // Create a new filter capability for each part
        parts.into_iter()
            .map(|part| Box::new(Self::new(part, self.filter.clone())) as Box<dyn Capability>)
            .collect()
    }
    
    fn can_join_with(&self, other: &dyn Capability) -> bool {
        // Can join with another filter capability with the same filter
        if let Some(other_filter) = other.as_any().downcast_ref::<Self>() {
            // TODO: Compare filters (not possible with current approach)
            self.inner.can_join_with(&*other_filter.inner)
        } else {
            false
        }
    }
    
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Join with another filter capability with the same filter
        if let Some(other_filter) = other.as_any().downcast_ref::<Self>() {
            // TODO: Compare filters (not possible with current approach)
            let joined = self.inner.join(&*other_filter.inner)?;
            Ok(Box::new(Self::new(joined, self.filter.clone())))
        } else {
            Err(CapabilityError::CompositionError(
                "Can only join with another filter capability with the same filter".into()
            ).into())
        }
    }
    
    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(Self {
            inner: self.inner.clone_box(),
            filter: self.filter.clone(),
        })
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl fmt::Debug for FilterCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterCapability")
            .field("inner", &self.inner)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::FileCapability;
    use std::path::PathBuf;
    
    #[test]
    fn test_filter_capability() {
        // Create a file capability
        let file_capability = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        
        // Create a filter that only allows read requests
        let filter = |request: &AccessRequest| {
            matches!(request, AccessRequest::File { read, write, execute, .. } if *read && !*write && !*execute)
        };
        
        // Create a filter capability
        let capability = FilterCapability::new(file_capability, filter);
        
        // Check that it permits read access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_ok());
        
        // Check that it denies write access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
        
        // Check that it denies read+write access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: true,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
    }
    
    #[test]
    fn test_filter_capability_constrain() {
        // Create a file capability
        let file_capability = Box::new(FileCapability::new(
            vec![PathBuf::from("/tmp")],
            true,
            true,
            false,
        ));
        
        // Create a filter that only allows read requests
        let filter = |request: &AccessRequest| {
            matches!(request, AccessRequest::File { read, write, execute, .. } if *read && !*write && !*execute)
        };
        
        // Create a filter capability
        let capability = FilterCapability::new(file_capability, filter);
        
        // Constrain to read-only
        let constraints = vec![crate::model::capability::Constraint::FileOperation {
            read: true,
            write: false,
            execute: false,
        }];
        let constrained = capability.constrain(&constraints).unwrap();
        
        // Check that it permits read access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Check that it denies write access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(constrained.permits(&request).is_err());
    }
}