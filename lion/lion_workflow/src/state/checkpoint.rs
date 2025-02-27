use crate::model::{WorkflowDefinition, WorkflowId, WorkflowError};
use crate::state::storage::StorageBackend;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;

/// Checkpoint error types
#[derive(Error, Debug)]
pub enum CheckpointError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Workflow error: {0}")]
    WorkflowError(#[from] WorkflowError),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Checkpoint not found: {0}")]
    NotFound(String),
    
    #[error("Checkpoint validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Schema version mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: String, found: String },
    
    #[error("Checkpoint in progress")]
    CheckpointInProgress,
}

/// Checkpoint metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Unique identifier
    pub id: String,
    
    /// Associated workflow ID
    pub workflow_id: WorkflowId,
    
    /// Checkpoint version
    pub version: String,
    
    /// Timestamp when this checkpoint was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Checkpoint size in bytes
    pub size: usize,
    
    /// Checksum (SHA-256) of the checkpoint data
    pub checksum: String,
    
    /// Custom metadata for this checkpoint
    pub custom_metadata: serde_json::Value,
}

/// Manager for persisting and restoring workflow state
#[derive(Clone)]
pub struct CheckpointManager<S: StorageBackend> {
    /// Storage backend
    storage: Arc<S>,
    
    /// Base directory for checkpoint files (if using file-based storage)
    base_dir: Option<PathBuf>,
    
    /// Current schema version
    schema_version: String,
    
    /// Lock to ensure only one checkpoint operation happens at a time per workflow
    locks: Arc<tokio::sync::Mutex<std::collections::HashMap<WorkflowId, Arc<Mutex<()>>>>>,
}

impl<S: StorageBackend> CheckpointManager<S> {
    /// Create a new checkpoint manager with the given storage backend
    pub fn new(storage: S, schema_version: &str) -> Self {
        CheckpointManager {
            storage: Arc::new(storage),
            base_dir: None,
            schema_version: schema_version.to_string(),
            locks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
    
    /// Create a new file-based checkpoint manager
    pub fn with_file_storage(base_dir: PathBuf, schema_version: &str) -> Result<Self, CheckpointError> {
        // Ensure the base directory exists
        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)?;
        }
        
        let mut manager = Self::new(crate::state::storage::FileStorage::new(base_dir.clone()), schema_version);
        manager.base_dir = Some(base_dir);
        
        Ok(manager)
    }
    
    /// Save a workflow state to a checkpoint
    pub async fn save_checkpoint(&self, workflow: &WorkflowDefinition) -> Result<String, CheckpointError> {
        // Get or create a lock for this workflow
        let mut locks = self.locks.lock().await;
        let lock = locks
            .entry(workflow.id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        
        // Drop the locks map lock
        drop(locks);
        
        // Acquire the specific workflow lock
        let _guard = match lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => return Err(CheckpointError::CheckpointInProgress),
        };
        
        // Generate a unique checkpoint ID
        let checkpoint_id = format!("{}-{}-{}", 
            workflow.id,
            chrono::Utc::now().timestamp_millis(),
            uuid::Uuid::new_v4().as_simple()
        );
        
        // Serialize the workflow
        let checkpoint_data = serde_json::to_vec(workflow)
            .map_err(|e| CheckpointError::SerializationError(e))?;
        
        // Calculate checksum
        let checksum = calculate_sha256(&checkpoint_data);
        
        // Create metadata
        let metadata = CheckpointMetadata {
            id: checkpoint_id.clone(),
            workflow_id: workflow.id,
            version: self.schema_version.clone(),
            created_at: chrono::Utc::now(),
            size: checkpoint_data.len(),
            checksum,
            custom_metadata: serde_json::Value::Null,
        };
        
        // Save to storage - first save to a temporary file
        let temp_key = format!("{}.tmp", checkpoint_id);
        self.storage.store(&temp_key, &checkpoint_data).await
            .map_err(|e| CheckpointError::StorageError(format!("Failed to store checkpoint: {}", e)))?;
        
        // Store metadata
        let metadata_key = format!("{}.meta", checkpoint_id);
        let metadata_data = serde_json::to_vec(&metadata)
            .map_err(|e| CheckpointError::SerializationError(e))?;
        
        self.storage.store(&metadata_key, &metadata_data).await
            .map_err(|e| CheckpointError::StorageError(format!("Failed to store metadata: {}", e)))?;
        
        // Atomically rename temporary file to final name
        self.storage.rename(&temp_key, &checkpoint_id).await
            .map_err(|e| CheckpointError::StorageError(format!("Failed to finalize checkpoint: {}", e)))?;
        
        // Return the checkpoint ID
        Ok(checkpoint_id)
    }
    
    /// Load the most recent checkpoint for a workflow
    pub async fn load_latest_checkpoint(&self, workflow_id: &WorkflowId) -> Result<WorkflowDefinition, CheckpointError> {
        // Find all checkpoints for this workflow
        let checkpoints = self.list_checkpoints(workflow_id).await?;
        
        if checkpoints.is_empty() {
            return Err(CheckpointError::NotFound(workflow_id.to_string()));
        }
        
        // Sort by creation time (newest first)
        let mut checkpoints = checkpoints;
        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Load the newest checkpoint
        self.load_checkpoint(&checkpoints[0].id).await
    }
    
    /// Load a specific checkpoint by ID
    pub async fn load_checkpoint(&self, checkpoint_id: &str) -> Result<WorkflowDefinition, CheckpointError> {
        // Load metadata first to check version
        let metadata_key = format!("{}.meta", checkpoint_id);
        let metadata_data = self.storage.load(&metadata_key).await
            .map_err(|e| CheckpointError::StorageError(format!("Failed to load metadata: {}", e)))?;
        
        let metadata: CheckpointMetadata = serde_json::from_slice(&metadata_data)
            .map_err(|e| CheckpointError::SerializationError(e))?;
        
        // Check schema version
        if metadata.version != self.schema_version {
            return Err(CheckpointError::SchemaVersionMismatch {
                expected: self.schema_version.clone(),
                found: metadata.version,
            });
        }
        
        // Load the checkpoint data
        let checkpoint_data = self.storage.load(checkpoint_id).await
            .map_err(|e| CheckpointError::StorageError(format!("Failed to load checkpoint: {}", e)))?;
        
        // Verify checksum
        let actual_checksum = calculate_sha256(&checkpoint_data);
        if actual_checksum != metadata.checksum {
            return Err(CheckpointError::ValidationFailed(
                format!("Checksum mismatch: expected {}, found {}", metadata.checksum, actual_checksum)
            ));
        }
        
        // Deserialize the workflow
        let workflow: WorkflowDefinition = serde_json::from_slice(&checkpoint_data)
            .map_err(|e| CheckpointError::SerializationError(e))?;
        
        Ok(workflow)
    }
    
    /// Delete a checkpoint
    pub async fn delete_checkpoint(&self, checkpoint_id: &str) -> Result<(), CheckpointError> {
        // Delete the checkpoint file
        self.storage.delete(checkpoint_id).await
            .map_err(|e| CheckpointError::StorageError(format!("Failed to delete checkpoint: {}", e)))?;
        
        // Delete the metadata file
        let metadata_key = format!("{}.meta", checkpoint_id);
        self.storage.delete(&metadata_key).await
            .map_err(|e| CheckpointError::StorageError(format!("Failed to delete metadata: {}", e)))?;
        
        Ok(())
    }
    
    /// List all checkpoints for a workflow
    pub async fn list_checkpoints(&self, workflow_id: &WorkflowId) -> Result<Vec<CheckpointMetadata>, CheckpointError> {
        // List all keys in storage
        let keys = self.storage.list().await
            .map_err(|e| CheckpointError::StorageError(format!("Failed to list checkpoints: {}", e)))?;
        
        // Filter for metadata files for this workflow
        let mut result = Vec::new();
        for key in keys {
            if key.ends_with(".meta") {
                // Load the metadata
                let metadata_data = self.storage.load(&key).await
                    .map_err(|e| CheckpointError::StorageError(format!("Failed to load metadata: {}", e)))?;
                
                let metadata: CheckpointMetadata = serde_json::from_slice(&metadata_data)
                    .map_err(|e| CheckpointError::SerializationError(e))?;
                
                // Check if this is for the requested workflow
                if metadata.workflow_id == *workflow_id {
                    result.push(metadata);
                }
            }
        }
        
        Ok(result)
    }
    
    /// Clean up old checkpoints, keeping only the most recent N
    pub async fn prune_checkpoints(&self, workflow_id: &WorkflowId, keep_count: usize) -> Result<usize, CheckpointError> {
        // List all checkpoints for this workflow
        let mut checkpoints = self.list_checkpoints(workflow_id).await?;
        
        if checkpoints.len() <= keep_count {
            return Ok(0);
        }
        
        // Sort by creation time (newest first)
        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Delete the oldest checkpoints
        let to_delete = checkpoints.into_iter().skip(keep_count).collect::<Vec<_>>();
        let delete_count = to_delete.len();
        
        for metadata in to_delete {
            self.delete_checkpoint(&metadata.id).await?;
        }
        
        Ok(delete_count)
    }
}

/// Calculate SHA-256 checksum of data
fn calculate_sha256(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Node, Edge, NodeId, EdgeId};
    use tempfile::TempDir;
    
    // Helper to create a simple test workflow
    fn create_test_workflow() -> WorkflowDefinition {
        let mut workflow = WorkflowDefinition::new(WorkflowId::new(), "Test Workflow".to_string());
        
        let node1 = Node::new(NodeId::new(), "Node 1".to_string());
        let node2 = Node::new(NodeId::new(), "Node 2".to_string());
        let node1_id = node1.id;
        let node2_id = node2.id;
        
        workflow.add_node(node1).unwrap();
        workflow.add_node(node2).unwrap();
        workflow.add_edge(Edge::new(EdgeId::new(), node1_id, node2_id)).unwrap();
        
        workflow
    }
    
    #[tokio::test]
    async fn test_checkpoint_save_load() {
        // Create a temporary directory for checkpoints
        let temp_dir = TempDir::new().unwrap();
        let manager = CheckpointManager::with_file_storage(temp_dir.path().to_path_buf(), "1.0.0").unwrap();
        
        // Create a test workflow
        let workflow = create_test_workflow();
        let workflow_id = workflow.id;
        
        // Save a checkpoint
        let checkpoint_id = manager.save_checkpoint(&workflow).await.unwrap();
        
        // Load the checkpoint
        let loaded = manager.load_checkpoint(&checkpoint_id).await.unwrap();
        
        // Verify the loaded workflow
        assert_eq!(loaded.id, workflow_id);
        assert_eq!(loaded.name, "Test Workflow");
        assert_eq!(loaded.nodes.len(), 2);
        assert_eq!(loaded.edges.len(), 1);
    }
    
    #[tokio::test]
    async fn test_list_checkpoints() {
        // Create a temporary directory for checkpoints
        let temp_dir = TempDir::new().unwrap();
        let manager = CheckpointManager::with_file_storage(temp_dir.path().to_path_buf(), "1.0.0").unwrap();
        
        // Create a test workflow
        let workflow = create_test_workflow();
        let workflow_id = workflow.id;
        
        // Save multiple checkpoints
        let _checkpoint1 = manager.save_checkpoint(&workflow).await.unwrap();
        // Wait a bit to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let _checkpoint2 = manager.save_checkpoint(&workflow).await.unwrap();
        
        // List checkpoints
        let checkpoints = manager.list_checkpoints(&workflow_id).await.unwrap();
        
        // Verify we have two checkpoints
        assert_eq!(checkpoints.len(), 2);
    }
    
    #[tokio::test]
    async fn test_load_latest_checkpoint() {
        // Create a temporary directory for checkpoints
        let temp_dir = TempDir::new().unwrap();
        let manager = CheckpointManager::with_file_storage(temp_dir.path().to_path_buf(), "1.0.0").unwrap();
        
        // Create a test workflow
        let mut workflow = create_test_workflow();
        let workflow_id = workflow.id;
        
        // Save first checkpoint
        let _checkpoint1 = manager.save_checkpoint(&workflow).await.unwrap();
        
        // Wait a bit to ensure different timestamp
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Modify workflow and save another checkpoint
        let node3 = Node::new(NodeId::new(), "Node 3".to_string());
        workflow.add_node(node3).unwrap();
        let _checkpoint2 = manager.save_checkpoint(&workflow).await.unwrap();
        
        // Load latest checkpoint
        let loaded = manager.load_latest_checkpoint(&workflow_id).await.unwrap();
        
        // Verify we got the latest version (with 3 nodes)
        assert_eq!(loaded.id, workflow_id);
        assert_eq!(loaded.nodes.len(), 3);
    }
    
    #[tokio::test]
    async fn test_prune_checkpoints() {
        // Create a temporary directory for checkpoints
        let temp_dir = TempDir::new().unwrap();
        let manager = CheckpointManager::with_file_storage(temp_dir.path().to_path_buf(), "1.0.0").unwrap();
        
        // Create a test workflow
        let workflow = create_test_workflow();
        let workflow_id = workflow.id;
        
        // Save multiple checkpoints
        let _checkpoint1 = manager.save_checkpoint(&workflow).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let _checkpoint2 = manager.save_checkpoint(&workflow).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let _checkpoint3 = manager.save_checkpoint(&workflow).await.unwrap();
        
        // Verify we have 3 checkpoints
        let checkpoints = manager.list_checkpoints(&workflow_id).await.unwrap();
        assert_eq!(checkpoints.len(), 3);
        
        // Prune to keep only 1 checkpoint
        let deleted = manager.prune_checkpoints(&workflow_id, 1).await.unwrap();
        assert_eq!(deleted, 2);
        
        // Verify we now have 1 checkpoint
        let checkpoints = manager.list_checkpoints(&workflow_id).await.unwrap();
        assert_eq!(checkpoints.len(), 1);
    }
}