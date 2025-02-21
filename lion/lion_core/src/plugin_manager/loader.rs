use super::error::PluginError;
use super::manifest::PluginManifest;
use crate::element::ElementData;
use serde_json::json;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PluginLoader {}

impl PluginLoader {
    pub fn new<P: AsRef<Path>>(_base_dir: P) -> Self {
        debug!("Creating PluginLoader");
        Self {}
    }

    fn resolve_entry_point(&self, manifest_path: &Path, entry_point: &str) -> PathBuf {
        // Get the directory containing the manifest
        let manifest_dir = manifest_path.parent().unwrap_or(Path::new("."));

        // If entry_point is absolute, use it directly
        let path = PathBuf::from(entry_point);
        if path.is_absolute() {
            debug!("Using absolute entry point: {:?}", path);
            return path;
        }

        // Resolve relative to manifest directory
        let resolved = manifest_dir
            .join(entry_point)
            .canonicalize()
            .unwrap_or_else(|_| manifest_dir.join(entry_point));
        debug!("Resolved entry point {:?} to {:?}", entry_point, resolved);
        resolved
    }

    pub fn load_plugin(
        &self,
        manifest: PluginManifest,
        manifest_path: &Path,
    ) -> Result<(Uuid, ElementData), PluginError> {
        debug!("Loading plugin {}", manifest.name);

        let entry_point = self.resolve_entry_point(manifest_path, &manifest.entry_point);
        debug!("Resolved entry point: {:?}", entry_point);

        // Check if entry point exists
        if !entry_point.exists() {
            error!("Entry point not found: {:?}", entry_point);
            return Err(PluginError::LoadError(format!(
                "Entry point not found: {}",
                entry_point.display()
            )));
        }

        debug!("Entry point exists checking if it's executable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = entry_point
                .metadata()
                .map_err(|e| PluginError::LoadError(format!("Failed to get metadata: {}", e)))?;
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            debug!("File permissions: {:o}", mode);
            if mode & 0o111 == 0 {
                error!("Entry point is not executable: {:?}", entry_point);
                return Err(PluginError::LoadError(format!(
                    "Entry point is not executable: {}",
                    entry_point.display()
                )));
            }
        }

        // Create an ElementData for this plugin
        let metadata = json!({
            "type": "plugin",
            "manifest": manifest,
            "status": "loaded"
        });
        let element = ElementData::new(metadata);
        let id = element.id;

        info!(
            "Plugin {} loaded successfully with ID {}",
            manifest.name, id
        );

        Ok((id, element))
    }

    pub fn invoke_plugin(
        &self,
        manifest: &PluginManifest,
        manifest_path: &Path,
        input: &str,
    ) -> Result<String, PluginError> {
        debug!("Invoking plugin {} with input: {}", manifest.name, input);

        let entry_point = self.resolve_entry_point(manifest_path, &manifest.entry_point);
        debug!("Resolved entry point for execution: {:?}", entry_point);

        // Execute the plugin as a subprocess
        let mut child = Command::new(&entry_point)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| PluginError::ProcessError(format!("Failed to spawn process: {}", e)))?;

        // Write input to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).map_err(|e| {
                PluginError::ProcessError(format!("Failed to write to stdin: {}", e))
            })?;
        }

        // Read output from stdout
        let output = child
            .wait_with_output()
            .map_err(|e| PluginError::ProcessError(format!("Failed to read output: {}", e)))?;

        if !output.status.success() {
            return Err(PluginError::ProcessError(format!(
                "Plugin process exited with status: {}",
                output.status
            )));
        }

        let result = String::from_utf8(output.stdout)
            .map_err(|e| PluginError::ProcessError(format!("Invalid UTF-8 output: {}", e)))?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn create_test_manifest(entry_point: &str) -> PluginManifest {
        PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: entry_point.to_string(),
            permissions: vec![],
            driver: None,
            functions: HashMap::new(),
        }
    }

    #[test]
    fn test_load_plugin_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let manifest_path = temp_dir.path().join("manifest.toml");
        let loader = PluginLoader::new(temp_dir.path());
        let manifest = create_test_manifest("nonexistent");
        let result = loader.load_plugin(manifest, &manifest_path);
        assert!(result.is_err());
    }
}
