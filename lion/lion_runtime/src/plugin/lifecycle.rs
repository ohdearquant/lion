//! Plugin Lifecycle Management
//!
//! This module handles the lifecycle of plugins, wrapping the isolation manager
//! and providing a unified interface for plugin operations.

use std::sync::Arc;

use anyhow::Result;
use lion_core::id::PluginId;
use lion_core::types::plugin::PluginState;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin ID
    pub id: PluginId,

    /// Human-readable name
    pub name: String,

    /// Version string
    pub version: String,

    /// Description of the plugin
    pub description: String,

    /// Author information
    pub author: String,

    /// Path to the plugin file
    pub path: String,

    /// Current state
    pub state: PluginState,

    /// Required capabilities for this plugin
    pub required_capabilities: Vec<String>,
}

/// Manager for plugin lifecycle operations
pub struct LifecycleManager {
    /// Path to the plugin file
    path: String,

    /// Current state
    state: RwLock<PluginState>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(path: &str) -> Result<Self> {
        Ok(Self {
            path: path.to_string(),
            state: RwLock::new(PluginState::Created),
        })
    }

    /// Get the current state
    pub async fn get_state(&self) -> PluginState {
        *self.state.read().await
    }

    /// Set the state
    pub async fn set_state(&self, new_state: PluginState) -> Result<()> {
        let mut state = self.state.write().await;
        *state = new_state;
        Ok(())
    }

    /// Load the plugin
    pub async fn load(&self) -> Result<()> {
        debug!("Loading plugin from path: {}", self.path);

        // Set state to Loaded
        self.set_state(PluginState::Ready).await?;

        Ok(())
    }

    /// Initialize the plugin with configuration
    pub async fn initialize(&self, config: serde_json::Value) -> Result<()> {
        debug!("Initializing plugin with config: {:?}", config);

        // For now, simply transition to Ready state
        self.set_state(PluginState::Ready).await?;

        Ok(())
    }

    /// Start the plugin
    pub async fn start(&self) -> Result<()> {
        debug!("Starting plugin");

        // Set state to Running
        self.set_state(PluginState::Running).await?;

        Ok(())
    }

    /// Pause the plugin
    pub async fn pause(&self) -> Result<()> {
        debug!("Pausing plugin");

        // Set state to Paused
        self.set_state(PluginState::Paused).await?;

        Ok(())
    }

    /// Stop the plugin
    pub async fn stop(&self) -> Result<()> {
        debug!("Stopping plugin");

        // Set state to Ready
        self.set_state(PluginState::Ready).await?;

        Ok(())
    }

    /// Unload the plugin
    pub async fn unload(&self) -> Result<()> {
        debug!("Unloading plugin");

        // Set state to Terminated
        self.set_state(PluginState::Terminated).await?;

        Ok(())
    }

    /// Call a function in the plugin
    pub async fn call_function(
        &self,
        function_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        debug!(
            "Calling function '{}' with params: {:?}",
            function_name, params
        );

        // In a real implementation, this would call the function in the plugin
        // For now, just return a mock response
        Ok(serde_json::json!({
            "result": "success",
            "function": function_name,
        }))
    }
}

/// Plugin lifecycle manager
pub struct PluginLifecycle {
    /// Plugin metadata
    metadata: RwLock<PluginMetadata>,

    /// Isolation manager
    isolation_manager: Arc<LifecycleManager>,
}

impl PluginLifecycle {
    /// Create a new plugin lifecycle
    pub async fn new(
        metadata: PluginMetadata,
        isolation_manager: Arc<LifecycleManager>,
    ) -> Result<Self> {
        Ok(Self {
            metadata: RwLock::new(metadata),
            isolation_manager,
        })
    }

    /// Get the current plugin metadata
    pub async fn get_metadata(&self) -> PluginMetadata {
        self.metadata.read().await.clone()
    }

    /// Get the current plugin state
    pub async fn get_state(&self) -> PluginState {
        self.isolation_manager.get_state().await
    }

    /// Load the plugin
    pub async fn load(&self) -> Result<()> {
        info!("Loading plugin: {}", self.metadata.read().await.name);

        // Load using the isolation manager
        self.isolation_manager.load().await?;

        // Update metadata
        let mut metadata = self.metadata.write().await;
        metadata.state = self.isolation_manager.get_state().await;

        Ok(())
    }

    /// Initialize the plugin
    pub async fn initialize(&self, config: serde_json::Value) -> Result<()> {
        info!("Initializing plugin: {}", self.metadata.read().await.name);

        // Initialize using the isolation manager
        self.isolation_manager.initialize(config).await?;

        // Update metadata
        let mut metadata = self.metadata.write().await;
        metadata.state = self.isolation_manager.get_state().await;

        Ok(())
    }

    /// Start the plugin
    pub async fn start(&self) -> Result<()> {
        info!("Starting plugin: {}", self.metadata.read().await.name);

        // Start using the isolation manager
        self.isolation_manager.start().await?;

        // Update metadata
        let mut metadata = self.metadata.write().await;
        metadata.state = self.isolation_manager.get_state().await;

        Ok(())
    }

    /// Pause the plugin
    pub async fn pause(&self) -> Result<()> {
        info!("Pausing plugin: {}", self.metadata.read().await.name);

        // Pause using the isolation manager
        self.isolation_manager.pause().await?;

        // Update metadata
        let mut metadata = self.metadata.write().await;
        metadata.state = self.isolation_manager.get_state().await;

        Ok(())
    }

    /// Stop the plugin
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping plugin: {}", self.metadata.read().await.name);

        // Stop using the isolation manager
        self.isolation_manager.stop().await?;

        // Update metadata
        let mut metadata = self.metadata.write().await;
        metadata.state = self.isolation_manager.get_state().await;

        Ok(())
    }

    /// Unload the plugin
    pub async fn unload(&self) -> Result<()> {
        info!("Unloading plugin: {}", self.metadata.read().await.name);

        // Unload using the isolation manager
        self.isolation_manager.unload().await?;

        // Update metadata
        let mut metadata = self.metadata.write().await;
        metadata.state = self.isolation_manager.get_state().await;

        Ok(())
    }

    /// Call a function in the plugin
    pub async fn call_function(
        &self,
        function_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        debug!(
            "Calling function '{}' in plugin '{}'",
            function_name,
            self.metadata.read().await.name
        );

        // Call using the isolation manager
        self.isolation_manager
            .call_function(function_name, params)
            .await
    }
}
