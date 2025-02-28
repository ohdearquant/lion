//! Workflow Execution for Lion Runtime
//!
//! Handles the execution of workflows using an actor-based approach.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use lion_capability::model::Capability;
use lion_core::id::{PluginId, WorkflowId};
use lion_core::traits::workflow::{NodeId, NodeStatus, WorkflowStatus};
use lion_workflow::model::definition::WorkflowDefinition;
use lion_workflow::model::node::Node;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::capabilities::manager::CapabilityManager;
use crate::plugin::manager::PluginManager;

/// Errors that can occur during workflow execution
#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    #[error("Node {0} not found")]
    NodeNotFound(NodeId),

    #[error("Cyclic dependency detected")]
    CyclicDependency,

    #[error("Plugin {0} not found")]
    PluginNotFound(PluginId),

    #[error("Node {0} execution failed: {1}")]
    NodeExecutionFailed(NodeId, String),

    #[error("Workflow {0} execution failed: {1}")]
    WorkflowExecutionFailed(WorkflowId, String),

    #[error("Workflow {0} not running")]
    WorkflowNotRunning(WorkflowId),

    #[error("Execution timeout")]
    Timeout,
}

/// Message for the workflow executor actor
#[derive(Debug)]
enum ExecutorMessage {
    /// Start a workflow
    Start(WorkflowId, serde_json::Value),

    /// Pause a workflow
    Pause(WorkflowId),

    /// Resume a workflow
    Resume(WorkflowId),

    /// Cancel a workflow
    Cancel(WorkflowId),

    /// Node execution completed
    NodeCompleted(WorkflowId, NodeId, serde_json::Value),

    /// Node execution failed
    NodeFailed(WorkflowId, NodeId, String),
}

/// Workflow execution state
#[derive(Debug, Clone)]
struct WorkflowExecutionState {
    /// Workflow definition
    definition: WorkflowDefinition,

    /// Status of the workflow
    status: WorkflowStatus,

    /// Status of each node
    node_statuses: HashMap<NodeId, NodeStatus>,

    /// In-degree (remaining dependencies) for each node
    in_degree: HashMap<NodeId, usize>,

    /// Output data from each node
    node_outputs: HashMap<NodeId, serde_json::Value>,

    /// Input data for the workflow
    input: serde_json::Value,

    /// Start time
    start_time: Option<Instant>,

    /// End time
    end_time: Option<Instant>,
}

/// Workflow executor for running and managing workflows
pub struct WorkflowExecutor {
    /// Sender for executor messages
    tx: mpsc::Sender<ExecutorMessage>,

    /// Workflow states by ID
    workflow_states: Arc<RwLock<HashMap<WorkflowId, WorkflowExecutionState>>>,

    /// Capability manager
    capability_manager: Arc<CapabilityManager>,

    /// Plugin manager
    plugin_manager: Arc<PluginManager>,
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new(
        capability_manager: Arc<CapabilityManager>,
        plugin_manager: Arc<PluginManager>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let workflow_states = Arc::new(RwLock::new(HashMap::new()));

        let executor = Self {
            tx: tx.clone(),
            workflow_states: workflow_states.clone(),
            capability_manager,
            plugin_manager,
        };

        // Spawn the actor task
        let executor_clone = executor.clone();
        tokio::spawn(async move {
            executor_clone.actor_loop(rx).await;
        });

        executor
    }

    /// Start a workflow
    pub async fn start_workflow(
        &self,
        workflow_id: WorkflowId,
        definition: WorkflowDefinition,
        input: serde_json::Value,
    ) -> Result<()> {
        // Check if workflow already exists
        let mut states = self.workflow_states.write().await;
        if states.contains_key(&workflow_id) {
            let state = states.get(&workflow_id).unwrap();
            match state.status {
                WorkflowStatus::Running => {
                    return Err(ExecutionError::WorkflowExecutionFailed(
                        workflow_id,
                        "Workflow already running".to_string(),
                    )
                    .into());
                }
                WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled => {
                    // Remove the old state
                    states.remove(&workflow_id);
                }
                WorkflowStatus::Paused => {
                    // We can resume the workflow
                    return self.resume_workflow(workflow_id).await;
                }
                _ => {}
            }
        }

        // Initialize workflow state
        let mut in_degree = HashMap::new();
        let mut node_statuses = HashMap::new();

        // Calculate in-degree for each node
        for node in &definition.nodes {
            in_degree.insert(node.id.clone(), 0);
            node_statuses.insert(node.id.clone(), NodeStatus::Pending);
        }

        // Count dependencies
        for node in &definition.nodes {
            for dep in &node.dependencies {
                if let Some(count) = in_degree.get_mut(dep) {
                    *count += 1;
                } else {
                    return Err(ExecutionError::NodeNotFound(dep.clone()).into());
                }
            }
        }

        // Check for cycles using Kahn's algorithm
        if !self.is_acyclic(&definition) {
            return Err(ExecutionError::CyclicDependency.into());
        }

        // Create workflow state
        let state = WorkflowExecutionState {
            definition,
            status: WorkflowStatus::Created,
            node_statuses,
            in_degree,
            node_outputs: HashMap::new(),
            input: input.clone(),
            start_time: None,
            end_time: None,
        };

        // Store the state
        states.insert(workflow_id.clone(), state);

        // Send the start message
        self.tx
            .send(ExecutorMessage::Start(workflow_id, input))
            .await
            .context("Failed to send start message to executor")?;

        Ok(())
    }

    /// Pause a workflow
    pub async fn pause_workflow(&self, workflow_id: WorkflowId) -> Result<()> {
        // Check if workflow exists and is running
        {
            let states = self.workflow_states.read().await;
            let state = states
                .get(&workflow_id)
                .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

            if state.status != WorkflowStatus::Running {
                return Err(ExecutionError::WorkflowNotRunning(workflow_id).into());
            }
        }

        // Send the pause message
        self.tx
            .send(ExecutorMessage::Pause(workflow_id))
            .await
            .context("Failed to send pause message to executor")?;

        Ok(())
    }

    /// Resume a workflow
    pub async fn resume_workflow(&self, workflow_id: WorkflowId) -> Result<()> {
        // Check if workflow exists and is paused
        {
            let states = self.workflow_states.read().await;
            let state = states
                .get(&workflow_id)
                .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

            if state.status != WorkflowStatus::Paused {
                return Err(ExecutionError::WorkflowNotRunning(workflow_id).into());
            }
        }

        // Send the resume message
        self.tx
            .send(ExecutorMessage::Resume(workflow_id))
            .await
            .context("Failed to send resume message to executor")?;

        Ok(())
    }

    /// Cancel a workflow
    pub async fn cancel_workflow(&self, workflow_id: WorkflowId) -> Result<()> {
        // Check if workflow exists
        {
            let states = self.workflow_states.read().await;
            if !states.contains_key(&workflow_id) {
                return Err(ExecutionError::WorkflowNotRunning(workflow_id).into());
            }
        }

        // Send the cancel message
        self.tx
            .send(ExecutorMessage::Cancel(workflow_id))
            .await
            .context("Failed to send cancel message to executor")?;

        Ok(())
    }

    /// Get workflow status
    pub async fn get_workflow_status(&self, workflow_id: &WorkflowId) -> Result<WorkflowStatus> {
        let states = self.workflow_states.read().await;

        states
            .get(workflow_id)
            .map(|state| state.status.clone())
            .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()).into())
    }

    /// Get workflow results
    pub async fn get_workflow_results(
        &self,
        workflow_id: &WorkflowId,
    ) -> Result<serde_json::Value> {
        let states = self.workflow_states.read().await;

        let state = states
            .get(workflow_id)
            .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

        if state.status != WorkflowStatus::Completed {
            return Err(ExecutionError::WorkflowNotRunning(workflow_id.clone()).into());
        }

        // Find the final nodes (nodes with no outgoing edges)
        let mut final_nodes = HashSet::new();

        for node in &state.definition.nodes {
            final_nodes.insert(node.id.clone());
        }

        for node in &state.definition.nodes {
            for dep in &node.dependencies {
                final_nodes.remove(dep);
            }
        }

        // Collect outputs from final nodes
        let mut results = serde_json::json!({});

        for node_id in final_nodes {
            if let Some(output) = state.node_outputs.get(&node_id) {
                // Add to results
                if let serde_json::Value::Object(obj) = output {
                    if let serde_json::Value::Object(results_obj) = &mut results {
                        for (k, v) in obj {
                            results_obj.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Actor loop
    async fn actor_loop(&self, mut rx: mpsc::Receiver<ExecutorMessage>) {
        info!("Workflow executor actor started");

        while let Some(msg) = rx.recv().await {
            match msg {
                ExecutorMessage::Start(workflow_id, input) => {
                    if let Err(e) = self.handle_start(workflow_id, input).await {
                        error!("Failed to start workflow: {}", e);
                    }
                }
                ExecutorMessage::Pause(workflow_id) => {
                    if let Err(e) = self.handle_pause(workflow_id).await {
                        error!("Failed to pause workflow: {}", e);
                    }
                }
                ExecutorMessage::Resume(workflow_id) => {
                    if let Err(e) = self.handle_resume(workflow_id).await {
                        error!("Failed to resume workflow: {}", e);
                    }
                }
                ExecutorMessage::Cancel(workflow_id) => {
                    if let Err(e) = self.handle_cancel(workflow_id).await {
                        error!("Failed to cancel workflow: {}", e);
                    }
                }
                ExecutorMessage::NodeCompleted(workflow_id, node_id, output) => {
                    if let Err(e) = self
                        .handle_node_completed(workflow_id, node_id, output)
                        .await
                    {
                        error!("Failed to handle node completion: {}", e);
                    }
                }
                ExecutorMessage::NodeFailed(workflow_id, node_id, error_msg) => {
                    if let Err(e) = self
                        .handle_node_failed(workflow_id, node_id, error_msg)
                        .await
                    {
                        error!("Failed to handle node failure: {}", e);
                    }
                }
            }
        }

        info!("Workflow executor actor stopped");
    }

    /// Handle start message
    async fn handle_start(&self, workflow_id: WorkflowId, input: serde_json::Value) -> Result<()> {
        info!("Starting workflow: {:?}", workflow_id);

        // Update workflow state
        let mut states = self.workflow_states.write().await;
        let state = states
            .get_mut(&workflow_id)
            .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

        // Update status
        state.status = WorkflowStatus::Running;
        state.start_time = Some(Instant::now());

        // Find nodes with no dependencies (in-degree = 0)
        let ready_nodes: Vec<NodeId> = state
            .in_degree
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(id, _)| id.clone())
            .collect();

        // Drop the lock before executing nodes
        drop(states);

        // Execute the ready nodes
        for node_id in ready_nodes {
            self.execute_node(workflow_id.clone(), node_id, input.clone())
                .await?;
        }

        Ok(())
    }

    /// Handle pause message
    async fn handle_pause(&self, workflow_id: WorkflowId) -> Result<()> {
        info!("Pausing workflow: {:?}", workflow_id);

        // Update workflow state
        let mut states = self.workflow_states.write().await;
        let state = states
            .get_mut(&workflow_id)
            .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

        // Update status only if running
        if state.status == WorkflowStatus::Running {
            state.status = WorkflowStatus::Paused;
        }

        Ok(())
    }

    /// Handle resume message
    async fn handle_resume(&self, workflow_id: WorkflowId) -> Result<()> {
        info!("Resuming workflow: {:?}", workflow_id);

        // Update workflow state
        let mut states = self.workflow_states.write().await;
        let state = states
            .get_mut(&workflow_id)
            .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

        // Update status only if paused
        if state.status == WorkflowStatus::Paused {
            state.status = WorkflowStatus::Running;

            // Find nodes that are ready but still pending
            let ready_nodes: Vec<(NodeId, Vec<serde_json::Value>)> = state
                .definition
                .nodes
                .iter()
                .filter(|node| {
                    state.node_statuses.get(&node.id) == Some(&NodeStatus::Pending)
                        && state.in_degree.get(&node.id) == Some(&0)
                })
                .map(|node| {
                    // Collect inputs from dependencies
                    let dep_outputs: Vec<serde_json::Value> = node
                        .dependencies
                        .iter()
                        .filter_map(|dep| state.node_outputs.get(dep).cloned())
                        .collect();

                    (node.id.clone(), dep_outputs)
                })
                .collect();

            // Drop the lock before executing nodes
            drop(states);

            // Execute the ready nodes
            for (node_id, dep_outputs) in ready_nodes {
                // Combine dependency outputs with workflow input
                let mut input = state.input.clone();

                if let serde_json::Value::Object(input_obj) = &mut input {
                    for dep_output in dep_outputs {
                        if let serde_json::Value::Object(dep_obj) = dep_output {
                            for (k, v) in dep_obj {
                                input_obj.insert(k, v);
                            }
                        }
                    }
                }

                self.execute_node(workflow_id.clone(), node_id, input)
                    .await?;
            }
        }

        Ok(())
    }

    /// Handle cancel message
    async fn handle_cancel(&self, workflow_id: WorkflowId) -> Result<()> {
        info!("Cancelling workflow: {:?}", workflow_id);

        // Update workflow state
        let mut states = self.workflow_states.write().await;
        let state = states
            .get_mut(&workflow_id)
            .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

        // Update status
        state.status = WorkflowStatus::Cancelled;
        state.end_time = Some(Instant::now());

        Ok(())
    }

    /// Handle node completed message
    async fn handle_node_completed(
        &self,
        workflow_id: WorkflowId,
        node_id: NodeId,
        output: serde_json::Value,
    ) -> Result<()> {
        debug!(
            "Node completed: {:?} in workflow {:?}",
            node_id, workflow_id
        );

        // Update workflow state
        let mut states = self.workflow_states.write().await;
        let state = states
            .get_mut(&workflow_id)
            .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

        // Check if workflow is running
        if state.status != WorkflowStatus::Running {
            return Ok(());
        }

        // Update node status
        if let Some(status) = state.node_statuses.get_mut(&node_id) {
            *status = NodeStatus::Completed;
        } else {
            return Err(ExecutionError::NodeNotFound(node_id).into());
        }

        // Store node output
        state.node_outputs.insert(node_id.clone(), output.clone());

        // Find nodes that depend on this one
        let dependent_nodes: Vec<(NodeId, bool)> = state
            .definition
            .nodes
            .iter()
            .filter(|node| node.dependencies.contains(&node_id))
            .map(|node| {
                // Decrease in-degree
                let new_in_degree = state
                    .in_degree
                    .get_mut(&node.id)
                    .map(|count| {
                        *count -= 1;
                        *count
                    })
                    .unwrap_or(0);

                (node.id.clone(), new_in_degree == 0)
            })
            .collect();

        // Check if all nodes are completed
        let all_completed = state
            .node_statuses
            .values()
            .all(|status| *status == NodeStatus::Completed);

        if all_completed {
            info!("Workflow completed: {:?}", workflow_id);
            state.status = WorkflowStatus::Completed;
            state.end_time = Some(Instant::now());
            return Ok(());
        }

        // Find ready nodes (in-degree = 0)
        let ready_nodes: Vec<(NodeId, Vec<serde_json::Value>)> = dependent_nodes
            .into_iter()
            .filter(|(_, is_ready)| *is_ready)
            .map(|(node_id, _)| {
                // Get the node
                let node = state
                    .definition
                    .nodes
                    .iter()
                    .find(|n| n.id == node_id)
                    .unwrap();

                // Collect inputs from dependencies
                let dep_outputs: Vec<serde_json::Value> = node
                    .dependencies
                    .iter()
                    .filter_map(|dep| state.node_outputs.get(dep).cloned())
                    .collect();

                (node_id, dep_outputs)
            })
            .collect();

        // Clone the workflow input
        let workflow_input = state.input.clone();

        // Drop the lock before executing nodes
        drop(states);

        // Execute the ready nodes
        for (node_id, dep_outputs) in ready_nodes {
            // Combine dependency outputs with workflow input
            let mut input = workflow_input.clone();

            if let serde_json::Value::Object(input_obj) = &mut input {
                for dep_output in dep_outputs {
                    if let serde_json::Value::Object(dep_obj) = dep_output {
                        for (k, v) in dep_obj {
                            input_obj.insert(k, v);
                        }
                    }
                }
            }

            self.execute_node(workflow_id.clone(), node_id, input)
                .await?;
        }

        Ok(())
    }

    /// Handle node failed message
    async fn handle_node_failed(
        &self,
        workflow_id: WorkflowId,
        node_id: NodeId,
        error_msg: String,
    ) -> Result<()> {
        error!(
            "Node failed: {:?} in workflow {:?}: {}",
            node_id, workflow_id, error_msg
        );

        // Update workflow state
        let mut states = self.workflow_states.write().await;
        let state = states
            .get_mut(&workflow_id)
            .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

        // Update node status
        if let Some(status) = state.node_statuses.get_mut(&node_id) {
            *status = NodeStatus::Failed;
        } else {
            return Err(ExecutionError::NodeNotFound(node_id).into());
        }

        // Update workflow status
        state.status = WorkflowStatus::Failed;
        state.end_time = Some(Instant::now());

        Ok(())
    }

    /// Execute a node
    async fn execute_node(
        &self,
        workflow_id: WorkflowId,
        node_id: NodeId,
        input: serde_json::Value,
    ) -> Result<()> {
        // Get the node
        let node = {
            let states = self.workflow_states.read().await;
            let state = states
                .get(&workflow_id)
                .ok_or_else(|| ExecutionError::WorkflowNotRunning(workflow_id.clone()))?;

            state
                .definition
                .nodes
                .iter()
                .find(|n| n.id == node_id)
                .cloned()
                .ok_or_else(|| ExecutionError::NodeNotFound(node_id.clone()))?
        };

        // Update node status
        {
            let mut states = self.workflow_states.write().await;
            if let Some(state) = states.get_mut(&workflow_id) {
                if let Some(status) = state.node_statuses.get_mut(&node_id) {
                    *status = NodeStatus::Running;
                }
            }
        }

        // Execute the node in a separate task
        let tx = self.tx.clone();
        let plugin_manager = self.plugin_manager.clone();

        tokio::spawn(async move {
            let result = match node.plugin_id {
                Some(plugin_id) => {
                    // Call the plugin function
                    match plugin_manager
                        .call_plugin_function(&plugin_id, &node.function, input)
                        .await
                    {
                        Ok(output) => {
                            // Node completed successfully
                            if let Err(e) = tx
                                .send(ExecutorMessage::NodeCompleted(workflow_id, node_id, output))
                                .await
                            {
                                error!("Failed to send node completion message: {}", e);
                            }
                        }
                        Err(e) => {
                            // Node execution failed
                            if let Err(e) = tx
                                .send(ExecutorMessage::NodeFailed(
                                    workflow_id,
                                    node_id,
                                    e.to_string(),
                                ))
                                .await
                            {
                                error!("Failed to send node failure message: {}", e);
                            }
                        }
                    }
                }
                None => {
                    // No plugin, just pass through the input
                    if let Err(e) = tx
                        .send(ExecutorMessage::NodeCompleted(workflow_id, node_id, input))
                        .await
                    {
                        error!("Failed to send node completion message: {}", e);
                    }
                }
            };
        });

        Ok(())
    }

    /// Check if a workflow is acyclic
    fn is_acyclic(&self, definition: &WorkflowDefinition) -> bool {
        // Count in-degrees for each node
        let mut in_degree = HashMap::new();
        for node in &definition.nodes {
            in_degree.insert(node.id.clone(), 0);
        }

        // Count dependencies
        for node in &definition.nodes {
            for dep in &node.dependencies {
                if let Some(count) = in_degree.get_mut(dep) {
                    *count += 1;
                }
            }
        }

        // Kahn's algorithm
        let mut q = VecDeque::new();

        // Add nodes with no dependencies
        for (id, &count) in &in_degree {
            if count == 0 {
                q.push_back(id.clone());
            }
        }

        let mut visited_count = 0;

        while let Some(node_id) = q.pop_front() {
            visited_count += 1;

            // Find nodes that depend on this one
            for node in &definition.nodes {
                if node.dependencies.contains(&node_id) {
                    if let Some(count) = in_degree.get_mut(&node.id) {
                        *count -= 1;
                        if *count == 0 {
                            q.push_back(node.id.clone());
                        }
                    }
                }
            }
        }

        // If we visited all nodes, there are no cycles
        visited_count == definition.nodes.len()
    }
}

impl Clone for WorkflowExecutor {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            workflow_states: self.workflow_states.clone(),
            capability_manager: self.capability_manager.clone(),
            plugin_manager: self.plugin_manager.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lion_workflow::model::definition::WorkflowDefinition;

    #[tokio::test]
    async fn test_acyclic_check() {
        // Create a capability manager
        let capability_manager = Arc::new(CapabilityManager::new().unwrap());

        // Create a runtime config
        let config = crate::system::config::RuntimeConfig::default();

        // Create a plugin manager
        let plugin_manager =
            Arc::new(PluginManager::new(config, capability_manager.clone()).unwrap());

        // Create an executor
        let executor = WorkflowExecutor::new(capability_manager, plugin_manager);

        // Create an acyclic workflow
        let acyclic = WorkflowDefinition {
            id: WorkflowId(Uuid::new_v4().to_string()),
            name: "Acyclic".to_string(),
            description: "Acyclic workflow".to_string(),
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
                Node {
                    id: "3".to_string(),
                    name: "Node 3".to_string(),
                    plugin_id: None,
                    function: "func3".to_string(),
                    dependencies: vec!["1".to_string(), "2".to_string()],
                },
            ],
        };

        // Create a cyclic workflow
        let cyclic = WorkflowDefinition {
            id: WorkflowId(Uuid::new_v4().to_string()),
            name: "Cyclic".to_string(),
            description: "Cyclic workflow".to_string(),
            nodes: vec![
                Node {
                    id: "1".to_string(),
                    name: "Node 1".to_string(),
                    plugin_id: None,
                    function: "func1".to_string(),
                    dependencies: vec!["3".to_string()],
                },
                Node {
                    id: "2".to_string(),
                    name: "Node 2".to_string(),
                    plugin_id: None,
                    function: "func2".to_string(),
                    dependencies: vec!["1".to_string()],
                },
                Node {
                    id: "3".to_string(),
                    name: "Node 3".to_string(),
                    plugin_id: None,
                    function: "func3".to_string(),
                    dependencies: vec!["2".to_string()],
                },
            ],
        };

        // Check acyclic
        assert!(executor.is_acyclic(&acyclic));

        // Check cyclic
        assert!(!executor.is_acyclic(&cyclic));
    }
}
