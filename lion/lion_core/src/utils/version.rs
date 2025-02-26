//! Version utilities.
//! 
//! This module provides utilities for version management.

use std::fmt;
use std::cmp::Ordering;
use std::str::FromStr;
use serde::{Serialize, Deserialize};

/// Version information.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    /// Major version.
    pub major: u32,
    
    /// Minor version.
    pub minor: u32,
    
    /// Patch version.
    pub patch: u32,
    
    /// Pre-release information.
    pub pre_release: Option<String>,
    
    /// Build metadata.
    pub build: Option<String>,
}

impl Version {
    /// Create a new version.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        }
    }
    
    /// Create a new version with pre-release information.
    pub fn with_pre_release(mut self, pre_release: impl Into<String>) -> Self {
        self.pre_release = Some(pre_release.into());
        self
    }
    
    /// Create a new version with build metadata.
    pub fn with_build(mut self, build: impl Into<String>) -> Self {
        self.build = Some(build.into());
        self
    }
    
    /// Check if this version is compatible with another version.
    ///
    /// Compatibility is defined as having the same major version.
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare major, minor, patch
        match self.major.cmp(&other.major) {
            Ordering::Equal => {
                match self.minor.cmp(&other.minor) {
                    Ordering::Equal => {
                        match self.patch.cmp(&other.patch) {
                            Ordering::Equal => {
                                // Compare pre-release
                                match (&self.pre_release, &other.pre_release) {
                                    (None, Some(_)) => Ordering::Greater,
                                    (Some(_), None) => Ordering::Less,
                                    (None, None) => Ordering::Equal,
                                    (Some(a), Some(b)) => a.cmp(b),
                                }
                            },
                            other => other,
                        }
                    },
                    other => other,
                }
            },
            other => other,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        
        if let Some(pre_release) = &self.pre_release {
            write!(f, "-{}", pre_release)?;
        }
        
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        
        Ok(())
    }
}

impl FromStr for Version {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse a string like "1.2.3-alpha+build.123"
        let mut parts = s.split('+');
        let version_part = parts.next().ok_or_else(|| "Empty version string".to_string())?;
        let build = parts.next().map(|s| s.to_string());
        
        // Check if there are too many '+' characters
        if parts.next().is_some() {
            return Err("Too many '+' characters in version string".to_string());
        }
        
        // Parse the version part
        let mut parts = version_part.split('-');
        let version_numbers = parts.next().ok_or_else(|| "Empty version numbers".to_string())?;
        let pre_release = parts.next().map(|s| s.to_string());
        
        // Check if there are too many '-' characters
        if parts.next().is_some() {
            return Err("Too many '-' characters in version string".to_string());
        }
        
        // Parse the version numbers
        let mut parts = version_numbers.split('.');
        let major = parts.next()
            .ok_or_else(|| "Missing major version".to_string())?
            .parse::<u32>()
            .map_err(|e| format!("Invalid major version: {}", e))?;
        
        let minor = parts.next()
            .ok_or_else(|| "Missing minor version".to_string())?
            .parse::<u32>()
            .map_err(|e| format!("Invalid minor version: {}", e))?;
        
        let patch = parts.next()
            .ok_or_else(|| "Missing patch version".to_string())?
            .parse::<u32>()
            .map_err(|e| format!("Invalid patch version: {}", e))?;
        
        // Check if there are too many version components
        if parts.next().is_some() {
            return Err("Too many version components".to_string());
        }
        
        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
            build,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_parsing() {
        let version = Version::from_str("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.pre_release, None);
        assert_eq!(version.build, None);
        
        let version = Version::from_str("1.2.3-alpha").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.pre_release, Some("alpha".to_string()));
        assert_eq!(version.build, None);
        
        let version = Version::from_str("1.2.3+build.123").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.pre_release, None);
        assert_eq!(version.build, Some("build.123".to_string()));
        
        let version = Version::from_str("1.2.3-alpha+build.123").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.pre_release, Some("alpha".to_string()));
        assert_eq!(version.build, Some("build.123".to_string()));
    }
    
    #[test]
    fn test_version_ordering() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 4);
        let v3 = Version::new(1, 3, 0);
        let v4 = Version::new(2, 0, 0);
        
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 3).with_pre_release("alpha");
        let v3 = Version::new(1, 2, 3).with_pre_release("beta");
        let v4 = Version::new(1, 2, 3);
        
        assert!(v2 < v1);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert_eq!(v1, v4);
        
        // Build metadata does not affect ordering
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 3).with_build("build.123");
        
        assert_eq!(v1.cmp(&v2), Ordering::Equal);
    }
    
    #[test]
    fn test_version_display() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
        
        let version = Version::new(1, 2, 3).with_pre_release("alpha");
        assert_eq!(version.to_string(), "1.2.3-alpha");
        
        let version = Version::new(1, 2, 3).with_build("build.123");
        assert_eq!(version.to_string(), "1.2.3+build.123");
        
        let version = Version::new(1, 2, 3).with_pre_release("alpha").with_build("build.123");
        assert_eq!(version.to_string(), "1.2.3-alpha+build.123");
    }
    
    #[test]
    fn test_compatibility() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 3, 0);
        let v3 = Version::new(2, 0, 0);
        
        assert!(v1.is_compatible_with(&v2));
        assert!(!v1.is_compatible_with(&v3));
    }
}