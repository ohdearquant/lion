//! Plugin Lifecycle Management
//!
//! Manages the lifecycle of plugins, including loading, initialization,
//! running, pausing, and stopping.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use lion_core::id::PluginId;
use lion_core::types::plugin::PluginState;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

// Replacement for the missing Capability struct
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: String,
    pub name: String,
    pub description: String,
}

// Replacement for the missing LifecycleManager
#[derive(Debug, Clone)]
pub struct LifecycleManager {
    state: Arc<Mutex<PluginState>>,
}

impl LifecycleManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(PluginState::Created)),
        }
    }
    
    pub async fn load_plugin(&self, _path: &str) -> Result<()> {
        *self.state.lock().unwrap() = PluginState::Ready;
        Ok(())
    }
    
    pub async fn initialize_plugin(&self, _config: serde_json::Value) -> Result<()> {
        *self.state.lock().unwrap() = PluginState::Ready;
        Ok(())
    }
    
    pub async fn start_plugin(&self) -> Result<()> {
        *self.state.lock().unwrap() = PluginState::Running;
        Ok(())
    }
    
    pub async fn pause_plugin(&self) -> Result<()> {
        *self.state.lock().unwrap() = PluginState::Paused;
        Ok(())
    }
    
    pub async fn stop_plugin(&self) -> Result<()> {
        *self.state.lock().unwrap() = PluginState::Terminated;
        Ok(())
    }
    
    pub async fn unload_plugin(&self) -> Result<()> {
        *self.state.lock().unwrap() = PluginState::Terminated;
        Ok(())
    }
    
    pub async fn call_plugin_function(&self, _function_name: &str, _params: serde_json::Value) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"result": "success"}))
    }
}

/// Errors that can occur during plugin lifecycle operations
#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),

    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    #[error("Plugin is in invalid state. Current: {current}, Expected: {expected}")]
    InvalidState {
        current: PluginState,
        expected: PluginState,
    },

    #[error("Plugin operation timed out")]
    Timeout,

    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),
}

/// Plugin metadata stored in the runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique identifier for the plugin
    pub id: PluginId,

    /// Display name of the plugin
    pub name: String,

    /// Version of the plugin
    pub version: String,

    /// Description of the plugin
    pub description: String,

    /// Author of the plugin
    pub author: String,

    /// Path to the plugin binary
    pub path: String,

    /// Current state of the plugin
    pub state: PluginState,

    /// Capabilities required by the plugin
    pub required_capabilities: Vec<String>,
}

/// Manages the lifecycle of a single plugin
pub struct PluginLifecycle {
    /// Plugin metadata
    metadata: RwLock<PluginMetadata>,

    /// Isolation manager for the plugin
    isolation_manager: Arc<LifecycleManager>,

    /// Capabilities granted to the plugin
    capabilities: Vec<Capability>,
}

impl PluginLifecycle {
    /// Create a new plugin lifecycle manager
    pub async fn new(
        metadata: PluginMetadata,
        isolation_manager: Arc<LifecycleManager>,
    ) -> Result<Self> {
        Ok(Self {
            metadata: RwLock::new(metadata),
            isolation_manager,
            capabilities: Vec::new(),
        })
    }

    /// Load the plugin
    pub async fn load(&self) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        // Check if the plugin is already loaded
        if metadata.state != PluginState::Created {
            return Err(LifecycleError::InvalidState {
                current: metadata.state,
                expected: PluginState::Created,
            }
            .into());
        }

        info!("Loading plugin: {}", metadata.name);

        // Load the plugin using the isolation manager
        self.isolation_manager
            .load_plugin(&metadata.path)
            .await
            .context(format!("Failed to load plugin: {}", metadata.name))?;

        // Update the state
        metadata.state = PluginState::Ready;
        info!("Plugin loaded: {}", metadata.name);

        Ok(())
    }

    /// Initialize the plugin
    pub async fn initialize(&self, config: serde_json::Value) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        // Check if the plugin is in the correct state
        if metadata.state != PluginState::Ready {
            return Err(LifecycleError::InvalidState {
                current: metadata.state,
                expected: PluginState::Ready,
            }
            .into());
        }

        info!("Initializing plugin: {}", metadata.name);

        // Initialize the plugin using the isolation manager
        self.isolation_manager
            .initialize_plugin(config)
            .await
            .context(format!("Failed to initialize plugin: {}", metadata.name))?;

        // Update the state
        metadata.state = PluginState::Ready;
        info!("Plugin initialized: {}", metadata.name);

        Ok(())
    }

    /// Start the plugin
    pub async fn start(&self) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        // Check if the plugin is in the correct state
        if metadata.state != PluginState::Ready && metadata.state != PluginState::Paused {
            return Err(LifecycleError::InvalidState {
                current: metadata.state,
                expected: PluginState::Ready,
            }
            .into());
        }

        info!("Starting plugin: {}", metadata.name);

        // Start the plugin using the isolation manager
        self.isolation_manager
            .start_plugin()
            .await
            .context(format!("Failed to start plugin: {}", metadata.name))?;

        // Update the state
        metadata.state = PluginState::Running;
        info!("Plugin started: {}", metadata.name);

        Ok(())
    }

    /// Pause the plugin
    pub async fn pause(&self) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        // Check if the plugin is in the correct state
        if metadata.state != PluginState::Running {
            return Err(LifecycleError::InvalidState {
                current: metadata.state,
                expected: PluginState::Running,
            }
            .into());
        }

        info!("Pausing plugin: {}", metadata.name);

        // Pause the plugin using the isolation manager
        self.isolation_manager
            .pause_plugin()
            .await
            .context(format!("Failed to pause plugin: {}", metadata.name))?;

        // Update the state
        metadata.state = PluginState::Paused;
        info!("Plugin paused: {}", metadata.name);

        Ok(())
    }

    /// Stop the plugin
    pub async fn stop(&self) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        // Check if the plugin is in a state that can be stopped
        if metadata.state != PluginState::Running
            && metadata.state != PluginState::Paused
            && metadata.state != PluginState::Ready
        {
            return Err(LifecycleError::InvalidState {
                current: metadata.state,
                expected: PluginState::Running,
            }
            .into());
        }

        info!("Stopping plugin: {}", metadata.name);

        // Stop the plugin using the isolation manager
        self.isolation_manager
            .stop_plugin()
            .await
            .context(format!("Failed to stop plugin: {}", metadata.name))?;

        // Update the state
        metadata.state = PluginState::Terminated;
        info!("Plugin stopped: {}", metadata.name);

        Ok(())
    }

    /// Unload the plugin
    pub async fn unload(&self) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        // Check if the plugin is in a state that can be unloaded
        if metadata.state != PluginState::Terminated && metadata.state != PluginState::Ready {
            return Err(LifecycleError::InvalidState {
                current: metadata.state,
                expected: PluginState::Terminated,
            }
            .into());
        }

        info!("Unloading plugin: {}", metadata.name);

        // Unload the plugin using the isolation manager
        self.isolation_manager
            .unload_plugin()
            .await
            .context(format!("Failed to unload plugin: {}", metadata.name))?;

        // Update the state
        metadata.state = PluginState::Terminated;
        info!("Plugin unloaded: {}", metadata.name);

        Ok(())
    }

    /// Call a function in the plugin
    pub async fn call_function(
        &self,
        function_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let metadata = self.metadata.read().await;

        // Check if the plugin is in the correct state
        if metadata.state != PluginState::Running {
            return Err(LifecycleError::InvalidState {
                current: metadata.state,
                expected: PluginState::Running,
            }
            .into());
        }

        debug!(
            "Calling function '{}' in plugin '{}'",
            function_name, metadata.name
        );

        // Call the function using the isolation manager
        let result = self
            .isolation_manager
            .call_plugin_function(function_name, params)
            .await
            .context(format!(
                "Failed to call function '{}' in plugin '{}'",
                function_name, metadata.name
            ))?;

        Ok(result)
    }

    /// Get the current state of the plugin
    pub async fn get_state(&self) -> PluginState {
        self.metadata.read().await.state
    }

    /// Get the metadata of the plugin
    pub async fn get_metadata(&self) -> PluginMetadata {
        self.metadata.read().await.clone()
    }

    /// Add a capability to the plugin
    pub async fn add_capability(&mut self, capability: Capability) {
        self.capabilities.push(capability);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    
    // Create a mock lifecycle manager for testing
    #[derive(Debug, Clone)]
    pub struct MockLifecycleManager {
        inner: LifecycleManager
    }
    
    impl MockLifecycleManager {
        pub fn new() -> Self {
            Self {
                inner: LifecycleManager::new()
            }
        }
    }
    
    impl std::ops::Deref for MockLifecycleManager {
        type Target = LifecycleManager;
        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        // Create a mock isolation manager
        let isolation_manager = Arc::new(MockLifecycleManager::new());

        // Create plugin metadata
        let metadata = PluginMetadata {
            id: PluginId::from_uuid(Uuid::new_v4()),
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            path: "/path/to/plugin".to_string(),
            state: PluginState::Created,
            required_capabilities: vec![],
        };

        // Create the plugin lifecycle
        let lifecycle = PluginLifecycle::new(metadata, isolation_manager)
            .await
            .unwrap();

        // Test the lifecycle
        lifecycle.load().await.unwrap();
        assert_eq!(lifecycle.get_state().await, PluginState::Ready);

        let config = serde_json::json!({});
        lifecycle.initialize(config).await.unwrap();
        assert_eq!(lifecycle.get_state().await, PluginState::Ready);

        lifecycle.start().await.unwrap();
        assert_eq!(lifecycle.get_state().await, PluginState::Running);

        lifecycle.pause().await.unwrap();
        assert_eq!(lifecycle.get_state().await, PluginState::Paused);

        lifecycle.start().await.unwrap();
        assert_eq!(lifecycle.get_state().await, PluginState::Running);

        lifecycle.stop().await.unwrap();
        assert_eq!(lifecycle.get_state().await, PluginState::Terminated);

        lifecycle.unload().await.unwrap();
        assert_eq!(lifecycle.get_state().await, PluginState::Terminated);
    }
}
