use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use uuid::Uuid;
use std::collections::HashSet;

use super::traits::{Describable, Identifiable, Validatable, Versionable};

mod response;
pub use response::PluginResponse;

/// Core plugin interface that all plugins must implement
#[async_trait]
pub trait Plugin: Identifiable + Describable + Versionable + Validatable {
    /// Initialize the plugin with its configuration
    async fn initialize(&mut self, config: Value) -> Result<(), Self::Error>;

    /// Execute the plugin with the given input
    async fn execute(&self, input: Value) -> Result<Value, Self::Error>;

    /// Handle a language message from an agent
    async fn handle_language_message(&self, message: Value) -> Result<Value, Self::Error> {
        // Default implementation treats it as a regular execute
        self.execute(message).await
    }

    /// Clean up any resources used by the plugin
    async fn cleanup(&mut self) -> Result<(), Self::Error>;

    /// Get the plugin's manifest
    fn manifest(&self) -> &PluginManifest;

    /// Get the plugin's current state
    fn state(&self) -> PluginState;
}

/// Metadata about a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique identifier for the plugin
    pub id: Uuid,

    /// The name of the plugin
    pub name: String,

    /// The version of the plugin
    pub version: String,

    /// A description of what the plugin does
    pub description: String,

    /// Path to the WASM file, relative to the manifest
    pub wasm_path: Option<String>,

    /// Optional configuration schema for the plugin
    #[serde(default)]
    pub config_schema: Option<Value>,

    /// Optional input schema for the plugin
    #[serde(default)]
    pub input_schema: Option<Value>,

    /// Optional output schema for the plugin
    #[serde(default)]
    pub output_schema: Option<Value>,

    /// Optional dependencies required by the plugin
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,

    /// Optional capabilities required by the plugin
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Language network protocol capabilities
    #[serde(default)]
    pub language_capabilities: LanguageCapabilities,

    /// Security settings
    #[serde(default)]
    pub security: SecuritySettings,
}

/// Language network protocol capabilities for a plugin
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageCapabilities {
    /// Whether this plugin can process language messages
    pub language_processor: bool,
    
    /// Supported language models or formats
    pub supported_models: HashSet<String>,
    
    /// Maximum concurrent language processing tasks
    pub max_concurrent_tasks: usize,
    
    /// Whether this plugin can generate responses
    pub can_generate: bool,
    
    /// Whether this plugin can modify messages
    pub can_modify: bool,
}

/// Security settings for a plugin
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Whether the plugin runs in a sandbox
    pub sandboxed: bool,
    
    /// Maximum memory usage in MB
    pub memory_limit_mb: usize,
    
    /// Maximum execution time in seconds
    pub time_limit_secs: usize,
    
    /// Allowed network domains
    pub allowed_domains: HashSet<String>,
    
    /// Whether file system access is allowed
    pub allow_fs_access: bool,
}

/// Dependency requirement for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Name of the required plugin
    pub name: String,

    /// Version requirement (semver)
    pub version_req: String,
}

/// Current state of a plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin is not initialized
    Uninitialized,
    /// Plugin is initialized and ready
    Ready,
    /// Plugin is currently executing
    Running,
    /// Plugin is processing a language message
    ProcessingLanguage,
    /// Plugin has encountered an error
    Error,
    /// Plugin has been disabled
    Disabled,
}

impl PluginManifest {
    /// Create a new plugin manifest
    pub fn new(name: String, version: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            version,
            description,
            wasm_path: None,
            config_schema: None,
            input_schema: None,
            output_schema: None,
            dependencies: Vec::new(),
            capabilities: Vec::new(),
            language_capabilities: LanguageCapabilities::default(),
            security: SecuritySettings::default(),
        }
    }

    /// Get the absolute path to the WASM file
    pub fn resolve_wasm_path(&self, manifest_dir: &PathBuf) -> Option<PathBuf> {
        self.wasm_path.as_ref().map(|path| manifest_dir.join(path))
    }

    /// Create a response from this manifest
    pub fn into_response(&self) -> PluginResponse {
        PluginResponse::new(
            self.id,
            self.name.clone(),
            self.version.clone(),
            self.description.clone(),
        )
    }

    /// Check if the plugin supports a specific language model
    pub fn supports_model(&self, model: &str) -> bool {
        self.language_capabilities.supported_models.contains(model)
    }

    /// Check if the plugin has access to a specific domain
    pub fn can_access_domain(&self, domain: &str) -> bool {
        self.security.allowed_domains.contains(domain)
    }
}

impl Validatable for PluginManifest {
    type Error = String;

    fn validate(&self) -> Result<(), Self::Error> {
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
}

impl Identifiable for PluginManifest {
    fn id(&self) -> Uuid {
        self.id
    }
}

impl Describable for PluginManifest {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

impl Versionable for PluginManifest {
    fn version(&self) -> String {
        self.version.clone()
    }

    fn is_compatible_with(&self, other_version: &str) -> bool {
        // Simple version check for now
        self.version == other_version
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
    fn test_manifest_into_response() {
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        let response = manifest.into_response();
        assert_eq!(response.id, manifest.id);
        assert_eq!(response.name, manifest.name);
        assert_eq!(response.version, manifest.version);
        assert_eq!(response.description, manifest.description);
    }

    #[test]
    fn test_language_capabilities() {
        let mut manifest = PluginManifest::new(
            "language-plugin".to_string(),
            "1.0.0".to_string(),
            "A language plugin".to_string(),
        );

        manifest.language_capabilities.language_processor = true;
        manifest.language_capabilities.supported_models.insert("gpt-4".to_string());
        
        assert!(manifest.supports_model("gpt-4"));
        assert!(!manifest.supports_model("gpt-3"));
    }

    #[test]
    fn test_security_settings() {
        let mut manifest = PluginManifest::new(
            "secure-plugin".to_string(),
            "1.0.0".to_string(),
            "A secure plugin".to_string(),
        );

        manifest.security.allowed_domains.insert("api.example.com".to_string());
        
        assert!(manifest.can_access_domain("api.example.com"));
        assert!(!manifest.can_access_domain("other.com"));
    }
}