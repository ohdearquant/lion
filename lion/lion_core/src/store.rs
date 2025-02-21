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

    pub fn create_element(&self, elem: ElementData) -> Uuid {
        let id = elem.id;
        self.elements.insert(id, elem);
        id
    }

    pub fn get_element(&self, id: &Uuid) -> Option<ElementData> {
        self.elements.get(id)
    }

    pub fn list_element_ids(&self) -> Vec<Uuid> {
        self.elements.list_ids()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_store_create_element() {
        let store = InMemoryStore::new();
        let data = ElementData::new(json!({ "hello": "world" }));
        let id = store.create_element(data.clone());
        let retrieved = store.get_element(&id).unwrap();
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
            let id = store.create_element(elem);
            created_ids.push(id);
        }

        let stored_ids = store.list_element_ids();
        assert_eq!(stored_ids.len(), 3);
        assert_eq!(store.len(), 3);

        for id in created_ids {
            assert!(stored_ids.contains(&id));
        }
    }

    #[test]
    fn test_store_get_nonexistent() {
        let store = InMemoryStore::new();
        let id = Uuid::new_v4();
        assert!(store.get_element(&id).is_none());
    }
}
