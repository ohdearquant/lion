use async_trait::async_trait;
use std::fs;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Error type for storage operations
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Rename error: {0}")]
    RenameError(String),

    #[error("Other storage error: {0}")]
    Other(String),
}

/// Trait for storage backends
#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    /// Store data with the given key
    async fn store(&self, key: &str, data: &[u8]) -> Result<(), StorageError>;

    /// Load data with the given key
    async fn load(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Delete data with the given key
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// Check if a key exists
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;

    /// List all keys
    async fn list(&self) -> Result<Vec<String>, StorageError>;

    /// Atomically rename a key
    async fn rename(&self, old_key: &str, new_key: &str) -> Result<(), StorageError>;
}

/// File-based storage backend
pub struct FileStorage {
    /// Base directory for storage
    base_dir: PathBuf,
}

impl FileStorage {
    /// Create a new file storage backend
    pub fn new(base_dir: PathBuf) -> Self {
        FileStorage { base_dir }
    }

    /// Get the full path for a key
    fn get_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(key)
    }
}

#[async_trait]
impl StorageBackend for FileStorage {
    async fn store(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        let path = self.get_path(key);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write atomically using a temporary file
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, data).await?;

        // Ensure the data is synced to disk
        let file = tokio::fs::File::open(&temp_path).await?;
        file.sync_all().await?;

        // Rename to final path (atomic on most filesystems)
        tokio::fs::rename(&temp_path, &path).await?;

        Ok(())
    }

    async fn load(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.get_path(key);

        match tokio::fs::read(&path).await {
            Ok(data) => Ok(data),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Err(StorageError::NotFound(key.to_string()))
            }
            Err(e) => Err(StorageError::IoError(e)),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.get_path(key);

        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // File doesn't exist - consider deletion successful
                Ok(())
            }
            Err(e) => Err(StorageError::IoError(e)),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let path = self.get_path(key);

        match tokio::fs::metadata(&path).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(StorageError::IoError(e)),
        }
    }

    async fn list(&self) -> Result<Vec<String>, StorageError> {
        let mut entries = Vec::new();

        // Read directory entries
        let mut dir = tokio::fs::read_dir(&self.base_dir).await?;

        while let Some(entry) = dir.next_entry().await? {
            if let Ok(file_type) = entry.file_type().await {
                if file_type.is_file() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        entries.push(file_name.to_string());
                    }
                }
            }
        }

        Ok(entries)
    }

    async fn rename(&self, old_key: &str, new_key: &str) -> Result<(), StorageError> {
        let old_path = self.get_path(old_key);
        let new_path = self.get_path(new_key);

        // Ensure parent directory of the destination exists
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }

        match tokio::fs::rename(&old_path, &new_path).await {
            Ok(()) => Ok(()),
            Err(e) => Err(StorageError::RenameError(e.to_string())),
        }
    }
}

/// In-memory storage backend (for testing)
pub struct MemoryStorage {
    /// Data store
    data: tokio::sync::RwLock<std::collections::HashMap<String, Vec<u8>>>,
}

impl MemoryStorage {
    /// Create a new in-memory storage backend
    pub fn new() -> Self {
        MemoryStorage {
            data: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn store(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        let mut map = self.data.write().await;
        map.insert(key.to_string(), data.to_vec());
        Ok(())
    }

    async fn load(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let map = self.data.read().await;

        match map.get(key) {
            Some(data) => Ok(data.clone()),
            None => Err(StorageError::NotFound(key.to_string())),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let mut map = self.data.write().await;
        map.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let map = self.data.read().await;
        Ok(map.contains_key(key))
    }

    async fn list(&self) -> Result<Vec<String>, StorageError> {
        let map = self.data.read().await;
        Ok(map.keys().cloned().collect())
    }

    async fn rename(&self, old_key: &str, new_key: &str) -> Result<(), StorageError> {
        let mut map = self.data.write().await;

        if let Some(data) = map.remove(old_key) {
            map.insert(new_key.to_string(), data);
            Ok(())
        } else {
            Err(StorageError::NotFound(old_key.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new(temp_dir.path().to_path_buf());

        // Store data
        let key = "test_key";
        let data = b"test data";
        storage.store(key, data).await.unwrap();

        // Check if exists
        assert!(storage.exists(key).await.unwrap());

        // Load data
        let loaded = storage.load(key).await.unwrap();
        assert_eq!(loaded, data);

        // List keys
        let keys = storage.list().await.unwrap();
        assert!(keys.contains(&key.to_string()));

        // Delete data
        storage.delete(key).await.unwrap();
        assert!(!storage.exists(key).await.unwrap());
    }

    #[tokio::test]
    async fn test_rename_file_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new(temp_dir.path().to_path_buf());

        // Store data
        let old_key = "old_key";
        let new_key = "new_key";
        let data = b"test data";
        storage.store(old_key, data).await.unwrap();

        // Rename key
        storage.rename(old_key, new_key).await.unwrap();

        // Check old key doesn't exist
        assert!(!storage.exists(old_key).await.unwrap());

        // Check new key exists
        assert!(storage.exists(new_key).await.unwrap());

        // Load data from new key
        let loaded = storage.load(new_key).await.unwrap();
        assert_eq!(loaded, data);
    }

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryStorage::new();

        // Store data
        let key = "test_key";
        let data = b"test data";
        storage.store(key, data).await.unwrap();

        // Check if exists
        assert!(storage.exists(key).await.unwrap());

        // Load data
        let loaded = storage.load(key).await.unwrap();
        assert_eq!(loaded, data);

        // List keys
        let keys = storage.list().await.unwrap();
        assert!(keys.contains(&key.to_string()));

        // Delete data
        storage.delete(key).await.unwrap();
        assert!(!storage.exists(key).await.unwrap());
    }

    #[tokio::test]
    async fn test_rename_memory_storage() {
        let storage = MemoryStorage::new();

        // Store data
        let old_key = "old_key";
        let new_key = "new_key";
        let data = b"test data";
        storage.store(old_key, data).await.unwrap();

        // Rename key
        storage.rename(old_key, new_key).await.unwrap();

        // Check old key doesn't exist
        assert!(!storage.exists(old_key).await.unwrap());

        // Check new key exists and has correct data
        assert!(storage.exists(new_key).await.unwrap());
        let loaded = storage.load(new_key).await.unwrap();
        assert_eq!(loaded, data);
    }
}
