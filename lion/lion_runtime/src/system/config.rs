//! Configuration for Lion Runtime
//!
//! Handles loading and managing runtime configuration.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;
use tracing::{debug, error, info, warn};

use crate::plugin::manager::PluginConfig;

/// Errors that can occur in configuration
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    LoadFailed(String),
    
    #[error("Failed to parse configuration: {0}")]
    ParseFailed(String),
    
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Whether monitoring is enabled
    #[serde(default)]
    pub enabled: bool,
    
    /// Port for monitoring HTTP server
    #[serde(default = "default_monitoring_port")]
    pub port: u16,
    
    /// Whether to bind to localhost only
    #[serde(default = "default_local_only")]
    pub local_only: bool,
    
    /// Monitoring token for authentication
    #[serde(default)]
    pub token: Option<String>,
}

fn default_monitoring_port() -> u16 {
    9090
}

fn default_local_only() -> bool {
    true
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_monitoring_port(),
            local_only: default_local_only(),
            token: None,
        }
    }
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Plugin directory
    #[serde(default = "default_plugin_directory")]
    pub plugin_directory: String,
    
    /// Plugin configurations
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,
    
    /// Bootstrap timeout (seconds) for each phase
    #[serde(default = "default_bootstrap_timeouts")]
    pub bootstrap_timeouts: HashMap<u8, u32>,
    
    /// Shutdown timeout (seconds)
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: u32,
    
    /// Monitoring configuration
    #[serde(default)]
    pub monitoring: MonitoringConfig,
    
    /// Maximum number of worker threads
    #[serde(default = "default_max_threads")]
    pub max_threads: usize,
    
    /// Additional configuration
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_plugin_directory() -> String {
    "./plugins".to_string()
}

fn default_bootstrap_timeouts() -> HashMap<u8, u32> {
    let mut timeouts = HashMap::new();
    timeouts.insert(0, 30); // Phase 0: Core
    timeouts.insert(1, 30); // Phase 1: Essential
    timeouts.insert(2, 30); // Phase 2: Standard
    timeouts.insert(3, 30); // Phase 3: Optional
    timeouts
}

fn default_shutdown_timeout() -> u32 {
    30
}

fn default_max_threads() -> usize {
    4
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            plugin_directory: default_plugin_directory(),
            plugins: HashMap::new(),
            bootstrap_timeouts: default_bootstrap_timeouts(),
            shutdown_timeout: default_shutdown_timeout(),
            monitoring: MonitoringConfig::default(),
            max_threads: default_max_threads(),
            extra: HashMap::new(),
        }
    }
}

impl RuntimeConfig {
    /// Load configuration from a file
    pub async fn load(path: Option<&str>) -> Result<Self> {
        // Start with default configuration
        let mut config = RuntimeConfig::default();
        
        // If a path is provided, try to load from it
        if let Some(path) = path {
            info!("Loading configuration from {}", path);
            
            // Check if the file exists
            if !Path::new(path).exists() {
                warn!("Configuration file not found: {}", path);
                return Ok(config);
            }
            
            // Read the file
            let content = fs::read_to_string(path)
                .await
                .context(format!("Failed to read configuration file: {}", path))?;
            
            // Parse the configuration
            config = serde_json::from_str(&content)
                .context(format!("Failed to parse configuration file: {}", path))?;
        } else {
            info!("No configuration file specified, using defaults");
        }
        
        // Validate the configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Check plugin directory
        if self.plugin_directory.is_empty() {
            return Err(ConfigError::Invalid("Plugin directory cannot be empty".to_string()).into());
        }
        
        // Check bootstrap timeouts
        for phase in 0..4 {
            if !self.bootstrap_timeouts.contains_key(&phase) {
                return Err(ConfigError::Invalid(
                    format!("Missing bootstrap timeout for phase {}", phase)
                ).into());
            }
        }
        
        // Check shutdown timeout
        if self.shutdown_timeout == 0 {
            return Err(ConfigError::Invalid("Shutdown timeout cannot be zero".to_string()).into());
        }
        
        // Check monitoring
        if self.monitoring.enabled && self.monitoring.token.is_none() {
            warn!("Monitoring is enabled but no token is specified");
        }
        
        // Check max threads
        if self.max_threads == 0 {
            return Err(ConfigError::Invalid("Max threads cannot be zero".to_string()).into());
        }
        
        Ok(())
    }
    
    /// Merge with another configuration
    pub fn merge(&mut self, other: RuntimeConfig) {
        // Merge plugin directory
        if !other.plugin_directory.is_empty() {
            self.plugin_directory = other.plugin_directory;
        }
        
        // Merge plugins
        for (name, config) in other.plugins {
            self.plugins.insert(name, config);
        }
        
        // Merge bootstrap timeouts
        for (phase, timeout) in other.bootstrap_timeouts {
            self.bootstrap_timeouts.insert(phase, timeout);
        }
        
        // Merge shutdown timeout
        if other.shutdown_timeout > 0 {
            self.shutdown_timeout = other.shutdown_timeout;
        }
        
        // Merge monitoring
        if other.monitoring.enabled {
            self.monitoring.enabled = true;
            self.monitoring.port = other.monitoring.port;
            self.monitoring.local_only = other.monitoring.local_only;
            
            if other.monitoring.token.is_some() {
                self.monitoring.token = other.monitoring.token;
            }
        }
        
        // Merge max threads
        if other.max_threads > 0 {
            self.max_threads = other.max_threads;
        }
        
        // Merge extra
        for (key, value) in other.extra {
            self.extra.insert(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_load_config() {
        // Create a temporary config file
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();
        
        // Write a test configuration
        let config_json = r#"
        {
            "plugin_directory": "/tmp/plugins",
            "shutdown_timeout": 60,
            "monitoring": {
                "enabled": true,
                "port": 8080,
                "token": "test-token"
            }
        }
        "#;
        
        fs::write(path, config_json).await.unwrap();
        
        // Load the configuration
        let config = RuntimeConfig::load(Some(path)).await.unwrap();
        
        // Verify loaded values
        assert_eq!(config.plugin_directory, "/tmp/plugins");
        assert_eq!(config.shutdown_timeout, 60);
        assert!(config.monitoring.enabled);
        assert_eq!(config.monitoring.port, 8080);
        assert_eq!(config.monitoring.token, Some("test-token".to_string()));
    }
    
    #[tokio::test]
    async fn test_default_config() {
        // Load the default configuration
        let config = RuntimeConfig::load(None).await.unwrap();
        
        // Verify default values
        assert_eq!(config.plugin_directory, "./plugins");
        assert_eq!(config.shutdown_timeout, 30);
        assert!(!config.monitoring.enabled);
        assert_eq!(config.max_threads, 4);
    }
    
    #[test]
    fn test_merge_config() {
        // Create a base configuration
        let mut base = RuntimeConfig::default();
        
        // Create an override configuration
        let mut override_config = RuntimeConfig::default();
        override_config.plugin_directory = "/override/plugins".to_string();
        override_config.shutdown_timeout = 60;
        override_config.monitoring.enabled = true;
        override_config.monitoring.token = Some("override-token".to_string());
        
        // Merge the configurations
        base.merge(override_config);
        
        // Verify merged values
        assert_eq!(base.plugin_directory, "/override/plugins");
        assert_eq!(base.shutdown_timeout, 60);
        assert!(base.monitoring.enabled);
        assert_eq!(base.monitoring.token, Some("override-token".to_string()));
    }
}