use std::any::Any;
use std::collections::HashMap;

use super::capability::{AccessRequest, Capability, CapabilityError, Constraint};

/// A composite capability that groups multiple capabilities of different types
/// This allows creating rich capability bundles that can control access to
/// multiple types of resources.
#[derive(Debug, Clone)]
pub struct CompositeCapability {
    /// Map of capability type to the actual capability
    capabilities: HashMap<String, Box<dyn Capability>>,
}

impl CompositeCapability {
    /// Creates a new empty composite capability
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    /// Creates a new composite capability from a list of capabilities
    pub fn from_capabilities(capabilities: Vec<Box<dyn Capability>>) -> Self {
        let mut map = HashMap::new();

        for cap in capabilities {
            map.insert(cap.capability_type().to_string(), cap);
        }

        Self { capabilities: map }
    }

    /// Adds a capability to this composite
    pub fn add_capability(&mut self, capability: Box<dyn Capability>) {
        self.capabilities
            .insert(capability.capability_type().to_string(), capability);
    }

    /// Gets a capability by type
    pub fn get_capability(&self, capability_type: &str) -> Option<&dyn Capability> {
        self.capabilities.get(capability_type).map(AsRef::as_ref)
    }

    /// Gets a mutable capability by type
    pub fn get_capability_mut(
        &mut self,
        capability_type: &str,
    ) -> Option<&mut Box<dyn Capability>> {
        self.capabilities.get_mut(capability_type)
    }

    /// Remove a capability by type
    pub fn remove_capability(&mut self, capability_type: &str) -> Option<Box<dyn Capability>> {
        self.capabilities.remove(capability_type)
    }

    /// Returns all capability types in this composite
    pub fn capability_types(&self) -> Vec<String> {
        self.capabilities.keys().cloned().collect()
    }

    /// Finds the right capability for an access request
    fn find_capability_for_request(&self, request: &AccessRequest) -> Option<&dyn Capability> {
        match request {
            AccessRequest::File { .. } => self.get_capability("file"),
            AccessRequest::Network { .. } => self.get_capability("network"),
            AccessRequest::Memory { .. } => self.get_capability("memory"),
            AccessRequest::PluginCall { .. } => self.get_capability("plugin_call"),
            AccessRequest::Message { .. } => self.get_capability("message"),
            AccessRequest::Custom { request_type, .. } => self.get_capability(request_type),
        }
    }
}

impl Capability for CompositeCapability {
    fn capability_type(&self) -> &str {
        "composite"
    }

    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        // Find the appropriate capability for this request
        if let Some(capability) = self.find_capability_for_request(request) {
            // Delegate to the specific capability
            capability.permits(request)
        } else {
            // No capability for this request type
            Err(CapabilityError::AccessDenied(format!(
                "No capability for request type {:?}",
                request
            )))
        }
    }

    fn constrain(
        &self,
        constraints: &[Constraint],
    ) -> Result<Box<dyn Capability>, CapabilityError> {
        // Group constraints by capability type
        let mut constraint_groups: HashMap<&str, Vec<Constraint>> = HashMap::new();

        for constraint in constraints {
            let capability_type = match constraint {
                Constraint::FilePath(_) | Constraint::FileOperation { .. } => "file",
                Constraint::NetworkHost(_)
                | Constraint::NetworkPort(_)
                | Constraint::NetworkOperation { .. } => "network",
                Constraint::MemoryRange { .. } => "memory",
                Constraint::PluginCall { .. } => "plugin_call",
                Constraint::MessageTopic(_) => "message",
                Constraint::Custom {
                    constraint_type, ..
                } => constraint_type,
            };

            constraint_groups
                .entry(capability_type)
                .or_insert_with(Vec::new)
                .push(constraint.clone());
        }

        // Create a new composite capability
        let mut result = self.clone();

        // Apply constraints to each capability
        for (capability_type, constraints) in constraint_groups {
            if let Some(capability) = self.get_capability(capability_type) {
                let constrained = capability.constrain(&constraints)?;
                result
                    .capabilities
                    .insert(capability_type.to_string(), constrained);
            } else {
                return Err(CapabilityError::InvalidConstraint(format!(
                    "No capability of type '{}' to constrain",
                    capability_type
                )));
            }
        }

        Ok(Box::new(result))
    }

    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut result = Vec::new();

        // For each capability type, create a new composite with just that capability
        for capability in self.capabilities.values() {
            // Get all sub-capabilities from the split
            let sub_capabilities = capability.split();

            // Create a new composite for each sub-capability
            for sub_capability in sub_capabilities {
                let mut composite = CompositeCapability::new();
                composite.add_capability(sub_capability);
                result.push(Box::new(composite) as Box<dyn Capability>);
            }
        }

        result
    }

    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to CompositeCapability
        if let Some(other_composite) = other.as_any().downcast_ref::<CompositeCapability>() {
            let mut result = self.clone();

            // For each capability in other, try to join with corresponding capability in self
            for (capability_type, other_capability) in &other_composite.capabilities {
                if let Some(self_capability) = self.get_capability(capability_type) {
                    // Join the two capabilities
                    let joined = self_capability.join(other_capability.as_ref())?;
                    result.capabilities.insert(capability_type.clone(), joined);
                } else {
                    // Just add the other capability
                    result
                        .capabilities
                        .insert(capability_type.clone(), other_capability.clone());
                }
            }

            Ok(Box::new(result))
        } else {
            // If the other capability is not a composite, try to add it based on its type
            let mut result = self.clone();
            let capability_type = other.capability_type().to_string();

            if let Some(self_capability) = self.get_capability(&capability_type) {
                // Join with existing capability of the same type
                let joined = self_capability.join(other)?;
                result.capabilities.insert(capability_type, joined);
            } else {
                // Just add the new capability
                result
                    .capabilities
                    .insert(capability_type, other.clone_box());
            }

            Ok(Box::new(result))
        }
    }

    fn leq(&self, other: &dyn Capability) -> bool {
        // Try to downcast the other capability to CompositeCapability
        if let Some(other_composite) = other.as_any().downcast_ref::<CompositeCapability>() {
            // For each capability in self, check if it's <= the corresponding capability in other
            for (capability_type, self_capability) in &self.capabilities {
                if let Some(other_capability) = other_composite.get_capability(capability_type) {
                    if !self_capability.leq(other_capability) {
                        return false;
                    }
                } else {
                    // Other doesn't have this capability type, so self can't be <= other
                    return false;
                }
            }

            true
        } else {
            // If the other capability is not a composite, it can only be less than or equal
            // if this composite has exactly one capability of the same type
            if self.capabilities.len() != 1 {
                return false;
            }

            let capability_type = other.capability_type();
            if let Some(self_capability) = self.get_capability(capability_type) {
                self_capability.leq(other)
            } else {
                false
            }
        }
    }

    fn meet(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to CompositeCapability
        if let Some(other_composite) = other.as_any().downcast_ref::<CompositeCapability>() {
            let mut result = CompositeCapability::new();
            let mut has_capabilities = false;

            // For each capability type in both, find the meet
            for capability_type in self.capability_types() {
                if let Some(other_capability) = other_composite.get_capability(&capability_type) {
                    if let Some(self_capability) = self.get_capability(&capability_type) {
                        // Compute meet of the two capabilities
                        if let Ok(meet) = self_capability.meet(other_capability) {
                            result.add_capability(meet);
                            has_capabilities = true;
                        }
                    }
                }
            }

            if !has_capabilities {
                return Err(CapabilityError::InvalidState(
                    "No common capabilities between the composites".to_string(),
                ));
            }

            Ok(Box::new(result))
        } else {
            // If the other capability is not a composite, we can only compute the meet
            // if we have a capability of the same type
            let capability_type = other.capability_type();

            if let Some(self_capability) = self.get_capability(capability_type) {
                let meet = self_capability.meet(other)?;

                let mut result = CompositeCapability::new();
                result.add_capability(meet);

                Ok(Box::new(result))
            } else {
                Err(CapabilityError::IncompatibleTypes(format!(
                    "No capability of type '{}' in this composite",
                    capability_type
                )))
            }
        }
    }

    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Default for CompositeCapability {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file::{FileCapability, FileOperations};
    use crate::model::network::{HostRule, NetworkCapability};
    use std::collections::HashSet;

    #[test]
    fn test_composite_capability_permits() {
        // Create a file capability
        let paths = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        // Create a network capability
        let mut host_rules = HashSet::new();
        host_rules.insert(HostRule::Domain("example.com".to_string()));

        let network_cap = NetworkCapability::outbound(host_rules);

        // Create a composite with both capabilities
        let mut composite = CompositeCapability::new();
        composite.add_capability(Box::new(file_cap));
        composite.add_capability(Box::new(network_cap));

        // Test file permission
        assert!(composite
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        // Test file permission (denied)
        assert!(composite
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_err());

        // Test network permission
        assert!(composite
            .permits(&AccessRequest::Network {
                host: Some("example.com".to_string()),
                port: None,
                connect: true,
                listen: false,
                bind: false,
            })
            .is_ok());

        // Test network permission (denied)
        assert!(composite
            .permits(&AccessRequest::Network {
                host: Some("other.com".to_string()),
                port: None,
                connect: true,
                listen: false,
                bind: false,
            })
            .is_err());

        // Test missing capability type
        assert!(composite
            .permits(&AccessRequest::Memory {
                address: 1000,
                size: 100,
                read: true,
                write: false,
            })
            .is_err());
    }

    #[test]
    fn test_composite_capability_constrain() {
        // Create a file capability
        let paths = ["/tmp/*".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ | FileOperations::WRITE);

        // Create a network capability
        let mut host_rules = HashSet::new();
        host_rules.insert(HostRule::Domain("example.com".to_string()));

        let network_cap = NetworkCapability::outbound(host_rules);

        // Create a composite with both capabilities
        let mut composite = CompositeCapability::new();
        composite.add_capability(Box::new(file_cap));
        composite.add_capability(Box::new(network_cap));

        // Constrain the file capability
        let constrained = composite
            .constrain(&[
                Constraint::FilePath("/tmp/specific.txt".to_string()),
                Constraint::FileOperation {
                    read: true,
                    write: false,
                    execute: false,
                },
            ])
            .unwrap();

        // Test the constrained capability
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
                path: "/tmp/specific.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_err());

        // Network capability should still work
        assert!(constrained
            .permits(&AccessRequest::Network {
                host: Some("example.com".to_string()),
                port: None,
                connect: true,
                listen: false,
                bind: false,
            })
            .is_ok());
    }

    #[test]
    fn test_composite_capability_join() {
        // Create a file capability for read
        let paths1 = ["/tmp/file1.txt".to_string()].into_iter().collect();
        let file_cap1 = FileCapability::new(paths1, FileOperations::READ);

        // Create a file capability for write
        let paths2 = ["/tmp/file2.txt".to_string()].into_iter().collect();
        let file_cap2 = FileCapability::new(paths2, FileOperations::WRITE);

        // Create two composite capabilities
        let mut composite1 = CompositeCapability::new();
        composite1.add_capability(Box::new(file_cap1));

        let mut composite2 = CompositeCapability::new();
        composite2.add_capability(Box::new(file_cap2));

        // Join the composites
        let joined = composite1.join(&composite2).unwrap();

        // Test the joined capability
        assert!(joined
            .permits(&AccessRequest::File {
                path: "/tmp/file1.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        assert!(joined
            .permits(&AccessRequest::File {
                path: "/tmp/file2.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_ok());
    }

    #[test]
    fn test_composite_capability_meet() {
        // Create two file capabilities with overlapping permissions
        let paths1 = ["/tmp/common.txt".to_string(), "/tmp/file1.txt".to_string()]
            .into_iter()
            .collect();
        let file_cap1 = FileCapability::new(paths1, FileOperations::READ | FileOperations::WRITE);

        let paths2 = ["/tmp/common.txt".to_string(), "/tmp/file2.txt".to_string()]
            .into_iter()
            .collect();
        let file_cap2 = FileCapability::new(paths2, FileOperations::READ);

        // Create two composite capabilities
        let mut composite1 = CompositeCapability::new();
        composite1.add_capability(Box::new(file_cap1));

        let mut composite2 = CompositeCapability::new();
        composite2.add_capability(Box::new(file_cap2));

        // Find the meet of the composites
        let meet = composite1.meet(&composite2).unwrap();

        // The meet should allow reading /tmp/common.txt but not writing
        assert!(meet
            .permits(&AccessRequest::File {
                path: "/tmp/common.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        assert!(meet
            .permits(&AccessRequest::File {
                path: "/tmp/common.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_err());

        // The meet should not allow access to the non-common files
        assert!(meet
            .permits(&AccessRequest::File {
                path: "/tmp/file1.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());

        assert!(meet
            .permits(&AccessRequest::File {
                path: "/tmp/file2.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());
    }
}
