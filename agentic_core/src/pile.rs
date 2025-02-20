use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// A thread-safe container for system objects that provides both
/// map-based access by ID and ordered access for sequences
#[derive(Debug, Clone)]
pub struct Pile<T> {
    // Map for direct ID-based access
    items: Arc<Mutex<HashMap<Uuid, T>>>,
    // Ordered sequence of IDs for maintaining order
    order: Arc<Mutex<VecDeque<Uuid>>>,
    // Optional maximum size for bounded collections
    max_size: Option<usize>,
}

impl<T> Pile<T> {
    pub fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            order: Arc::new(Mutex::new(VecDeque::new())),
            max_size: None,
        }
    }

    /// Create a new Pile with a maximum size (oldest items are removed when full)
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            order: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size: Some(max_size),
        }
    }

    /// Insert an item with a specific ID
    pub fn insert(&self, id: Uuid, item: T) {
        let mut items = self.items.lock().unwrap();
        let mut order = self.order.lock().unwrap();

        // If we're at capacity, remove oldest item
        if let Some(max) = self.max_size {
            if items.len() >= max {
                if let Some(old_id) = order.pop_front() {
                    items.remove(&old_id);
                }
            }
        }

        items.insert(id, item);
        order.push_back(id);
    }

    /// Get an item by ID
    pub fn get(&self, id: &Uuid) -> Option<T>
    where
        T: Clone,
    {
        let guard = self.items.lock().unwrap();
        guard.get(id).cloned()
    }

    /// Get all items in insertion order
    pub fn get_ordered(&self) -> Vec<T>
    where
        T: Clone,
    {
        let items = self.items.lock().unwrap();
        let order = self.order.lock().unwrap();
        order
            .iter()
            .filter_map(|id| items.get(id).cloned())
            .collect()
    }

    /// Get the most recent N items
    pub fn get_recent(&self, n: usize) -> Vec<T>
    where
        T: Clone,
    {
        let items = self.items.lock().unwrap();
        let order = self.order.lock().unwrap();
        order
            .iter()
            .rev()
            .take(n)
            .filter_map(|id| items.get(id).cloned())
            .collect()
    }

    /// List all IDs in insertion order
    pub fn list_ids(&self) -> Vec<Uuid> {
        let order = self.order.lock().unwrap();
        order.iter().cloned().collect()
    }

    /// Check if the pile contains an item with the given ID
    pub fn contains(&self, id: &Uuid) -> bool {
        let guard = self.items.lock().unwrap();
        guard.contains_key(id)
    }

    /// Get the number of items in the pile
    pub fn len(&self) -> usize {
        let guard = self.items.lock().unwrap();
        guard.len()
    }

    /// Check if the pile is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove an item by ID
    pub fn remove(&self, id: &Uuid) -> Option<T> {
        let mut items = self.items.lock().unwrap();
        let mut order = self.order.lock().unwrap();

        if let Some(item) = items.remove(id) {
            order.retain(|x| x != id);
            Some(item)
        } else {
            None
        }
    }

    /// Clear all items
    pub fn clear(&self) {
        let mut items = self.items.lock().unwrap();
        let mut order = self.order.lock().unwrap();
        items.clear();
        order.clear();
    }

    /// Filter items and return those matching the predicate
    pub fn filter<F>(&self, predicate: F) -> Vec<T>
    where
        T: Clone,
        F: Fn(&T) -> bool,
    {
        let items = self.items.lock().unwrap();
        let order = self.order.lock().unwrap();
        order
            .iter()
            .filter_map(|id| {
                items.get(id).and_then(|item| {
                    if predicate(item) {
                        Some(item.clone())
                    } else {
                        None
                    }
                })
            })
            .collect()
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
    fn test_pile_ordered_access() {
        let pile = Pile::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        pile.insert(id1, "first".to_string());
        pile.insert(id2, "second".to_string());
        pile.insert(id3, "third".to_string());

        let ordered = pile.get_ordered();
        assert_eq!(ordered, vec!["first", "second", "third"]);

        let recent = pile.get_recent(2);
        assert_eq!(recent, vec!["third", "second"]);
    }

    #[test]
    fn test_pile_max_size() {
        let pile = Pile::with_max_size(2);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        pile.insert(id1, "first".to_string());
        pile.insert(id2, "second".to_string());
        pile.insert(id3, "third".to_string());

        assert_eq!(pile.len(), 2);
        assert!(!pile.contains(&id1)); // First item should be removed
        assert!(pile.contains(&id2));
        assert!(pile.contains(&id3));

        let ordered = pile.get_ordered();
        assert_eq!(ordered, vec!["second", "third"]);
    }

    #[test]
    fn test_pile_filter() {
        let pile = Pile::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        pile.insert(id1, "apple".to_string());
        pile.insert(id2, "banana".to_string());
        pile.insert(id3, "apple pie".to_string());

        let apple_items = pile.filter(|item| item.contains("apple"));
        assert_eq!(apple_items.len(), 2);
        assert!(apple_items.contains(&"apple".to_string()));
        assert!(apple_items.contains(&"apple pie".to_string()));
    }

    #[test]
    fn test_pile_remove() {
        let pile = Pile::new();
        let id = Uuid::new_v4();
        pile.insert(id, "test".to_string());

        assert_eq!(pile.len(), 1);
        let removed = pile.remove(&id);
        assert_eq!(removed, Some("test".to_string()));
        assert_eq!(pile.len(), 0);
        assert!(pile.get_ordered().is_empty());
    }
}
