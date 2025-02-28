# Lion Workflow Engine

A capability-based workflow engine for the Lion microkernel architecture. This crate provides a secure, fault-tolerant, and efficient workflow system with support for directed acyclic graphs (DAGs), event-driven patterns, and distributed sagas.

## Features

- **Capability-Based Security**: Workflows and steps require specific capabilities to execute, ensuring strong isolation and security.
- **Fault Tolerance**: Robust error handling with checkpointing, at-least-once event delivery, saga compensation, and more.
- **Efficient Execution**: Prioritized scheduling, cooperative preemption, and backpressure for high performance.
- **DAG Workflows**: Define and execute workflows as directed acyclic graphs with conditional edges.
- **Event-Driven Patterns**: Publish-subscribe with at-least-once or exactly-once delivery semantics.
- **Saga Pattern**: Distributed transactions with compensation actions for partial failures.
- **State Persistence**: Checkpoint and restore workflow state with atomic guarantees.

## Architecture

The Lion Workflow Engine is structured around several key components:

1. **Model**: Core workflow definitions including nodes, edges, and graph structures.
2. **Engine**: Execution context, scheduler, and executor for running workflows.
3. **State**: State management, checkpointing, and storage backends.
4. **Patterns**: Higher-level workflow patterns like events and sagas.

## Getting Started

### Basic Workflow Definition

```rust
use lion_workflow::model::{WorkflowDefinition, Node, Edge, NodeId, EdgeId, WorkflowBuilder};
use std::sync::Arc;

// Define nodes
let node1 = Node::new(NodeId::new(), "Start".to_string());
let node2 = Node::new(NodeId::new(), "Process".to_string());
let node3 = Node::new(NodeId::new(), "End".to_string());

let node1_id = node1.id;
let node2_id = node2.id;
let node3_id = node3.id;

// Build workflow
let workflow = WorkflowBuilder::new("Example Workflow")
    .add_node(node1).unwrap()
    .add_node(node2).unwrap()
    .add_node(node3).unwrap()
    .add_edge(Edge::new(EdgeId::new(), node1_id, node2_id)).unwrap()
    .add_edge(Edge::new(EdgeId::new(), node2_id, node3_id)).unwrap()
    .build();
```

### Executing a Workflow

```rust
use lion_workflow::engine::{WorkflowExecutor, ExecutorConfig, NodeHandler};
use lion_workflow::state::{StateMachineManager, MemoryStorage};
use lion_workflow::engine::scheduler::{WorkflowScheduler, SchedulerConfig};
use lion_workflow::engine::context::NodeResult;
use std::sync::Arc;
use tokio::runtime::Runtime;

// Create execution components
let workflow_def = Arc::new(workflow);
let scheduler = Arc::new(WorkflowScheduler::new(SchedulerConfig::default()));
let state_manager = Arc::new(StateMachineManager::<MemoryStorage>::new());
let executor = WorkflowExecutor::new(scheduler, state_manager, ExecutorConfig::default());

// Register node handlers
Runtime::new().unwrap().block_on(async {
    executor.register_node_handler("Start", Arc::new(|ctx| {
        Box::new(async move {
            // Start node handler logic
            Ok(NodeResult::success(
                ctx.current_node_id.unwrap(),
                serde_json::json!({"message": "Started"}),
            ))
        })
    })).await;
    
    // Register more handlers...
    
    // Start the executor
    executor.start().await.unwrap();
    
    // Execute the workflow
    let instance_id = executor.execute_workflow(workflow_def).await.unwrap();
    println!("Workflow instance started: {}", instance_id);
});
```

### Event-Driven Pattern

```rust
use lion_workflow::patterns::{Event, EventBroker, EventBrokerConfig, InMemoryEventStore};
use std::sync::Arc;

// Create an event broker
let event_store = Arc::new(InMemoryEventStore::new());
let event_broker = EventBroker::new(EventBrokerConfig::default())
    .with_event_store(event_store);

// Subscribe to events
let (mut event_rx, ack_tx) = event_broker.subscribe(
    "order.created", 
    "order-processor",
    None,
).await.unwrap();

// Publish an event
let event = Event::new(
    "order.created",
    serde_json::json!({"order_id": "12345", "customer": "ABC Corp"}),
);
event_broker.publish(event).await.unwrap();

// Process events
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        println!("Received event: {}", event.id);
        
        // Process the event...
        
        // Acknowledge receipt
        let ack = EventAck::success(&event.id, "order-processor");
        ack_tx.send(ack).await.unwrap();
    }
});
```

### Saga Pattern

```rust
use lion_workflow::patterns::{SagaManager, SagaManagerConfig, SagaDefinitionBuilder, SagaStep};
use std::time::Duration;

// Create a saga definition
let saga = SagaDefinitionBuilder::new("order-saga", "Order Processing")
    .add_step(
        SagaStep::new(
            "create-order",
            "Create Order",
            serde_json::json!({"action": "create_order", "data": {...}}),
            "order-service",
        )
        .with_compensation(serde_json::json!({"action": "cancel_order", "data": {...}}))
    )
    .add_step(
        SagaStep::new(
            "reserve-inventory",
            "Reserve Inventory",
            serde_json::json!({"action": "reserve_inventory", "data": {...}}),
            "inventory-service",
        )
        .with_compensation(serde_json::json!({"action": "release_inventory", "data": {...}}))
    )
    .add_step(
        SagaStep::new(
            "process-payment",
            "Process Payment",
            serde_json::json!({"action": "charge_payment", "data": {...}}),
            "payment-service",
        )
        .with_compensation(serde_json::json!({"action": "refund_payment", "data": {...}}))
    )
    .build();

// Register saga with manager
let saga_manager = SagaManager::new(event_broker, SagaManagerConfig::default());
saga_manager.register_definition(saga).await.unwrap();

// Create and start a saga instance
let instance = saga_manager.create_instance(
    "order-saga",
    Some(serde_json::json!({"order_id": "12345"})),
).await.unwrap();
saga_manager.start_instance(&instance.id).await.unwrap();
```

## Advanced Features

### Checkpointing and State Persistence

```rust
use lion_workflow::state::{CheckpointManager, FileStorage};
use std::path::PathBuf;

// Create a file-based checkpoint manager
let checkpoint_dir = PathBuf::from("/var/lib/lion/checkpoints");
let checkpoint_manager = CheckpointManager::with_file_storage(
    checkpoint_dir,
    "1.0.0", // Schema version
).unwrap();

// Save a workflow checkpoint
let checkpoint_id = checkpoint_manager.save_checkpoint(&workflow).await.unwrap();

// Load the most recent checkpoint
let workflow = checkpoint_manager.load_latest_checkpoint(&workflow.id).await.unwrap();
```

### Advanced Scheduling

```rust
use lion_workflow::engine::scheduler::{SchedulerConfig, SchedulingPolicy, Priority};

// Create a scheduler with EDF policy (Earliest Deadline First)
let scheduler_config = SchedulerConfig {
    max_queued_tasks: 1000,
    max_concurrent_tasks: 50,
    policy: SchedulingPolicy::EDF,
    ..Default::default()
};

let scheduler = WorkflowScheduler::new(scheduler_config);

// Change policy at runtime
scheduler.change_policy(SchedulingPolicy::Priority).await.unwrap();
```

## Security Model

The Lion Workflow Engine uses Lion's capability-based security model to ensure that workflows only access resources they have explicit permission to use. Each workflow node requires specific capabilities to execute, and cross-component communication is mediated by capability tokens.

For example, a workflow step that needs to access a database would require a specific database capability, which must be explicitly granted. This follows the principle of least privilege, minimizing the potential damage from compromised components.

## Performance Considerations

The engine is designed for high performance while maintaining security and fault tolerance:

- **Efficient Scheduling**: Priority-based, MLFQ, or EDF scheduling with backpressure
- **Cooperative Preemption**: Tasks yield periodically to prevent CPU monopolization
- **Memory Efficiency**: Adjacency list representation for workflow graphs minimizes memory usage
- **Concurrency**: Actor-based model prevents lock contention and enables parallelism
- **Zero-Copy Messaging**: When possible, avoids unnecessary data copying between components

## Contributing

Contributions to the Lion Workflow Engine are welcome! Please see our [contribution guidelines](../CONTRIBUTING.md) for more information.

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.