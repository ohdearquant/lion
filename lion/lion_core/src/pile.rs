use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Pile<T> {
    inner: Arc<Mutex<HashMap<Uuid, T>>>,
}

impl<T> Pile<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn insert(&self, id: Uuid, item: T) {
        let mut guard = self.inner.lock().unwrap();
        guard.insert(id, item);
    }

    pub fn get(&self, id: &Uuid) -> Option<T>
    where
        T: Clone,
    {
        let guard = self.inner.lock().unwrap();
        guard.get(id).cloned()
    }

    pub fn list_ids(&self) -> Vec<Uuid> {
        let guard = self.inner.lock().unwrap();
        guard.keys().cloned().collect()
    }

    pub fn contains(&self, id: &Uuid) -> bool {
        let guard = self.inner.lock().unwrap();
        guard.contains_key(id)
    }

    pub fn len(&self) -> usize {
        let guard = self.inner.lock().unwrap();
        guard.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Default for Pile<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_pile_insert_retrieve() {
        let pile = Pile::new();
        let id = Uuid::new_v4();
        pile.insert(id, "test_data".to_string());
        let retrieved = pile.get(&id);
        assert_eq!(retrieved, Some("test_data".to_string()));
    }

    #[test]
    fn test_pile_concurrency() {
        let pile = Pile::new();
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let p = pile.clone();
                thread::spawn(move || {
                    let id = Uuid::new_v4();
                    p.insert(id, format!("val-{}", id));
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(pile.len(), 10);
    }

    #[test]
    fn test_pile_empty_operations() {
        let pile: Pile<String> = Pile::new();
        assert!(pile.is_empty());
        assert_eq!(pile.len(), 0);

        let id = Uuid::new_v4();
        pile.insert(id, "test".to_string());

        assert!(!pile.is_empty());
        assert_eq!(pile.len(), 1);
        assert!(pile.contains(&id));
    }
}
