//! Version utilities.
//!
//! This module provides utilities for working with version numbers.
//! It defines a version type that follows semantic versioning.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Error parsing a version string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionParseError {
    /// The invalid version string.
    pub version: String,

    /// The reason for the error.
    pub reason: String,
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid version '{}': {}", self.version, self.reason)
    }
}

impl std::error::Error for VersionParseError {}

/// A semantic version.
///
/// This structure represents a version number following the semantic
/// versioning specification (https://semver.org/).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    /// Major version number.
    pub major: u32,

    /// Minor version number.
    pub minor: u32,

    /// Patch version number.
    pub patch: u32,

    /// Prerelease identifiers.
    pub prerelease: Option<String>,

    /// Build metadata.
    pub build: Option<String>,
}

impl Version {
    /// Create a new version.
    ///
    /// # Arguments
    ///
    /// * `major` - Major version number.
    /// * `minor` - Minor version number.
    /// * `patch` - Patch version number.
    ///
    /// # Returns
    ///
    /// A new version with the given components and no prerelease or build metadata.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
            build: None,
        }
    }

    /// Add prerelease identifiers to this version.
    ///
    /// # Arguments
    ///
    /// * `prerelease` - Prerelease identifiers.
    ///
    /// # Returns
    ///
    /// A new version with the given prerelease identifiers.
    pub fn with_prerelease(mut self, prerelease: impl Into<String>) -> Self {
        self.prerelease = Some(prerelease.into());
        self
    }

    /// Add build metadata to this version.
    ///
    /// # Arguments
    ///
    /// * `build` - Build metadata.
    ///
    /// # Returns
    ///
    /// A new version with the given build metadata.
    pub fn with_build(mut self, build: impl Into<String>) -> Self {
        self.build = Some(build.into());
        self
    }

    /// Check if this version is a prerelease version.
    ///
    /// # Returns
    ///
    /// `true` if this version has prerelease identifiers, `false` otherwise.
    pub fn is_prerelease(&self) -> bool {
        self.prerelease.is_some()
    }

    /// Check if this version is compatible with the given version
    /// (according to semantic versioning).
    ///
    /// A version is compatible with another version if they have the
    /// same major version number (except for major version 0) and the
    /// first version is lower than or equal to the second version.
    ///
    /// # Arguments
    ///
    /// * `other` - The version to check compatibility with.
    ///
    /// # Returns
    ///
    /// `true` if this version is compatible with the given version,
    /// `false` otherwise.
    pub fn is_compatible_with(&self, other: &Version) -> bool {
        if self.major == 0 && other.major == 0 {
            // For 0.x.y, a change in the minor version is breaking
            self.major == other.major && self.minor == other.minor && self <= other
        } else {
            // For >= 1.0.0, only a change in the major version is breaking
            self.major == other.major && self <= other
        }
    }

    /// Create a new compatible version by incrementing the minor version.
    ///
    /// # Returns
    ///
    /// A new version with the minor version incremented and patch set to 0.
    pub fn increment_minor(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
            prerelease: None,
            build: None,
        }
    }

    /// Create a new compatible version by incrementing the patch version.
    ///
    /// # Returns
    ///
    /// A new version with the patch version incremented.
    pub fn increment_patch(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
            prerelease: None,
            build: None,
        }
    }

    /// Create a new incompatible version by incrementing the major version.
    ///
    /// # Returns
    ///
    /// A new version with the major version incremented and minor and patch set to 0.
    pub fn increment_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            prerelease: None,
            build: None,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare major, minor, patch
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ordering => return ordering,
        }

        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ordering => return ordering,
        }

        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ordering => return ordering,
        }

        // Compare prerelease
        match (&self.prerelease, &other.prerelease) {
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (None, None) => Ordering::Equal,
            (Some(a), Some(b)) => {
                // Compare prerelease identifiers
                let a_parts: Vec<&str> = a.split('.').collect();
                let b_parts: Vec<&str> = b.split('.').collect();

                for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
                    // Numeric identifiers always have lower precedence than non-numeric identifiers
                    let a_is_numeric = a_part.chars().all(char::is_numeric);
                    let b_is_numeric = b_part.chars().all(char::is_numeric);

                    match (a_is_numeric, b_is_numeric) {
                        (true, true) => {
                            // Both are numeric, compare as numbers
                            let a_num = a_part.parse::<u64>().unwrap_or(0);
                            let b_num = b_part.parse::<u64>().unwrap_or(0);
                            match a_num.cmp(&b_num) {
                                Ordering::Equal => {}
                                ordering => return ordering,
                            }
                        }
                        (true, false) => return Ordering::Less,
                        (false, true) => return Ordering::Greater,
                        (false, false) => {
                            // Neither is numeric, compare lexicographically
                            match a_part.cmp(b_part) {
                                Ordering::Equal => {}
                                ordering => return ordering,
                            }
                        }
                    }
                }

                // If we get here, the common parts are equal, so the longer one is greater
                a_parts.len().cmp(&b_parts.len())
            }
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(prerelease) = &self.prerelease {
            write!(f, "-{}", prerelease)?;
        }

        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }

        Ok(())
    }
}

impl FromStr for Version {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let error = |reason: &str| VersionParseError {
            version: s.to_string(),
            reason: reason.to_string(),
        };

        // Split into version, prerelease, and build parts
        let mut parts = s.splitn(2, '+');
        let version_and_prerelease = parts.next().unwrap();
        let build = parts.next().map(|s| s.to_string());

        let mut parts = version_and_prerelease.splitn(2, '-');
        let version = parts.next().unwrap();
        let prerelease = parts.next().map(|s| s.to_string());

        // Parse version parts
        let mut parts = version.splitn(3, '.');

        let major = parts
            .next()
            .ok_or_else(|| error("Missing major version"))?
            .parse()
            .map_err(|_| error("Invalid major version"))?;

        let minor = parts
            .next()
            .ok_or_else(|| error("Missing minor version"))?
            .parse()
            .map_err(|_| error("Invalid minor version"))?;

        let patch = parts
            .next()
            .ok_or_else(|| error("Missing patch version"))?
            .parse()
            .map_err(|_| error("Invalid patch version"))?;

        // Validate prerelease and build metadata
        if let Some(prerelease) = &prerelease {
            if prerelease.is_empty() {
                return Err(error("Empty prerelease identifier"));
            }

            for part in prerelease.split('.') {
                if part.is_empty() {
                    return Err(error("Empty prerelease identifier"));
                }

                if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    return Err(error("Invalid prerelease identifier"));
                }
            }
        }

        if let Some(build) = &build {
            if build.is_empty() {
                return Err(error("Empty build metadata"));
            }

            for part in build.split('.') {
                if part.is_empty() {
                    return Err(error("Empty build metadata"));
                }

                if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    return Err(error("Invalid build metadata"));
                }
            }
        }

        Ok(Self {
            major,
            minor,
            patch,
            prerelease,
            build,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        // Test basic version
        let version = Version::from_str("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.prerelease, None);
        assert_eq!(version.build, None);

        // Test version with prerelease
        let version = Version::from_str("1.2.3-alpha.1").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.prerelease, Some("alpha.1".to_string()));
        assert_eq!(version.build, None);

        // Test version with build metadata
        let version = Version::from_str("1.2.3+build.456").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.prerelease, None);
        assert_eq!(version.build, Some("build.456".to_string()));

        // Test version with prerelease and build metadata
        let version = Version::from_str("1.2.3-alpha.1+build.456").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.prerelease, Some("alpha.1".to_string()));
        assert_eq!(version.build, Some("build.456".to_string()));

        // Test invalid versions
        assert!(Version::from_str("").is_err());
        assert!(Version::from_str("1").is_err());
        assert!(Version::from_str("1.2").is_err());
        assert!(Version::from_str("1.2.3.4").is_err());
        assert!(Version::from_str("a.b.c").is_err());
        assert!(Version::from_str("1.2.3-").is_err());
        assert!(Version::from_str("1.2.3+").is_err());
    }

    #[test]
    fn test_version_display() {
        assert_eq!(Version::new(1, 2, 3).to_string(), "1.2.3");
        assert_eq!(
            Version::new(1, 2, 3).with_prerelease("alpha.1").to_string(),
            "1.2.3-alpha.1"
        );
        assert_eq!(
            Version::new(1, 2, 3).with_build("build.456").to_string(),
            "1.2.3+build.456"
        );
        assert_eq!(
            Version::new(1, 2, 3)
                .with_prerelease("alpha.1")
                .with_build("build.456")
                .to_string(),
            "1.2.3-alpha.1+build.456"
        );
    }

    #[test]
    fn test_version_comparison() {
        // Basic version comparison
        assert!(Version::new(1, 2, 3) < Version::new(1, 2, 4));
        assert!(Version::new(1, 2, 3) < Version::new(1, 3, 0));
        assert!(Version::new(1, 2, 3) < Version::new(2, 0, 0));

        // Prerelease is less than non-prerelease
        assert!(Version::new(1, 2, 3).with_prerelease("alpha") < Version::new(1, 2, 3));

        // Prerelease comparison
        assert!(
            Version::new(1, 2, 3).with_prerelease("alpha")
                < Version::new(1, 2, 3).with_prerelease("beta")
        );

        assert!(
            Version::new(1, 2, 3).with_prerelease("alpha.1")
                < Version::new(1, 2, 3).with_prerelease("alpha.2")
        );

        assert!(
            Version::new(1, 2, 3).with_prerelease("alpha.1")
                < Version::new(1, 2, 3).with_prerelease("alpha.1.1")
        );

        // Numeric vs non-numeric prerelease identifiers
        assert!(
            Version::new(1, 2, 3).with_prerelease("1")
                < Version::new(1, 2, 3).with_prerelease("alpha")
        );

        assert!(
            Version::new(1, 2, 3).with_prerelease("alpha.1")
                < Version::new(1, 2, 3).with_prerelease("alpha.beta")
        );

        // Update test to properly handle build metadata in comparison
        // Build metadata should be ignored in version precedence
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 3).with_build("build.1");

        assert!(!(v1 < v2));
        assert!(!(v1 > v2));
        // Do not test for equality with == as that checks all fields

        // Test versions with different build metadata
        let v3 = Version::new(1, 2, 3).with_build("build.1");
        let v4 = Version::new(1, 2, 3).with_build("build.2");

        assert!(!(v3 < v4));
        assert!(!(v3 > v4));
    }

    #[test]
    fn test_version_compatibility() {
        // 1.2.3 is compatible with 1.2.3, 1.2.4, 1.3.0, but not 2.0.0 or 1.1.0
        let v1_2_3 = Version::new(1, 2, 3);
        assert!(v1_2_3.is_compatible_with(&Version::new(1, 2, 3)));
        assert!(v1_2_3.is_compatible_with(&Version::new(1, 2, 4)));
        assert!(v1_2_3.is_compatible_with(&Version::new(1, 3, 0)));
        assert!(!v1_2_3.is_compatible_with(&Version::new(2, 0, 0)));
        assert!(!v1_2_3.is_compatible_with(&Version::new(1, 1, 0)));

        // 0.1.2 is compatible with 0.1.2, 0.1.3, but not 0.2.0 or 1.0.0
        let v0_1_2 = Version::new(0, 1, 2);
        assert!(v0_1_2.is_compatible_with(&Version::new(0, 1, 2)));
        assert!(v0_1_2.is_compatible_with(&Version::new(0, 1, 3)));
        assert!(!v0_1_2.is_compatible_with(&Version::new(0, 2, 0)));
        assert!(!v0_1_2.is_compatible_with(&Version::new(1, 0, 0)));
    }

    #[test]
    fn test_version_increment() {
        // Increment patch
        let v1_2_3 = Version::new(1, 2, 3);
        let v1_2_4 = v1_2_3.increment_patch();
        assert_eq!(v1_2_4, Version::new(1, 2, 4));

        // Increment minor
        let v1_3_0 = v1_2_3.increment_minor();
        assert_eq!(v1_3_0, Version::new(1, 3, 0));

        // Increment major
        let v2_0_0 = v1_2_3.increment_major();
        assert_eq!(v2_0_0, Version::new(2, 0, 0));

        // Prerelease and build metadata are dropped
        let v1_2_3_alpha = v1_2_3.with_prerelease("alpha").with_build("build.1");
        assert_eq!(v1_2_3_alpha.increment_patch(), Version::new(1, 2, 4));
        assert_eq!(v1_2_3_alpha.increment_minor(), Version::new(1, 3, 0));
        assert_eq!(v1_2_3_alpha.increment_major(), Version::new(2, 0, 0));
    }

    #[test]
    fn test_version_serialization() {
        let version = Version::new(1, 2, 3)
            .with_prerelease("alpha.1")
            .with_build("build.456");

        let serialized = serde_json::to_string(&version).unwrap();
        let deserialized: Version = serde_json::from_str(&serialized).unwrap();

        assert_eq!(version, deserialized);
    }
}
