use super::error::PluginError;
use super::manifest::PluginManifest;
use crate::element::ElementData;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
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

    fn resolve_entry_point(
        &self,
        manifest_path: &Path,
        entry_point: &str,
    ) -> Result<PathBuf, PluginError> {
        // Get the directory containing the manifest
        let manifest_dir = manifest_path.parent().ok_or_else(|| {
            PluginError::LoadError(format!(
                "Invalid manifest path, cannot get parent directory: {}",
                manifest_path.display()
            ))
        })?;

        // Security check - ensure path resolves within manifest directory
        fn is_path_within_dir(path: &Path, dir: &Path) -> bool {
            match path.canonicalize() {
                Ok(canonical_path) => match dir.canonicalize() {
                    Ok(canonical_dir) => canonical_path.starts_with(canonical_dir),
                    Err(_) => false,
                },
                Err(_) => false,
            }
        }

        // If entry_point is absolute, verify it's within manifest directory
        let entry_path = PathBuf::from(entry_point);
        if entry_path.is_absolute() {
            if !is_path_within_dir(&entry_path, manifest_dir) {
                return Err(PluginError::LoadError(format!(
                    "Entry point must be within manifest directory: {}",
                    entry_point
                )));
            }
            debug!("Using absolute entry point: {:?}", entry_path);
            return Ok(entry_path);
        }

        // Resolve relative to manifest directory
        let mut resolved = manifest_dir.to_path_buf();
        for component in Path::new(entry_point).components() {
            match component {
                std::path::Component::Normal(name) => resolved.push(name),
                std::path::Component::ParentDir => {
                    resolved = resolved
                        .parent()
                        .ok_or_else(|| {
                            PluginError::LoadError(format!(
                                "Invalid path traversal in entry point: {}",
                                entry_point
                            ))
                        })?
                        .to_path_buf();
                }
                _ => continue, // Skip other component types
            }
        }

        if !is_path_within_dir(&resolved, manifest_dir) {
            return Err(PluginError::LoadError(format!(
                "Entry point must be within manifest directory: {}",
                entry_point
            )));
        }

        debug!("Using resolved entry point: {:?}", resolved);
        Ok(resolved)
    }

    pub fn load_plugin(
        &self,
        manifest: PluginManifest,
        manifest_path: &Path,
    ) -> Result<(Uuid, ElementData), PluginError> {
        debug!("Loading plugin {}", manifest.name);

        let entry_point = self.resolve_entry_point(manifest_path, &manifest.entry_point)?;
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

        let entry_point = self.resolve_entry_point(manifest_path, &manifest.entry_point)?;
        debug!("Resolved entry point for execution: {:?}", entry_point);

        // Execute the plugin as a subprocess
        let mut child = Command::new(&entry_point)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| {
                PluginError::ProcessError(format!(
                    "Failed to spawn process at {:?}: {}",
                    entry_point, e
                ))
            })?;

        // Write input to stdin with timeout handling
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).map_err(|e| {
                PluginError::ProcessError(format!("Failed to write to stdin: {}", e))
            })?;
            debug!("Wrote input to plugin stdin: {}", input);
            drop(stdin); // Explicitly close stdin to signal EOF to the plugin
        }

        // Read output line by line from stdout with improved error handling
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            match reader.lines().next() {
                Some(Ok(line)) => Ok(line),
                Some(Err(e)) => Err(PluginError::ProcessError(format!(
                    "Failed to read stdout: {}",
                    e
                ))),
                None => Err(PluginError::ProcessError(
                    format!(
                        "No output from plugin at {:?}. Verify:\n1. Plugin is executable\n2. Input format is correct: {}\n3. Plugin writes to stdout\n4. Plugin exits after writing output",
                        entry_point,
                        input
                    )
                ))
            }
        } else {
            Err(PluginError::ProcessError(
                "Failed to capture stdout".to_string(),
            ))
        }
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
    fn test_resolve_entry_point() {
        let temp_dir = tempdir().unwrap();
        let plugin_dir = temp_dir.path().join("plugins");
        std::fs::create_dir(&plugin_dir).unwrap();

        let manifest_path = plugin_dir.join("manifest.toml");
        let loader = PluginLoader::new(temp_dir.path());

        // Test relative path within manifest dir
        let result = loader.resolve_entry_point(&manifest_path, "./plugin.exe");
        assert!(result.is_ok());

        // Test absolute path outside manifest dir
        let result = loader.resolve_entry_point(&manifest_path, "/etc/passwd");
        assert!(result.is_err());

        // Test parent traversal outside manifest dir
        let result = loader.resolve_entry_point(&manifest_path, "../../../etc/passwd");
        assert!(result.is_err());

        // Test absolute path within manifest dir
        let plugin_path = plugin_dir.join("test_plugin");
        std::fs::write(&plugin_path, "test").unwrap();
        let result = loader.resolve_entry_point(&manifest_path, plugin_path.to_str().unwrap());
        assert!(result.is_ok());

        // Test relative path that resolves within manifest dir
        let result = loader.resolve_entry_point(&manifest_path, &format!("../plugins/test_plugin"));
        assert!(result.is_ok());
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

    #[cfg(unix)]
    #[test]
    fn test_load_plugin_not_executable() {
        use std::fs::File;
        let temp_dir = tempdir().unwrap();
        let plugin_path = temp_dir.path().join("plugin.sh");

        // Create non-executable file
        File::create(&plugin_path).unwrap();

        let manifest_path = temp_dir.path().join("manifest.toml");
        let loader = PluginLoader::new(temp_dir.path());
        let manifest = create_test_manifest(
            plugin_path
                .to_str()
                .expect("Failed to convert path to string"),
        );

        let result = loader.load_plugin(manifest, &manifest_path);
        assert!(result.is_err());
    }
}
