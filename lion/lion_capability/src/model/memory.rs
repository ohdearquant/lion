//! Memory capability model.
//! 
//! This module defines capabilities for memory access.

use std::collections::HashSet;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;
use lion_core::id::RegionId;

use super::capability::{Capability, Constraint};

/// A capability that grants permission to access memory regions.
#[derive(Debug, Clone)]
pub struct MemoryCapability {
    /// The regions that are allowed.
    regions: HashSet<String>,
    
    /// Whether read operations are allowed.
    read: bool,
    
    /// Whether write operations are allowed.
    write: bool,
}

impl MemoryCapability {
    /// Create a new memory capability.
    ///
    /// # Arguments
    ///
    /// * `regions` - The regions that are allowed.
    /// * `read` - Whether read operations are allowed.
    /// * `write` - Whether write operations are allowed.
    ///
    /// # Returns
    ///
    /// A new memory capability.
    pub fn new(
        regions: impl IntoIterator<Item = String>,
        read: bool,
        write: bool,
    ) -> Self {
        Self {
            regions: regions.into_iter().collect(),
            read,
            write,
        }
    }
    
    /// Create a new read-only memory capability.
    ///
    /// # Arguments
    ///
    /// * `regions` - The regions that are allowed.
    ///
    /// # Returns
    ///
    /// A new read-only memory capability.
    pub fn read_only(regions: impl IntoIterator<Item = String>) -> Self {
        Self::new(regions, true, false)
    }
    
    /// Create a new write-only memory capability.
    ///
    /// # Arguments
    ///
    /// * `regions` - The regions that are allowed.
    ///
    /// # Returns
    ///
    /// A new write-only memory capability.
    pub fn write_only(regions: impl IntoIterator<Item = String>) -> Self {
        Self::new(regions, false, true)
    }
    
    /// Create a new read-write memory capability.
    ///
    /// # Arguments
    ///
    /// * `regions` - The regions that are allowed.
    ///
    /// # Returns
    ///
    /// A new read-write memory capability.
    pub fn read_write(regions: impl IntoIterator<Item = String>) -> Self {
        Self::new(regions, true, true)
    }
    
    /// Get the allowed regions.
    pub fn regions(&self) -> &HashSet<String> {
        &self.regions
    }
    
    /// Check if read operations are allowed.
    pub fn can_read(&self) -> bool {
        self.read
    }
    
    /// Check if write operations are allowed.
    pub fn can_write(&self) -> bool {
        self.write
    }
    
    /// Check if a region is allowed.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The region to check.
    ///
    /// # Returns
    ///
    /// `true` if the region is allowed, `false` otherwise.
    fn is_region_allowed(&self, region_id: &str) -> bool {
        self.regions.contains(region_id)
    }
}

impl Capability for MemoryCapability {
    fn capability_type(&self) -> &str {
        "memory"
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::Memory { region_id, read, write } => {
                // Check if the region is allowed
                if !self.is_region_allowed(region_id) {
                    return Err(CapabilityError::PermissionDenied(
                        format!("Access to memory region {} is not allowed", region_id)
                    ).into());
                }
                
                // Check if the operations are allowed
                if *read && !self.read {
                    return Err(CapabilityError::PermissionDenied(
                        "Read access to memory is not allowed".into()
                    ).into());
                }
                
                if *write && !self.write {
                    return Err(CapabilityError::PermissionDenied(
                        "Write access to memory is not allowed".into()
                    ).into());
                }
                
                Ok(())
            },
            _ => Err(CapabilityError::PermissionDenied(
                "Only memory access is allowed".into()
            ).into()),
        }
    }
    
    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        let mut regions = self.regions.clone();
        let mut read = self.read;
        let mut write = self.write;
        
        for constraint in constraints {
            match constraint {
                Constraint::MemoryRegion { region_id, read: r, write: w } => {
                    // Check if the region is already allowed
                    if !regions.contains(region_id) {
                        regions.insert(region_id.clone());
                    }
                    
                    // Can only remove permissions, not add them
                    read = read && *r;
                    write = write && *w;
                    
                    // If all operations are disallowed, return an error
                    if !read && !write {
                        return Err(CapabilityError::ConstraintError(
                            "No operations allowed after applying constraint".into()
                        ).into());
                    }
                },
                _ => return Err(CapabilityError::ConstraintError(
                    format!("Constraint type {} not supported for memory capability", constraint.constraint_type())
                ).into()),
            }
        }
        
        Ok(Box::new(Self { regions, read, write }))
    }
    
    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut capabilities = Vec::new();
        
        // Split by operation
        if self.read {
            capabilities.push(Box::new(Self::new(
                self.regions.iter().cloned(),
                true,
                false,
            )) as Box<dyn Capability>);
        }
        
        if self.write {
            capabilities.push(Box::new(Self::new(
                self.regions.iter().cloned(),
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
        other.capability_type() == "memory"
    }
    
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        if !self.can_join_with(other) {
            return Err(CapabilityError::CompositionError(
                format!("Cannot join memory capability with {}", other.capability_type())
            ).into());
        }
        
        // Downcast the other capability to a MemoryCapability
        let other = match other.permits(&AccessRequest::Memory {
            region_id: "test".to_string(),
            read: true,
            write: true,
        }) {
            Ok(()) => {
                // If it permits everything, it's probably a super-capability
                return Ok(Box::new(Self {
                    regions: self.regions.union(&self.regions).cloned().collect(),
                    read: true,
                    write: true,
                }));
            },
            Err(_) => {
                // Try to get more precise information
                let mut regions = self.regions.clone();
                let mut read = self.read;
                let mut write = self.write;
                
                // Check if it permits read
                if other.permits(&AccessRequest::Memory {
                    region_id: "test".to_string(),
                    read: true,
                    write: false,
                }).is_ok() {
                    read = true;
                }
                
                // Check if it permits write
                if other.permits(&AccessRequest::Memory {
                    region_id: "test".to_string(),
                    read: false,
                    write: true,
                }).is_ok() {
                    write = true;
                }
                
                // TODO: More precise region information
                
                Self {
                    regions,
                    read,
                    write,
                }
            }
        };
        
        // Join the capabilities
        let joined = Self {
            regions: self.regions.union(&other.regions).cloned().collect(),
            read: self.read || other.read,
            write: self.write || other.write,
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
    
    #[test]
    fn test_memory_capability_permits() {
        let capability = MemoryCapability::new(
            vec!["region1".to_string(), "region2".to_string()],
            true,
            false,
        );
        
        // Test read access to allowed region
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: true,
            write: false,
        };
        assert!(capability.permits(&request).is_ok());
        
        // Test write access to allowed region (should fail)
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: false,
            write: true,
        };
        assert!(capability.permits(&request).is_err());
        
        // Test read access to disallowed region
        let request = AccessRequest::Memory {
            region_id: "region3".to_string(),
            read: true,
            write: false,
        };
        assert!(capability.permits(&request).is_err());
        
        // Test non-memory access
        let request = AccessRequest::File {
            path: std::path::PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
    }
    
    #[test]
    fn test_memory_capability_constrain() {
        let capability = MemoryCapability::new(
            vec!["region1".to_string(), "region2".to_string()],
            true,
            true,
        );
        
        // Constrain to read-only
        let constraints = vec![Constraint::MemoryRegion {
            region_id: "region1".to_string(),
            read: true,
            write: false,
        }];
        let constrained = capability.constrain(&constraints).unwrap();
        
        // Should allow read access to region1
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: true,
            write: false,
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Should deny write access to region1
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: false,
            write: true,
        };
        assert!(constrained.permits(&request).is_err());
        
        // Should still allow access to region2
        let request = AccessRequest::Memory {
            region_id: "region2".to_string(),
            read: true,
            write: true,
        };
        assert!(constrained.permits(&request).is_ok());
    }
    
    #[test]
    fn test_memory_capability_split() {
        let capability = MemoryCapability::new(
            vec!["region1".to_string()],
            true,
            true,
        );
        
        let split = capability.split();
        assert_eq!(split.len(), 2);
        
        // Check that the first capability allows read but not write
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: true,
            write: false,
        };
        assert!(split[0].permits(&request).is_ok());
        
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: false,
            write: true,
        };
        assert!(split[0].permits(&request).is_err());
        
        // Check that the second capability allows write but not read
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: true,
            write: false,
        };
        assert!(split[1].permits(&request).is_err());
        
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: false,
            write: true,
        };
        assert!(split[1].permits(&request).is_ok());
    }
    
    #[test]
    fn test_memory_capability_join() {
        let capability1 = MemoryCapability::new(
            vec!["region1".to_string()],
            true,
            false,
        );
        
        let capability2 = MemoryCapability::new(
            vec!["region2".to_string()],
            false,
            true,
        );
        
        let joined = capability1.join(&capability2).unwrap();
        
        // Should allow read access to region1
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: true,
            write: false,
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should allow write access to region2
        let request = AccessRequest::Memory {
            region_id: "region2".to_string(),
            read: false,
            write: true,
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should deny write access to region1
        let request = AccessRequest::Memory {
            region_id: "region1".to_string(),
            read: false,
            write: true,
        };
        assert!(joined.permits(&request).is_err());
        
        // Should deny read access to region2
        let request = AccessRequest::Memory {
            region_id: "region2".to_string(),
            read: true,
            write: false,
        };
        assert!(joined.permits(&request).is_err());
    }
}