//! Lion Workflow - Parallel workflow execution
//!
//! This crate provides workflow execution with parallel node processing,
//! persistent checkpoints, and error handling.

mod engine;
mod node;
mod storage;
mod execution;

pub use engine::{WorkflowEngine, WorkflowConfig};
pub use node::{WorkflowNode, NodeType, NodeConfig, ErrorPolicy};
pub use storage::{WorkflowStorage, MemoryWorkflowStorage};
pub use execution::{Execution, ExecutionStatus, ExecutionOptions};

use std::sync::Arc;
use dashmap::DashMap;

use core::error::{Result, WorkflowError};
use core::types::{PluginId};
use concurrency::ConcurrencyManager;

use uuid::Uuid;

/// Unique identifier for a workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WorkflowId(pub Uuid);

impl WorkflowId {
    /// Create a new random workflow ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a workflow execution.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ExecutionId(pub Uuid);

impl ExecutionId {
    /// Create a new random execution ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a workflow node.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Create a new random node ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A workflow is a directed acyclic graph of nodes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Workflow {
    /// Unique workflow ID.
    pub id: WorkflowId,
    
    /// Human-readable name.
    pub name: String,
    
    /// Description of what this workflow does.
    pub description: String,
    
    /// Nodes in the workflow.
    pub nodes: DashMap<NodeId, WorkflowNode>,
    
    /// Edges connecting nodes (dependencies).
    pub edges: DashMap<NodeId, Vec<NodeId>>,
    
    /// Entry point nodes (no dependencies).
    pub entry_nodes: Vec<NodeId>,
    
    /// When this workflow was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// When this workflow was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    
    /// Workflow version.
    pub version: String,
}

impl Workflow {
    /// Create a new workflow.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        
        Self {
            id: WorkflowId::new(),
            name: name.into(),
            description: description.into(),
            nodes: DashMap::new(),
            edges: DashMap::new(),
            entry_nodes: Vec::new(),
            created_at: now,
            updated_at: now,
            version: "1.0.0".to_string(),
        }
    }
    
    /// Add a node to the workflow.
    pub fn add_node(&self, node: WorkflowNode) -> NodeId {
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        
        // If this node has no dependencies, add it as an entry node
        if !self.edges.contains_key(&id) {
            let mut entry_nodes = self.entry_nodes.clone();
            entry_nodes.push(id.clone());
            self.entry_nodes = entry_nodes;
        }
        
        id
    }
    
    /// Add a dependency between nodes.
    pub fn add_dependency(&self, from: &NodeId, to: &NodeId) -> Result<()> {
        // Check if nodes exist
        if !self.nodes.contains_key(from) {
            return Err(WorkflowError::NodeNotFound(from.to_string()).into());
        }
        
        if !self.nodes.contains_key(to) {
            return Err(WorkflowError::NodeNotFound(to.to_string()).into());
        }
        
        // Add the dependency
        if let Some(mut deps) = self.edges.get_mut(to) {
            if !deps.contains(from) {
                deps.push(from.clone());
            }
        } else {
            self.edges.insert(to.clone(), vec![from.clone()]);
        }
        
        // If this was an entry node and now has dependencies, remove it
        if self.entry_nodes.contains(to) {
            let mut entry_nodes = self.entry_nodes.clone();
            entry_nodes.retain(|id| id != to);
            self.entry_nodes = entry_nodes;
        }
        
        Ok(())
    }
    
    /// Validate the workflow for correctness.
    pub fn validate(&self) -> Result<()> {
        // Check for cycles
        let mut visited = std::collections::HashSet::new();
        let mut path = std::collections::HashSet::new();
        
        for node_id in &self.entry_nodes {
            if self.has_cycle(node_id, &mut visited, &mut path)? {
                return Err(WorkflowError::CyclicDependency.into());
            }
        }
        
        // Check for disconnected nodes
        let mut reachable = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        
        for node_id in &self.entry_nodes {
            queue.push_back(node_id.clone());
        }
        
        while let Some(node_id) = queue.pop_front() {
            if reachable.contains(&node_id) {
                continue;
            }
            
            reachable.insert(node_id.clone());
            
            // Find nodes that depend on this one
            for (to, deps) in self.edges.iter() {
                if deps.contains(&node_id) && !reachable.contains(to.key()) {
                    queue.push_back(to.key().clone());
                }
            }
        }
        
        // Check if all nodes are reachable
        for node in self.nodes.iter() {
            if !reachable.contains(node.key()) {
                return Err(WorkflowError::InvalidDefinition(
                    format!("Node {} is not reachable from any entry node", node.key())
                ).into());
            }
        }
        
        Ok(())
    }
    
    /// Check if the workflow has a cycle starting from the given node.
    fn has_cycle(
        &self,
        node_id: &NodeId,
        visited: &mut std::collections::HashSet<NodeId>,
        path: &mut std::collections::HashSet<NodeId>,
    ) -> Result<bool> {
        // If we've already visited this node in this path, we have a cycle
        if path.contains(node_id) {
            return Ok(true);
        }
        
        // If we've already visited this node and found no cycles, we can skip it
        if visited.contains(node_id) {
            return Ok(false);
        }
        
        // Mark the node as visited and add it to the current path
        visited.insert(node_id.clone());
        path.insert(node_id.clone());
        
        // Get nodes that depend on this one
        for (to, deps) in self.edges.iter() {
            if deps.contains(node_id) {
                if self.has_cycle(to.key(), visited, path)? {
                    return Ok(true);
                }
            }
        }
        
        // Remove the node from the current path
        path.remove(node_id);
        
        Ok(false)
    }
}

/// Core workflow manager for executing workflows.
pub struct WorkflowManager {
    /// The workflow engine.
    engine: Arc<WorkflowEngine>,
    
    /// Stored workflows.
    workflows: DashMap<WorkflowId, Workflow>,
    
    /// Active executions.
    executions: DashMap<ExecutionId, Arc<Execution>>,
}

impl WorkflowManager {
    /// Create a new workflow manager.
    pub fn new(
        concurrency_manager: Arc<dyn ConcurrencyManager>,
        storage: Arc<dyn WorkflowStorage>,
        config: WorkflowConfig,
    ) -> Self {
        let engine = Arc::new(WorkflowEngine::new(
            concurrency_manager,
            storage,
            config,
        ));
        
        Self {
            engine,
            workflows: DashMap::new(),
            executions: DashMap::new(),
        }
    }
    
    /// Create a workflow.
    pub fn create_workflow(&self, workflow: Workflow) -> Result<WorkflowId> {
        // Validate the workflow
        workflow.validate()?;
        
        // Store the workflow
        let id = workflow.id.clone();
        self.workflows.insert(id.clone(), workflow);
        
        Ok(id)
    }
    
    /// Get a workflow by ID.
    pub fn get_workflow(&self, id: &WorkflowId) -> Result<Workflow> {
        self.workflows.get(id)
            .map(|w| w.clone())
            .ok_or_else(|| WorkflowError::WorkflowNotFound(id.to_string()).into())
    }
    
    /// List all workflows.
    pub fn list_workflows(&self) -> Vec<Workflow> {
        self.workflows.iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    /// Execute a workflow.
    pub fn execute_workflow(
        &self,
        id: &WorkflowId,
        input: serde_json::Value,
        options: ExecutionOptions,
    ) -> Result<ExecutionId> {
        // Get the workflow
        let workflow = self.get_workflow(id)?;
        
        // Create the execution
        let execution = self.engine.create_execution(workflow, input, options)?;
        
        // Store the execution
        let execution_id = execution.id.clone();
        self.executions.insert(execution_id.clone(), execution);
        
        Ok(execution_id)
    }
    
    /// Get the status of an execution.
    pub fn get_execution_status(&self, id: &ExecutionId) -> Result<ExecutionStatus> {
        self.executions.get(id)
            .map(|e| e.status())
            .ok_or_else(|| WorkflowError::ExecutionNotFound(id.to_string()).into())
    }
    
    /// Get the results of an execution.
    pub fn get_execution_results(&self, id: &ExecutionId) -> Result<serde_json::Value> {
        self.executions.get(id)
            .map(|e| e.results())
            .ok_or_else(|| WorkflowError::ExecutionNotFound(id.to_string()).into())?
    }
    
    /// Cancel an execution.
    pub fn cancel_execution(&self, id: &ExecutionId) -> Result<()> {
        if let Some(execution) = self.executions.get(id) {
            execution.cancel()?;
            Ok(())
        } else {
            Err(WorkflowError::ExecutionNotFound(id.to_string()).into())
        }
    }
    
    /// Clean up completed executions.
    pub fn cleanup_executions(&self) -> Result<usize> {
        let mut to_remove = Vec::new();
        
        // Find completed executions
        for entry in self.executions.iter() {
            let execution = entry.value();
            match execution.status() {
                ExecutionStatus::Completed { .. } | ExecutionStatus::Failed { .. } | ExecutionStatus::Cancelled => {
                    to_remove.push(execution.id.clone());
                }
                _ => {}
            }
        }
        
        // Remove them
        for id in &to_remove {
            self.executions.remove(id);
        }
        
        Ok(to_remove.len())
    }
}