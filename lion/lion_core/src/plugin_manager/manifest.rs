use crate::types::{
    traits::{Describable, Validatable, Versionable},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Language processing capabilities of a plugin
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageCapabilities {
    /// Whether this plugin can process language
    pub language_processor: bool,
    /// Whether this plugin can generate new content
    pub can_generate: bool,
    /// Whether this plugin can modify existing content
    pub can_modify: bool,
    /// Maximum input length this plugin can handle
    pub max_input_length: Option<usize>,
    /// Maximum output length this plugin can produce
    pub max_output_length: Option<usize>,
    /// Supported language models
    pub supported_models: HashSet<String>,
    /// Supported languages (ISO codes)
    pub supported_languages: HashSet<String>,
}

/// Security settings for a plugin
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Whether this plugin runs in a sandbox
    pub sandboxed: bool,
    /// Memory limit in megabytes
    pub memory_limit_mb: usize,
    /// Time limit in seconds
    pub time_limit_secs: u64,
    /// Network access allowed
    pub network_enabled: bool,
    /// File system access allowed
    pub filesystem_enabled: bool,
    /// Allowed capabilities (e.g., "network", "storage")
    pub allowed_capabilities: HashSet<String>,
}

/// A plugin dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Name of the required plugin
    pub name: String,
    /// Version requirement (semver)
    pub version_req: String,
    /// Whether this is an optional dependency
    pub optional: bool,
}

/// A plugin manifest that describes its capabilities and requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique identifier
    pub id: Uuid,
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Path to WASM file
    pub wasm_path: Option<String>,
    /// Language capabilities
    pub language_capabilities: LanguageCapabilities,
    /// Security settings
    pub security: SecuritySettings,
    /// Plugin dependencies
    pub dependencies: Vec<PluginDependency>,
    /// Additional configuration
    pub config: serde_json::Value,
}

impl PluginManifest {
    /// Create a new plugin manifest
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            version: version.into(),
            description: description.into(),
            wasm_path: None,
            language_capabilities: LanguageCapabilities::default(),
            security: SecuritySettings::default(),
            dependencies: Vec::new(),
            config: serde_json::json!({}),
        }
    }

    /// Add a WASM path
    pub fn with_wasm_path(mut self, path: impl Into<String>) -> Self {
        self.wasm_path = Some(path.into());
        self
    }

    /// Add language capabilities
    pub fn with_language_capabilities(mut self, capabilities: LanguageCapabilities) -> Self {
        self.language_capabilities = capabilities;
        self
    }

    /// Add security settings
    pub fn with_security(mut self, security: SecuritySettings) -> Self {
        self.security = security;
        self
    }

    /// Add a dependency
    pub fn add_dependency(
        &mut self,
        name: impl Into<String>,
        version_req: impl Into<String>,
        optional: bool,
    ) {
        self.dependencies.push(PluginDependency {
            name: name.into(),
            version_req: version_req.into(),
            optional,
        });
    }
}

impl Validatable for PluginManifest {
    fn validate(&self) -> Result<()> {
        // Validate basic fields
        if self.name.is_empty() {
            return Err(Error::InvalidState("Plugin name cannot be empty".into()));
        }
        if self.version.is_empty() {
            return Err(Error::InvalidState("Plugin version cannot be empty".into()));
        }
        if self.description.is_empty() {
            return Err(Error::InvalidState(
                "Plugin description cannot be empty".into(),
            ));
        }

        // Validate security settings
        if self.security.sandboxed {
            if self.security.memory_limit_mb == 0 {
                return Err(Error::InvalidState("Memory limit cannot be zero".into()));
            }
            if self.security.time_limit_secs == 0 {
                return Err(Error::InvalidState("Time limit cannot be zero".into()));
            }
        }

        // Validate language capabilities
        if self.language_capabilities.language_processor {
            if self.language_capabilities.supported_models.is_empty() {
                return Err(Error::InvalidState(
                    "Language processor must support at least one model".into(),
                ));
            }
            if self.language_capabilities.supported_languages.is_empty() {
                return Err(Error::InvalidState(
                    "Language processor must support at least one language".into(),
                ));
            }
        }

        Ok(())
    }
}

impl Describable for PluginManifest {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn metadata(&self) -> &serde_json::Value {
        &self.config
    }
}

impl Versionable for PluginManifest {
    fn version(&self) -> &str {
        &self.version
    }

    fn is_compatible_with(&self, requirement: &str) -> bool {
        // TODO: Implement proper semver compatibility check
        self.version == requirement
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_validation() {
        let valid = PluginManifest::new("test-plugin", "1.0.0", "A test plugin");
        assert!(valid.validate().is_ok());

        let invalid = PluginManifest::new("", "1.0.0", "A test plugin");
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_language_capabilities() {
        let mut manifest = PluginManifest::new("test-plugin", "1.0.0", "A test plugin");

        let mut capabilities = LanguageCapabilities::default();
        capabilities.language_processor = true;
        manifest.language_capabilities = capabilities;

        // Should fail without supported models/languages
        assert!(manifest.validate().is_err());

        manifest
            .language_capabilities
            .supported_models
            .insert("gpt-4".to_string());
        manifest
            .language_capabilities
            .supported_languages
            .insert("en".to_string());

        // Should pass with supported models/languages
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_security_settings() {
        let mut manifest = PluginManifest::new("test-plugin", "1.0.0", "A test plugin");

        manifest.security.sandboxed = true;
        manifest.security.memory_limit_mb = 0;

        // Should fail with zero memory limit
        assert!(manifest.validate().is_err());

        manifest.security.memory_limit_mb = 512;
        manifest.security.time_limit_secs = 30;

        // Should pass with valid limits
        assert!(manifest.validate().is_ok());
    }
}
