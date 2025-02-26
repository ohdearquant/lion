//! Composite capability model.
//! 
//! This module defines a capability that combines multiple other capabilities.

use std::fmt;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;

use super::capability::{Capability, Constraint};

/// A capability that combines multiple other capabilities.
pub struct CompositeCapability {
    /// The name of this composite capability.
    name: String,
    
    /// The capabilities that make up this composite.
    capabilities: Vec<Box<dyn Capability>>,
}

impl CompositeCapability {
    /// Create a new composite capability.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of this composite capability.
    /// * `capabilities` - The capabilities that make up this composite.
    ///
    /// # Returns
    ///
    /// A new composite capability.
    pub fn new(name: impl Into<String>, capabilities: Vec<Box<dyn Capability>>) -> Self {
        Self {
            name: name.into(),
            capabilities,
        }
    }
    
    /// Get the capabilities that make up this composite.
    pub fn capabilities(&self) -> &[Box<dyn Capability>] {
        &self.capabilities
    }
}

impl Capability for CompositeCapability {
    fn capability_type(&self) -> &str {
        "composite"
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        // Try each capability in turn
        for capability in &self.capabilities {
            if capability.permits(request).is_ok() {
                return Ok(());
            }
        }
        
        // No capability permitted the request
        Err(CapabilityError::PermissionDenied(
            format!("No capability in composite '{}' permitted the request", self.name)
        ).into())
    }
    
    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        // Apply the constraints to each capability
        let mut constrained_capabilities = Vec::new();
        
        for capability in &self.capabilities {
            match capability.constrain(constraints) {
                Ok(constrained) => constrained_capabilities.push(constrained),
                Err(_) => {}, // Skip capabilities that can't be constrained
            }
        }
        
        // If no capabilities could be constrained, return an error
        if constrained_capabilities.is_empty() {
            return Err(CapabilityError::ConstraintError(
                format!("No capability in composite '{}' could be constrained", self.name)
            ).into());
        }
        
        Ok(Box::new(Self {
            name: self.name.clone(),
            capabilities: constrained_capabilities,
        }))
    }
    
    fn split(&self) -> Vec<Box<dyn Capability>> {
        // Split each capability and collect the results
        let mut split_capabilities = Vec::new();
        
        for capability in &self.capabilities {
            split_capabilities.extend(capability.split());
        }
        
        // If we didn't split, just clone
        if split_capabilities.is_empty() {
            return vec![Box::new(self.clone())];
        }
        
        split_capabilities
    }
    
    fn can_join_with(&self, other: &dyn Capability) -> bool {
        // Can join with any capability
        true
    }
    
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // If other is a composite, merge the capabilities
        if other.capability_type() == "composite" {
            let mut capabilities = self.capabilities.clone();
            
            // Add the other's capabilities
            // This is a simplification; in a real implementation, we would
            // check the actual capabilities of the other composite
            capabilities.push(other.clone_box());
            
            return Ok(Box::new(Self {
                name: format!("{} + {}", self.name, other.capability_type()),
                capabilities,
            }));
        }
        
        // Otherwise, add the other capability to our list
        let mut capabilities = self.capabilities.clone();
        capabilities.push(other.clone_box());
        
        Ok(Box::new(Self {
            name: format!("{} + {}", self.name, other.capability_type()),
            capabilities,
        }))
    }
    
    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }
}

impl Clone for CompositeCapability {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            capabilities: self.capabilities.iter().map(|c| c.clone_box()).collect(),
        }
    }
}

impl fmt::Debug for CompositeCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompositeCapability")
            .field("name", &self.name)
            .field("capabilities", &format!("{} capabilities", self.capabilities.len()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::FileCapability;
    use crate::model::network::NetworkCapability;
    use crate::model::network::{NetworkHost, NetworkPort};
    use std::collections::HashMap;
    use std::path::PathBuf;
    
    #[test]
    fn test_composite_capability_permits() {
        // Create a file capability
        let file_capability = FileCapability::new(
            vec![PathBuf::from("/tmp")],
            true,
            false,
            false,
        );
        
        // Create a network capability
        let network_capability = NetworkCapability::outbound_only(
            vec![NetworkHost::Domain("example.com".to_string())],
            vec![NetworkPort::Port(80)],
        );
        
        // Create a composite capability
        let composite = CompositeCapability::new(
            "test_composite",
            vec![Box::new(file_capability), Box::new(network_capability)],
        );
        
        // Test file access
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(composite.permits(&request).is_ok());
        
        // Test network access
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(composite.permits(&request).is_ok());
        
        // Test disallowed file access
        let request = AccessRequest::File {
            path: PathBuf::from("/etc/passwd"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(composite.permits(&request).is_err());
        
        // Test disallowed network access
        let request = AccessRequest::Network {
            host: "evil.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(composite.permits(&request).is_err());
    }
    
    #[test]
    fn test_composite_capability_constrain() {
        // Create a file capability
        let file_capability = FileCapability::new(
            vec![PathBuf::from("/tmp")],
            true,
            true,
            false,
        );
        
        // Create a network capability
        let network_capability = NetworkCapability::outbound_only(
            vec![NetworkHost::Domain("example.com".to_string())],
            vec![NetworkPort::Port(80)],
        );
        
        // Create a composite capability
        let composite = CompositeCapability::new(
            "test_composite",
            vec![Box::new(file_capability), Box::new(network_capability)],
        );
        
        // Constrain to read-only
        let constraints = vec![Constraint::FileOperation {
            read: true,
            write: false,
            execute: false,
        }];
        let constrained = composite.constrain(&constraints).unwrap();
        
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
        
        // Should still allow network access
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(constrained.permits(&request).is_ok());
    }
    
    #[test]
    fn test_composite_capability_split() {
        // Create a file capability
        let file_capability = FileCapability::new(
            vec![PathBuf::from("/tmp")],
            true,
            false,
            false,
        );
        
        // Create a network capability
        let network_capability = NetworkCapability::outbound_only(
            vec![NetworkHost::Domain("example.com".to_string())],
            vec![NetworkPort::Port(80)],
        );
        
        // Create a composite capability
        let composite = CompositeCapability::new(
            "test_composite",
            vec![Box::new(file_capability), Box::new(network_capability)],
        );
        
        let split = composite.split();
        assert!(split.len() >= 2);
        
        // Check that file and network capabilities are separated
        let has_file_capability = split.iter().any(|c| {
            let request = AccessRequest::File {
                path: PathBuf::from("/tmp/file"),
                read: true,
                write: false,
                execute: false,
            };
            c.permits(&request).is_ok()
        });
        assert!(has_file_capability);
        
        let has_network_capability = split.iter().any(|c| {
            let request = AccessRequest::Network {
                host: "example.com".to_string(),
                port: 80,
                connect: true,
                listen: false,
            };
            c.permits(&request).is_ok()
        });
        assert!(has_network_capability);
    }
    
    #[test]
    fn test_composite_capability_join() {
        // Create a file capability
        let file_capability = FileCapability::new(
            vec![PathBuf::from("/tmp")],
            true,
            false,
            false,
        );
        
        // Create a network capability
        let network_capability = NetworkCapability::outbound_only(
            vec![NetworkHost::Domain("example.com".to_string())],
            vec![NetworkPort::Port(80)],
        );
        
        // Create two composite capabilities
        let composite1 = CompositeCapability::new(
            "file_composite",
            vec![Box::new(file_capability)],
        );
        
        let composite2 = CompositeCapability::new(
            "network_composite",
            vec![Box::new(network_capability)],
        );
        
        // Join the composites
        let joined = composite1.join(&composite2).unwrap();
        
        // Should allow file access
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should allow network access
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should deny write access
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: false,
            write: true,
            execute: false,
        };
        assert!(joined.permits(&request).is_err());
    }
}