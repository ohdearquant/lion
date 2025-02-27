//! Lion Workflow Engine
//!
//! A capability-based workflow engine for the Lion microkernel architecture. This crate provides
//! a secure, fault-tolerant, and efficient workflow engine with support for various workflow
//! patterns like directed acyclic graphs, events, and distributed sagas.
//!
//! # Features
//!
//! - Capability-based security: workflows and steps require specific capabilities to execute
//! - Fault tolerance: checkpointing, at-least-once event delivery, saga compensation, etc.
//! - Efficient execution: prioritized scheduling, cooperative preemption, backpressure
//! - Graph-based workflows: directed acyclic graph (DAG) execution with dynamic updates
//! - Event-driven patterns: publish-subscribe with at-least-once/exactly-once semantics
//! - Saga pattern: distributed transactions with compensation for partial failures
//!
//! # Getting Started
//!
//! ```rust,no_run
//! use lion_workflow::model::{WorkflowDefinition, Node, Edge, NodeId, WorkflowBuilder};
//! use lion_workflow::engine::{WorkflowExecutor, ExecutorConfig, NodeHandler};
//! use lion_workflow::state::{StateMachineManager, CheckpointManager, MemoryStorage};
//! use lion_workflow::engine::scheduler::{WorkflowScheduler, SchedulerConfig};
//! use std::sync::Arc;
//!
//! // Define a simple workflow
//! let mut builder = WorkflowBuilder::new("Example Workflow");
//! let node1 = Node::new(NodeId::new(), "Start".to_string());
//! let node2 = Node::new(NodeId::new(), "Process".to_string());
//! let node3 = Node::new(NodeId::new(), "End".to_string());
//!
//! let node1_id = node1.id;
//! let node2_id = node2.id;
//! let node3_id = node3.id;
//!
//! let workflow = builder
//!     .add_node(node1).unwrap()
//!     .add_node(node2).unwrap()
//!     .add_node(node3).unwrap()
//!     .add_edge(Edge::new(EdgeId::new(), node1_id, node2_id)).unwrap()
//!     .add_edge(Edge::new(EdgeId::new(), node2_id, node3_id)).unwrap()
//!     .build();
//!
//! // Create execution components
//! let workflow_def = Arc::new(workflow);
//! let scheduler = Arc::new(WorkflowScheduler::new(SchedulerConfig::default()));
//! let state_manager = Arc::new(StateMachineManager::<MemoryStorage>::new());
//! let executor = WorkflowExecutor::new(scheduler, state_manager, ExecutorConfig::default());
//!
//! // Start the executor
//! tokio::runtime::Runtime::new().unwrap().block_on(async {
//!     executor.start().await.unwrap();
//!     
//!     // Execute the workflow
//!     let instance_id = executor.execute_workflow(workflow_def).await.unwrap();
//!     println!("Workflow instance started: {}", instance_id);
//! });
//! ```

/// Core model types and definitions for workflows
pub mod model;

/// State management and persistence
pub mod state;

/// Workflow execution engine
pub mod engine;

/// Common workflow patterns
pub mod patterns;

// Re-export important types
pub use model::{WorkflowDefinition, WorkflowId, WorkflowError, Node, NodeId, NodeStatus, Edge, EdgeId, WorkflowBuilder};
pub use state::{WorkflowState, StateMachineManager, CheckpointManager, StorageBackend, MemoryStorage, FileStorage};
pub use engine::{WorkflowExecutor, ExecutorConfig, WorkflowScheduler, SchedulerConfig, ExecutionContext, NodeResult, TaskStatus, SchedulingPolicy};
pub use patterns::{Event, EventBroker, SagaManager, SagaDefinition, SagaInstance, SagaDefinitionBuilder};

/// Error types from across the workflow engine
pub mod error {
    pub use crate::model::WorkflowError;
    pub use crate::state::{StateMachineError, CheckpointError, StorageError};
    pub use crate::engine::{ExecutorError, SchedulerError, ContextError};
    pub use crate::patterns::{EventError, SagaError};
}

/// Create a new workflow definition
pub fn create_workflow(name: &str) -> WorkflowBuilder {
    WorkflowBuilder::new(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Node, Edge, NodeId, EdgeId};
    
    #[test]
    fn test_create_workflow() {
        let mut builder = create_workflow("Test Workflow");
        let node1 = Node::new(NodeId::new(), "Node 1".to_string());
        let node2 = Node::new(NodeId::new(), "Node 2".to_string());
        
        let node1_id = node1.id;
        let node2_id = node2.id;
        
        let workflow = builder
            .add_node(node1).unwrap()
            .add_node(node2).unwrap()
            .add_edge(Edge::new(EdgeId::new(), node1_id, node2_id)).unwrap()
            .build();
        
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.nodes.len(), 2);
        assert_eq!(workflow.edges.len(), 1);
    }
}