//! Workflow storage for persistence.
//!
//! This module provides storage for workflows and execution state,
//! supporting both in-memory and persistent backends.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use core::error::{Result, WorkflowError};
use crate::{Workflow, WorkflowId, ExecutionId};

/// Interface for workflow storage.
pub trait WorkflowStorage: Send + Sync {
    /// Save a workflow.
    fn save_workflow(&self, workflow: &Workflow) -> Result<()>;
    
    /// Get a workflow by ID.
    fn get_workflow(&self, id: &WorkflowId) -> Result<Workflow>;
    
    /// Delete a workflow.
    fn delete_workflow(&self, id: &WorkflowId) -> Result<()>;
    
    /// List all workflows.
    fn list_workflows(&self) -> Result<Vec<Workflow>>;
    
    /// Save a checkpoint of execution state.
    fn save_checkpoint(
        &self,
        execution_id: &ExecutionId,
        checkpoint_id: &str,
        state: &serde_json::Value,
    ) -> Result<()>;
    
    /// Get a checkpoint of execution state.
    fn get_checkpoint(
        &self,
        execution_id: &ExecutionId,
        checkpoint_id: &str,
    ) -> Result<serde_json::Value>;
    
    /// List checkpoints for an execution.
    fn list_checkpoints(&self, execution_id: &ExecutionId) -> Result<Vec<String>>;
    
    /// Delete a checkpoint.
    fn delete_checkpoint(
        &self,
        execution_id: &ExecutionId,
        checkpoint_id: &str,
    ) -> Result<()>;
    
    /// Clear all checkpoints for an execution.
    fn clear_checkpoints(&self, execution_id: &ExecutionId) -> Result<()>;
}

/// In-memory workflow storage.
pub struct MemoryWorkflowStorage {
    /// Stored workflows.
    workflows: RwLock<HashMap<WorkflowId, Workflow>>,
    
    /// Stored checkpoints.
    checkpoints: RwLock<HashMap<(ExecutionId, String), serde_json::Value>>,
}

impl MemoryWorkflowStorage {
    /// Create a new in-memory workflow storage.
    pub fn new() -> Self {
        Self {
            workflows: RwLock::new(HashMap::new()),
            checkpoints: RwLock::new(HashMap::new()),
        }
    }
}

impl WorkflowStorage for MemoryWorkflowStorage {
    fn save_workflow(&self, workflow: &Workflow) -> Result<()> {
        let mut workflows = self.workflows.write();
        workflows.insert(workflow.id.clone(), workflow.clone());
        Ok(())
    }
    
    fn get_workflow(&self, id: &WorkflowId) -> Result<Workflow> {
        let workflows = self.workflows.read();
        workflows.get(id)
            .cloned()
            .ok_or_else(|| WorkflowError::WorkflowNotFound(id.to_string()).into())
    }
    
    fn delete_workflow(&self, id: &WorkflowId) -> Result<()> {
        let mut workflows = self.workflows.write();
        workflows.remove(id);
        Ok(())
    }
    
    fn list_workflows(&self) -> Result<Vec<Workflow>> {
        let workflows = self.workflows.read();
        Ok(workflows.values().cloned().collect())
    }
    
    fn save_checkpoint(
        &self,
        execution_id: &ExecutionId,
        checkpoint_id: &str,
        state: &serde_json::Value,
    ) -> Result<()> {
        let mut checkpoints = self.checkpoints.write();
        checkpoints.insert((execution_id.clone(), checkpoint_id.to_string()), state.clone());
        Ok(())
    }
    
    fn get_checkpoint(
        &self,
        execution_id: &ExecutionId,
        checkpoint_id: &str,
    ) -> Result<serde_json::Value> {
        let checkpoints = self.checkpoints.read();
        checkpoints.get(&(execution_id.clone(), checkpoint_id.to_string()))
            .cloned()
            .ok_or_else(|| WorkflowError::InvalidDefinition(
                format!("Checkpoint {} not found for execution {}", checkpoint_id, execution_id)
            ).into())
    }
    
    fn list_checkpoints(&self, execution_id: &ExecutionId) -> Result<Vec<String>> {
        let checkpoints = self.checkpoints.read();
        let mut result = Vec::new();
        
        for (id, _) in checkpoints.keys() {
            if id == execution_id {
                result.push(id.1.clone());
            }
        }
        
        Ok(result)
    }
    
    fn delete_checkpoint(
        &self,
        execution_id: &ExecutionId,
        checkpoint_id: &str,
    ) -> Result<()> {
        let mut checkpoints = self.checkpoints.write();
        checkpoints.remove(&(execution_id.clone(), checkpoint_id.to_string()));
        Ok(())
    }
    
    fn clear_checkpoints(&self, execution_id: &ExecutionId) -> Result<()> {
        let mut checkpoints = self.checkpoints.write();
        checkpoints.retain(|k, _| k.0 != *execution_id);
        Ok(())
    }
}

/// Persistent workflow storage using sled.
#[cfg(feature = "persistence")]
pub struct SledWorkflowStorage {
    /// Database instance.
    db: sled::Db,
    
    /// Workflows tree.
    workflows: sled::Tree,
    
    /// Checkpoints tree.
    checkpoints: sled::Tree,
}

#[cfg(feature = "persistence")]
impl SledWorkflowStorage {
    /// Create a new persistent workflow storage.
    pub fn new(path: &std::path::Path) -> Result<Self> {
        // Open the database
        let db = sled::open(path)
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to open database: {}", e)
            ))?;
        
        // Open trees
        let workflows = db.open_tree("workflows")
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to open workflows tree: {}", e)
            ))?;
        
        let checkpoints = db.open_tree("checkpoints")
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to open checkpoints tree: {}", e)
            ))?;
        
        Ok(Self {
            db,
            workflows,
            checkpoints,
        })
    }
    
    /// Convert a workflow ID to a key.
    fn workflow_key(id: &WorkflowId) -> Vec<u8> {
        format!("workflow:{}", id).into_bytes()
    }
    
    /// Convert a checkpoint ID to a key.
    fn checkpoint_key(execution_id: &ExecutionId, checkpoint_id: &str) -> Vec<u8> {
        format!("checkpoint:{}:{}", execution_id, checkpoint_id).into_bytes()
    }
}

#[cfg(feature = "persistence")]
impl WorkflowStorage for SledWorkflowStorage {
    fn save_workflow(&self, workflow: &Workflow) -> Result<()> {
        // Serialize the workflow
        let data = serde_json::to_vec(workflow)
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to serialize workflow: {}", e)
            ))?;
        
        // Save to the database
        self.workflows.insert(Self::workflow_key(&workflow.id), data)
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to save workflow: {}", e)
            ))?;
        
        Ok(())
    }
    
    fn get_workflow(&self, id: &WorkflowId) -> Result<Workflow> {
        // Get from the database
        let data = self.workflows.get(Self::workflow_key(id))
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to get workflow: {}", e)
            ))?
            .ok_or_else(|| WorkflowError::WorkflowNotFound(id.to_string()))?;
        
        // Deserialize the workflow
        let workflow = serde_json::from_slice(&data)
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to deserialize workflow: {}", e)
            ))?;
        
        Ok(workflow)
    }
    
    fn delete_workflow(&self, id: &WorkflowId) -> Result<()> {
        // Delete from the database
        self.workflows.remove(Self::workflow_key(id))
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to delete workflow: {}", e)
            ))?;
        
        Ok(())
    }
    
    fn list_workflows(&self) -> Result<Vec<Workflow>> {
        let mut workflows = Vec::new();
        
        // Iterate over all workflows
        for item in self.workflows.iter() {
            let (_, data) = item.map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to iterate workflows: {}", e)
            ))?;
            
            // Deserialize the workflow
            let workflow = serde_json::from_slice(&data)
                .map_err(|e| WorkflowError::PersistenceError(
                    format!("Failed to deserialize workflow: {}", e)
                ))?;
            
            workflows.push(workflow);
        }
        
        Ok(workflows)
    }
    
    fn save_checkpoint(
        &self,
        execution_id: &ExecutionId,
        checkpoint_id: &str,
        state: &serde_json::Value,
    ) -> Result<()> {
        // Serialize the state
        let data = serde_json::to_vec(state)
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to serialize checkpoint: {}", e)
            ))?;
        
        // Save to the database
        self.checkpoints.insert(Self::checkpoint_key(execution_id, checkpoint_id), data)
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to save checkpoint: {}", e)
            ))?;
        
        Ok(())
    }
    
    fn get_checkpoint(
        &self,
        execution_id: &ExecutionId,
        checkpoint_id: &str,
    ) -> Result<serde_json::Value> {
        // Get from the database
        let data = self.checkpoints.get(Self::checkpoint_key(execution_id, checkpoint_id))
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to get checkpoint: {}", e)
            ))?
            .ok_or_else(|| WorkflowError::InvalidDefinition(
                format!("Checkpoint {} not found for execution {}", checkpoint_id, execution_id)
            ))?;
        
        // Deserialize the state
        let state = serde_json::from_slice(&data)
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to deserialize checkpoint: {}", e)
            ))?;
        
        Ok(state)
    }
    
    fn list_checkpoints(&self, execution_id: &ExecutionId) -> Result<Vec<String>> {
        let prefix = format!("checkpoint:{}:", execution_id).into_bytes();
        let mut checkpoints = Vec::new();
        
        // Iterate over checkpoints with the prefix
        for item in self.checkpoints.scan_prefix(&prefix) {
            let (key, _) = item.map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to iterate checkpoints: {}", e)
            ))?;
            
            // Extract the checkpoint ID
            let key_str = String::from_utf8_lossy(&key);
            let parts: Vec<&str> = key_str.split(':').collect();
            if parts.len() >= 3 {
                checkpoints.push(parts[2].to_string());
            }
        }
        
        Ok(checkpoints)
    }
    
    fn delete_checkpoint(
        &self,
        execution_id: &ExecutionId,
        checkpoint_id: &str,
    ) -> Result<()> {
        // Delete from the database
        self.checkpoints.remove(Self::checkpoint_key(execution_id, checkpoint_id))
            .map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to delete checkpoint: {}", e)
            ))?;
        
        Ok(())
    }
    
    fn clear_checkpoints(&self, execution_id: &ExecutionId) -> Result<()> {
        let prefix = format!("checkpoint:{}:", execution_id).into_bytes();
        
        // Iterate over checkpoints with the prefix
        for item in self.checkpoints.scan_prefix(&prefix) {
            let (key, _) = item.map_err(|e| WorkflowError::PersistenceError(
                format!("Failed to iterate checkpoints: {}", e)
            ))?;
            
            // Delete the checkpoint
            self.checkpoints.remove(&key)
                .map_err(|e| WorkflowError::PersistenceError(
                    format!("Failed to delete checkpoint: {}", e)
                ))?;
        }
        
        Ok(())
    }
}