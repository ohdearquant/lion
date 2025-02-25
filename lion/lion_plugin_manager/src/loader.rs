//! Plugin loading utilities.

use crate::error::PluginManagerError;
use lion_core::plugin::PluginSource;
use std::fs;
use std::path::{Path, PathBuf};

/// Plugin loader for loading WebAssembly plugins from various sources
pub struct PluginLoader {
    /// Optional cache directory for downloaded plugins
    cache_dir: Option<PathBuf>,
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Self {
        Self { cache_dir: None }
    }
    
    /// Create a new plugin loader with a cache directory
    pub fn with_cache_dir<P: AsRef<Path>>(cache_dir: P) -> Self {
        Self {
            cache_dir: Some(cache_dir.as_ref().to_path_buf()),
        }
    }
    
    /// Load a plugin from a source
    pub fn load_source(&self, source: &PluginSource) -> Result<Vec<u8>, PluginManagerError> {
        match source {
            PluginSource::FilePath(path) => self.load_file(path),
            PluginSource::InMemory(bytes) => Ok(bytes.clone()),
            PluginSource::Url(url) => self.load_url(url),
        }
    }
    
    /// Load a plugin from a file
    fn load_file(&self, path: &Path) -> Result<Vec<u8>, PluginManagerError> {
        fs::read(path).map_err(|e| {
            PluginManagerError::IoError(format!("Failed to read file {}: {}", path.display(), e))
        })
    }
    
    /// Load a plugin from a URL
    fn load_url(&self, url: &str) -> Result<Vec<u8>, PluginManagerError> {
        // If we have a cache directory, check if the URL is already cached
        if let Some(cache_dir) = &self.cache_dir {
            let file_name = self.url_to_file_name(url);
            let cache_path = cache_dir.join(file_name);
            
            if cache_path.exists() {
                return self.load_file(&cache_path);
            }
        }
        
        // Download the file
        let response = ureq::get(url)
            .call()
            .map_err(|e| {
                PluginManagerError::IoError(format!("Failed to download {}: {}", url, e))
            })?;
        
        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| {
                PluginManagerError::IoError(format!(
                    "Failed to read response body from {}: {}",
                    url, e
                ))
            })?;
        
        // If we have a cache directory, save the file
        if let Some(cache_dir) = &self.cache_dir {
            let file_name = self.url_to_file_name(url);
            let cache_path = cache_dir.join(file_name);
            
            // Create the cache directory if it doesn't exist
            if !cache_dir.exists() {
                fs::create_dir_all(cache_dir).map_err(|e| {
                    PluginManagerError::IoError(format!(
                        "Failed to create cache directory {}: {}",
                        cache_dir.display(), e
                    ))
                })?;
            }
            
            // Write the file
            fs::write(&cache_path, &bytes).map_err(|e| {
                PluginManagerError::IoError(format!(
                    "Failed to write cache file {}: {}",
                    cache_path.display(), e
                ))
            })?;
        }
        
        Ok(bytes)
    }
    
    /// Convert a URL to a file name for caching
    fn url_to_file_name(&self, url: &str) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        // Get the last path component of the URL
        let mut file_name = url
            .split('/')
            .last()
            .unwrap_or("unknown")
            .to_string();
        
        // If the file name doesn't have an extension, add a hash of the URL
        if !file_name.contains('.') {
            let mut hasher = DefaultHasher::new();
            url.hash(&mut hasher);
            let hash = hasher.finish();
            file_name = format!("{}_{}.wasm", file_name, hash);
        }
        
        file_name
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}