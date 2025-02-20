mod error;
mod file;
mod types;

pub use error::{Result, StorageError};
pub use file::FileStorage;
pub use types::{Element, ElementData, ElementId, ElementMetadata};

// Constants
pub const DEFAULT_STORAGE_PATH: &str = "data";
