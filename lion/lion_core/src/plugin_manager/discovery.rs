use super::error::PluginError;
use super::manifest::PluginManifest;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct PluginDiscovery {
    manifest_dir: PathBuf,
}

#[allow(unknown_lints)]
#[allow(clippy::unnecessary_map_or)]
impl PluginDiscovery {
    pub fn new<P: AsRef<Path>>(manifest_dir: P) -> Self {
        let dir = manifest_dir.as_ref().to_path_buf();
        debug!(
            "Creating PluginDiscovery with manifest directory: {:?}",
            dir
        );
        Self { manifest_dir: dir }
    }

    pub fn discover_plugins(&self) -> Result<Vec<(PluginManifest, PathBuf)>, PluginError> {
        debug!("Discovering plugins in directory: {:?}", self.manifest_dir);
        let mut manifests = Vec::new();

        if !self.manifest_dir.exists() {
            debug!(
                "Manifest directory does not exist: {}",
                self.manifest_dir.display()
            );
            return Err(PluginError::ManifestError(format!(
                "Manifest directory does not exist: {}",
                self.manifest_dir.display()
            )));
        }

        // Then check for other plugins in subdirectories
        for entry in fs::read_dir(&self.manifest_dir).map_err(|e| {
            PluginError::ManifestError(format!("Failed to read manifest directory: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                PluginError::ManifestError(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();
            debug!("Checking path: {:?}", path);

            // Skip data directory
            if path.ends_with("data") {
                continue;
            }

            // Check for manifest.toml in subdirectories
            if path.is_dir() {
                let manifest_path = path.join("manifest.toml");
                debug!("Looking for manifest at: {:?}", manifest_path);
                if let Some(manifest) = PluginManifest::try_load(&manifest_path) {
                    debug!("Successfully loaded manifest from: {:?}", manifest_path);
                    manifests.push((manifest, manifest_path));
                }
            }
            // Also check for manifest.toml files directly in the plugins directory
            else if path
                .file_name()
                .map_or(false, |name| name == "manifest.toml")
            {
                if let Some(manifest) = PluginManifest::try_load(&path) {
                    debug!("Successfully loaded manifest from: {:?}", path);
                    manifests.push((manifest, path));
                }
            }
        }

        debug!("Discovered {} plugins", manifests.len());
        Ok(manifests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_discover_plugins() {
        let temp_dir = tempdir().unwrap();

        // Create test plugin directory
        let plugin_dir = temp_dir.path().join("test_plugin");
        fs::create_dir(&plugin_dir).unwrap();

        // Create manifest for test plugin
        let manifest = PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: "nonexistent".to_string(),
            permissions: vec![],
            driver: None,
            functions: std::collections::HashMap::new(),
        };

        let manifest_content = toml::to_string(&manifest).unwrap();
        let manifest_path = plugin_dir.join("manifest.toml");
        fs::write(manifest_path, manifest_content).unwrap();

        // Test discovery
        let discovery = PluginDiscovery::new(temp_dir.path());
        let discovered = discovery.discover_plugins().unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].0.name, "test_plugin");
    }
}
