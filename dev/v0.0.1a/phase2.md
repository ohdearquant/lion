Below is an **exhaustively detailed** instruction set for **Phase 2** of the lion project. Building on Phase 1’s foundation (workspace setup, core primitives, and basic CLI), Phase 2 introduces the **microkernel orchestrator** and **system-level events** for handling tasks in an event-driven manner. All references to coding style, concurrency, and security come from the overarching **lion Developer Guide**.

---

# Phase 2 – Microkernel Orchestrator & System Events

## 1. High-Level Objectives

1. **Introduce an Orchestrator**:  
   - Implement either an actor-based or a custom event-loop approach for handling system events (e.g., `TaskSubmitted`, `TaskCompleted`).  
   - This orchestrator or “microkernel” acts as the central dispatcher.

2. **Define & Test Basic System Events**:  
   - Create a `SystemEvent` enum describing events like “task submitted,” “task completed,” “error occurred,” etc.

3. **Add a Simple Event Flow**:  
   - Submitting a “task” leads the orchestrator to produce a “completed” event, simulating minimal “work.”

4. **Extend the CLI**:  
   - Possibly add a new command `submit-task --data <string>` that sends a “TaskSubmitted” event to the orchestrator.  
   - Optionally, a `run-orchestrator` command that blocks and processes events (if using a separate process approach).

5. **Validation**:  
   - Unit tests to confirm the orchestrator logic works (actor messages or channel-based event loop).  
   - CLI/integration tests showing a user can submit tasks and see them completed.

**Expected Outcome**  
By the end of Phase 2, the codebase will have a **functioning orchestrator** that receives system events, simulates minimal concurrency or scheduling logic, and logs or outputs “TaskCompleted” events. We tag the final commit as `v0.0.1a-phase2`.

---

## 2. Technical Requirements & Outcomes

1. **Orchestrator Module**  
   - A new file `orchestrator.rs` in `agentic_core/src/` implementing either:
     - An **actor model** (using [Actix](https://actix.rs/) or ractor)  
     - A **custom Tokio-based** event loop with a `mpsc::Sender<SystemEvent>` and `mpsc::Receiver<SystemEvent>`

2. **SystemEvent Enum**  
   - Located in `orchestrator.rs` or a separate file like `events.rs`.
   - Variants to model minimal workflows, e.g.:
     ```rust
     pub enum SystemEvent {
         TaskSubmitted { task_id: Uuid, payload: String },
         TaskCompleted { task_id: Uuid, result: String },
         // ...
     }
     ```

3. **Event Flow**  
   - On receiving `TaskSubmitted`, the orchestrator simulates “work” (like a short async delay or immediate “TaskCompleted”).  
   - This confirms the “submit → handle → complete” flow.

4. **CLI Enhancements**  
   - `submit-task --data "..."` to send the orchestrator a “TaskSubmitted” event.  
   - Possibly `run-orchestrator` if you want an interactive orchestrator session (optional at Phase 2).  
   - Or a single “agentic-cli submit-task --data ...” command that internally calls orchestrator code (if no separate process is needed).

5. **Testing**  
   - Unit tests for orchestrator logic (mock or actual event channels).  
   - Integration tests for the CLI to ensure “task submitted → eventually completed.”

---

## 3. Step-by-Step Implementation

This phase can be split into four major tasks: **(A) Setting up the Orchestrator Approach**, **(B) Defining System Events**, **(C) Integrating the CLI**, and **(D) Validation & Tests**.

### 3.A. **Setting Up the Orchestrator Approach**

1. **Decide on Actor vs. Custom Event Loop**  
   - **Actor-based** (e.g., using [Actix](https://actix.rs/)):
     - You’ll create an `Orchestrator` struct that implements `Actor` and handle messages that correspond to your `SystemEvent`.
     - Great for easily spawning multiple “agent” actors in later phases.
   - **Custom** (using raw Tokio `mpsc::channel`):
     - A `struct Orchestrator` with `event_rx: mpsc::Receiver<SystemEvent>`, plus a `run()` method processing events in a loop with `select!`.

2. **Add Dependencies** (If Actor-based)  
   - In `agentic_core/Cargo.toml`:
     ```toml
     [dependencies]
     actix = "0.12"       # or latest
     actix-rt = "2.8"     # for the runtime
     ```
   - Or if purely custom, ensure `[dependencies] tokio = { version = "1", features = ["rt-multi-thread", "macros"] }`.

3. **Create `orchestrator.rs`**  
   - `agentic_core/src/orchestrator.rs`

**Actor Example**:
```rust
// agentic_core/src/orchestrator.rs
use actix::prelude::*;
use uuid::Uuid;
use tracing::{info, error};

#[derive(Debug)]
pub enum SystemEvent {
    TaskSubmitted { task_id: Uuid, payload: String },
    TaskCompleted { task_id: Uuid, result: String },
}

pub struct Orchestrator;

impl Actor for Orchestrator {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OrchestratorEvent(pub SystemEvent);

impl Handler<OrchestratorEvent> for Orchestrator {
    type Result = ();

    fn handle(&mut self, msg: OrchestratorEvent, _ctx: &mut Context<Self>) -> Self::Result {
        match msg.0 {
            SystemEvent::TaskSubmitted { task_id, payload } => {
                info!("Orchestrator received TaskSubmitted({task_id}, {payload})");
                // Simulate immediate completion:
                let result = format!("Processed: {payload}");
                // Possibly notify other actors or store data
                info!("TaskCompleted for {task_id}");
            },
            SystemEvent::TaskCompleted { task_id, result } => {
                info!("Task Completed: {task_id} => {result}");
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::System;

    #[actix::test]
    async fn test_orchestrator_flow() {
        let addr = Orchestrator.start();
        let task_id = Uuid::new_v4();
        addr.do_send(OrchestratorEvent(SystemEvent::TaskSubmitted {
            task_id,
            payload: "Hello".into(),
        }));
        // Additional checks: see logs or store results in future phases
        // For now, just ensure no crash
    }
}
```

**Custom Tokio Loop Example**:
```rust
// agentic_core/src/orchestrator.rs
use uuid::Uuid;
use tokio::sync::mpsc;
use tracing::{info};

#[derive(Debug)]
pub enum SystemEvent {
    TaskSubmitted { task_id: Uuid, payload: String },
    TaskCompleted { task_id: Uuid, result: String },
}

pub struct Orchestrator {
    event_tx: mpsc::Sender<SystemEvent>,
    event_rx: mpsc::Receiver<SystemEvent>,
}

impl Orchestrator {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self { event_tx: tx, event_rx: rx }
    }

    pub fn sender(&self) -> mpsc::Sender<SystemEvent> {
        self.event_tx.clone()
    }

    pub async fn run(mut self) {
        while let Some(event) = self.event_rx.recv().await {
            match event {
                SystemEvent::TaskSubmitted { task_id, payload } => {
                    info!("Received TaskSubmitted({task_id}, {payload})");
                    // Simulate immediate “work”
                    let completed = SystemEvent::TaskCompleted {
                        task_id,
                        result: format!("Processed: {payload}"),
                    };
                    // Send it back into the queue or handle it here
                    info!("TaskCompleted: {task_id}");
                },
                SystemEvent::TaskCompleted { task_id, result } => {
                    info!("Task Completed: {task_id} => {result}");
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::task;

    #[tokio::test]
    async fn test_orchestrator_flow() {
        let orchestrator = Orchestrator::new();
        let sender = orchestrator.sender();
        task::spawn(orchestrator.run());

        let task_id = Uuid::new_v4();
        sender.send(SystemEvent::TaskSubmitted {
            task_id,
            payload: "Hello".into(),
        }).await.unwrap();

        // A more advanced test would check logs, or add code to read “TaskCompleted”
        // For now, just ensure no panic
    }
}
```

4. **Expose the Orchestrator**  
   - In `agentic_core/src/lib.rs` add:
     ```rust
     pub mod element;
     pub mod pile;
     pub mod progression;
     pub mod store;
     pub mod orchestrator;  // new
     ```
   - Ensure it compiles and tests pass.

### 3.B. **Defining System Events**

If you want a separate file for events:

- `events.rs` (optional) with:
  ```rust
  #[derive(Debug)]
  pub enum SystemEvent {
      TaskSubmitted { task_id: Uuid, payload: String },
      TaskCompleted { task_id: Uuid, result: String },
  }
  ```
But typically, you can keep it inside `orchestrator.rs` for now.

### 3.C. **Integrating the CLI**

1. **Extend or Add Commands** in `agentic_cli/src/main.rs`:
   - Option A: Provide a `submit-task` subcommand that directly calls the orchestrator’s logic in-process.
   - Option B: Provide a `run-orchestrator` command to start an actor or event loop. Then a separate `submit-task` command triggers events (e.g., by sending via a channel, or Actix mailbox).

**Option A** – Single-process approach:
```rust
// agentic_cli/src/main.rs
use clap::{Parser, Subcommand};
use agentic_core::orchestrator::{Orchestrator, SystemEvent};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Parser)]
#[command(name="lion-cli", version="0.0.1a")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    SubmitTask { data: String },
    // Possibly other commands from Phase 1
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let orchestrator = Orchestrator::new();
    let sender = orchestrator.sender();
    // Spawn the orchestrator’s run loop
    tokio::spawn(orchestrator.run());

    match cli.command {
        Commands::SubmitTask { data } => {
            let task_id = Uuid::new_v4();
            sender.send(SystemEvent::TaskSubmitted {
                task_id,
                payload: data,
            }).await.unwrap();
            println!("Submitted task with ID: {task_id}");
        }
    }
}
```
- *Note*: This ephemeral approach means the orchestrator stops once the CLI command ends, unless you block the main thread or run something persistent.

**Option B** – Separate commands:
- `run-orchestrator`: blocks in an infinite loop waiting for events.  
- `submit-task`: spawns a new process that sends an IPC request or writes to a known channel. This is more advanced and typically requires a message bus or some external channel.

2. **Manual Testing**:
   - `cargo run -p agentic_cli -- submit-task --data "Hello Phase2"`
   - Expect logs: “Received TaskSubmitted(...)”, “TaskCompleted(...)”.

### 3.D. **Validation & Tests**

1. **Compile & Lint**  
   ```bash
   cargo fmt --all
   cargo clippy --all-targets
   ```
   Fix issues as needed.

2. **Orchestrator Unit Tests**  
   - Already included in the snippet above (`test_orchestrator_flow`).  
   - For Actix-based approach, you can confirm messages are handled.  
   - For custom approach, confirm events are processed as expected.

3. **Integration Test** (CLI Approach)  
   - Possibly create `tests/orchestrator_integration.rs`:
     ```rust
     #[test]
     fn test_submit_task_command() {
         // Use std::process::Command to run:
         // cargo run -p agentic_cli -- submit-task --data "test"
         // Check output. For now, we just confirm the process returns 0 exit code
     }
     ```
   - Or do a manual test by launching the CLI with multiple tasks.

4. **Phase 2 Tag**  
   - Once stable, tag final commit as `v0.0.1a-phase2`:
     ```bash
     git tag v0.0.1a-phase2
     git push origin v0.0.1a-phase2
     ```
   - Write a short phase-level report in `docs/phase2_report.md` describing:
     - The new `SystemEvent` enum  
     - Orchestrator logic (actor or custom)  
     - Updated CLI  
     - Tests performed

---

## 4. Common Pitfalls & Troubleshooting

1. **Orchestrator Doesn’t “Stay Alive”**  
   - If you do ephemeral approach with a single CLI call, the orchestrator might exit right after. That’s okay for Phase 2. In later phases, you might run a persistent orchestrator.

2. **Actix Actor Startup Issues**  
   - If using Actix, ensure the `#[actix::test]` or normal `actix::System::run()` environment is properly set. If you get “no current reactor” errors, you need the Actix or tokio runtime.

3. **Missing Dependencies**  
   - Confirm you have the right `[dependencies]` in `Cargo.toml`, e.g., `actix`, `tokio`, `uuid`, etc.

4. **Interference with Phase 1’s Store**  
   - The Phase 1 store is unaffected by the orchestrator. Phase 2 focuses on event flows, not storing tasks yet. Integration with `InMemoryStore` can happen in later phases if you want the orchestrator to persist tasks.

5. **Logging**  
   - Use `tracing` or at least `println!` to see if the orchestrator processes events. If logs aren’t showing, check that your logging/tracing subscriber is set up (in `main` or tests).

---

## 5. Next Steps (Transition to Phase 3)

After Phase 2, you have:

- A **microkernel orchestrator** that processes basic system events.
- A new or updated CLI subcommand `submit-task` or `run-orchestrator`.
- Verified concurrency using tests.

**Phase 3** will introduce **Event Sourcing & Explainability**:

- Appending every `SystemEvent` to an immutable log or DB.
- Possibly replaying events to rebuild state.
- Further structured logging and correlation IDs.

With Phase 2 complete, your code can handle minimal tasks in an event-driven manner—**a foundation** for deeper concurrency and multi-agent capabilities in future phases.

---

# Phase 2 Summary

- **Goal**: Implement a minimal orchestrator for system events.  
- **Outcome**: The CLI can submit tasks, and the orchestrator simulates a “TaskCompleted” event.  
- **Validation**: Through unit tests (or actor tests) and CLI integration tests.  
- **Milestone**: Tag as `v0.0.1a-phase2` upon successful demonstration and documentation in a phase report.

**Congratulations!** You now have a working microkernel orchestrator in Rust. On to **Phase 3** for event sourcing and advanced logging.