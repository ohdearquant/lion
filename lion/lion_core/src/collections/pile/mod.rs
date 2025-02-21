mod tests;

use crate::types::traits::{LanguageMessage, LanguageMessageType};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur when working with a Pile
#[derive(Error, Debug)]
pub enum PileError {
    #[error("Lock acquisition failed: {0}")]
    LockError(String),
    #[error("Item not found: {0}")]
    NotFound(Uuid),
    #[error("Pile is at capacity")]
    AtCapacity,
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Result type for Pile operations
pub type PileResult<T> = Result<T, PileError>;

/// A thread-safe container for system objects that provides both
/// map-based access by ID and ordered access for sequences.
/// Optimized for concurrent access in multi-agent scenarios.
#[derive(Debug, Clone)]
pub struct Pile<T> {
    // Map for direct ID-based access
    items: Arc<Mutex<HashMap<Uuid, T>>>,
    // Ordered sequence of IDs for maintaining order
    order: Arc<Mutex<VecDeque<Uuid>>>,
    // Optional maximum size for bounded collections
    max_size: Option<usize>,
    // Optional partition key for sharding
    partition_key: Option<String>,
}

impl<T> Pile<T> {
    pub fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            order: Arc::new(Mutex::new(VecDeque::new())),
            max_size: None,
            partition_key: None,
        }
    }

    /// Create a new Pile with a maximum size (oldest items are removed when full)
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            order: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size: Some(max_size),
            partition_key: None,
        }
    }

    /// Create a new Pile with a partition key for sharding
    pub fn with_partition(key: impl Into<String>) -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            order: Arc::new(Mutex::new(VecDeque::new())),
            max_size: None,
            partition_key: Some(key.into()),
        }
    }

    /// Insert an item with a specific ID
    pub fn insert(&self, id: Uuid, item: T) -> PileResult<()> {
        let mut items = self
            .items
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        let mut order = self
            .order
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;

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
        Ok(())
    }

    /// Get an item by ID
    pub fn get(&self, id: &Uuid) -> PileResult<T>
    where
        T: Clone,
    {
        let guard = self
            .items
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        guard.get(id).cloned().ok_or(PileError::NotFound(*id))
    }

    /// Get all items in insertion order
    pub fn get_ordered(&self) -> PileResult<Vec<T>>
    where
        T: Clone,
    {
        let items = self
            .items
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        let order = self
            .order
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        Ok(order
            .iter()
            .filter_map(|id| items.get(id).cloned())
            .collect())
    }

    /// Get the most recent N items
    pub fn get_recent(&self, n: usize) -> PileResult<Vec<T>>
    where
        T: Clone,
    {
        let items = self
            .items
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        let order = self
            .order
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        Ok(order
            .iter()
            .rev()
            .take(n)
            .filter_map(|id| items.get(id).cloned())
            .collect())
    }

    /// List all IDs in insertion order
    pub fn list_ids(&self) -> PileResult<Vec<Uuid>> {
        let order = self
            .order
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        Ok(order.iter().cloned().collect())
    }

    /// Check if the pile contains an item with the given ID
    pub fn contains(&self, id: &Uuid) -> PileResult<bool> {
        let guard = self
            .items
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        Ok(guard.contains_key(id))
    }

    /// Get the number of items in the pile
    pub fn len(&self) -> PileResult<usize> {
        let guard = self
            .items
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        Ok(guard.len())
    }

    /// Check if the pile is empty
    pub fn is_empty(&self) -> PileResult<bool> {
        Ok(self.len()? == 0)
    }

    /// Remove an item by ID
    pub fn remove(&self, id: &Uuid) -> PileResult<Option<T>> {
        let mut items = self
            .items
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        let mut order = self
            .order
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;

        if let Some(item) = items.remove(id) {
            order.retain(|x| x != id);
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    /// Clear all items
    pub fn clear(&self) -> PileResult<()> {
        let mut items = self
            .items
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        let mut order = self
            .order
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        items.clear();
        order.clear();
        Ok(())
    }

    /// Filter items and return those matching the predicate
    pub fn filter<F>(&self, predicate: F) -> PileResult<Vec<T>>
    where
        T: Clone,
        F: Fn(&T) -> bool,
    {
        let items = self
            .items
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        let order = self
            .order
            .lock()
            .map_err(|e| PileError::LockError(e.to_string()))?;
        Ok(order
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
            .collect())
    }

    /// Get the partition key if one exists
    pub fn partition_key(&self) -> Option<&str> {
        self.partition_key.as_deref()
    }
}

/// Specialized methods for handling language messages
impl Pile<LanguageMessage> {
    /// Get all messages for a specific recipient
    pub fn get_messages_for_recipient(
        &self,
        recipient_id: &Uuid,
    ) -> PileResult<Vec<LanguageMessage>> {
        self.filter(|msg| msg.recipient_ids.contains(recipient_id))
    }

    /// Get all messages from a specific sender
    pub fn get_messages_from_sender(&self, sender_id: &Uuid) -> PileResult<Vec<LanguageMessage>> {
        self.filter(|msg| msg.sender_id == *sender_id)
    }

    /// Get all messages of a specific type
    pub fn get_messages_by_type(
        &self,
        msg_type: LanguageMessageType,
    ) -> PileResult<Vec<LanguageMessage>> {
        self.filter(|msg| msg.message_type == msg_type)
    }

    /// Get the conversation history between two participants
    pub fn get_conversation_history(
        &self,
        participant1: &Uuid,
        participant2: &Uuid,
    ) -> PileResult<Vec<LanguageMessage>> {
        self.filter(|msg| {
            (msg.sender_id == *participant1 && msg.recipient_ids.contains(participant2))
                || (msg.sender_id == *participant2 && msg.recipient_ids.contains(participant1))
        })
    }
}

impl<T> Default for Pile<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Helper trait for recovering from poisoned mutexes
trait PoisonRecovery<T> {
    fn recover(self) -> Result<T, PileError>;
}

impl<T> PoisonRecovery<T> for Result<T, PoisonError<T>> {
    fn recover(self) -> Result<T, PileError> {
        self.map_err(|e| PileError::LockError(format!("Mutex poisoned: {}", e)))
    }
}
