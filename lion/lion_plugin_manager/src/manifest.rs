//! Plugin manifest parsing and validation.

use crate::error::PluginManagerError;
use lion_core::capability::CoreCapability;
use lion_core::plugin::{PluginManifest, PluginSource};
use std::fs;
use std::path::{Path, PathBuf};

/// Parser for plugin manifests
pub struct ManifestParser;

impl ManifestParser {
    /// Create a new manifest parser
    pub fn new() -> Self {
        Self
    }
    
    /// Parse a manifest file
    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> Result<PluginManifest, PluginManagerError> {
        // Read the file
        let content = fs::read_to_string(path.as_ref()).map_err(|e| {
            PluginManagerError::IoError(format!(
                "Failed to read manifest file {}: {}",
                path.as_ref().display(),
                e
            ))
        })?;
        
        // Parse as TOML
        let manifest: PluginManifestToml = toml::from_str(&content).map_err(|e| {
            PluginManagerError::ParseError(format!("Failed to parse manifest: {}", e))
        })?;
        
        // Convert to core manifest
        self.to_core_manifest(manifest, Some(path.as_ref()))
    }
    
    /// Parse a manifest string
    pub fn parse_string(&self, content: &str) -> Result<PluginManifest, PluginManagerError> {
        // Parse as TOML
        let manifest: PluginManifestToml = toml::from_str(content).map_err(|e| {
            PluginManagerError::ParseError(format!("Failed to parse manifest: {}", e))
        })?;
        
        // Convert to core manifest
        self.to_core_manifest(manifest, None)
    }
    
    /// Convert a TOML manifest to a core manifest
    fn to_core_manifest(
        &self,
        toml: PluginManifestToml,
        manifest_path: Option<&Path>,
    ) -> Result<PluginManifest, PluginManagerError> {
        // Parse capabilities
        let capabilities = toml
            .capabilities
            .unwrap_or_default()
            .into_iter()
            .map(|cap| match cap.as_str() {
                "fs_read" => Ok(CoreCapability::FileSystemRead { path: None }),
                "fs_write" => Ok(CoreCapability::FileSystemWrite { path: None }),
                "network" => Ok(CoreCapability::NetworkClient { hosts: None }),
                "messaging" => Ok(CoreCapability::InterPluginComm),
                _ => Err(PluginManagerError::InvalidManifest(format!(
                    "Unknown capability: {}",
                    cap
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?;
        
        // Parse source
        let source = match (toml.source_type.as_str(), manifest_path) {
            ("file", Some(manifest_path)) => {
                // Resolve path relative to the manifest file
                let dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
                let path = dir.join(toml.source_path.unwrap_or_default());
                PluginSource::FilePath(path)
            }
            ("file", None) => {
                PluginSource::FilePath(PathBuf::from(toml.source_path.unwrap_or_default()))
            }
            ("url", _) => PluginSource::Url(toml.source_url.unwrap_or_default()),
            ("inline", _) => PluginSource::InMemory(toml.source_code.unwrap_or_default().into_bytes()),
            _ => {
                return Err(PluginManagerError::InvalidManifest(format!(
                    "Unknown source type: {}",
                    toml.source_type
                )));
            }
        };
        
        Ok(PluginManifest {
            name: toml.name,
            version: toml.version.unwrap_or_else(|| "0.1.0".to_string()),
            description: toml.description,
            author: toml.author,
            source,
            requested_capabilities: capabilities,
        })
    }
}

/// TOML representation of a plugin manifest
#[derive(Debug, Clone, serde::Deserialize)]
struct PluginManifestToml {
    /// The name of the plugin
    name: String,
    
    /// The version of the plugin
    #[serde(default)]
    version: Option<String>,
    
    /// Optional description
    #[serde(default)]
    description: Option<String>,
    
    /// Optional author information
    #[serde(default)]
    author: Option<String>,
    
    /// The source type (file, url, inline)
    source_type: String,
    
    /// The source path (for file source)
    #[serde(default)]
    source_path: Option<String>,
    
    /// The source URL (for url source)
    #[serde(default)]
    source_url: Option<String>,
    
    /// The source code (for inline source)
    #[serde(default)]
    source_code: Option<String>,
    
    /// The capabilities requested by this plugin
    #[serde(default)]
    capabilities: Option<Vec<String>>,
}
