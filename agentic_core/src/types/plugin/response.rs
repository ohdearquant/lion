use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Standard response format for plugin operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// Unique identifier for the plugin
    pub id: Uuid,
    /// Name of the plugin
    pub name: String,
    /// Version of the plugin
    pub version: String,
    /// Description of the plugin
    pub description: String,
    /// Timestamp when the response was created
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    /// Optional status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PluginResponse {
    /// Create a new plugin response
    pub fn new(id: Uuid, name: String, version: String, description: String) -> Self {
        Self {
            id,
            name,
            version,
            description,
            timestamp: Utc::now(),
            status: None,
            error: None,
        }
    }

    /// Create a new error response
    pub fn error(error: impl Into<String>) -> Self {
        Self {
            id: Uuid::nil(),
            name: String::new(),
            version: String::new(),
            description: String::new(),
            timestamp: Utc::now(),
            status: None,
            error: Some(error.into()),
        }
    }

    /// Set the status message
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Set the error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_plugin_response_with_status() {
        let response = PluginResponse::new(
            Uuid::new_v4(),
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        )
        .with_status("loading");

        assert_eq!(response.status, Some("loading".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_plugin_error_response() {
        let response = PluginResponse::error("Failed to load plugin");

        assert_eq!(response.id, Uuid::nil());
        assert!(response.name.is_empty());
        assert!(response.version.is_empty());
        assert!(response.description.is_empty());
        assert!(response.status.is_none());
        assert_eq!(response.error, Some("Failed to load plugin".to_string()));
    }
}
