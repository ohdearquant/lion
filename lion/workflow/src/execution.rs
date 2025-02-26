//! Workflow execution tracking.
//!
//! This module provides tracking and management of workflow executions,
//! including parallel node execution and error handling.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};

use core::error::{Result, WorkflowError};
use crate::{ExecutionId, NodeId, Workflow, WorkflowNode, ErrorPolicy, NodeType};

/// Status of a node execution.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NodeStatus {
    /// Node is waiting for dependencies.
    Pending,
    
    /// Node is ready to execute.
    Ready,
    
    /// Node is currently executing.
    Running {
        /// When the node started.
        started_at: chrono::DateTime<chrono::Utc>,
        
        /// Number of retry attempts.
        retry_count: usize,
    },
    
    /// Node has completed successfully.
    Completed {
        /// When the node completed.
        completed_at: chrono::DateTime<chrono::Utc>,
        
        /// How long the node took to execute.
        duration_ms: u64,
        
        /// The node's results.
        results: serde_json::Value,
    },
    
    /// Node has failed.
    Failed {
        /// When the node failed.
        failed_at: chrono::DateTime<chrono::Utc>,
        
        /// How long the node took to execute.
        duration_ms: u64,
        
        /// The error message.
        error: String,
        
        /// Number of retry attempts.
        retry_count: usize,
    },
    
    /// Node was skipped due to condition.
    Skipped,
    
    /// Node was cancelled.
    Cancelled,
}

/// Status of a workflow execution.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ExecutionStatus {
    /// Workflow is pending execution.
    Pending,
    
    /// Workflow is currently executing.
    Running {
        /// When the workflow started.
        started_at: chrono::DateTime<chrono::Utc>,
    },
    
    /// Workflow has completed successfully.
    Completed {
        /// When the workflow completed.
        completed_at: chrono::DateTime<chrono::Utc>,
        
        /// How long the workflow took to execute.
        duration_ms: u64,
        
        /// The workflow's results.
        results: serde_json::Value,
    },
    
    /// Workflow has failed.
    Failed {
        /// When the workflow failed.
        failed_at: chrono::DateTime<chrono::Utc>,
        
        /// How long the workflow took to execute.
        duration_ms: u64,
        
        /// The error message.
        error: String,
    },
    
    /// Workflow was cancelled.
    Cancelled,
}

/// Options for workflow execution.
#[derive(Clone, Debug)]
pub struct ExecutionOptions {
    /// Maximum parallel nodes.
    pub max_parallel_nodes: usize,
    
    /// Timeout for the entire workflow, in milliseconds.
    pub timeout_ms: Option<u64>,
    
    /// Whether to continue execution if a node fails.
    pub continue_on_failure: bool,
    
    /// Whether to use checkpoints for persistence.
    pub use_checkpoints: bool,
    
    /// Checkpoint interval, in milliseconds.
    pub checkpoint_interval_ms: Option<u64>,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            max_parallel_nodes: 4,
            timeout_ms: Some(300000), // 5 minutes
            continue_on_failure: false,
            use_checkpoints: false,
            checkpoint_interval_ms: Some(60000), // 1 minute
        }
    }
}

/// Event in workflow execution.
#[derive(Clone, Debug)]
pub enum ExecutionEvent {
    /// Node status changed.
    NodeStatusChanged {
        /// The node ID.
        node_id: NodeId,
        
        /// The new status.
        status: NodeStatus,
    },
    
    /// Execution status changed.
    ExecutionStatusChanged {
        /// The new status.
        status: ExecutionStatus,
    },
    
    /// Checkpoint created.
    CheckpointCreated {
        /// The checkpoint ID.
        checkpoint_id: String,
    },
}

/// A workflow execution instance.
pub struct Execution {
    /// Unique execution ID.
    pub id: ExecutionId,
    
    /// The workflow being executed.
    workflow: Workflow,
    
    /// Input data.
    input: serde_json::Value,
    
    /// Execution options.
    options: ExecutionOptions,
    
    /// Current execution status.
    status: RwLock<ExecutionStatus>,
    
    /// Node statuses.
    node_statuses: DashMap<NodeId, NodeStatus>,
    
    /// Execution context.
    context: RwLock<serde_json::Map<String, serde_json::Value>>,
    
    /// Whether the execution is cancelled.
    cancelled: AtomicBool,
    
    /// Event listeners.
    event_listeners: Mutex<Vec<Box<dyn Fn(ExecutionEvent) + Send + Sync>>>,
    
    /// Start time.
    start_time: Option<Instant>,
}

impl Execution {
    /// Create a new workflow execution.
    pub fn new(
        id: ExecutionId,
        workflow: Workflow,
        input: serde_json::Value,
        options: ExecutionOptions,
    ) -> Self {
        let mut context = serde_json::Map::new();
        context.insert("input".to_string(), input.clone());
        
        Self {
            id,
            workflow,
            input,
            options,
            status: RwLock::new(ExecutionStatus::Pending),
            node_statuses: DashMap::new(),
            context: RwLock::new(context),
            cancelled: AtomicBool::new(false),
            event_listeners: Mutex::new(Vec::new()),
            start_time: None,
        }
    }
    
    /// Add an event listener.
    pub fn add_event_listener<F>(&self, listener: F)
    where
        F: Fn(ExecutionEvent) + Send + Sync + 'static,
    {
        let mut listeners = self.event_listeners.lock();
        listeners.push(Box::new(listener));
    }
    
    /// Emit an event.
    fn emit_event(&self, event: ExecutionEvent) {
        let listeners = self.event_listeners.lock();
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }
    
    /// Get the current execution status.
    pub fn status(&self) -> ExecutionStatus {
        self.status.read().clone()
    }
    
    /// Get execution results, if available.
    pub fn results(&self) -> Result<serde_json::Value> {
        match self.status() {
            ExecutionStatus::Completed { results, .. } => Ok(results),
            ExecutionStatus::Failed { error, .. } => Err(WorkflowError::ExecutionFailed(error).into()),
            ExecutionStatus::Cancelled => Err(WorkflowError::Cancelled.into()),
            _ => Err(WorkflowError::ExecutionFailed("Execution not completed".to_string()).into()),
        }
    }
    
    /// Get the status of a node.
    pub fn node_status(&self, node_id: &NodeId) -> Option<NodeStatus> {
        self.node_statuses.get(node_id).map(|status| status.clone())
    }
    
    /// Start the execution.
    pub fn start(&mut self) -> Result<()> {
        // Check if already started
        if matches!(*self.status.read(), ExecutionStatus::Running { .. }) {
            return Ok(());
        }
        
        // Initialize node statuses
        for node in self.workflow.nodes.iter() {
            self.node_statuses.insert(node.key().clone(), NodeStatus::Pending);
        }
        
        // Find ready nodes
        let ready_nodes = self.find_ready_nodes();
        for node_id in ready_nodes {
            self.node_statuses.insert(node_id, NodeStatus::Ready);
        }
        
        // Set status to running
        let now = chrono::Utc::now();
        let new_status = ExecutionStatus::Running { started_at: now };
        *self.status.write() = new_status.clone();
        
        // Set start time
        self.start_time = Some(Instant::now());
        
        // Emit event
        self.emit_event(ExecutionEvent::ExecutionStatusChanged {
            status: new_status,
        });
        
        Ok(())
    }
    
    /// Find nodes that are ready to execute.
    fn find_ready_nodes(&self) -> Vec<NodeId> {
        let mut ready_nodes = Vec::new();
        
        // Check each node
        for node in self.workflow.nodes.iter() {
            let node_id = node.key().clone();
            
            // Skip if node is not pending
            if let Some(status) = self.node_status(&node_id) {
                if !matches!(status, NodeStatus::Pending) {
                    continue;
                }
            }
            
            // Check if dependencies are satisfied
            let deps = self.workflow.edges.get(&node_id);
            let deps_satisfied = match deps {
                Some(deps) => {
                    // Check each dependency
                    let mut all_satisfied = true;
                    for dep_id in deps.iter() {
                        match self.node_status(dep_id) {
                            Some(NodeStatus::Completed { .. }) => {
                                // Dependency is satisfied
                            },
                            _ => {
                                // Dependency not satisfied
                                all_satisfied = false;
                                break;
                            }
                        }
                    }
                    all_satisfied
                },
                None => true, // No dependencies
            };
            
            if deps_satisfied {
                ready_nodes.push(node_id);
            }
        }
        
        ready_nodes
    }
    
    /// Update the status of a node.
    pub fn update_node_status(&self, node_id: &NodeId, status: NodeStatus) -> Result<()> {
        // Check if node exists
        if !self.workflow.nodes.contains_key(node_id) {
            return Err(WorkflowError::NodeNotFound(node_id.to_string()).into());
        }
        
        // Update the status
        self.node_statuses.insert(node_id.clone(), status.clone());
        
        // Emit event
        self.emit_event(ExecutionEvent::NodeStatusChanged {
            node_id: node_id.clone(),
            status,
        });
        
        // Check if execution is complete
        self.check_completion()?;
        
        Ok(())
    }
    
    /// Check if the execution is complete.
    fn check_completion(&self) -> Result<()> {
        // Check if already completed
        match *self.status.read() {
            ExecutionStatus::Completed { .. } | ExecutionStatus::Failed { .. } | ExecutionStatus::Cancelled => {
                return Ok(());
            },
            _ => {}
        }
        
        // Check if all nodes are in a terminal state
        let mut all_completed = true;
        let mut any_failed = false;
        let mut failure_message = String::new();
        
        for node in self.node_statuses.iter() {
            match node.value() {
                NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Running { .. } => {
                    all_completed = false;
                    break;
                },
                NodeStatus::Failed { error, .. } => {
                    any_failed = true;
                    if failure_message.is_empty() {
                        failure_message = format!("Node {} failed: {}", node.key(), error);
                    }
                },
                _ => {}
            }
        }
        
        if all_completed {
            // All nodes are in a terminal state
            let now = chrono::Utc::now();
            let duration_ms = self.start_time
                .map(|start| start.elapsed().as_millis() as u64)
                .unwrap_or(0);
            
            let new_status = if any_failed && !self.options.continue_on_failure {
                // Execution failed
                ExecutionStatus::Failed {
                    failed_at: now,
                    duration_ms,
                    error: failure_message,
                }
            } else {
                // Execution completed
                let results = self.collect_results();
                ExecutionStatus::Completed {
                    completed_at: now,
                    duration_ms,
                    results,
                }
            };
            
            // Update status
            *self.status.write() = new_status.clone();
            
            // Emit event
            self.emit_event(ExecutionEvent::ExecutionStatusChanged {
                status: new_status,
            });
        }
        
        Ok(())
    }
    
    /// Collect results from completed nodes.
    fn collect_results(&self) -> serde_json::Value {
        let mut results = serde_json::Map::new();
        
        // Add node results
        let mut nodes = serde_json::Map::new();
        for entry in self.node_statuses.iter() {
            let node_id = entry.key();
            let status = entry.value();
            
            match status {
                NodeStatus::Completed { results: node_results, .. } => {
                    nodes.insert(node_id.to_string(), node_results.clone());
                },
                _ => {
                    nodes.insert(node_id.to_string(), serde_json::Value::Null);
                }
            }
        }
        
        results.insert("nodes".to_string(), serde_json::Value::Object(nodes));
        
        // Add execution metadata
        results.insert("execution_id".to_string(), serde_json::Value::String(self.id.to_string()));
        results.insert("workflow_id".to_string(), serde_json::Value::String(self.workflow.id.to_string()));
        
        serde_json::Value::Object(results)
    }
    
    /// Cancel the execution.
    pub fn cancel(&self) -> Result<()> {
        // Set cancelled flag
        self.cancelled.store(true, Ordering::SeqCst);
        
        // Update status if not already terminal
        match *self.status.read() {
            ExecutionStatus::Pending | ExecutionStatus::Running { .. } => {
                // Update to cancelled
                *self.status.write() = ExecutionStatus::Cancelled;
                
                // Emit event
                self.emit_event(ExecutionEvent::ExecutionStatusChanged {
                    status: ExecutionStatus::Cancelled,
                });
                
                // Cancel all running nodes
                for entry in self.node_statuses.iter_mut() {
                    let node_id = entry.key().clone();
                    let status = entry.value();
                    
                    match status {
                        NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Running { .. } => {
                            // Update to cancelled
                            *status = NodeStatus::Cancelled;
                            
                            // Emit event
                            self.emit_event(ExecutionEvent::NodeStatusChanged {
                                node_id,
                                status: NodeStatus::Cancelled,
                            });
                        },
                        _ => {}
                    }
                }
            },
            _ => {}
        }
        
        Ok(())
    }
    
    /// Check if the execution is cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    
    /// Update the execution context.
    pub fn update_context(&self, updates: serde_json::Map<String, serde_json::Value>) -> Result<()> {
        let mut context = self.context.write();
        
        // Apply updates
        for (key, value) in updates {
            context.insert(key, value);
        }
        
        Ok(())
    }
    
    /// Get a value from the context.
    pub fn get_context_value(&self, path: &str) -> Option<serde_json::Value> {
        // Parse the path
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return None;
        }
        
        // Get the context
        let context = self.context.read();
        
        // Start with the root object
        let mut current = context.get(parts[0])?;
        
        // Traverse the path
        for part in &parts[1..] {
            match current {
                serde_json::Value::Object(obj) => {
                    current = obj.get(*part)?;
                },
                serde_json::Value::Array(arr) => {
                    let index = part.parse::<usize>().ok()?;
                    current = arr.get(index)?;
                },
                _ => return None,
            }
        }
        
        Some(current.clone())
    }
    
    /// Create a checkpoint of the execution state.
    #[cfg(feature = "persistence")]
    pub fn create_checkpoint(&self) -> Result<String> {
        // Generate checkpoint ID
        let checkpoint_id = uuid::Uuid::new_v4().to_string();
        
        // Collect state
        let state = self.collect_state();
        
        // Emit event
        self.emit_event(ExecutionEvent::CheckpointCreated {
            checkpoint_id: checkpoint_id.clone(),
        });
        
        Ok(checkpoint_id)
    }
    
    /// Collect execution state for checkpointing.
    #[cfg(feature = "persistence")]
    fn collect_state(&self) -> serde_json::Value {
        let mut state = serde_json::Map::new();
        
        // Add execution ID and status
        state.insert("id".to_string(), serde_json::Value::String(self.id.to_string()));
        state.insert("status".to_string(), serde_json::to_value(self.status()).unwrap_or(serde_json::Value::Null));
        
        // Add node statuses
        let mut node_statuses = serde_json::Map::new();
        for entry in self.node_statuses.iter() {
            let node_id = entry.key();
            let status = entry.value();
            
            node_statuses.insert(
                node_id.to_string(),
                serde_json::to_value(status.clone()).unwrap_or(serde_json::Value::Null),
            );
        }
        
        state.insert("node_statuses".to_string(), serde_json::Value::Object(node_statuses));
        
        // Add context
        state.insert("context".to_string(), serde_json::Value::Object(self.context.read().clone()));
        
        serde_json::Value::Object(state)
    }
    
    /// Get the workflow.
    pub fn workflow(&self) -> &Workflow {
        &self.workflow
    }
    
    /// Get input data.
    pub fn input(&self) -> &serde_json::Value {
        &self.input
    }
    
    /// Get execution options.
    pub fn options(&self) -> &ExecutionOptions {
        &self.options
    }
}