use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// The name of the plugin
    pub name: String,

    /// The version of the plugin
    pub version: String,

    /// A description of what the plugin does
    pub description: String,

    /// Path to the WASM file, relative to the manifest
    pub wasm_path: Option<String>,

    /// Optional configuration for the plugin
    #[serde(default)]
    pub config: serde_json::Value,

    /// Optional dependencies required by the plugin
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,

    /// Optional capabilities required by the plugin
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Name of the required plugin
    pub name: String,

    /// Version requirement (semver)
    pub version_req: String,
}

impl PluginManifest {
    /// Create a new plugin manifest
    pub fn new(name: String, version: String, description: String) -> Self {
        Self {
            name,
            version,
            description,
            wasm_path: None,
            config: serde_json::Value::Null,
            dependencies: Vec::new(),
            capabilities: Vec::new(),
        }
    }

    /// Validate the manifest
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Plugin name cannot be empty".to_string());
        }
        if self.version.is_empty() {
            return Err("Plugin version cannot be empty".to_string());
        }
        if self.description.is_empty() {
            return Err("Plugin description cannot be empty".to_string());
        }
        Ok(())
    }

    /// Get the absolute path to the WASM file
    pub fn resolve_wasm_path(&self, manifest_dir: &PathBuf) -> Option<PathBuf> {
        self.wasm_path.as_ref().map(|path| manifest_dir.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_validation() {
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );
        assert!(manifest.validate().is_ok());

        let invalid = PluginManifest::new(
            "".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_wasm_path_resolution() {
        let mut manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );
        manifest.wasm_path = Some("plugin.wasm".to_string());

        let manifest_dir = PathBuf::from("/plugins/test-plugin");
        let wasm_path = manifest.resolve_wasm_path(&manifest_dir).unwrap();
        assert_eq!(wasm_path, PathBuf::from("/plugins/test-plugin/plugin.wasm"));
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        let serialized = serde_json::to_string(&manifest).unwrap();
        let deserialized: PluginManifest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(manifest.name, deserialized.name);
        assert_eq!(manifest.version, deserialized.version);
        assert_eq!(manifest.description, deserialized.description);
    }
}
