//! Lion Runtime - Core runtime for Lion microkernel architecture
//!
//! This crate provides the main runtime components for the Lion system,
//! including capability management, plugin lifecycle, workflow execution,
//! and system bootstrap/shutdown.

pub mod capabilities;
pub mod plugin;
pub mod system;
pub mod workflow;

use std::sync::Arc;

use anyhow::Result;
use lion_capability::model::CapabilityId;
use lion_core::id::WorkflowId;
use tracing::info;

/// Runtime facade that provides a unified interface to the Lion runtime.
pub struct Runtime {
    /// Capability manager for handling capability grants, checks, and revocation
    pub capabilities: Arc<capabilities::manager::CapabilityManager>,

    /// Plugin manager for loading, unloading, and managing plugins
    pub plugins: Arc<plugin::manager::PluginManager>,

    /// Workflow manager for managing workflow execution
    pub workflows: Arc<workflow::manager::WorkflowManager>,

    /// System component for bootstrap and shutdown
    pub system: Arc<system::bootstrap::System>,
}

impl Runtime {
    /// Create a new Runtime instance
    pub async fn new(config_path: Option<&str>) -> Result<Self> {
        info!("Initializing Lion Runtime");

        // Load configuration from the provided path or defaults
        let config = system::config::RuntimeConfig::load(config_path).await?;

        // Initialize the system component
        let system = Arc::new(system::bootstrap::System::new(config.clone())?);

        // Initialize the capability manager
        let capabilities = Arc::new(capabilities::manager::CapabilityManager::new()?);

        // Initialize the plugin manager with capability manager
        let plugins = Arc::new(plugin::manager::PluginManager::new(
            config.clone(),
            capabilities.clone(),
        )?);

        // Initialize the workflow manager
        let workflows = Arc::new(workflow::manager::WorkflowManager::new(
            config.clone(),
            capabilities.clone(),
            plugins.clone(),
        )?);

        info!("Lion Runtime initialized successfully");

        Ok(Self {
            capabilities,
            plugins,
            workflows,
            system,
        })
    }

    /// Start the runtime
    pub async fn start(&self) -> Result<()> {
        info!("Starting Lion Runtime");

        // Bootstrap the system
        self.system.bootstrap().await?;

        // Start the plugin manager
        self.plugins.start().await?;

        // Start the workflow manager
        self.workflows.start().await?;

        info!("Lion Runtime started successfully");

        Ok(())
    }

    /// Gracefully shut down the runtime
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Lion Runtime");

        // Use the shutdown manager
        self.system.shutdown().await?;

        info!("Lion Runtime shut down successfully");

        Ok(())
    }

    /// Grant a capability to a subject
    pub async fn grant_capability(
        &self,
        subject_id: String,
        object_id: String,
        rights: Vec<String>,
    ) -> Result<CapabilityId> {
        self.capabilities
            .grant_capability(subject_id, object_id, rights)
            .await
    }

    /// Revoke a capability
    pub async fn revoke_capability(&self, capability_id: CapabilityId) -> Result<()> {
        self.capabilities.revoke_capability(capability_id).await
    }

    /// Start a workflow with the given ID and input
    pub async fn start_workflow(
        &self,
        workflow_id: WorkflowId,
        input: serde_json::Value,
    ) -> Result<()> {
        self.workflows.start_workflow(workflow_id, input).await
    }
}
