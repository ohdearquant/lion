# Lion Concurrency

`lion_concurrency` provides actor-based concurrency primitives for the Lion
microkernel system, enabling isolated components to communicate through message
passing with robust error handling and supervision.

## Features

- **Actor System**: Isolated actors with mailboxes for message passing
- **Supervision**: Hierarchical error handling and recovery strategies
- **Thread Pools**: Efficient worker thread management
- **Scheduling**: Task scheduling with various policies
- **Synchronization**: Atomic operations and lock-free data structures
- **Resource Management**: Pooling and limiting of concurrent resources

## Architecture

The concurrency system is built around several key components:

1. **Actors**: Independent units of computation with private state
   - Mailboxes: Message queues for actor communication
   - Supervisors: Monitor actors and handle failures
   - Systems: Coordinate multiple actors

2. **Pools**: Manage shared resources
   - Thread Pools: Reusable worker threads
   - Instance Pools: Shared resource instances
   - Resource Limits: Prevent resource exhaustion

3. **Schedulers**: Control task execution
   - Executors: Run tasks according to scheduling policies
   - Priorities: Task prioritization mechanisms

4. **Synchronization**: Thread-safe data access
   - Atomic Operations: Lock-free primitives
   - Locks: When necessary for complex state

## Usage

### Creating an Actor System

```rust
use lion_concurrency::actor::{ActorSystem, Actor, Message, Mailbox};
use std::sync::Arc;

// Define a message type
#[derive(Clone)]
struct Ping;

impl Message for Ping {
    type Response = String;
}

// Define an actor
struct EchoActor;

impl Actor for EchoActor {
    fn handle_message<M: Message>(&mut self, msg: M, ctx: &ActorContext) -> Result<M::Response, ActorError> {
        if let Some(ping) = msg.downcast_ref::<Ping>() {
            Ok("Pong!".to_string())
        } else {
            Err(ActorError::UnhandledMessage)
        }
    }
}

// Create an actor system
let system = ActorSystem::new("example-system");

// Spawn an actor
let actor_ref = system.spawn(EchoActor::new());

// Send a message and await response
let response = actor_ref.ask(Ping).await?;
assert_eq!(response, "Pong!");
```

### Using Thread Pools

```rust
use lion_concurrency::pool::{ThreadPool, ThreadPoolConfig};
use std::time::Duration;

// Create a thread pool
let pool = ThreadPool::new(ThreadPoolConfig {
    min_threads: 2,
    max_threads: 10,
    keep_alive: Duration::from_secs(60),
    ..Default::default()
});

// Submit tasks to the pool
let handle = pool.submit(|| {
    // Task logic here
    42
});

// Wait for the result
let result = handle.await?;
assert_eq!(result, 42);
```

### Scheduling Tasks

```rust
use lion_concurrency::scheduler::{Scheduler, Task, Priority};

// Create a scheduler
let scheduler = Scheduler::new();

// Schedule a task with priority
let task = Task::new(|| {
    // Task logic here
    println!("High priority task executed");
}, Priority::High);

scheduler.schedule(task);

// Run the scheduler
scheduler.run();
```

## Integration with Other Lion Crates

The concurrency system integrates with other Lion crates:

- **lion_core**: For core traits and types
- **lion_runtime**: For runtime orchestration
- **lion_workflow**: For workflow execution
- **lion_isolation**: For isolated task execution

## Performance Considerations

- **Mailbox Optimization**: Efficient message passing with minimal copying
- **Work Stealing**: Balanced load across worker threads
- **Backpressure**: Preventing system overload
- **Cooperative Scheduling**: Yielding to prevent CPU monopolization

## License

Licensed under the Apache License, Version 2.0 - see the
[LICENSE](../../LICENSE) file for details.
