use crate::model::{Edge, EdgeId, NodeId, NodeStatus, WorkflowDefinition, WorkflowId};
use crate::state::checkpoint::{CheckpointError, CheckpointManager};
use crate::state::storage::StorageBackend;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Error types for state machine operations
#[derive(Error, Debug)]
pub enum StateMachineError {
    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),

    #[error("Edge not found: {0}")]
    EdgeNotFound(EdgeId),

    #[error("Workflow not found: {0}")]
    WorkflowNotFound(WorkflowId),

    #[error("Node not ready: {0}")]
    NodeNotReady(NodeId),

    #[error("Invalid transition: {0} -> {1}")]
    InvalidTransition(NodeStatus, NodeStatus),

    #[error("Checkpoint error: {0}")]
    CheckpointError(#[from] CheckpointError),

    #[error("Other error: {0}")]
    Other(String),
}

/// Result of evaluating edge conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConditionResult {
    /// Condition passed
    Passed,

    /// Condition failed
    Failed,

    /// Condition evaluation not yet complete
    Pending,

    /// Error during condition evaluation
    Error,
}

/// State of a workflow execution instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Workflow definition
    #[serde(skip)]
    pub definition: Option<Arc<WorkflowDefinition>>,

    /// Workflow ID
    pub workflow_id: WorkflowId,

    /// Unique instance ID for this execution
    pub instance_id: String,

    /// Current status of each node
    pub node_status: HashMap<NodeId, NodeStatus>,

    /// Current in-degree (incomplete dependencies) of each node
    pub node_in_degree: HashMap<NodeId, usize>,

    /// Results/outputs for completed nodes
    pub node_results: HashMap<NodeId, serde_json::Value>,

    /// Evaluation results for edge conditions
    pub edge_conditions: HashMap<EdgeId, ConditionResult>,

    /// Set of nodes that are currently ready (in-degree = 0, status = Pending)
    pub ready_nodes: HashSet<NodeId>,

    /// Creation time
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last updated time
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Whether this workflow is complete
    pub is_completed: bool,

    /// Whether this workflow has failed
    pub has_failed: bool,

    /// Additional metadata for this workflow instance
    pub metadata: serde_json::Value,
}

impl WorkflowState {
    /// Create a new workflow state from a definition
    pub fn new(definition: Arc<WorkflowDefinition>) -> Self {
        let workflow_id = definition.id.clone();
        let now = chrono::Utc::now();
        let instance_id = format!("{}-{}", workflow_id, uuid::Uuid::new_v4());

        // Initialize node status and in-degree
        let mut node_status = HashMap::new();
        let mut node_in_degree = HashMap::new();
        let mut ready_nodes = HashSet::new();

        for (id, node) in &definition.nodes {
            node_status.insert(id.clone(), NodeStatus::Pending);
            node_in_degree.insert(id.clone(), node.in_degree);

            // Add start nodes to ready nodes
            if node.in_degree == 0 {
                ready_nodes.insert(id.clone());
            }
        }

        WorkflowState {
            definition: Some(definition),
            workflow_id,
            instance_id,
            node_status,
            node_in_degree,
            node_results: HashMap::new(),
            edge_conditions: HashMap::new(),
            ready_nodes,
            created_at: now,
            updated_at: now,
            is_completed: false,
            has_failed: false,
            metadata: serde_json::Value::Null,
        }
    }

    /// Set the definition for this workflow state
    pub fn with_definition(mut self, definition: Arc<WorkflowDefinition>) -> Self {
        self.definition = Some(definition);
        self
    }

    /// Get the current status of a node
    pub fn get_node_status(&self, node_id: &NodeId) -> Option<NodeStatus> {
        self.node_status.get(node_id).copied()
    }

    /// Get the current in-degree of a node
    pub fn get_node_in_degree(&self, node_id: &NodeId) -> Option<usize> {
        self.node_in_degree.get(node_id).copied()
    }

    /// Check if a node is ready to execute
    pub fn is_node_ready(&self, node_id: &NodeId) -> bool {
        self.ready_nodes.contains(node_id)
    }

    /// Set a node as running
    pub fn set_node_running(&mut self, node_id: &NodeId) -> Result<(), StateMachineError> {
        // Check if node exists
        if !self.node_status.contains_key(node_id) {
            return Err(StateMachineError::NodeNotFound(node_id.clone()));
        }

        // Check if node is ready
        if !self.ready_nodes.contains(node_id) {
            return Err(StateMachineError::NodeNotReady(node_id.clone()));
        }

        // Update status
        self.node_status
            .insert(node_id.clone(), NodeStatus::Running);
        self.ready_nodes.remove(&node_id);
        self.updated_at = chrono::Utc::now();

        Ok(())
    }

    /// Set a node as completed
    pub fn set_node_completed(
        &mut self,
        node_id: &NodeId,
        result: serde_json::Value,
    ) -> Result<Vec<NodeId>, StateMachineError> {
        // Check if node exists
        if !self.node_status.contains_key(node_id) {
            return Err(StateMachineError::NodeNotFound(node_id.clone()));
        }

        // Check if node is in a valid state to complete
        let current_status = self.node_status[node_id];
        if current_status != NodeStatus::Running && current_status != NodeStatus::Ready {
            return Err(StateMachineError::InvalidTransition(
                current_status,
                NodeStatus::Completed,
            ));
        }

        // Update status and store result
        self.node_status
            .insert(node_id.clone(), NodeStatus::Completed);
        self.node_results.insert(node_id.clone(), result);
        self.updated_at = chrono::Utc::now();

        // Find outgoing edges to activate next nodes
        let mut newly_ready = Vec::new();

        if let Some(definition) = &self.definition {
            if let Some(node) = definition.nodes.get(node_id) {
                for edge_id in &node.outgoing_edges {
                    if let Some(edge) = definition.edges.get(edge_id) {
                        // Decrease in-degree of target node
                        if let Some(in_degree) = self.node_in_degree.get_mut(&edge.target) {
                            if *in_degree > 0 {
                                *in_degree -= 1;

                                // If in-degree is now 0, mark as ready
                                if *in_degree == 0 {
                                    if let Some(status) = self.node_status.get(&edge.target) {
                                        if *status == NodeStatus::Pending {
                                            self.ready_nodes.insert(edge.target.clone());
                                            newly_ready.push(edge.target.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check if workflow is completed (all nodes completed or failed)
        self.check_workflow_completion();

        Ok(newly_ready)
    }

    /// Set a node as failed
    pub fn set_node_failed(
        &mut self,
        node_id: &NodeId,
        error: serde_json::Value,
    ) -> Result<(), StateMachineError> {
        // Check if node exists
        if !self.node_status.contains_key(node_id) {
            return Err(StateMachineError::NodeNotFound(node_id.clone()));
        }

        // Check if node is in a valid state to fail
        let current_status = self.node_status[node_id];
        if current_status != NodeStatus::Running && current_status != NodeStatus::Ready {
            return Err(StateMachineError::InvalidTransition(
                current_status,
                NodeStatus::Failed,
            ));
        }

        // Update status and store error
        self.node_status.insert(node_id.clone(), NodeStatus::Failed);
        self.node_results.insert(node_id.clone(), error);
        self.ready_nodes.remove(&node_id);
        self.has_failed = true;
        self.updated_at = chrono::Utc::now();

        // Check if workflow is completed
        self.check_workflow_completion();

        Ok(())
    }

    /// Update edge condition result
    pub fn set_edge_condition(
        &mut self,
        edge_id: &EdgeId,
        result: ConditionResult,
    ) -> Result<(), StateMachineError> {
        // Check if edge exists
        if let Some(definition) = &self.definition {
            if !definition.edges.contains_key(edge_id) {
                return Err(StateMachineError::EdgeNotFound(edge_id.clone()));
            }
        }

        // Update condition result
        self.edge_conditions.insert(edge_id.clone(), result);
        self.updated_at = chrono::Utc::now();

        Ok(())
    }

    /// Check if all nodes are completed or failed
    fn check_workflow_completion(&mut self) {
        if self.has_failed {
            return;
        }

        // Check if all nodes are in a terminal state
        let all_completed = self.node_status.values().all(|status| {
            matches!(
                status,
                NodeStatus::Completed | NodeStatus::Failed | NodeStatus::Skipped
            )
        });

        if all_completed {
            self.is_completed = true;
        }
    }

    /// Reset the state of this workflow
    pub fn reset(&mut self) {
        self.updated_at = chrono::Utc::now();
        self.is_completed = false;
        self.has_failed = false;
        self.ready_nodes.clear();
        self.node_results.clear();
        self.edge_conditions.clear();

        // Reset node status and in-degree
        if let Some(definition) = &self.definition {
            for (id, node) in &definition.nodes {
                self.node_status.insert(id.clone(), NodeStatus::Pending);
                self.node_in_degree.insert(id.clone(), node.in_degree);

                // Add start nodes to ready nodes
                if node.in_degree == 0 {
                    self.ready_nodes.insert(id.clone());
                }
            }
        }
    }
}

/// Manager for workflow state machines
pub struct StateMachineManager<S: StorageBackend> {
    /// Active workflow states
    states: Arc<RwLock<HashMap<String, Arc<RwLock<WorkflowState>>>>>,

    /// Checkpoint manager for persistence
    checkpoint_manager: Option<CheckpointManager<S>>,

    /// Workflow definitions (cache)
    definitions: Arc<RwLock<HashMap<WorkflowId, Arc<WorkflowDefinition>>>>,
}

impl<S: StorageBackend> StateMachineManager<S> {
    /// Create a new state machine manager
    pub fn new() -> Self {
        StateMachineManager {
            states: Arc::new(RwLock::new(HashMap::new())),
            checkpoint_manager: None,
            definitions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new state machine manager with checkpoint support
    pub fn with_checkpoint_manager(checkpoint_manager: CheckpointManager<S>) -> Self {
        StateMachineManager {
            states: Arc::new(RwLock::new(HashMap::new())),
            checkpoint_manager: Some(checkpoint_manager),
            definitions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new workflow instance
    pub async fn create_instance(
        &self,
        definition: Arc<WorkflowDefinition>,
    ) -> Result<Arc<RwLock<WorkflowState>>, StateMachineError> {
        // Cache the definition
        {
            let mut defs = self.definitions.write().await;
            defs.insert(definition.id, definition.clone());
        }

        // Create a new state
        let state = WorkflowState::new(definition);
        let instance_id = state.instance_id.clone();
        let state = Arc::new(RwLock::new(state));

        // Register the state
        {
            let mut states = self.states.write().await;
            states.insert(instance_id.clone(), state.clone());
        }

        // Create initial checkpoint if enabled
        if let Some(manager) = &self.checkpoint_manager {
            let state_guard = state.read().await;
            if let Some(def) = &state_guard.definition {
                let mut definition = (**def).clone();
                definition.reset(); // Reset the definition to ensure clean state
                manager.save_checkpoint(&definition).await?;
            }
        }

        Ok(state)
    }

    /// Get a workflow instance by ID
    pub async fn get_instance(&self, instance_id: &str) -> Option<Arc<RwLock<WorkflowState>>> {
        let states = self.states.read().await;
        states.get(instance_id).cloned()
    }

    /// List all active workflow instances
    pub async fn list_instances(&self) -> Vec<String> {
        let states = self.states.read().await;
        states.keys().cloned().collect()
    }

    /// Check if a workflow instance exists
    pub async fn instance_exists(&self, instance_id: &str) -> bool {
        let states = self.states.read().await;
        states.contains_key(instance_id)
    }

    /// Load a workflow definition from checkpoint
    pub async fn load_definition(
        &self,
        workflow_id: &WorkflowId,
    ) -> Result<Arc<WorkflowDefinition>, StateMachineError> {
        // Check cache first
        {
            let defs = self.definitions.read().await;
            if let Some(def) = defs.get(workflow_id) {
                return Ok(def.clone());
            }
        }

        // Load from checkpoint
        if let Some(manager) = &self.checkpoint_manager {
            let definition = manager.load_latest_checkpoint(workflow_id).await?;
            let definition = Arc::new(definition);

            // Cache it
            {
                let mut defs = self.definitions.write().await;
                defs.insert(*workflow_id, definition.clone());
            }

            return Ok(definition);
        }

        Err(StateMachineError::WorkflowNotFound(workflow_id.clone()))
    }

    /// Checkpoint a workflow instance
    pub async fn checkpoint_instance(
        &self,
        instance_id: &str,
    ) -> Result<String, StateMachineError> {
        // Get the instance
        let state_lock = match self.get_instance(instance_id).await {
            Some(state) => state,
            None => {
                return Err(StateMachineError::Other(format!(
                    "Instance not found: {}",
                    instance_id
                )))
            }
        };

        // Get the definition
        let workflow_id = {
            let state = state_lock.read().await;
            state.workflow_id
        };

        let definition = self.load_definition(&workflow_id).await?;

        // Create a checkpoint
        if let Some(manager) = &self.checkpoint_manager {
            let checkpoint_id = manager.save_checkpoint(&definition).await?;
            Ok(checkpoint_id)
        } else {
            Err(StateMachineError::Other(
                "Checkpoint manager not configured".to_string(),
            ))
        }
    }

    /// Get nodes that are ready to execute
    pub async fn get_ready_nodes(
        &self,
        instance_id: &str,
    ) -> Result<Vec<NodeId>, StateMachineError> {
        let state_lock = match self.get_instance(instance_id).await {
            Some(state) => state,
            None => {
                return Err(StateMachineError::Other(format!(
                    "Instance not found: {}",
                    instance_id
                )))
            }
        };

        let state = state_lock.read().await;
        Ok(state.ready_nodes.iter().copied().collect())
    }

    /// Mark a node as running
    pub async fn set_node_running(
        &self,
        instance_id: &str,
        node_id: &NodeId,
    ) -> Result<(), StateMachineError> {
        let state_lock = match self.get_instance(instance_id).await {
            Some(state) => state,
            None => {
                return Err(StateMachineError::Other(format!(
                    "Instance not found: {}",
                    instance_id
                )))
            }
        };

        let mut state = state_lock.write().await;
        state.set_node_running(node_id)
    }

    /// Mark a node as completed
    pub async fn set_node_completed(
        &self,
        instance_id: &str,
        node_id: &NodeId,
        result: serde_json::Value,
    ) -> Result<Vec<NodeId>, StateMachineError> {
        let state_lock = match self.get_instance(instance_id).await {
            Some(state) => state,
            None => {
                return Err(StateMachineError::Other(format!(
                    "Instance not found: {}",
                    instance_id
                )))
            }
        };

        let mut state = state_lock.write().await;
        let newly_ready = state.set_node_completed(node_id, result)?;

        // If workflow completed, create a final checkpoint
        if state.is_completed && !state.has_failed {
            drop(state); // Release write lock before checkpoint
            if let Some(manager) = &self.checkpoint_manager {
                // TODO: Save the final state
            }
        }

        Ok(newly_ready)
    }

    /// Mark a node as failed
    pub async fn set_node_failed(
        &self,
        instance_id: &str,
        node_id: &NodeId,
        error: serde_json::Value,
    ) -> Result<(), StateMachineError> {
        let state_lock = match self.get_instance(instance_id).await {
            Some(state) => state,
            None => {
                return Err(StateMachineError::Other(format!(
                    "Instance not found: {}",
                    instance_id
                )))
            }
        };

        let mut state = state_lock.write().await;
        state.set_node_failed(node_id, error)
    }
}

impl<S: StorageBackend> Default for StateMachineManager<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, Node};
    use crate::state::storage::MemoryStorage;

    // Helper to create a test workflow definition
    fn create_test_workflow() -> Arc<WorkflowDefinition> {
        let mut workflow = WorkflowDefinition::new(WorkflowId::new(), "Test Workflow".to_string());

        // Create nodes
        let node1 = Node::new(NodeId::new(), "Start".to_string());
        let node2 = Node::new(NodeId::new(), "Middle".to_string());
        let node3 = Node::new(NodeId::new(), "End".to_string());

        let node1_id = node1.id.clone();
        let node2_id = node2.id.clone();
        let node3_id = node3.id.clone();

        // Add nodes
        workflow.add_node(node1).unwrap();
        workflow.add_node(node2).unwrap();
        workflow.add_node(node3).unwrap();

        // Add edges
        workflow
            .add_edge(Edge::new(EdgeId::new(), &node1_id, &node2_id))
            .unwrap();
        workflow
            .add_edge(Edge::new(EdgeId::new(), &node2_id, &node3_id))
            .unwrap();

        Arc::new(workflow)
    }

    #[tokio::test]
    async fn test_state_machine_basic_flow() {
        let workflow = create_test_workflow();
        let node_ids: Vec<NodeId> = workflow.nodes.keys().cloned().collect();

        // Create manager
        let manager = StateMachineManager::<MemoryStorage>::new();

        // Create instance
        let instance = manager.create_instance(workflow).await.unwrap();
        let instance_id = {
            let state = instance.read().await;
            state.instance_id.clone()
        };

        // Get ready nodes (should be just the start node)
        let ready_nodes = manager.get_ready_nodes(&instance_id).await.unwrap();
        assert_eq!(ready_nodes.len(), 1);
        assert_eq!(ready_nodes[0], node_ids[0]); // First node should be ready

        // Mark start node as running
        manager
            .set_node_running(&instance_id, &node_ids[0])
            .await
            .unwrap();

        // Mark start node as completed
        let newly_ready = manager
            .set_node_completed(&instance_id, &node_ids[0], serde_json::json!({}))
            .await
            .unwrap();

        // Middle node should now be ready
        assert_eq!(newly_ready.len(), 1);
        assert_eq!(newly_ready[0], node_ids[1]);

        // Mark middle node as running
        manager
            .set_node_running(&instance_id, &node_ids[1])
            .await
            .unwrap();

        // Mark middle node as completed
        let newly_ready = manager
            .set_node_completed(&instance_id, &node_ids[1], serde_json::json!({}))
            .await
            .unwrap();

        // End node should now be ready
        assert_eq!(newly_ready.len(), 1);
        assert_eq!(newly_ready[0], node_ids[2]);

        // Mark end node as running
        manager
            .set_node_running(&instance_id, &node_ids[2])
            .await
            .unwrap();

        // Mark end node as completed
        let newly_ready = manager
            .set_node_completed(&instance_id, &node_ids[2], serde_json::json!({}))
            .await
            .unwrap();

        // No more nodes should be ready
        assert_eq!(newly_ready.len(), 0);

        // Check if workflow is completed
        let state = instance.read().await;
        assert!(state.is_completed);
        assert!(!state.has_failed);
    }

    #[tokio::test]
    async fn test_state_machine_node_failure() {
        let workflow = create_test_workflow();
        let node_ids: Vec<NodeId> = workflow.nodes.keys().cloned().collect();

        // Create manager
        let manager = StateMachineManager::<MemoryStorage>::new();

        // Create instance
        let instance = manager.create_instance(workflow).await.unwrap();
        let instance_id = {
            let state = instance.read().await;
            state.instance_id.clone()
        };

        // Mark start node as running
        manager
            .set_node_running(&instance_id, &node_ids[0])
            .await
            .unwrap();

        // Mark start node as failed
        manager
            .set_node_failed(
                &instance_id,
                &node_ids[0],
                serde_json::json!({"error": "Test error"}),
            )
            .await
            .unwrap();

        // Check if workflow is failed
        let state = instance.read().await;
        assert!(state.has_failed);
        assert_eq!(
            state.get_node_status(&node_ids[0]),
            Some(NodeStatus::Failed)
        );
    }

    #[tokio::test]
    async fn test_state_machine_with_checkpoint() {
        let storage = MemoryStorage::new();
        let checkpoint_manager = CheckpointManager::new(storage, "1.0.0");
        let manager = StateMachineManager::with_checkpoint_manager(checkpoint_manager);

        let workflow = create_test_workflow();
        let workflow_id = workflow.id;

        // Create instance
        let instance = manager.create_instance(workflow).await.unwrap();
        let instance_id = {
            let state = instance.read().await;
            state.instance_id.clone()
        };

        // Test checkpoint creation
        let checkpoint_id = manager.checkpoint_instance(&instance_id).await.unwrap();

        // Load definition from checkpoint
        let loaded_definition = manager.load_definition(&workflow_id).await.unwrap();

        // Verify definition was loaded
        assert_eq!(loaded_definition.id, workflow_id);
        assert_eq!(loaded_definition.nodes.len(), 3);
        assert_eq!(loaded_definition.edges.len(), 2);
    }
}
