use super::{
    error::{Result, StorageError},
    types::{Element, ElementData, ElementId},
    FileStorage,
};
use serde_json::Value;
use std::sync::Arc;

/// A high-level store for managing elements, providing a simplified interface
/// around the underlying storage implementation.
#[derive(Debug, Clone)]
pub struct Store {
    storage: Arc<FileStorage>,
}

impl Store {
    /// Create a new store with file-based storage
    pub fn new(storage_path: &str) -> Self {
        Self {
            storage: Arc::new(FileStorage::new(storage_path)),
        }
    }

    /// Get an element by ID
    pub fn get(&self, id: impl Into<ElementId>) -> Option<Element> {
        self.storage.get(id)
    }

    /// Set an element's data
    pub fn set(&self, id: impl Into<ElementId>, data: Value) -> Result<()> {
        self.storage.set(id, data)
    }

    /// Remove an element
    pub fn remove(&self, id: impl Into<ElementId>) -> bool {
        self.storage.remove(id)
    }

    /// List all elements
    pub fn list(&self) -> Vec<Element> {
        self.storage.list()
    }

    /// Get the underlying storage implementation
    pub fn storage(&self) -> &FileStorage {
        &self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn test_store_operations() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_str().unwrap());

        // Test set and get
        let id = ElementId(Uuid::new_v4());
        let data = json!({
            "name": "test",
            "value": 42
        });

        store.set(id, data.clone()).unwrap();
        let element = store.get(id).unwrap();
        assert_eq!(element.data.content, data);

        // Test list
        let elements = store.list();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].id, id);

        // Test remove
        assert!(store.remove(id));
        assert!(store.get(id).is_none());
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn test_store_clone() {
        let temp_dir = tempdir().unwrap();
        let store1 = Store::new(temp_dir.path().to_str().unwrap());
        let store2 = store1.clone();

        // Both stores should work with the same data
        let id = ElementId(Uuid::new_v4());
        let data = json!({ "test": true });

        store1.set(id, data.clone()).unwrap();
        let element = store2.get(id).unwrap();
        assert_eq!(element.data.content, data);
    }

    #[test]
    fn test_store_error_handling() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_str().unwrap());

        // Test getting non-existent element
        let id = ElementId(Uuid::new_v4());
        assert!(store.get(id).is_none());

        // Test removing non-existent element
        assert!(!store.remove(id));
    }
}
