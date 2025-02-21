mod response;

pub use response::PluginResponse;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use super::traits::{Identifiable, Stateful, Validatable};
use super::{Error, ParticipantState, Result};

/// Represents the state of a plugin in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is registered but not initialized
    Uninitialized,
    /// Plugin is being initialized
    Initializing,
    /// Plugin is ready for use
    Ready,
    /// Plugin is currently running
    Running,
    /// Plugin has encountered an error
    Error,
    /// Plugin is disabled
    Disabled,
    /// Plugin is processing a language task
    ProcessingLanguage,
}

impl From<PluginState> for ParticipantState {
    fn from(state: PluginState) -> Self {
        match state {
            PluginState::Uninitialized => ParticipantState::Uninitialized,
            PluginState::Initializing => ParticipantState::Initializing,
            PluginState::Ready => ParticipantState::Ready,
            PluginState::Running => ParticipantState::Running,
            PluginState::Error => ParticipantState::Error,
            PluginState::Disabled => ParticipantState::Disabled,
            PluginState::ProcessingLanguage => ParticipantState::ProcessingLanguage,
        }
    }
}

/// Language processing capabilities of a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for LanguageCapabilities {
    fn default() -> Self {
        Self {
            language_processor: false,
            can_generate: false,
            can_modify: false,
            max_input_length: None,
            max_output_length: None,
            supported_models: HashSet::new(),
            supported_languages: HashSet::new(),
        }
    }
}

/// Security settings for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            sandboxed: true,
            memory_limit_mb: 512,
            time_limit_secs: 30,
            network_enabled: false,
            filesystem_enabled: false,
            allowed_capabilities: HashSet::new(),
        }
    }
}

/// A plugin instance in the system
#[derive(Debug)]
pub struct Plugin {
    /// Unique identifier
    id: Uuid,
    /// Current state
    state: PluginState,
    /// Language capabilities
    capabilities: LanguageCapabilities,
    /// Security settings
    security: SecuritySettings,
}

impl Plugin {
    /// Create a new plugin instance
    pub fn new(
        id: Uuid,
        capabilities: LanguageCapabilities,
        security: SecuritySettings,
    ) -> Self {
        Self {
            id,
            state: PluginState::Uninitialized,
            capabilities,
            security,
        }
    }

    /// Get the plugin's language capabilities
    pub fn capabilities(&self) -> &LanguageCapabilities {
        &self.capabilities
    }

    /// Get the plugin's security settings
    pub fn security(&self) -> &SecuritySettings {
        &self.security
    }
}

impl Identifiable for Plugin {
    fn id(&self) -> Uuid {
        self.id
    }
}

impl Stateful for Plugin {
    fn state(&self) -> ParticipantState {
        self.state.clone().into()
    }

    fn set_state(&mut self, state: ParticipantState) {
        self.state = match state {
            ParticipantState::Uninitialized => PluginState::Uninitialized,
            ParticipantState::Initializing => PluginState::Initializing,
            ParticipantState::Ready => PluginState::Ready,
            ParticipantState::Running => PluginState::Running,
            ParticipantState::Error => PluginState::Error,
            ParticipantState::Disabled => PluginState::Disabled,
            ParticipantState::ProcessingLanguage => PluginState::ProcessingLanguage,
            _ => PluginState::Error,
        };
    }
}

impl Validatable for Plugin {
    fn validate(&self) -> Result<()> {
        // Validate ID
        if self.id == Uuid::nil() {
            return Err(Error::InvalidState("Plugin ID cannot be nil".into()));
        }

        // Basic capability checks
        if self.capabilities.language_processor {
            if self.capabilities.supported_models.is_empty() {
                return Err(Error::InvalidState(
                    "Language processor must support at least one model".into(),
                ));
            }
            if self.capabilities.supported_languages.is_empty() {
                return Err(Error::InvalidState(
                    "Language processor must support at least one language".into(),
                ));
            }
        }

        // Security checks
        if self.security.sandboxed {
            if self.security.memory_limit_mb == 0 {
                return Err(Error::InvalidState("Memory limit cannot be zero".into()));
            }
            if self.security.time_limit_secs == 0 {
                return Err(Error::InvalidState("Time limit cannot be zero".into()));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_state_conversion() {
        assert_eq!(
            ParticipantState::from(PluginState::Ready),
            ParticipantState::Ready
        );
        assert_eq!(
            ParticipantState::from(PluginState::Error),
            ParticipantState::Error
        );
    }

    #[test]
    fn test_plugin_response() {
        let id = Uuid::new_v4();
        let response = PluginResponse::new(
            id,
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        assert_eq!(response.id, id);
        assert_eq!(response.name, "test-plugin");
        assert_eq!(response.version, "1.0.0");
        assert_eq!(response.description, "A test plugin");
        assert!(response.status.is_none());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_plugin_validation() {
        let mut capabilities = LanguageCapabilities::default();
        capabilities.language_processor = true;
        capabilities.supported_models.insert("gpt-4".to_string());
        capabilities.supported_languages.insert("en".to_string());

        let plugin = Plugin::new(
            Uuid::new_v4(),
            capabilities,
            SecuritySettings::default(),
        );

        assert!(plugin.validate().is_ok());

        // Test invalid plugin (nil UUID)
        let invalid_plugin = Plugin::new(
            Uuid::nil(),
            LanguageCapabilities::default(),
            SecuritySettings::default(),
        );

        assert!(invalid_plugin.validate().is_err());
    }
}
