use crate::{
    orchestrator::metadata::{create_metadata, EventMetadata},
    plugin_manager::PluginManifest,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::SystemEvent;

/// Events related to plugin operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    /// Load a plugin
    Load {
        /// Plugin ID
        plugin_id: Uuid,
        /// Plugin manifest
        manifest: PluginManifest,
        /// Optional path to manifest file
        manifest_path: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    /// Invoke a plugin
    Invoked {
        /// Plugin ID
        plugin_id: Uuid,
        /// Input for the plugin
        input: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    /// List all plugins
    List,
    /// Plugin operation result
    Result {
        /// Plugin ID
        plugin_id: Uuid,
        /// Operation result
        result: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    /// Plugin operation error
    Error {
        /// Plugin ID
        plugin_id: Uuid,
        /// Error message
        error: String,
        /// Event metadata
        metadata: EventMetadata,
    },
}

impl PluginEvent {
    /// Create a new plugin load event
    pub fn load(
        plugin_id: Uuid,
        manifest: PluginManifest,
        manifest_path: Option<String>,
        correlation_id: Option<Uuid>,
    ) -> SystemEvent {
        SystemEvent::Plugin(PluginEvent::Load {
            plugin_id,
            manifest,
            manifest_path,
            metadata: create_metadata(correlation_id),
        })
    }

    /// Create a new plugin invocation event
    pub fn invoke(
        plugin_id: Uuid,
        input: impl Into<String>,
        correlation_id: Option<Uuid>,
    ) -> SystemEvent {
        SystemEvent::Plugin(PluginEvent::Invoked {
            plugin_id,
            input: input.into(),
            metadata: create_metadata(correlation_id),
        })
    }

    /// Create a new plugin result event
    pub fn result(
        plugin_id: Uuid,
        result: impl Into<String>,
        correlation_id: Option<Uuid>,
    ) -> SystemEvent {
        SystemEvent::Plugin(PluginEvent::Result {
            plugin_id,
            result: result.into(),
            metadata: create_metadata(correlation_id),
        })
    }

    /// Create a new plugin error event
    pub fn error(
        plugin_id: Uuid,
        error: impl Into<String>,
        correlation_id: Option<Uuid>,
    ) -> SystemEvent {
        SystemEvent::Plugin(PluginEvent::Error {
            plugin_id,
            error: error.into(),
            metadata: create_metadata(correlation_id),
        })
    }

    /// Create a new plugin list event
    pub fn list() -> SystemEvent {
        SystemEvent::Plugin(PluginEvent::List)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_events() {
        let plugin_id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        // Test load event
        match PluginEvent::load(
            plugin_id,
            manifest.clone(),
            Some("manifest.toml".to_string()),
            correlation_id,
        ) {
            SystemEvent::Plugin(PluginEvent::Load {
                plugin_id: pid,
                manifest: m,
                manifest_path,
                metadata,
            }) => {
                assert_eq!(pid, plugin_id);
                assert_eq!(m.name, manifest.name);
                assert_eq!(manifest_path, Some("manifest.toml".to_string()));
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Load event"),
        }

        // Test invoke event
        match PluginEvent::invoke(plugin_id, "test input", correlation_id) {
            SystemEvent::Plugin(PluginEvent::Invoked {
                plugin_id: pid,
                input,
                metadata,
            }) => {
                assert_eq!(pid, plugin_id);
                assert_eq!(input, "test input");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Invoked event"),
        }

        // Test result event
        match PluginEvent::result(plugin_id, "test result", correlation_id) {
            SystemEvent::Plugin(PluginEvent::Result {
                plugin_id: pid,
                result,
                metadata,
            }) => {
                assert_eq!(pid, plugin_id);
                assert_eq!(result, "test result");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Result event"),
        }

        // Test error event
        match PluginEvent::error(plugin_id, "test error", correlation_id) {
            SystemEvent::Plugin(PluginEvent::Error {
                plugin_id: pid,
                error,
                metadata,
            }) => {
                assert_eq!(pid, plugin_id);
                assert_eq!(error, "test error");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Error event"),
        }

        // Test list event
        match PluginEvent::list() {
            SystemEvent::Plugin(PluginEvent::List) => (),
            _ => panic!("Expected List event"),
        }
    }

    #[test]
    fn test_serialization() {
        let plugin_id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());
        let manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        let event = PluginEvent::Load {
            plugin_id,
            manifest: manifest.clone(),
            manifest_path: Some("manifest.toml".to_string()),
            metadata: create_metadata(correlation_id),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: PluginEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            PluginEvent::Load {
                plugin_id: pid,
                manifest: m,
                manifest_path,
                metadata,
            } => {
                assert_eq!(pid, plugin_id);
                assert_eq!(m.name, manifest.name);
                assert_eq!(manifest_path, Some("manifest.toml".to_string()));
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Load event after deserialization"),
        }
    }
}
