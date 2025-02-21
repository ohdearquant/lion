mod tests;

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
