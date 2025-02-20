use crate::element::ElementData;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Debug)]
pub struct FileStorage {
    base_path: PathBuf,
    cache: Arc<Mutex<HashMap<Uuid, ElementData>>>,
}

impl FileStorage {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        let base_path = base_path.as_ref().to_path_buf();

        debug!("Creating FileStorage with base path: {:?}", base_path);

        // Create directory if it doesn't exist
        if !base_path.exists() {
            debug!("Base path doesn't exist, creating directory");
            fs::create_dir_all(&base_path).unwrap_or_else(|e| {
                error!("Failed to create storage directory: {}", e);
            });
        }

        Self {
            base_path,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_element_path(&self, id: &Uuid) -> PathBuf {
        let path = self.base_path.join(format!("{}.json", id));
        debug!("Element path for {}: {:?}", id, path);
        path
    }

    pub fn store(&self, element: ElementData) {
        debug!("Storing element {} in FileStorage", element.id);
        let path = self.get_element_path(&element.id);

        let json = match serde_json::to_string_pretty(&element) {
            Ok(json) => {
                debug!("Serialized element to JSON: {}", json);
                json
            }
            Err(e) => {
                error!("Failed to serialize element: {}", e);
                return;
            }
        };

        debug!("Writing element to file: {:?}", path);
        match fs::write(&path, json) {
            Ok(_) => debug!("Successfully wrote element to file"),
            Err(e) => {
                error!("Failed to write element to file: {}", e);
                return;
            }
        }

        // Update cache
        debug!("Updating cache with element {}", element.id);
        let mut cache = self.cache.lock().unwrap();
        cache.insert(element.id, element);
        debug!("Cache updated successfully");
    }

    pub fn get(&self, id: &Uuid) -> Option<ElementData> {
        debug!("Getting element {} from FileStorage", id);

        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(element) = cache.get(id) {
                debug!("Found element {} in cache", id);
                return Some(element.clone());
            }
        }

        debug!("Element {} not in cache, checking file storage", id);

        // Try to load from file
        let path = self.get_element_path(id);
        if !path.exists() {
            debug!("Element file does not exist: {:?}", path);
            return None;
        }

        debug!("Reading element file: {:?}", path);
        match fs::read_to_string(&path) {
            Ok(json) => {
                debug!("Successfully read file contents: {}", json);
                match serde_json::from_str::<ElementData>(&json) {
                    Ok(element) => {
                        debug!("Successfully deserialized element");
                        // Update cache
                        let mut cache = self.cache.lock().unwrap();
                        cache.insert(*id, element.clone());
                        Some(element)
                    }
                    Err(e) => {
                        error!("Failed to deserialize element: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                error!("Failed to read element file: {}", e);
                None
            }
        }
    }

    pub fn list(&self) -> Vec<ElementData> {
        debug!("Listing all elements in FileStorage");
        let mut elements = Vec::new();

        // Read all files in the directory
        match fs::read_dir(&self.base_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if let Some(extension) = entry.path().extension() {
                        if extension == "json" {
                            debug!("Found JSON file: {:?}", entry.path());
                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                match serde_json::from_str::<ElementData>(&content) {
                                    Ok(element) => {
                                        debug!("Successfully parsed element {}", element.id);
                                        elements.push(element);
                                    }
                                    Err(e) => error!("Failed to parse element file: {}", e),
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => error!("Failed to read directory: {}", e),
        }

        debug!("Found {} elements", elements.len());
        elements
    }

    pub fn remove(&self, id: &Uuid) -> bool {
        debug!("Removing element {} from FileStorage", id);
        let path = self.get_element_path(id);
        if path.exists() {
            match fs::remove_file(&path) {
                Ok(_) => {
                    debug!("Successfully removed file");
                    // Update cache
                    let mut cache = self.cache.lock().unwrap();
                    cache.remove(id);
                    true
                }
                Err(e) => {
                    error!("Failed to remove element file: {}", e);
                    false
                }
            }
        } else {
            debug!("Element file does not exist: {:?}", path);
            false
        }
    }

    pub fn clear(&self) {
        debug!("Clearing all elements from FileStorage");
        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                if let Some(extension) = entry.path().extension() {
                    if extension == "json" {
                        debug!("Removing file: {:?}", entry.path());
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }

        // Clear cache
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        debug!("Cache cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_store_and_retrieve() {
        let dir = tempdir().unwrap();
        let storage = FileStorage::new(dir.path());

        let element = ElementData::new(json!({
            "test": "data"
        }));
        let id = element.id;

        storage.store(element);

        let retrieved = storage.get(&id).unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.metadata["test"], "data");
    }

    #[test]
    fn test_list_elements() {
        let dir = tempdir().unwrap();
        let storage = FileStorage::new(dir.path());

        let element1 = ElementData::new(json!({"index": 1}));
        let element2 = ElementData::new(json!({"index": 2}));

        storage.store(element1.clone());
        storage.store(element2.clone());

        let elements = storage.list();
        assert_eq!(elements.len(), 2);
        assert!(elements.iter().any(|e| e.id == element1.id));
        assert!(elements.iter().any(|e| e.id == element2.id));
    }

    #[test]
    fn test_remove_element() {
        let dir = tempdir().unwrap();
        let storage = FileStorage::new(dir.path());

        let element = ElementData::new(json!({"test": "data"}));
        let id = element.id;

        storage.store(element);
        assert!(storage.get(&id).is_some());

        storage.remove(&id);
        assert!(storage.get(&id).is_none());
    }

    #[test]
    fn test_clear_storage() {
        let dir = tempdir().unwrap();
        let storage = FileStorage::new(dir.path());

        storage.store(ElementData::new(json!({"test": 1})));
        storage.store(ElementData::new(json!({"test": 2})));

        assert_eq!(storage.list().len(), 2);

        storage.clear();
        assert_eq!(storage.list().len(), 0);
    }
}
