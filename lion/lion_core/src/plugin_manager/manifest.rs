use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub entry_point: String,
    pub permissions: Vec<String>,
    pub driver: Option<String>,
    pub functions: HashMap<String, PluginFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFunction {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
}

impl PluginManifest {
    pub fn try_load(path: &Path) -> Option<Self> {
        debug!("Attempting to load manifest from: {:?}", path);
        if path.is_file() {
            debug!("Found manifest file");
            match fs::read_to_string(path) {
                Ok(content) => {
                    debug!("Read manifest content: {}", content);
                    match toml::from_str::<PluginManifest>(&content) {
                        Ok(manifest) => {
                            debug!("Successfully parsed manifest for plugin: {}", manifest.name);
                            Some(manifest)
                        }
                        Err(e) => {
                            error!("Failed to parse manifest: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read manifest file: {}", e);
                    None
                }
            }
        } else {
            debug!("Path is not a file: {:?}", path);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_manifest() -> PluginManifest {
        PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: "nonexistent".to_string(),
            permissions: vec![],
            driver: None,
            functions: HashMap::new(),
        }
    }

    #[test]
    fn test_load_manifest() {
        let temp_dir = tempdir().unwrap();
        let manifest_path = temp_dir.path().join("manifest.toml");

        let manifest = create_test_manifest();
        let manifest_content = toml::to_string(&manifest).unwrap();
        fs::write(&manifest_path, manifest_content).unwrap();

        let loaded = PluginManifest::try_load(&manifest_path).unwrap();
        assert_eq!(loaded.name, "test_plugin");
    }
}
