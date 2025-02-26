//! Workflow trait definitions.
//! 
//! This module defines the core traits for the workflow system.

use serde_json::Value;

use crate::error::{Result, WorkflowError};
use crate::id::{WorkflowId, ExecutionId, NodeId};
use crate::types::{Workflow, WorkflowNode, ExecutionStatus, NodeStatus, ExecutionOptions};

/// Core trait for workflow execution.
///
/// A workflow engine is responsible for executing workflows.
///
/// # Examples
///
/// ```
/// use lion_core::traits::WorkflowEngine;
/// use lion_core::error::{Result, WorkflowError};
/// use lion_core::id::{WorkflowId, ExecutionId, NodeId};
/// use lion_core::types::{Workflow, ExecutionStatus, ExecutionOptions};
/// use serde_json::Value;
///
/// struct DummyWorkflowEngine;
///
/// impl WorkflowEngine for DummyWorkflowEngine {
///     fn execute_workflow(
///         &self,
///         workflow_id: &WorkflowId,
///         input: Value,
///         options: ExecutionOptions,
///     ) -> Result<ExecutionId> {
///         // In a real implementation, we would execute the workflow
///         let execution_id = ExecutionId::new();
///         println!("Executed workflow: {}", workflow_id);
///         Ok(execution_id)
///     }
///
///     fn get_execution_status(&self, execution_id: &ExecutionId) -> Result<ExecutionStatus> {
///         // In a real implementation, we would get the status
///         Ok(ExecutionStatus::Pending)
///     }
///
///     fn cancel_execution(&self, execution_id: &ExecutionId) -> Result<()> {
///         // In a real implementation, we would cancel the execution
///         println!("Cancelled execution: {}", execution_id);
///         Ok(())
///     }
/// }
/// ```
pub trait WorkflowEngine: Send + Sync {
    /// Execute a workflow.
    ///
    /// # Arguments
    ///
    /// * `workflow_id` - The ID of the workflow to execute.
    /// * `input` - The input data for the workflow.
    /// * `options` - Execution options.
    ///
    /// # Returns
    ///
    /// * `Ok(ExecutionId)` - The ID of the execution.
    /// * `Err(WorkflowError)` - If the workflow could not be executed.
    fn execute_workflow(
        &self,
        workflow_id: &WorkflowId,
        input: Value,
        options: ExecutionOptions,
    ) -> Result<ExecutionId>;
    
    /// Get the status of a workflow execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution to check.
    ///
    /// # Returns
    ///
    /// * `Ok(ExecutionStatus)` - The current status of the execution.
    /// * `Err(WorkflowError)` - If the status could not be retrieved.
    fn get_execution_status(&self, execution_id: &ExecutionId) -> Result<ExecutionStatus>;
    
    /// Get the status of a node in a workflow execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution to check.
    /// * `node_id` - The ID of the node to check.
    ///
    /// # Returns
    ///
    /// * `Ok(NodeStatus)` - The current status of the node.
    /// * `Err(WorkflowError)` - If the status could not be retrieved.
    fn get_node_status(&self, execution_id: &ExecutionId, node_id: &NodeId) -> Result<NodeStatus> {
        Err(WorkflowError::ExecutionNotFound(execution_id.to_string()).into())
    }
    
    /// Cancel a workflow execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution to cancel.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the execution was successfully cancelled.
    /// * `Err(WorkflowError)` - If the execution could not be cancelled.
    fn cancel_execution(&self, execution_id: &ExecutionId) -> Result<()>;
    
    /// Wait for a workflow execution to complete.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution to wait for.
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - The result of the execution.
    /// * `Err(WorkflowError)` - If waiting failed or the execution failed.
    fn wait_for_completion(&self, execution_id: &ExecutionId) -> Result<Value> {
        Err(WorkflowError::ExecutionNotFound(execution_id.to_string()).into())
    }
    
    /// Get the results of a completed workflow execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution to get results for.
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - The results of the execution.
    /// * `Err(WorkflowError)` - If the results could not be retrieved.
    fn get_execution_results(&self, execution_id: &ExecutionId) -> Result<Value> {
        Err(WorkflowError::ExecutionNotFound(execution_id.to_string()).into())
    }
}

/// Trait for workflow storage.
pub trait WorkflowStorage: Send + Sync {
    /// Save a workflow.
    ///
    /// # Arguments
    ///
    /// * `workflow` - The workflow to save.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the workflow was successfully saved.
    /// * `Err(WorkflowError)` - If the workflow could not be saved.
    fn save_workflow(&self, workflow: &Workflow) -> Result<()>;
    
    /// Get a workflow by ID.
    ///
    /// # Arguments
    ///
    /// * `workflow_id` - The ID of the workflow to get.
    ///
    /// # Returns
    ///
    /// * `Ok(Workflow)` - The requested workflow.
    /// * `Err(WorkflowError)` - If the workflow could not be retrieved.
    fn get_workflow(&self, workflow_id: &WorkflowId) -> Result<Workflow>;
    
    /// Delete a workflow.
    ///
    /// # Arguments
    ///
    /// * `workflow_id` - The ID of the workflow to delete.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the workflow was successfully deleted.
    /// * `Err(WorkflowError)` - If the workflow could not be deleted.
    fn delete_workflow(&self, workflow_id: &WorkflowId) -> Result<()>;
    
    /// List all workflows.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Workflow>)` - All stored workflows.
    /// * `Err(WorkflowError)` - If the workflows could not be listed.
    fn list_workflows(&self) -> Result<Vec<Workflow>>;
    
    /// Save execution state.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution to save state for.
    /// * `state` - The execution state to save.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the state was successfully saved.
    /// * `Err(WorkflowError)` - If the state could not be saved.
    fn save_execution_state(&self, execution_id: &ExecutionId, state: Value) -> Result<()>;
    
    /// Get execution state.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution to get state for.
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - The execution state.
    /// * `Err(WorkflowError)` - If the state could not be retrieved.
    fn get_execution_state(&self, execution_id: &ExecutionId) -> Result<Value>;
}