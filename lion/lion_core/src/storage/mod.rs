use crate::element::ElementData;
use uuid::Uuid;

/// Common interface for element storage implementations
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
