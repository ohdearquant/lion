use crate::model::{Capability, CapabilityError};
use std::collections::{HashMap, HashSet};

/// A set of capabilities grouped by type
///
/// This is used to efficiently merge multiple capabilities of the same type,
/// and to provide a convenient way to check if a plugin has permission for
/// a specific access request.
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    /// Map from capability type to a list of capabilities of that type
    capabilities: HashMap<String, Vec<Box<dyn Capability>>>,
}

impl CapabilitySet {
    /// Creates a new empty capability set
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    /// Adds a capability to the set
    pub fn add_capability(&mut self, capability: Box<dyn Capability>) {
        let capability_type = capability.capability_type().to_string();

        let type_capabilities = self.capabilities.entry(capability_type).or_default();

        type_capabilities.push(capability);
    }

    /// Gets the capabilities of a specific type
    pub fn get_capabilities(&self, capability_type: &str) -> Option<&Vec<Box<dyn Capability>>> {
        self.capabilities.get(capability_type)
    }

    /// Gets all capability types in this set
    pub fn capability_types(&self) -> HashSet<String> {
        self.capabilities.keys().cloned().collect()
    }

    /// Gets the total number of capabilities in this set
    pub fn len(&self) -> usize {
        self.capabilities.values().map(|v| v.len()).sum()
    }

    /// Returns true if this set is empty
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty() || self.len() == 0
    }

    /// Merges capabilities of the same type into a single capability per type
    pub fn merge_by_type(&self) -> Result<CapabilitySet, CapabilityError> {
        let mut result = CapabilitySet::new();

        for capabilities in self.capabilities.values() {
            if capabilities.is_empty() {
                continue;
            }

            if capabilities.len() == 1 {
                // Just add the single capability
                result.add_capability(capabilities[0].clone_box());
                continue;
            }

            // Merge all capabilities of this type
            let merged = merge_capabilities_by_type(capabilities.clone())?;
            result.add_capability(merged);
        }

        Ok(result)
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::new()
    }
}

/// Merges a list of capabilities of the same type into a single capability
///
/// This function assumes that all capabilities in the list are of the same type.
/// It joins them together to create a single capability that permits all the
/// access that any of the input capabilities would permit.
pub fn merge_capabilities_by_type(
    capabilities: Vec<Box<dyn Capability>>,
) -> Result<Box<dyn Capability>, CapabilityError> {
    if capabilities.is_empty() {
        return Err(CapabilityError::InvalidState(
            "Cannot merge empty list of capabilities".to_string(),
        ));
    }

    if capabilities.len() == 1 {
        return Ok(capabilities[0].clone_box());
    }

    // Check that all capabilities are of the same type
    let first_type = capabilities[0].capability_type();

    for cap in &capabilities[1..] {
        if cap.capability_type() != first_type {
            return Err(CapabilityError::IncompatibleTypes(format!(
                "Cannot merge capabilities of different types: '{}' and '{}'",
                first_type,
                cap.capability_type()
            )));
        }
    }

    // Join all capabilities
    let mut result = capabilities[0].clone_box();

    for cap in &capabilities[1..] {
        result = result.join(cap.as_ref())?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    use crate::model::file::{FileCapability, FileOperations};
    use crate::model::network::{HostRule, NetworkCapability};
    use crate::model::AccessRequest;

    #[test]
    fn test_capability_set() {
        // Create a file capability
        let paths = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        // Create a network capability
        let mut host_rules = HashSet::new();
        host_rules.insert(HostRule::Domain("example.com".to_string()));

        let network_cap = NetworkCapability::outbound(host_rules);

        // Create a capability set
        let mut set = CapabilitySet::new();
        set.add_capability(Box::new(file_cap));
        set.add_capability(Box::new(network_cap));

        // Check that the set has the right capabilities
        assert_eq!(set.capability_types().len(), 2);
        assert!(set.capability_types().contains("file"));
        assert!(set.capability_types().contains("network"));

        assert_eq!(set.len(), 2);

        // Check that we can get the capabilities
        let file_caps = set.get_capabilities("file").unwrap();
        assert_eq!(file_caps.len(), 1);

        let network_caps = set.get_capabilities("network").unwrap();
        assert_eq!(network_caps.len(), 1);
    }

    #[test]
    fn test_merge_capabilities_by_type() {
        // Create two file capabilities
        let paths1 = ["/tmp/file1.txt".to_string()].into_iter().collect();
        let file_cap1 = FileCapability::new(paths1, FileOperations::READ);

        let paths2 = ["/tmp/file2.txt".to_string()].into_iter().collect();
        let file_cap2 = FileCapability::new(paths2, FileOperations::WRITE);

        // Merge them
        let merged =
            merge_capabilities_by_type(vec![Box::new(file_cap1), Box::new(file_cap2)]).unwrap();

        // Check that the merged capability permits both accesses
        assert!(merged
            .permits(&AccessRequest::File {
                path: "/tmp/file1.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        assert!(merged
            .permits(&AccessRequest::File {
                path: "/tmp/file2.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_ok());
    }

    #[test]
    fn test_merge_different_types() {
        // Create a file capability
        let paths = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        // Create a network capability
        let mut host_rules = HashSet::new();
        host_rules.insert(HostRule::Domain("example.com".to_string()));

        let network_cap = NetworkCapability::outbound(host_rules);

        // Try to merge them
        let result = merge_capabilities_by_type(vec![Box::new(file_cap), Box::new(network_cap)]);

        // Should fail because they are different types
        assert!(result.is_err());
    }

    #[test]
    fn test_capability_set_merge_by_type() {
        // Create two file capabilities
        let paths1 = ["/tmp/file1.txt".to_string()].into_iter().collect();
        let file_cap1 = FileCapability::new(paths1, FileOperations::READ);

        let paths2 = ["/tmp/file2.txt".to_string()].into_iter().collect();
        let file_cap2 = FileCapability::new(paths2, FileOperations::WRITE);

        // Create a network capability
        let mut host_rules = HashSet::new();
        host_rules.insert(HostRule::Domain("example.com".to_string()));

        let network_cap = NetworkCapability::outbound(host_rules);

        // Create a capability set
        let mut set = CapabilitySet::new();
        set.add_capability(Box::new(file_cap1));
        set.add_capability(Box::new(file_cap2));
        set.add_capability(Box::new(network_cap));

        // Merge by type
        let merged = set.merge_by_type().unwrap();

        // Check that the merged set has the right capabilities
        assert_eq!(merged.capability_types().len(), 2);
        assert!(merged.capability_types().contains("file"));
        assert!(merged.capability_types().contains("network"));

        // Check that each type has been merged to a single capability
        assert_eq!(merged.get_capabilities("file").unwrap().len(), 1);
        assert_eq!(merged.get_capabilities("network").unwrap().len(), 1);

        // Check that the merged file capability permits both accesses
        let file_cap = &merged.get_capabilities("file").unwrap()[0];

        assert!(file_cap
            .permits(&AccessRequest::File {
                path: "/tmp/file1.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());

        assert!(file_cap
            .permits(&AccessRequest::File {
                path: "/tmp/file2.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_ok());
    }
}
