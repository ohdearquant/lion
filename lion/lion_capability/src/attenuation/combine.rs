//! Combined capability attenuation.
//! 
//! This module provides a capability that combines multiple capabilities.

use std::fmt;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;

use crate::model::Capability;

/// A capability that combines multiple capabilities.
pub struct CombineCapability {
    /// The capabilities.
    capabilities: Vec<Box<dyn Capability>>,
    
    /// The combine strategy.
    strategy: CombineStrategy,
}

/// Strategy for combining capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombineStrategy {
    /// All capabilities must permit the access.
    All,
    
    /// Any capability can permit the access.
    Any,
}

impl CombineCapability {
    /// Create a new combine capability.
    ///
    /// # Arguments
    ///
    /// * `capabilities` - The capabilities to combine.
    /// * `strategy` - The combine strategy.
    ///
    /// # Returns
    ///
    /// A new combine capability.
    pub fn new(capabilities: Vec<Box<dyn Capability>>, strategy: CombineStrategy) -> Self {
        Self { capabilities, strategy }
    }
    
    /// Create a new combine capability where all capabilities must permit the access.
    ///
    /// # Arguments
    ///
    /// * `capabilities` - The capabilities to combine.
    ///
    /// # Returns
    ///
    /// A new combine capability.
    pub fn all(capabilities: Vec<Box<dyn Capability>>) -> Self {
        Self::new(capabilities, CombineStrategy::All)
    }
    
    /// Create a new combine capability where any capability can permit the access.
    ///
    /// # Arguments
    ///
    /// * `capabilities` - The capabilities to combine.
    ///
    /// # Returns
    ///
    /// A new combine capability.
    pub fn any(capabilities: Vec<Box<dyn Capability>>) -> Self {
        Self::new(capabilities, CombineStrategy::Any)
    }
    
    /// Get the capabilities.
    pub fn capabilities(&self) -> &[Box<dyn Capability>] {
        &self.capabilities
    }
    
    /// Get the combine strategy.
    pub fn strategy(&self) -> CombineStrategy {
        self.strategy
    }
}

impl Capability for CombineCapability {
    fn capability_type(&self) -> &str {
        "combine"
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match self.strategy {
            CombineStrategy::All => {
                // All capabilities must permit the access
                for capability in &self.capabilities {
                    capability.permits(request)?;
                }
                
                Ok(())
            },
            CombineStrategy::Any => {
                // Any capability can permit the access
                let mut last_error = None;
                
                for capability in &self.capabilities {
                    match capability.permits(request) {
                        Ok(()) => return Ok(()),
                        Err(e) => last_error = Some(e),
                    }
                }
                
                // No capability permitted the access
                Err(last_error.unwrap_or_else(|| {
                    CapabilityError::PermissionDenied(
                        "No capability permitted the access".into()
                    ).into()
                }))
            },
        }
    }
    
    fn constrain(&self, constraints: &[crate::model::capability::Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        // Constrain each capability
        let mut constrained = Vec::new();
        
        for capability in &self.capabilities {
            match capability.constrain(constraints) {
                Ok(c) => constrained.push(c),
                Err(e) => {
                    if self.strategy == CombineStrategy::All {
                        // For ALL strategy, all capabilities must be constrainable
                        return Err(e);
                    }
                    // For ANY strategy, we can skip capabilities that can't be constrained
                }
            }
        }
        
        // For ANY strategy, at least one capability must be constrainable
        if self.strategy == CombineStrategy::Any && constrained.is_empty() {
            return Err(CapabilityError::ConstraintError(
                "No capability could be constrained".into()
            ).into());
        }
        
        Ok(Box::new(Self::new(constrained, self.strategy)))
    }
    
    fn split(&self) -> Vec<Box<dyn Capability>> {
        // For now, just clone
        vec![Box::new(self.clone())]
    }
    
    fn can_join_with(&self, other: &dyn Capability) -> bool {
        // Can join with another combine capability with the same strategy
        if let Some(other_combine) = other.as_any().downcast_ref::<Self>() {
            self.strategy == other_combine.strategy
        } else {
            false
        }
    }
    
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Join with another combine capability with the same strategy
        if let Some(other_combine) = other.as_any().downcast_ref::<Self>() {
            if self.strategy != other_combine.strategy {
                return Err(CapabilityError::CompositionError(
                    "Cannot join combine capabilities with different strategies".into()
                ).into());
            }
            
            // Combine the capabilities
            let mut capabilities = self.capabilities.clone();
            capabilities.extend(other_combine.capabilities.iter().map(|c| c.clone_box()));
            
            Ok(Box::new(Self::new(capabilities, self.strategy)))
        } else {
            Err(CapabilityError::CompositionError(
                "Can only join with another combine capability".into()
            ).into())
        }
    }
    
    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Clone for CombineCapability {
    fn clone(&self) -> Self {
        Self {
            capabilities: self.capabilities.iter().map(|c| c.clone_box()).collect(),
            strategy: self.strategy,
        }
    }
}

impl fmt::Debug for CombineCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CombineCapability")
            .field("strategy", &self.strategy)
            .field("capabilities", &format!("{} capabilities", self.capabilities.len()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::FileCapability;
    use crate::model::network::{NetworkCapability, NetworkHost, NetworkPort};
    use std::path::PathBuf;
    
    #[test]
    fn test_combine_capability_all() {
        // Create file capabilities
        let file_read = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        let file_write = Box::new(FileCapability::write_only(vec![PathBuf::from("/tmp")]));
        
        // Create a combine capability that requires both read and write access
        let capability = CombineCapability::all(vec![file_read, file_write]);
        
        // Check that it denies read-only access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
        
        // Check that it denies write-only access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
        
        // Check that it permits read+write access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: true,
            execute: false,
        };
        assert!(capability.permits(&request).is_ok());
    }
    
    #[test]
    fn test_combine_capability_any() {
        // Create capabilities
        let file = Box::new(FileCapability::read_only(vec![PathBuf::from("/tmp")]));
        let network = Box::new(NetworkCapability::outbound_only(
            vec![NetworkHost::Domain("example.com".to_string())],
            vec![NetworkPort::Port(80)],
        ));
        
        // Create a combine capability that allows either file or network access
        let capability = CombineCapability::any(vec![file, network]);
        
        // Check that it permits read access to /tmp
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_ok());
        
        // Check that it permits network access to example.com:80
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
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
    }
}