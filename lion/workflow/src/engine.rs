//! Workflow execution engine.
//!
//! This module provides the core engine for executing workflows,
//! including parallel node processing and checkpointing.

use std::sync::Arc;
use std::time::{Duration, Instant};

use core::error::{Result, WorkflowError};
use concurrency::ConcurrencyManager;

use crate::{
    Workflow, ExecutionId, NodeId, Execution, ExecutionOptions,
    ExecutionStatus, NodeStatus, storage::WorkflowStorage,
    node::{NodeType, ErrorPolicy}
};

/// Configuration for the workflow engine.
#[derive(Clone, Debug)]
pub struct WorkflowConfig {
    /// Default max parallel nodes.
    pub default_max_parallel_nodes: usize,
    
    /// Default execution timeout, in milliseconds.
    pub default_timeout_ms: u64,
    
    /// Whether to continue execution if a node fails.
    pub default_continue_on_failure: bool,
    
    /// Whether to use checkpoints by default.
    pub default_use_checkpoints: bool,
    
    /// Default checkpoint interval, in milliseconds.
    pub default_checkpoint_interval_ms: u64,
    
    /// Maximum number of active executions.
    pub max_active_executions: usize,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            default_max_parallel_nodes: 4,
            default_timeout_ms: 300000, // 5 minutes
            default_continue_on_failure: false,
            default_use_checkpoints: false,
            default_checkpoint_interval_ms: 60000, // 1 minute
            max_active_executions: 100,
        }
    }
}

/// Core workflow execution engine.
pub struct WorkflowEngine {
    /// Concurrency manager for parallel execution.
    concurrency_manager: Arc<dyn ConcurrencyManager>,
    
    /// Storage for workflow state.
    storage: Arc<dyn WorkflowStorage>,
    
    /// Engine configuration.
    config: WorkflowConfig,
    
    /// Active executions.
    active_executions: std::sync::atomic::AtomicUsize,
}

impl WorkflowEngine {
    /// Create a new workflow engine.
    pub fn new(
        concurrency_manager: Arc<dyn ConcurrencyManager>,
        storage: Arc<dyn WorkflowStorage>,
        config: WorkflowConfig,
    ) -> Self {
        Self {
            concurrency_manager,
            storage,
            config,
            active_executions: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    
    /// Create a new execution.
    pub fn create_execution(
        &self,
        workflow: Workflow,
        input: serde_json::Value,
        options: ExecutionOptions,
    ) -> Result<Arc<Execution>> {
        // Check active execution limit
        let active = self.active_executions.load(std::sync::atomic::Ordering::SeqCst);
        if active >= self.config.max_active_executions {
            return Err(WorkflowError::ExecutionFailed(
                format!("Maximum active executions ({}) reached", self.config.max_active_executions)
            ).into());
        }
        
        // Create execution
        let execution_id = ExecutionId::new();
        let mut execution = Execution::new(
            execution_id,
            workflow,
            input,
            options,
        );
        
        // Start the execution
        execution.start()?;
        
        // Increment active executions
        self.active_executions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        // Wrap in Arc
        let execution = Arc::new(execution);
        
        // Start executing nodes
        self.execute_ready_nodes(execution.clone())?;
        
        Ok(execution)
    }
    
    /// Execute nodes that are ready.
    fn execute_ready_nodes(&self, execution: Arc<Execution>) -> Result<()> {
        // Find ready nodes
        let mut ready_nodes = Vec::new();
        for entry in execution.node_statuses.iter() {
            if let NodeStatus::Ready = entry.value() {
                ready_nodes.push(entry.key().clone());
            }
        }
        
        // Check execution status and cancellation
        if !matches!(execution.status(), ExecutionStatus::Running { .. }) || execution.is_cancelled() {
            return Ok(());
        }
        
        // Respect max parallel nodes
        let max_parallel = execution.options().max_parallel_nodes;
        
        // Count currently running nodes
        let mut running_nodes = 0;
        for entry in execution.node_statuses.iter() {
            if let NodeStatus::Running { .. } = entry.value() {
                running_nodes += 1;
            }
        }
        
        // Calculate how many more nodes we can start
        let can_start = if running_nodes < max_parallel {
            max_parallel - running_nodes
        } else {
            0
        };
        
        // Limit to what we can start
        if ready_nodes.len() > can_start {
            ready_nodes.truncate(can_start);
        }
        
        // Start each node
        for node_id in ready_nodes {
            let node = match execution.workflow().nodes.get(&node_id) {
                Some(node) => node.clone(),
                None => continue,
            };
            
            // Start the node in a new thread
            let engine = self.clone();
            let execution_clone = execution.clone();
            
            #[cfg(feature = "async")]
            {
                tokio::spawn(async move {
                    // Execute the node
                    let _ = engine.execute_node(execution_clone, node);
                });
            }
            
            #[cfg(not(feature = "async"))]
            {
                std::thread::spawn(move || {
                    // Execute the node
                    let _ = engine.execute_node(execution_clone, node);
                });
            }
            
            // Update node status to running
            let now = chrono::Utc::now();
            execution.update_node_status(&node_id, NodeStatus::Running {
                started_at: now,
                retry_count: 0,
            })?;
        }
        
        Ok(())
    }
    
    /// Execute a single node.
    fn execute_node(&self, execution: Arc<Execution>, node: WorkflowNode) -> Result<()> {
        // Check execution status and cancellation
        if !matches!(execution.status(), ExecutionStatus::Running { .. }) || execution.is_cancelled() {
            return Ok(());
        }
        
        // Execute the node
        let result = match &node.node_type {
            NodeType::PluginCall { plugin_id, function, input_mapping, output_mapping } => {
                self.execute_plugin_call(
                    &execution,
                    plugin_id,
                    function,
                    input_mapping,
                    output_mapping,
                )
            },
            NodeType::Condition { condition, true_branch, false_branch } => {
                self.execute_condition(
                    &execution,
                    condition,
                    true_branch,
                    false_branch,
                )
            },
            NodeType::Subflow { workflow_id, input_mapping, output_mapping } => {
                self.execute_subflow(
                    &execution,
                    workflow_id,
                    input_mapping,
                    output_mapping,
                )
            },
            NodeType::Custom { type_name, params } => {
                Err(WorkflowError::NodeExecutionFailed(
                    format!("Custom node type '{}' not implemented", type_name)
                ).into())
            },
        };
        
        // Handle the result
        match result {
            Ok(node_result) => {
                // Node executed successfully
                let now = chrono::Utc::now();
                let duration_ms = match execution.node_status(&node.id) {
                    Some(NodeStatus::Running { started_at, .. }) => {
                        let start = started_at.timestamp_millis();
                        let end = now.timestamp_millis();
                        (end - start) as u64
                    },
                    _ => 0,
                };
                
                // Update node status
                execution.update_node_status(&node.id, NodeStatus::Completed {
                    completed_at: now,
                    duration_ms,
                    results: node_result,
                })?;
                
                // Find and execute newly ready nodes
                self.execute_ready_nodes(execution.clone())?;
            },
            Err(err) => {
                // Node execution failed
                let now = chrono::Utc::now();
                let duration_ms = match execution.node_status(&node.id) {
                    Some(NodeStatus::Running { started_at, retry_count, .. }) => {
                        // Check if we should retry
                        match &node.config.error_policy {
                            ErrorPolicy::Retry { max_retries, retry_delay_ms, exponential_backoff } => {
                                if retry_count < *max_retries {
                                    // Calculate delay
                                    let delay = if *exponential_backoff {
                                        Duration::from_millis(*retry_delay_ms * 2u64.pow(retry_count as u32))
                                    } else {
                                        Duration::from_millis(*retry_delay_ms)
                                    };
                                    
                                    // Sleep for the delay
                                    std::thread::sleep(delay);
                                    
                                    // Update status for retry
                                    execution.update_node_status(&node.id, NodeStatus::Running {
                                        started_at: chrono::Utc::now(),
                                        retry_count: retry_count + 1,
                                    })?;
                                    
                                    // Retry the node
                                    let engine = self.clone();
                                    let execution_clone = execution.clone();
                                    let node_clone = node.clone();
                                    
                                    #[cfg(feature = "async")]
                                    {
                                        tokio::spawn(async move {
                                            let _ = engine.execute_node(execution_clone, node_clone);
                                        });
                                    }
                                    
                                    #[cfg(not(feature = "async"))]
                                    {
                                        std::thread::spawn(move || {
                                            let _ = engine.execute_node(execution_clone, node_clone);
                                        });
                                    }
                                    
                                    return Ok(());
                                }
                                
                                // Max retries reached, fail the node
                                let start = started_at.timestamp_millis();
                                let end = now.timestamp_millis();
                                (end - start) as u64
                            },
                            _ => {
                                // No retry, fail the node
                                let start = started_at.timestamp_millis();
                                let end = now.timestamp_millis();
                                (end - start) as u64
                            },
                        }
                    },
                    _ => 0,
                };
                
                let retry_count = match execution.node_status(&node.id) {
                    Some(NodeStatus::Running { retry_count, .. }) => retry_count,
                    _ => 0,
                };
                
                // Update node status
                execution.update_node_status(&node.id, NodeStatus::Failed {
                    failed_at: now,
                    duration_ms,
                    error: format!("{}", err),
                    retry_count,
                })?;
                
                // Check if we should continue
                if execution.options().continue_on_failure {
                    // Continue execution
                    self.execute_ready_nodes(execution.clone())?;
                } else {
                    // Fail the execution
                    let now = chrono::Utc::now();
                    let duration_ms = match execution.status() {
                        ExecutionStatus::Running { started_at } => {
                            let start = started_at.timestamp_millis();
                            let end = now.timestamp_millis();
                            (end - start) as u64
                        },
                        _ => 0,
                    };
                    
                    // Update execution status
                    execution.update_node_status(&node.id, NodeStatus::Failed {
                        failed_at: now,
                        duration_ms,
                        error: format!("{}", err),
                        retry_count,
                    })?;
                }
            },
        }
        
        Ok(())
    }
    
    /// Execute a plugin call node.
    fn execute_plugin_call(
        &self,
        execution: &Arc<Execution>,
        plugin_id: &PluginId,
        function: &str,
        input_mapping: &serde_json::Map<String, serde_json::Value>,
        output_mapping: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Build the input parameters from context
        let mut params = serde_json::Map::new();
        
        for (param, path) in input_mapping {
            // Resolve the path
            let value = match path {
                serde_json::Value::String(path) => {
                    execution.get_context_value(path).unwrap_or(serde_json::Value::Null)
                },
                value => value.clone(),
            };
            
            params.insert(param.clone(), value);
        }
        
        // Serialize parameters
        let params_json = serde_json::Value::Object(params);
        let params_bytes = serde_json::to_vec(&params_json)
            .map_err(|e| WorkflowError::NodeExecutionFailed(
                format!("Failed to serialize parameters: {}", e)
            ))?;
        
        // Call the function
        let result_bytes = self.concurrency_manager.call_function(
            plugin_id,
            function,
            &params_bytes,
        )?;
        
        // Deserialize result
        let result: serde_json::Value = serde_json::from_slice(&result_bytes)
            .map_err(|e| WorkflowError::NodeExecutionFailed(
                format!("Failed to deserialize result: {}", e)
            ))?;
        
        // Apply output mapping to context
        let mut context_updates = serde_json::Map::new();
        
        for (context_path, result_path) in output_mapping {
            // Resolve the result path
            let value = match result_path {
                serde_json::Value::String(path) => {
                    // Parse the path
                    let parts: Vec<&str> = path.split('.').collect();
                    if parts.is_empty() {
                        result.clone()
                    } else {
                        // Start with the root object
                        let mut current = &result;
                        
                        // Traverse the path
                        let mut valid_path = true;
                        for part in &parts {
                            match current {
                                serde_json::Value::Object(obj) => {
                                    if let Some(value) = obj.get(*part) {
                                        current = value;
                                    } else {
                                        valid_path = false;
                                        break;
                                    }
                                },
                                serde_json::Value::Array(arr) => {
                                    if let Ok(index) = part.parse::<usize>() {
                                        if let Some(value) = arr.get(index) {
                                            current = value;
                                        } else {
                                            valid_path = false;
                                            break;
                                        }
                                    } else {
                                        valid_path = false;
                                        break;
                                    }
                                },
                                _ => {
                                    valid_path = false;
                                    break;
                                }
                            }
                        }
                        
                        if valid_path {
                            current.clone()
                        } else {
                            serde_json::Value::Null
                        }
                    }
                },
                value => value.clone(),
            };
            
            context_updates.insert(context_path.clone(), value);
        }
        
        // Update the context
        execution.update_context(context_updates)?;
        
        Ok(result)
    }
    
    /// Execute a condition node.
    fn execute_condition(
        &self,
        execution: &Arc<Execution>,
        condition: &str,
        true_branch: &NodeId,
        false_branch: &NodeId,
    ) -> Result<serde_json::Value> {
        // Evaluate the condition
        // For now, we'll just use a simple check if the condition evaluates to a truthy value
        let result = execution.get_context_value(condition);
        let is_true = match result {
            Some(serde_json::Value::Bool(b)) => b,
            Some(serde_json::Value::Number(n)) => !n.is_zero(),
            Some(serde_json::Value::String(s)) => !s.is_empty(),
            Some(serde_json::Value::Array(a)) => !a.is_empty(),
            Some(serde_json::Value::Object(o)) => !o.is_empty(),
            _ => false,
        };
        
        // Determine the next branch
        let next_branch = if is_true { true_branch } else { false_branch };
        
        // Make the next branch ready
        execution.update_node_status(
            next_branch,
            NodeStatus::Ready,
        )?;
        
        // Return the condition result
        Ok(serde_json::Value::Bool(is_true))
    }
    
    /// Execute a subflow node.
    fn execute_subflow(
        &self,
        execution: &Arc<Execution>,
        workflow_id: &crate::WorkflowId,
        input_mapping: &serde_json::Map<String, serde_json::Value>,
        output_mapping: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Resolve input mapping
        let mut input = serde_json::Map::new();
        
        for (param, path) in input_mapping {
            // Resolve the path
            let value = match path {
                serde_json::Value::String(path) => {
                    execution.get_context_value(path).unwrap_or(serde_json::Value::Null)
                },
                value => value.clone(),
            };
            
            input.insert(param.clone(), value);
        }
        
        // Load the workflow
        let workflow = self.storage.get_workflow(workflow_id)?;
        
        // Create execution options
        let options = ExecutionOptions {
            max_parallel_nodes: execution.options().max_parallel_nodes,
            timeout_ms: execution.options().timeout_ms,
            continue_on_failure: execution.options().continue_on_failure,
            use_checkpoints: execution.options().use_checkpoints,
            checkpoint_interval_ms: execution.options().checkpoint_interval_ms,
        };
        
        // Execute the subflow
        let subflow_execution = self.create_execution(
            workflow,
            serde_json::Value::Object(input),
            options,
        )?;
        
        // Wait for the execution to complete
        // This is a blocking operation!
        let mut result = None;
        let start = Instant::now();
        let timeout = execution.options().timeout_ms.map(Duration::from_millis);
        
        loop {
            // Check for timeout
            if let Some(timeout) = timeout {
                if start.elapsed() > timeout {
                    return Err(WorkflowError::Timeout(timeout.as_millis() as u64).into());
                }
            }
            
            // Check execution status
            match subflow_execution.status() {
                ExecutionStatus::Completed { results, .. } => {
                    result = Some(results);
                    break;
                },
                ExecutionStatus::Failed { error, .. } => {
                    return Err(WorkflowError::NodeExecutionFailed(
                        format!("Subflow execution failed: {}", error)
                    ).into());
                },
                ExecutionStatus::Cancelled => {
                    return Err(WorkflowError::Cancelled.into());
                },
                _ => {
                    // Still running, wait a bit
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
            
            // Check if parent is cancelled
            if execution.is_cancelled() {
                subflow_execution.cancel()?;
                return Err(WorkflowError::Cancelled.into());
            }
        }
        
        // Apply output mapping to context
        let result = result.unwrap_or(serde_json::Value::Null);
        let mut context_updates = serde_json::Map::new();
        
        for (context_path, result_path) in output_mapping {
            // Resolve the result path
            let value = match result_path {
                serde_json::Value::String(path) => {
                    // Parse the path
                    let parts: Vec<&str> = path.split('.').collect();
                    if parts.is_empty() {
                        result.clone()
                    } else {
                        // Start with the root object
                        let mut current = &result;
                        
                        // Traverse the path
                        let mut valid_path = true;
                        for part in &parts {
                            match current {
                                serde_json::Value::Object(obj) => {
                                    if let Some(value) = obj.get(*part) {
                                        current = value;
                                    } else {
                                        valid_path = false;
                                        break;
                                    }
                                },
                                serde_json::Value::Array(arr) => {
                                    if let Ok(index) = part.parse::<usize>() {
                                        if let Some(value) = arr.get(index) {
                                            current = value;
                                        } else {
                                            valid_path = false;
                                            break;
                                        }
                                    } else {
                                        valid_path = false;
                                        break;
                                    }
                                },
                                _ => {
                                    valid_path = false;
                                    break;
                                }
                            }
                        }
                        
                        if valid_path {
                            current.clone()
                        } else {
                            serde_json::Value::Null
                        }
                    }
                },
                value => value.clone(),
            };
            
            context_updates.insert(context_path.clone(), value);
        }
        
        // Update the context
        execution.update_context(context_updates)?;
        
        Ok(result)
    }
}

impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            concurrency_manager: self.concurrency_manager.clone(),
            storage: self.storage.clone(),
            config: self.config.clone(),
            active_executions: std::sync::atomic::AtomicUsize::new(
                self.active_executions.load(std::sync::atomic::Ordering::SeqCst)
            ),
        }
    }
}