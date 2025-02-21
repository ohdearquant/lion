use crate::element::ElementData;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct Cache {
    elements: Arc<Mutex<HashMap<Uuid, ElementData>>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            elements: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get(&self, id: &Uuid) -> Option<ElementData> {
        let cache = self.elements.lock().unwrap();
        cache.get(id).cloned()
    }

    pub fn insert(&self, element: ElementData) {
        let mut cache = self.elements.lock().unwrap();
        cache.insert(element.id, element);
    }

    pub fn remove(&self, id: &Uuid) {
        let mut cache = self.elements.lock().unwrap();
        cache.remove(id);
    }

    pub fn clear(&self) {
        let mut cache = self.elements.lock().unwrap();
        cache.clear();
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}
