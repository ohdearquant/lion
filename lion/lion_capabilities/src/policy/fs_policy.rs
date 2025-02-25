//! File system policy implementation.

use std::path::{Path, PathBuf};

/// A policy for file system access
pub struct FilesystemPolicy {
    /// Allowed paths for read access
    allowed_read_paths: Vec<PathBuf>,
    
    /// Allowed paths for write access
    allowed_write_paths: Vec<PathBuf>,
    
    /// Whether to disallow access to any path outside the allowed paths
    strict: bool,
}

impl FilesystemPolicy {
    /// Create a new filesystem policy
    pub fn new() -> Self {
        Self {
            allowed_read_paths: Vec::new(),
            allowed_write_paths: Vec::new(),
            strict: true,
        }
    }
    
    /// Set whether to enforce strict path checking
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }
    
    /// Add an allowed read path
    pub fn add_read_path<P: AsRef<Path>>(&mut self, path: P) {
        self.allowed_read_paths.push(path.as_ref().to_path_buf());
    }
    
    /// Add an allowed write path
    pub fn add_write_path<P: AsRef<Path>>(&mut self, path: P) {
        self.allowed_write_paths.push(path.as_ref().to_path_buf());
    }
    
    /// Set allowed read paths
    pub fn with_read_paths<P: AsRef<Path>>(mut self, paths: Vec<P>) -> Self {
        self.allowed_read_paths = paths.into_iter().map(|p| p.as_ref().to_path_buf()).collect();
        self
    }
    
    /// Set allowed write paths
    pub fn with_write_paths<P: AsRef<Path>>(mut self, paths: Vec<P>) -> Self {
        self.allowed_write_paths = paths.into_iter().map(|p| p.as_ref().to_path_buf()).collect();
        self
    }
    
    /// Check if a path is allowed for reading
    pub fn can_read<P: AsRef<Path>>(&self, path: P) -> bool {
        if !self.strict {
            return true;
        }
        
        let path = path.as_ref();
        self.allowed_read_paths.iter().any(|allowed| path.starts_with(allowed))
    }
    
    /// Check if a path is allowed for writing
    pub fn can_write<P: AsRef<Path>>(&self, path: P) -> bool {
        if !self.strict {
            return true;
        }
        
        let path = path.as_ref();
        self.allowed_write_paths.iter().any(|allowed| path.starts_with(allowed))
    }
    
    /// Normalize and validate a path against a list of allowed paths
    pub fn validate_path<P: AsRef<Path>>(
        &self,
        path: P,
        allowed_paths: &[PathBuf],
    ) -> Result<PathBuf, String> {
        let path = path.as_ref();
        
        // Check for path traversal attacks
        if path.components().any(|c| c.as_os_str() == "..") {
            return Err("Path traversal attempt detected".to_string());
        }
        
        // Normalize path
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?
                .join(path)
        };
        
        // Check against allowed paths
        if self.strict && !allowed_paths.iter().any(|allowed| abs_path.starts_with(allowed)) {
            return Err(format!(
                "Path '{}' is not under any allowed directory",
                abs_path.display()
            ));
        }
        
        Ok(abs_path)
    }
}

impl Default for FilesystemPolicy {
    fn default() -> Self {
        Self {
            allowed_read_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from(std::env::temp_dir()),
            ],
            allowed_write_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from(std::env::temp_dir()),
            ],
            strict: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_filesystem_policy() {
        let mut policy = FilesystemPolicy::new();
        policy.add_read_path("/home/user/data");
        policy.add_write_path("/home/user/output");
        
        // Test read permissions
        assert!(policy.can_read("/home/user/data/file.txt"));
        assert!(policy.can_read("/home/user/data/subdir/file.txt"));
        assert!(!policy.can_read("/home/user/other/file.txt"));
        
        // Test write permissions
        assert!(policy.can_write("/home/user/output/file.txt"));
        assert!(policy.can_write("/home/user/output/subdir/file.txt"));
        assert!(!policy.can_write("/home/user/data/file.txt"));
    }
    
    #[test]
    fn test_path_validation() {
        let policy = FilesystemPolicy::new()
            .with_read_paths(vec!["/home/user/data"]);
        
        // Valid paths
        assert!(policy.validate_path(
            "/home/user/data/file.txt",
            &policy.allowed_read_paths
        ).is_ok());
        
        // Path traversal attack
        let result = policy.validate_path(
            "/home/user/data/../../../etc/passwd",
            &policy.allowed_read_paths
        );
        assert!(result.is_err());
        
        // Path outside allowed directories
        let result = policy.validate_path(
            "/etc/passwd",
            &policy.allowed_read_paths
        );
        assert!(result.is_err());
    }
}