use std::collections::HashSet;

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use lion_core::id::{CapabilityId, PluginId};

use super::{partial_revocation::apply_partial_revocation, CapabilityStore};
use crate::model::{AccessRequest, Capability, CapabilityError};

/// Capability entry with a generation counter for safe revocation
struct CapabilityEntry {
    /// The actual capability
    capability: Box<dyn Capability>,
    /// Generation of this capability, incremented on each update
    /// Used to detect stale capability references
    generation: AtomicU64,
}

/// An in-memory implementation of the CapabilityStore trait
///
/// This store uses DashMap for thread-safe concurrent access and supports
/// generational indices for safer capability revocation.
#[derive(Default)]
pub struct InMemoryCapabilityStore {
    /// Map from (plugin_id, capability_id) to capability entry
    capabilities: DashMap<(PluginId, CapabilityId), CapabilityEntry>,

    /// Map from plugin_id to set of capability_ids
    plugin_capabilities: DashMap<PluginId, HashSet<CapabilityId>>,

    /// The next capability ID to assign
    next_capability_id: AtomicU64,
}

impl InMemoryCapabilityStore {
    /// Creates a new empty in-memory capability store
    pub fn new() -> Self {
        Self {
            capabilities: DashMap::new(),
            plugin_capabilities: DashMap::new(),
            next_capability_id: AtomicU64::new(1),
        }
    }

    /// Generates a new unique capability ID
    fn generate_capability_id(&self) -> CapabilityId {
        let id = self.next_capability_id.fetch_add(1, Ordering::Relaxed);
        // Using the same test_capability_id function we define in the tests module
        // to maintain consistent behavior
        fn test_capability_id(value: u64) -> CapabilityId {
            // Create a deterministic UUID from the value
            let bytes = value.to_le_bytes();
            let mut uuid_bytes = [0u8; 16];
            let copy_len = std::cmp::min(8, uuid_bytes.len());
            uuid_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);
            let uuid = uuid::Uuid::from_bytes(uuid_bytes);
            CapabilityId::from_uuid(uuid)
        }

        test_capability_id(id)
    }

    /// Gets a capability entry by plugin ID and capability ID
    fn get_entry(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
    ) -> Result<dashmap::mapref::one::Ref<(PluginId, CapabilityId), CapabilityEntry>, CapabilityError>
    {
        let key = (*plugin_id, *capability_id);

        self.capabilities.get(&key).ok_or_else(|| {
            CapabilityError::NotFound(format!(
                "Capability {} not found for plugin {}",
                capability_id, plugin_id
            ))
        })
    }

    /// Gets a mutable capability entry by plugin ID and capability ID
    fn get_entry_mut(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
    ) -> Result<
        dashmap::mapref::one::RefMut<(PluginId, CapabilityId), CapabilityEntry>,
        CapabilityError,
    > {
        let key = (*plugin_id, *capability_id);

        self.capabilities.get_mut(&key).ok_or_else(|| {
            CapabilityError::NotFound(format!(
                "Capability {} not found for plugin {}",
                capability_id, plugin_id
            ))
        })
    }

    /// Gets or creates the set of capability IDs for a plugin
    fn get_or_create_capability_set(
        &self,
        plugin_id: &PluginId,
    ) -> dashmap::mapref::one::RefMut<PluginId, HashSet<CapabilityId>> {
        if !self.plugin_capabilities.contains_key(plugin_id) {
            self.plugin_capabilities.insert(*plugin_id, HashSet::new());
        }

        self.plugin_capabilities.get_mut(plugin_id).unwrap()
    }
}

impl CapabilityStore for InMemoryCapabilityStore {
    fn add_capability(
        &self,
        plugin_id: PluginId,
        capability: Box<dyn Capability>,
    ) -> Result<CapabilityId, CapabilityError> {
        let capability_id = self.generate_capability_id();
        let key = (plugin_id, capability_id);

        // Create the capability entry
        let entry = CapabilityEntry {
            capability,
            generation: AtomicU64::new(1),
        };

        // Add to the capabilities map
        self.capabilities.insert(key, entry);

        // Add to the plugin's set of capabilities
        let mut capability_set = self.get_or_create_capability_set(&plugin_id);
        capability_set.insert(capability_id);

        Ok(capability_id)
    }

    fn get_capability(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
    ) -> Result<Box<dyn Capability>, CapabilityError> {
        let entry = self.get_entry(plugin_id, capability_id)?;
        Ok(entry.capability.clone_box())
    }

    fn remove_capability(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
    ) -> Result<(), CapabilityError> {
        let key = (*plugin_id, *capability_id);

        // Remove from the capabilities map
        if !self.capabilities.contains_key(&key) {
            return Err(CapabilityError::NotFound(format!(
                "Capability {} not found for plugin {}",
                capability_id, plugin_id
            )));
        }

        self.capabilities.remove(&key);

        // Remove from the plugin's set of capabilities
        if let Some(mut capability_set) = self.plugin_capabilities.get_mut(plugin_id) {
            capability_set.remove(capability_id);
        }

        Ok(())
    }

    fn replace_capability(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
        new_cap: Box<dyn Capability>,
    ) -> Result<(), CapabilityError> {
        let mut entry = self.get_entry_mut(plugin_id, capability_id)?;

        // Update the capability
        entry.capability = new_cap;

        // Increment the generation
        entry.generation.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    fn list_capabilities(
        &self,
        plugin_id: &PluginId,
    ) -> Result<Vec<(CapabilityId, Box<dyn Capability>)>, CapabilityError> {
        let capability_set = self.plugin_capabilities.get(plugin_id);

        if let Some(capability_set) = capability_set {
            let mut result = Vec::new();

            for capability_id in capability_set.iter() {
                // Get the capability if it exists
                let key = (*plugin_id, *capability_id);
                if let Some(entry) = self.capabilities.get(&key) {
                    result.push((*capability_id, entry.capability.clone_box()));
                }
            }

            Ok(result)
        } else {
            // No capabilities for this plugin
            Ok(Vec::new())
        }
    }

    fn clear_plugin_capabilities(&self, plugin_id: &PluginId) -> Result<(), CapabilityError> {
        // Remove all capabilities for this plugin
        if let Some(capability_set) = self.plugin_capabilities.get(plugin_id) {
            // Clone the set to avoid borrowing issues
            let capability_ids: Vec<CapabilityId> = capability_set.iter().copied().collect();

            for capability_id in capability_ids {
                let key = (*plugin_id, capability_id);
                self.capabilities.remove(&key);
            }
        }

        // Clear the plugin's set of capabilities
        self.plugin_capabilities.remove(plugin_id);

        Ok(())
    }

    fn partial_revoke(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
        request: &AccessRequest,
    ) -> Result<(), CapabilityError> {
        // Get the capability
        let capability = self.get_capability(plugin_id, capability_id)?;

        // Apply partial revocation
        let reduced = apply_partial_revocation(capability, request)?;

        // Replace the original capability with the reduced one
        self.replace_capability(plugin_id, capability_id, reduced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use uuid::Uuid;

    use crate::model::file::{FileCapability, FileOperations};

    fn test_plugin_id(value: u64) -> PluginId {
        // Create a deterministic UUID from the value
        let bytes = value.to_le_bytes();
        let mut uuid_bytes = [0u8; 16];
        let copy_len = std::cmp::min(8, uuid_bytes.len());
        uuid_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);
        let uuid = Uuid::from_bytes(uuid_bytes);
        PluginId::from_uuid(uuid)
    }

    #[allow(dead_code)]
    fn test_capability_id(value: u64) -> CapabilityId {
        // Create a deterministic UUID from the value
        let bytes = value.to_le_bytes();
        let mut uuid_bytes = [0u8; 16];
        let copy_len = std::cmp::min(8, uuid_bytes.len());
        uuid_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);
        let uuid = Uuid::from_bytes(uuid_bytes);
        CapabilityId::from_uuid(uuid)
    }

    #[test]
    fn test_add_and_get_capability() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = test_plugin_id(1);

        // Create a file capability
        let paths: HashSet<String> = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        // Add the capability to the store
        let capability_id = store.add_capability(plugin_id, Box::new(file_cap)).unwrap();

        // Get the capability back
        let retrieved = store.get_capability(&plugin_id, &capability_id).unwrap();

        // Check that it's the same type
        assert_eq!(retrieved.capability_type(), "file");

        // Check that it has the expected permissions
        assert!(retrieved
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_ok());
    }

    #[test]
    fn test_list_capabilities() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = test_plugin_id(1);

        // Create two file capabilities
        let paths1: HashSet<String> = ["/tmp/file1.txt".to_string()].into_iter().collect();
        let file_cap1 = FileCapability::new(paths1, FileOperations::READ);

        let paths2: HashSet<String> = ["/tmp/file2.txt".to_string()].into_iter().collect();
        let file_cap2 = FileCapability::new(paths2, FileOperations::WRITE);

        // Add both capabilities to the store
        let capability_id1 = store
            .add_capability(plugin_id, Box::new(file_cap1))
            .unwrap();
        let capability_id2 = store
            .add_capability(plugin_id, Box::new(file_cap2))
            .unwrap();

        // List the capabilities
        let capabilities = store.list_capabilities(&plugin_id).unwrap();

        // Check that both capabilities are there
        assert_eq!(capabilities.len(), 2);

        // Check that the capability IDs match
        let capability_ids: HashSet<CapabilityId> =
            capabilities.iter().map(|(id, _)| *id).collect();

        assert!(capability_ids.contains(&capability_id1));
        assert!(capability_ids.contains(&capability_id2));
    }

    #[test]
    fn test_remove_capability() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = test_plugin_id(1);

        // Create a file capability
        let paths: HashSet<String> = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        // Add the capability to the store
        let capability_id = store.add_capability(plugin_id, Box::new(file_cap)).unwrap();

        // Check that the capability is there
        assert!(store.get_capability(&plugin_id, &capability_id).is_ok());

        // Remove the capability
        store.remove_capability(&plugin_id, &capability_id).unwrap();

        // Check that the capability is gone
        assert!(store.get_capability(&plugin_id, &capability_id).is_err());
    }

    #[test]
    fn test_replace_capability() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = test_plugin_id(1);

        // Create a file capability with read permission
        let paths: HashSet<String> = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths.clone(), FileOperations::READ);

        // Add the capability to the store
        let capability_id = store.add_capability(plugin_id, Box::new(file_cap)).unwrap();

        // Create a new capability with write permission
        let new_file_cap = FileCapability::new(paths, FileOperations::WRITE);

        // Replace the capability
        store
            .replace_capability(&plugin_id, &capability_id, Box::new(new_file_cap))
            .unwrap();

        // Get the capability back
        let retrieved = store.get_capability(&plugin_id, &capability_id).unwrap();

        // Check that it has the new permissions
        assert!(retrieved
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: false,
                write: true,
                execute: false,
            })
            .is_ok());

        assert!(retrieved
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());
    }

    #[test]
    fn test_clear_plugin_capabilities() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = test_plugin_id(1);

        // Create a file capability
        let paths: HashSet<String> = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        // Add the capability to the store
        store.add_capability(plugin_id, Box::new(file_cap)).unwrap();

        // Check that the plugin has capabilities
        assert_eq!(store.list_capabilities(&plugin_id).unwrap().len(), 1);

        // Clear the plugin's capabilities
        store.clear_plugin_capabilities(&plugin_id).unwrap();

        // Check that the plugin has no capabilities
        assert_eq!(store.list_capabilities(&plugin_id).unwrap().len(), 0);
    }

    #[test]
    fn test_check_permission() {
        let store = InMemoryCapabilityStore::new();
        let plugin_id = test_plugin_id(1);

        // Create a file capability
        let paths: HashSet<String> = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        // Add the capability to the store
        store.add_capability(plugin_id, Box::new(file_cap)).unwrap();

        // Check a permitted request
        assert!(store
            .check_permission(
                &plugin_id,
                &AccessRequest::File {
                    path: "/tmp/file.txt".to_string(),
                    read: true,
                    write: false,
                    execute: false,
                }
            )
            .is_ok());

        // Check a denied request
        assert!(store
            .check_permission(
                &plugin_id,
                &AccessRequest::File {
                    path: "/tmp/file.txt".to_string(),
                    read: false,
                    write: true,
                    execute: false,
                }
            )
            .is_err());

        // Check a request for a different type
        assert!(store
            .check_permission(
                &plugin_id,
                &AccessRequest::Network {
                    host: Some("example.com".to_string()),
                    port: None,
                    connect: true,
                    listen: false,
                    bind: false,
                }
            )
            .is_err());
    }
}
