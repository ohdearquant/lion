//! Proxy capability attenuation.
//! 
//! This module provides a capability that proxies access requests.

use std::fmt;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;

use crate::model::Capability;

/// A capability that proxies access requests.
pub struct ProxyCapability {
    /// The inner capability.
    inner: Box<dyn Capability>,
    
    /// The transform function.
    transform: Box<dyn Fn(&AccessRequest) -> AccessRequest + Send + Sync>,
}

impl ProxyCapability {
    /// Create a new proxy capability.
    ///
    /// # Arguments
    ///
    /// * `inner` - The inner capability.
    /// * `transform` - The transform function.
    ///
    /// # Returns
    ///
    /// A new proxy capability.
    pub fn new<F>(inner: Box<dyn Capability>, transform: F) -> Self
    where
        F: Fn(&AccessRequest) -> AccessRequest + Send + Sync + 'static,
    {
        Self {
            inner,
            transform: Box::new(transform),
        }
    }
    
    /// Get the inner capability.
    pub fn inner(&self) -> &dyn Capability {
        &*self.inner
    }
}

impl Capability for ProxyCapability {
    fn capability_type(&self) -> &str {
        self.inner.capability_type()
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        // Transform the request
        let transformed = (self.transform)(request);
        
        // Check if the inner capability permits the transformed request
        self.inner.permits(&transformed)
    }
    
    fn constrain(&self, constraints: &[crate::model::capability::Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        // Constrain the inner capability
        let constrained = self.inner.constrain(constraints)?;
        
        // Return a new proxy capability with the constrained inner capability
        Ok(Box::new(Self::new(constrained, self.transform.clone())))
    }
    
    fn split(&self) -> Vec<Box<dyn Capability>> {
        // Split the inner capability
        let parts = self.inner.split();
        
        // Create a new proxy capability for each part
        parts.into_iter()
            .map(|part| Box::new(Self::new(part, self.transform.clone())) as Box<dyn Capability>)
            .collect()
    }
    
    fn can_join_with(&self, other: &dyn Capability) -> bool {
        // Can join with another proxy capability with the same transform
        if let Some(other_proxy) = other.as_any().downcast_ref::<Self>() {
            // TODO: Compare transforms (not possible with current approach)
            self.inner.can_join_with(&*other_proxy.inner)
        } else {
            false
        }
    }
    
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Join with another proxy capability with the same transform
        if let Some(other_proxy) = other.as_any().downcast_ref::<Self>() {
            // TODO: Compare transforms (not possible with current approach)
            let joined = self.inner.join(&*other_proxy.inner)?;
            Ok(Box::new(Self::new(joined, self.transform.clone())))
        } else {
            Err(CapabilityError::CompositionError(
                "Can only join with another proxy capability with the same transform".into()
            ).into())
        }
    }
    
    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(Self {
            inner: self.inner.clone_box(),
            transform: self.transform.clone(),
        })
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl fmt::Debug for ProxyCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyCapability")
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
    fn test_proxy_capability() {
        // Create a file capability
        let file_capability = Box::new(FileCapability::read_only(vec![PathBuf::from("/var/www")]));
        
        // Create a transform that maps /tmp to /var/www
        let transform = |request: &AccessRequest| {
            match request {
                AccessRequest::File { path, read, write, execute } => {
                    let new_path = if path.starts_with("/tmp") {
                        let rel_path = path.strip_prefix("/tmp").unwrap();
                        PathBuf::from("/var/www").join(rel_path)
                    } else {
                        path.clone()
                    };
                    
                    AccessRequest::File {
                        path: new_path,
                        read: *read,
                        write: *write,
                        execute: *execute,
                    }
                },
                _ => request.clone(),
            }
        };
        
        // Create a proxy capability
        let capability = ProxyCapability::new(file_capability, transform);
        
        // Check that it permits read access to /tmp (mapped to /var/www)
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_ok());
        
        // Check that it denies write access to /tmp (mapped to /var/www)
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
        
        // Check that it denies access to /etc (not mapped)
        let request = AccessRequest::File {
            path: PathBuf::from("/etc/passwd"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
    }
}