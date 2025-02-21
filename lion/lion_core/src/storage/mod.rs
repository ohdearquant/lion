//! Storage module providing a unified interface for storing and retrieving ElementData.
//!
//! This module defines the ElementStore trait which serves as a common interface for
//! different storage backends. It currently includes two implementations:
//! - FileStorage: A persistent storage implementation that saves elements to disk
//! - InMemoryStore: An ephemeral storage implementation for testing and temporary storage

use crate::element::ElementData;
use uuid::Uuid;

/// Common interface for storing and retrieving ElementData objects.
///
/// This trait defines the core operations that any storage backend must implement.
pub trait ElementStore {
    /// Store an element
    fn store(&self, element: ElementData);

    /// Retrieve an element by ID
    fn get(&self, id: &Uuid) -> Option<ElementData>;

    /// List all elements
    fn list(&self) -> Vec<ElementData>;

    /// Remove an element by ID
    fn remove(&self, id: &Uuid) -> bool;

    /// Clear all elements
    fn clear(&self);
}

mod cache;
mod file;
mod memory;

pub use file::FileStorage;
pub use memory::InMemoryStore;
