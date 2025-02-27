//! Workflow Manager for Lion Runtime
//!
//! Manages the creation, execution, and monitoring of workflows.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use lion_capability::model::Capability;
use lion_core::id::{PluginId, WorkflowId};
use lion_core::traits::workflow::WorkflowStatus;
use lion_workflow::model::definition::WorkflowDefinition;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::execution::WorkflowExecutor;
use crate::capabilities::manager::CapabilityManager;
use crate::plugin::manager::PluginManager;
use crate::system::config::RuntimeConfig;

/// Errors that can occur in workflow manager operations
#[derive(thiserror::Error, Debug)]
pub enum WorkflowManagerError {
    #[error("Workflow {0} not found")]
    NotFound(WorkflowId),
    
    #[error("Workflow {0} already exists")]
    AlreadyExists(WorkflowId),
    
    #[error("Plugin {0} not found")]
    PluginNotFound(PluginId),
    
    #[error("Invalid workflow definition: {0}")]
    InvalidDefinition(String),
}

/// Workflow manager for creating and managing workflows
pub struct WorkflowManager {
    /// Map of workflow IDs to definitions
    workflows: RwLock<HashMap<WorkflowId, WorkflowDefinition>>,
    
    /// Workflow executor
    executor: WorkflowExecutor,
    
    /// Capability manager
    capability_manager: Arc<CapabilityManager>,
    
    /// Plugin manager
    plugin_manager: Arc<PluginManager>,
    
    /// Runtime configuration
    config: RuntimeConfig,
}

impl WorkflowManager {
    /// Create a new workflow manager
    pub fn new(
        config: RuntimeConfig,
        capability_manager: Arc<CapabilityManager>,
        plugin_manager: Arc<PluginManager>,
    ) -> Result<Self> {
        // Create the workflow executor
        let executor = WorkflowExecutor::new(
            capability_manager.clone(),
            plugin_manager.clone(),
        );
        
        Ok(Self {
            workflows: RwLock::new(HashMap::new()),
            executor,
            capability_manager,
            plugin_manager,
            config,
        })
    }
    
    /// Start the workflow manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting workflow manager");
        
        // TODO: Load persisted workflows if needed
        
        Ok(())
    }
    
    /// Register a workflow
    pub async fn register_workflow(&self, definition: WorkflowDefinition) -> Result<WorkflowId> {
        info!("Registering workflow: {}", definition.name);
        
        // Validate the workflow
        self.validate_workflow(&definition).await?;
        
        // Generate an ID if not provided
        let workflow_id = if definition.id.0.is_empty() {
            WorkflowId(Uuid::new_v4().to_string())
        } else {
            definition.id.clone()
        };
        
        // Check if workflow already exists
        let mut workflows = self.workflows.write().await;
        if workflows.contains_key(&workflow_id) {
            return Err(WorkflowManagerError::AlreadyExists(workflow_id).into());
        }
        
        // Store the workflow
        let mut new_def = definition.clone();
        new_def.id = workflow_id.clone();
        workflows.insert(workflow_id.clone(), new_def);
        
        Ok(workflow_id)
    }
    
    /// Validate a workflow definition
    async fn validate_workflow(&self, definition: &WorkflowDefinition) -> Result<()> {
        // Check for empty nodes
        if definition.nodes.is_empty() {
            return Err(WorkflowManagerError::InvalidDefinition(
                "Workflow must have at least one node".to_string()
            ).into());
        }
        
        // Check for duplicate node IDs
        let mut node_ids = Vec::new();
        for node in &definition.nodes {
            if node_ids.contains(&node.id) {
                return Err(WorkflowManagerError::InvalidDefinition(
                    format!("Duplicate node ID: {}", node.id)
                ).into());
            }
            node_ids.push(node.id.clone());
        }
        
        // Check that all dependencies exist
        for node in &definition.nodes {
            for dep in &node.dependencies {
                if !node_ids.contains(dep) {
                    return Err(WorkflowManagerError::InvalidDefinition(
                        format!("Dependency {} not found for node {}", dep, node.id)
                    ).into());
                }
            }
        }
        
        // Check that all referenced plugins exist
        for node in &definition.nodes {
            if let Some(plugin_id) = &node.plugin_id {
                // Verify plugin exists
                if let Err(e) = self.plugin_manager.get_plugin(plugin_id).await {
                    return Err(WorkflowManagerError::PluginNotFound(plugin_id.clone()).into());
                }
            }
        }
        
        // TODO: More validation as needed
        
        Ok(())
    }
    
    /// Start a workflow
    pub async fn start_workflow(
        &self,
        workflow_id: WorkflowId,
        input: serde_json::Value,
    ) -> Result<()> {
        info!("Starting workflow: {:?}", workflow_id);
        
        // Get the workflow definition
        let definition = {
            let workflows = self.workflows.read().await;
            workflows.get(&workflow_id)
                .cloned()
                .ok_or_else(|| WorkflowManagerError::NotFound(workflow_id.clone()))?
        };
        
        // Start the workflow
        self.executor.start_workflow(workflow_id, definition, input).await?;
        
        Ok(())
    }
    
    /// Pause a workflow
    pub async fn pause_workflow(&self, workflow_id: WorkflowId) -> Result<()> {
        info!("Pausing workflow: {:?}", workflow_id);
        
        // Verify workflow exists
        {
            let workflows = self.workflows.read().await;
            if !workflows.contains_key(&workflow_id) {
                return Err(WorkflowManagerError::NotFound(workflow_id).into());
            }
        }
        
        // Pause the workflow
        self.executor.pause_workflow(workflow_id).await?;
        
        Ok(())
    }
    
    /// Resume a workflow
    pub async fn resume_workflow(&self, workflow_id: WorkflowId) -> Result<()> {
        info!("Resuming workflow: {:?}", workflow_id);
        
        // Verify workflow exists
        {
            let workflows = self.workflows.read().await;
            if !workflows.contains_key(&workflow_id) {
                return Err(WorkflowManagerError::NotFound(workflow_id).into());
            }
        }
        
        // Resume the workflow
        self.executor.resume_workflow(workflow_id).await?;
        
        Ok(())
    }
    
    /// Cancel a workflow
    pub async fn cancel_workflow(&self, workflow_id: WorkflowId) -> Result<()> {
        info!("Cancelling workflow: {:?}", workflow_id);
        
        // Verify workflow exists
        {
            let workflows = self.workflows.read().await;
            if !workflows.contains_key(&workflow_id) {
                return Err(WorkflowManagerError::NotFound(workflow_id).into());
            }
        }
        
        // Cancel the workflow
        self.executor.cancel_workflow(workflow_id).await?;
        
        Ok(())
    }
    
    /// Get workflow status
    pub async fn get_workflow_status(&self, workflow_id: &WorkflowId) -> Result<WorkflowStatus> {
        // Verify workflow exists
        {
            let workflows = self.workflows.read().await;
            if !workflows.contains_key(workflow_id) {
                return Err(WorkflowManagerError::NotFound(workflow_id.clone()).into());
            }
        }
        
        // Get status
        self.executor.get_workflow_status(workflow_id).await
    }
    
    /// Get workflow results
    pub async fn get_workflow_results(&self, workflow_id: &WorkflowId) -> Result<serde_json::Value> {
        // Verify workflow exists
        {
            let workflows = self.workflows.read().await;
            if !workflows.contains_key(workflow_id) {
                return Err(WorkflowManagerError::NotFound(workflow_id.clone()).into());
            }
        }
        
        // Get results
        self.executor.get_workflow_results(workflow_id).await
    }
    
    /// Get a registered workflow
    pub async fn get_workflow(&self, workflow_id: &WorkflowId) -> Result<WorkflowDefinition> {
        let workflows = self.workflows.read().await;
        
        workflows.get(workflow_id)
            .cloned()
            .ok_or_else(|| WorkflowManagerError::NotFound(workflow_id.clone()).into())
    }
    
    /// Get all registered workflows
    pub async fn get_workflows(&self) -> Vec<WorkflowDefinition> {
        let workflows = self.workflows.read().await;
        workflows.values().cloned().collect()
    }
    
    /// Update a workflow
    pub async fn update_workflow(&self, definition: WorkflowDefinition) -> Result<()> {
        info!("Updating workflow: {}", definition.name);
        
        // Validate the workflow
        self.validate_workflow(&definition).await?;
        
        // Check if workflow exists
        let mut workflows = self.workflows.write().await;
        if !workflows.contains_key(&definition.id) {
            return Err(WorkflowManagerError::NotFound(definition.id.clone()).into());
        }
        
        // Update the workflow
        workflows.insert(definition.id.clone(), definition);
        
        Ok(())
    }
    
    /// Delete a workflow
    pub async fn delete_workflow(&self, workflow_id: &WorkflowId) -> Result<()> {
        info!("Deleting workflow: {:?}", workflow_id);
        
        // Check if workflow exists
        let mut workflows = self.workflows.write().await;
        if !workflows.contains_key(workflow_id) {
            return Err(WorkflowManagerError::NotFound(workflow_id.clone()).into());
        }
        
        // Remove the workflow
        workflows.remove(workflow_id);
        
        Ok(())
    }
    
    /// Shutdown all workflows
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down all workflows");
        
        // Get all workflow IDs
        let workflow_ids: Vec<WorkflowId> = {
            let workflows = self.workflows.read().await;
            workflows.keys().cloned().collect()
        };
        
        // Cancel each workflow
        for workflow_id in workflow_ids {
            if let Err(e) = self.cancel_workflow(workflow_id.clone()).await {
                warn!("Failed to cancel workflow {:?}: {}", workflow_id, e);
                // Continue with next workflow
            }
        }
        
        info!("All workflows shut down");
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lion_workflow::model::node::Node;
    
    #[tokio::test]
    async fn test_register_workflow() {
        // Create a capability manager
        let capability_manager = Arc::new(CapabilityManager::new().unwrap());
        
        // Create a runtime config
        let config = RuntimeConfig::default();
        
        // Create a plugin manager
        let plugin_manager = Arc::new(PluginManager::new(
            config.clone(),
            capability_manager.clone(),
        ).unwrap());
        
        // Create a workflow manager
        let manager = WorkflowManager::new(
            config,
            capability_manager,
            plugin_manager,
        ).unwrap();
        
        // Create a workflow definition
        let definition = WorkflowDefinition {
            id: WorkflowId(Uuid::new_v4().to_string()),
            name: "Test Workflow".to_string(),
            description: "A test workflow".to_string(),
            nodes: vec![
                Node {
                    id: "1".to_string(),
                    name: "Node 1".to_string(),
                    plugin_id: None,
                    function: "func1".to_string(),
                    dependencies: vec![],
                },
                Node {
                    id: "2".to_string(),
                    name: "Node 2".to_string(),
                    plugin_id: None,
                    function: "func2".to_string(),
                    dependencies: vec!["1".to_string()],
                },
            ],
        };
        
        // Register the workflow
        let workflow_id = manager.register_workflow(definition.clone()).await.unwrap();
        
        // Verify the workflow was registered
        let retrieved = manager.get_workflow(&workflow_id).await.unwrap();
        assert_eq!(retrieved.name, "Test Workflow");
        assert_eq!(retrieved.nodes.len(), 2);
    }
    
    #[tokio::test]
    async fn test_invalid_workflow() {
        // Create a capability manager
        let capability_manager = Arc::new(CapabilityManager::new().unwrap());
        
        // Create a runtime config
        let config = RuntimeConfig::default();
        
        // Create a plugin manager
        let plugin_manager = Arc::new(PluginManager::new(
            config.clone(),
            capability_manager.clone(),
        ).unwrap());
        
        // Create a workflow manager
        let manager = WorkflowManager::new(
            config,
            capability_manager,
            plugin_manager,
        ).unwrap();
        
        // Create an invalid workflow with no nodes
        let invalid = WorkflowDefinition {
            id: WorkflowId(Uuid::new_v4().to_string()),
            name: "Invalid Workflow".to_string(),
            description: "An invalid workflow".to_string(),
            nodes: vec![],
        };
        
        // Try to register the workflow
        let result = manager.register_workflow(invalid).await;
        assert!(result.is_err());
        
        // Create an invalid workflow with missing dependency
        let invalid = WorkflowDefinition {
            id: WorkflowId(Uuid::new_v4().to_string()),
            name: "Invalid Workflow".to_string(),
            description: "An invalid workflow".to_string(),
            nodes: vec![
                Node {
                    id: "1".to_string(),
                    name: "Node 1".to_string(),
                    plugin_id: None,
                    function: "func1".to_string(),
                    dependencies: vec!["missing".to_string()],
                },
            ],
        };
        
        // Try to register the workflow
        let result = manager.register_workflow(invalid).await;
        assert!(result.is_err());
    }
}