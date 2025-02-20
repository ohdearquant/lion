use super::{
    error::{Result, StorageError},
    types::{Element, ElementData, ElementId},
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

/// File-based storage implementation
#[derive(Debug)]
pub struct FileStorage {
    path: PathBuf,
    cache: RwLock<HashMap<ElementId, Element>>,
}

impl FileStorage {
    /// Create a new file storage instance
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        // Create directory if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path).unwrap_or_else(|e| {
                panic!(
                    "Failed to create storage directory {}: {}",
                    path.display(),
                    e
                )
            });
        }
        Self {
            path,
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get the file path for an element
    fn get_file_path(&self, id: ElementId) -> PathBuf {
        self.path.join(format!("{}.json", id))
    }

    /// Store an element
    pub fn set(&self, id: impl Into<ElementId>, data: Value) -> Result<()> {
        let id = id.into();
        let element = Element::new(id, ElementData::new(data));

        // Update cache
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(id, element.clone());
        } else {
            return Err(StorageError::LockError(
                "Failed to acquire write lock".into(),
            ));
        }

        // Write to file
        let file_path = self.get_file_path(id);
        fs::write(file_path, serde_json::to_string_pretty(&element)?)?;

        Ok(())
    }

    /// Retrieve an element
    pub fn get(&self, id: impl Into<ElementId>) -> Option<Element> {
        let id = id.into();

        // Try cache first
        if let Ok(cache) = self.cache.read() {
            if let Some(element) = cache.get(&id) {
                return Some(element.clone());
            }
        }

        // Try reading from file
        let file_path = self.get_file_path(id);
        if file_path.exists() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Ok(element) = serde_json::from_str::<Element>(&content) {
                    // Update cache
                    if let Ok(mut cache) = self.cache.write() {
                        cache.insert(id, element.clone());
                    }
                    return Some(element);
                }
            }
        }

        None
    }

    /// Remove an element
    pub fn remove(&self, id: impl Into<ElementId>) -> bool {
        let id = id.into();

        // Remove from cache
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(&id);
        }

        // Remove file
        let file_path = self.get_file_path(id);
        if file_path.exists() {
            fs::remove_file(file_path).is_ok()
        } else {
            false
        }
    }

    /// List all elements
    pub fn list(&self) -> Vec<Element> {
        let mut elements = Vec::new();

        // Read directory
        if let Ok(dir_entries) = fs::read_dir(&self.path) {
            for entry in dir_entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(ext) = entry.path().extension() {
                            if ext == "json" {
                                if let Ok(content) = fs::read_to_string(entry.path()) {
                                    if let Ok(element) = serde_json::from_str::<Element>(&content) {
                                        elements.push(element);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn test_file_storage() {
        let temp_dir = tempdir().unwrap();
        let storage = FileStorage::new(temp_dir.path());

        // Test set and get
        let id = ElementId(Uuid::new_v4());
        let data = json!({
            "name": "test",
            "value": 42
        });

        storage.set(id, data.clone()).unwrap();
        let element = storage.get(id).unwrap();
        assert_eq!(element.data.content, data);

        // Test list
        let elements = storage.list();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].id, id);

        // Test remove
        assert!(storage.remove(id));
        assert!(storage.get(id).is_none());
        assert_eq!(storage.list().len(), 0);
    }

    #[test]
    fn test_storage_persistence() {
        let temp_dir = tempdir().unwrap();
        let storage = FileStorage::new(temp_dir.path());

        // Store some data
        let id = ElementId(Uuid::new_v4());
        let data = json!({
            "name": "test",
            "value": 42
        });
        storage.set(id, data.clone()).unwrap();

        // Create new storage instance with same path
        let storage2 = FileStorage::new(temp_dir.path());
        let element = storage2.get(id).unwrap();
        assert_eq!(element.data.content, data);
    }

    #[test]
    fn test_invalid_data() {
        let temp_dir = tempdir().unwrap();
        let storage = FileStorage::new(temp_dir.path());

        // Write invalid JSON to file
        let id = ElementId(Uuid::new_v4());
        let file_path = storage.get_file_path(id);
        fs::write(&file_path, "invalid json").unwrap();

        // Should return None for invalid data
        assert!(storage.get(id).is_none());
    }
}
