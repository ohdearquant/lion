use super::ElementStore;
use crate::element::ElementData;
use crate::pile::Pile;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InMemoryStore {
    elements: Pile<ElementData>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            elements: Pile::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementStore for InMemoryStore {
    fn store(&self, element: ElementData) {
        self.elements.insert(element.id, element);
    }

    fn get(&self, id: &Uuid) -> Option<ElementData> {
        self.elements.get(id)
    }

    fn list(&self) -> Vec<ElementData> {
        self.elements
            .list_ids()
            .into_iter()
            .filter_map(|id| self.elements.get(&id))
            .collect()
    }

    fn remove(&self, id: &Uuid) -> bool {
        self.elements.remove(id).is_some()
    }

    fn clear(&self) {
        self.elements.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_store_create_element() {
        let store = InMemoryStore::new();
        let data = ElementData::new(json!({ "hello": "world" }));
        let id = data.id;
        store.store(data.clone());
        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.id, data.id);
        assert_eq!(retrieved.metadata, data.metadata);
    }

    #[test]
    fn test_store_list_elements() {
        let store = InMemoryStore::new();
        assert!(store.is_empty());

        let mut created_ids = Vec::new();
        for i in 0..3 {
            let elem = ElementData::new(json!({ "index": i }));
            let id = elem.id;
            store.store(elem);
            created_ids.push(id);
        }

        let stored_elements = store.list();
        assert_eq!(stored_elements.len(), 3);
        assert_eq!(store.len(), 3);

        for id in created_ids {
            assert!(stored_elements.iter().any(|e| e.id == id));
        }
    }

    #[test]
    fn test_store_get_nonexistent() {
        let store = InMemoryStore::new();
        let id = Uuid::new_v4();
        assert!(store.get(&id).is_none());
    }

    #[test]
    fn test_remove_element() {
        let store = InMemoryStore::new();
        let elem = ElementData::new(json!({ "test": "data" }));
        let id = elem.id;

        store.store(elem);
        assert!(store.get(&id).is_some());

        assert!(store.remove(&id));
        assert!(store.get(&id).is_none());
    }

    #[test]
    fn test_clear_store() {
        let store = InMemoryStore::new();

        store.store(ElementData::new(json!({ "test": 1 })));
        store.store(ElementData::new(json!({ "test": 2 })));

        assert_eq!(store.list().len(), 2);

        store.clear();
        assert_eq!(store.list().len(), 0);
        assert!(store.is_empty());
    }
}
