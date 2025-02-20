use crate::storage::{ElementData, ElementId, FileStorage};
use std::sync::Arc;

/// A store for managing elements
#[derive(Debug, Clone)]
pub struct Store {
    storage: Arc<FileStorage>,
}

impl Store {
    /// Create a new store
    pub fn new(storage_path: &str) -> Self {
        Self {
            storage: Arc::new(FileStorage::new(storage_path)),
        }
    }

    /// Get an element by ID
    pub fn get(&self, id: ElementId) -> Option<ElementData> {
        self.storage.get(id)
    }

    /// Set an element
    pub fn set(&self, id: ElementId, data: serde_json::Value) -> bool {
        self.storage.set(id, data)
    }

    /// Remove an element
    pub fn remove(&self, id: ElementId) -> bool {
        self.storage.remove(id)
    }

    /// List all elements
    pub fn list(&self) -> Vec<ElementData> {
        self.storage.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn test_store_create_element() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_str().unwrap());

        let id = ElementId(Uuid::new_v4());
        let data = json!({
            "name": "test",
            "value": 42
        });

        assert!(store.set(id, data.clone()));

        let element = store.get(id).unwrap();
        assert_eq!(element.id, id);
        assert_eq!(element.data.content, data);
    }

    #[test]
    fn test_store_get_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_str().unwrap());

        let id = ElementId(Uuid::new_v4());
        assert!(store.get(id).is_none());
    }

    #[test]
    fn test_store_list_elements() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_str().unwrap());

        let id1 = ElementId(Uuid::new_v4());
        let id2 = ElementId(Uuid::new_v4());

        store.set(id1, json!({"name": "test1"}));
        store.set(id2, json!({"name": "test2"}));

        let elements = store.list();
        assert_eq!(elements.len(), 2);
        assert!(elements.iter().any(|e| e.id == id1));
        assert!(elements.iter().any(|e| e.id == id2));
    }
}
