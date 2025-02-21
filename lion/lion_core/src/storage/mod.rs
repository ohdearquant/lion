mod error;
mod file;
mod store;
mod types;

pub use error::{Result, StorageError};
pub use file::FileStorage;
pub use store::Store;
pub use types::{Element, ElementData, ElementId, ElementMetadata};

// Constants
pub const DEFAULT_STORAGE_PATH: &str = "data";
