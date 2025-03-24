**Stage 1: Foundation and Core Functionality (v0.0.1-alpha)**

**Overall Goal:** Establish a working Rust-based microkernel with core
primitives, event handling, a basic plugin system, multi-agent concurrency
demonstration, and initial CLI tooling. This stage focused on building a solid,
testable, and extensible base for future development.

**Timeline:** Stage 1 encompassed Phases 1 through 5, each with a specific
focus.

**Key Events and Decisions (Chronological Order, referencing Phases):**

**Phase 1: Workspace Setup & Core Primitives (v0.0.1a-phase1)**

1. **Project Initialization:**
   - **Decision:** Create a Rust workspace with two crates: `agentic_core`
     (library) and `agentic_cli` (binary).
   - **Rationale:** This structure separates core logic from the command-line
     interface, promoting modularity and maintainability. It aligns with Rust
     best practices for larger projects.
   - **Event:** Execution of `cargo new` commands to generate the workspace and
     crates.
   - **Files Affected:** `Cargo.toml` (root), `agentic_core/Cargo.toml`,
     `agentic_core/src/lib.rs`, `agentic_cli/Cargo.toml`,
     `agentic_cli/src/main.rs`.

2. **Core Data Structures:**
   - **Decision:** Define `ElementData`, `Pile<T>`, and `Progression` structs.
     - `ElementData`: Represents a fundamental unit of data with a UUID,
       creation timestamp, and arbitrary metadata.
     - `Pile<T>`: A thread-safe container for storing objects by UUID, using
       `Arc<Mutex<HashMap<Uuid, T>>>`.
     - `Progression`: An ordered sequence of UUIDs, also thread-safe using
       `Arc<Mutex<Vec<Uuid>>>`.
   - **Rationale:** These structures provide the building blocks for managing
     data within the microkernel. `Pile<T>` and `Progression` are designed for
     concurrent access, anticipating future multi-agent scenarios. `ElementData`
     provides a flexible way to represent diverse data types.
   - **Event:** Creation of `element.rs`, `pile.rs`, and `progression.rs` within
     `agentic_core/src`.
   - **Files Affected:** `agentic_core/src/element.rs`,
     `agentic_core/src/pile.rs`, `agentic_core/src/progression.rs`.

3. **In-Memory Store:**
   - **Decision:** Implement `InMemoryStore` using `Pile<ElementData>`.
   - **Rationale:** Provides a simple, ephemeral storage mechanism for
     `ElementData` objects, suitable for initial development and testing. It
     leverages the thread-safe `Pile` for data management.
   - **Event:** Creation of `store.rs` within `agentic_core/src`.
   - **Files Affected:** `agentic_core/src/store.rs`.

4. **Basic CLI:**
   - **Decision:** Create a CLI with `create-element` and `list-elements`
     commands.
   - **Rationale:** Allows basic interaction with the `InMemoryStore`, enabling
     creation and listing of elements. This provides a way to manually test the
     core functionality. `clap` crate was chosen for argument parsing.
   - **Event:** Modification of `agentic_cli/src/main.rs` to include
     command-line parsing and interaction with `InMemoryStore`.
   - **Files Affected:** `agentic_cli/src/main.rs`.

5. **Testing:**
   - **Decision:** Implement unit tests for all core primitives and integration
     tests for the CLI.
   - **Rationale:** Ensures correctness and thread safety of the fundamental
     building blocks. Early testing promotes stability and reduces bugs later in
     the development cycle.
   - **Event:** Addition of `#[cfg(test)]` blocks within each module and
     creation of integration tests (optional in Phase 1, but implemented).
   - **Files Affected:** All `.rs` files within `agentic_core/src` and
     `agentic_cli/src/main.rs`, potentially `tests/` directory.

6. **Project Configuration and Tooling:**
   - **Decision:** Add `.rustfmt.toml`, `.gitignore`, GitHub issue/PR templates,
     and a CI workflow (`rust-ci.yml`).
   - **Rationale:** Enforces consistent code style, manages repository hygiene,
     standardizes contributions, and automates build and test processes.
   - **Event:** Creation of configuration files and the `.github` directory.
   - **Files Affected:** `.rustfmt.toml`, `.gitignore`,
     `.github/workflows/rust-ci.yml`, `.github/ISSUE_TEMPLATE/*`,
     `.github/PULL_REQUEST_TEMPLATE.md`.

7. **Phase 1 Completion:**
   - **Decision:** Tag the commit as `v0.0.1a-phase1`.
   - **Rationale:** Marks the completion of the first phase, providing a clear
     checkpoint.

**Phase 2: Orchestrator & System Events (v0.0.1a-phase2)**

1. **Orchestrator Implementation:**
   - **Decision:** Choose a Tokio-based custom event loop over an actor model
     (e.g., Actix).
   - **Rationale:** Provides more direct control over concurrency and aligns
     better with the microkernel principles. Simpler to understand and maintain
     for the initial implementation.
   - **Event:** Creation of `orchestrator.rs` within `agentic_core/src`.
   - **Files Affected:** `agentic_core/src/orchestrator.rs`.

2. **SystemEvent Enum:**
   - **Decision:** Define `SystemEvent` enum with `TaskSubmitted` and
     `TaskCompleted` variants.
   - **Rationale:** Establishes a fundamental event-driven architecture. These
     initial events represent a simple task lifecycle.
   - **Event:** Modification of `orchestrator.rs` (or creation of `events.rs`,
     though kept in `orchestrator.rs`).
   - **Files Affected:** `agentic_core/src/orchestrator.rs`.

3. **Event Flow:**
   - **Decision:** Implement a basic event loop in `Orchestrator` that processes
     `TaskSubmitted` and generates `TaskCompleted`.
   - **Rationale:** Simulates a minimal "work" process, confirming the event
     handling mechanism.
   - **Event:** Modification of `Orchestrator::run` method in `orchestrator.rs`.
   - **Files Affected:** `agentic_core/src/orchestrator.rs`.

4. **CLI Integration:**
   - **Decision:** Add a `submit-task` command to the CLI.
   - **Rationale:** Allows users to trigger task submission and observe the
     resulting `TaskCompleted` event (via logs).
   - **Event:** Modification of `agentic_cli/src/main.rs`.
   - **Files Affected:** `agentic_cli/src/main.rs`.

5. **Testing:**
   - **Decision:** Implement unit tests for the orchestrator and integration
     tests for the CLI.
   - **Rationale:** Verifies the event loop processes events correctly and the
     CLI interacts with the orchestrator as expected.
   - **Event:** Addition of tests to `orchestrator.rs` and potentially `tests/`
     directory.
   - **Files Affected:** `agentic_core/src/orchestrator.rs`,
     `agentic_cli/src/main.rs`, and potentially `tests/`.

6. **Phase 2 Completion:**
   - **Decision:** Tag the commit as `v0.0.1a-phase2`.
   - **Rationale:** Marks the successful implementation of the event-driven
     orchestrator.

**Phase 3: Event Sourcing & Explainability Foundations (v0.0.1a-phase3)**

1. **Event Log:**
   - **Decision:** Implement an `EventLog` to store all `SystemEvent`s.
   - **Rationale:** Introduces event sourcing, enabling replay and auditing
     capabilities. Uses an in-memory `Vec<EventRecord>` for simplicity in this
     phase.
   - **Event:** Creation of `event_log.rs` within `agentic_core/src`.
   - **Files Affected:** `agentic_core/src/event_log.rs`.

2. **Orchestrator Integration:**
   - **Decision:** Modify the `Orchestrator` to append every processed event to
     the `EventLog`.
   - **Rationale:** Ensures all system events are recorded for later analysis or
     replay.
   - **Event:** Modification of `Orchestrator::run` in `orchestrator.rs`.
   - **Files Affected:** `agentic_core/src/orchestrator.rs`.

3. **Replay Function:**
   - **Decision:** Implement a `replay_events` function.
   - **Rationale:** Demonstrates the ability to reconstruct system state from
     the event log. The initial implementation is minimal, focusing on the
     principle.
   - **Event:** Addition of `replay_events` function (either in `event_log.rs`
     or a new `replay.rs`, but kept in `event_log.rs`).
   - **Files Affected:** `agentic_core/src/event_log.rs`.

4. **Tracing Integration:**
   - **Decision:** Replace `println!` with `tracing` macros (`info!`, `debug!`,
     `error!`).
   - **Rationale:** Provides structured logging with metadata, improving
     observability and debugging.
   - **Event:** Modification of code in `orchestrator.rs`, `main.rs`, and other
     relevant files.
   - **Files Affected:** Multiple files, including
     `agentic_core/src/orchestrator.rs` and `agentic_cli/src/main.rs`.

5. **Testing:**
   - **Decision:** Add unit tests for event logging and replay.
   - **Rationale:** Verifies events are correctly appended to the log and the
     replay function produces the expected state.
   - **Event:** Addition of tests to `event_log.rs` and potentially
     `orchestrator.rs`.
   - **Files Affected:** `agentic_core/src/event_log.rs`,
     `agentic_core/src/orchestrator.rs`.
6. **Phase 3 Completion:**
   - **Decision:** Tag the commit as `v0.0.1a-phase3`.
   - **Rationale:** Marks the successful integration of event sourcing and
     enhanced observability.

**Phase 4: Secure Plugin System (v0.0.1a-phase4)**

1. **Plugin Manager:**
   - **Decision:** Implement a `PluginManager` to load and manage plugins.
   - **Rationale:** Introduces a mechanism for extending the microkernel's
     functionality through dynamically loaded code.
   - **Event:** Creation of `plugin_manager.rs` within `agentic_core/src`.
   - **Files Affected:** `agentic_core/src/plugin_manager.rs`.

2. **Plugin Manifest:**
   - **Decision:** Define a `PluginManifest` struct to describe plugin metadata
     (name, version, entry point, permissions).
   - **Rationale:** Provides a standardized way to define plugin properties and
     control their access to system resources. Uses TOML format.
   - **Event:** Modification of `plugin_manager.rs`.
   - **Files Affected:** `agentic_core/src/plugin_manager.rs`.

3. **Sandbox Demonstration:**
   - **Decision:** Implement a mock WASM sandbox approach (checking for file
     existence).
   - **Rationale:** Demonstrates the principle of sandboxing without requiring
     full WASM integration in this phase.
   - **Event:** Modification of `PluginManager::invoke_plugin` in
     `plugin_manager.rs`.
   - **Files Affected:** `agentic_core/src/plugin_manager.rs`.

4. **Orchestrator Integration:**
   - **Decision:** Add `PluginInvoked`, `PluginResult`, and `PluginError`
     variants to `SystemEvent`.
   - **Rationale:** Enables the orchestrator to interact with the plugin system
     through the event-driven architecture.
   - **Event:** Modification of `orchestrator.rs`.
   - **Files Affected:** `agentic_core/src/orchestrator.rs`.

5. **CLI Commands:**
   - **Decision:** Add `load-plugin` and `invoke-plugin` commands to the CLI.
   - **Rationale:** Allows users to load plugin manifests and invoke plugin
     functions.
   - **Event:** Modification of `agentic_cli/src/main.rs`.
   - **Files Affected:** `agentic_cli/src/main.rs`.

6. **Testing:**
   - **Decision:** Add unit tests for the `PluginManager` and integration tests
     for the CLI commands. Include negative tests for permission checks.
   - **Rationale:** Ensures the plugin system loads and invokes plugins
     correctly, and handles errors appropriately.
   - **Event:** Addition of tests to `plugin_manager.rs` and updates to other
     test files. Creation of a mock WASM file for testing.
   - **Files Affected:** `agentic_core/src/plugin_manager.rs`,
     `agentic_cli/src/main.rs`, `examples/hello_plugin/*`.

7. **Phase 4 Completion:**
   - **Decision:** Tag the commit as `v0.0.1a-phase4`.
   - **Rationale:** Marks the successful implementation of the basic plugin
     system.

**Phase 5: Multi-Agent Concurrency & Streaming (v0.0.1a-phase5)**

1. **Agent Abstraction:**
   - **Decision:** Introduce an `agent.rs` module with a `MockStreamingAgent`
     and an `AgentProtocol` trait.
   - **Rationale:** Defines a clear interface for agents and provides a mock
     implementation for demonstrating streaming output.
   - **Event:** Creation of `agent.rs` within `agentic_core/src`.
   - **Files Affected:** `agentic_core/src/agent.rs`.

2. **Streaming Mock:**
   - **Decision:** Implement `MockStreamingAgent` with `stream_response()` and
     `on_event()` methods to simulate partial outputs.
   - **Rationale:** Provides a way to demonstrate streaming behavior without
     requiring a full LLM integration in this phase.
   - **Event:** Modification of `agent.rs`.
   - **Files Affected:** `agentic_core/src/agent.rs`.

3. **Orchestrator Enhancements:**
   - **Decision:** Add `AgentSpawned`, `AgentPartialOutput`, `AgentCompleted`,
     and `AgentError` variants to `SystemEvent`.
   - **Rationale:** Enables the orchestrator to manage agent lifecycles and
     handle streaming outputs.
   - **Event:** Modification of `orchestrator.rs`.
   - **Files Affected:** `agentic_core/src/orchestrator.rs`.

4. **CLI Command:**
   - **Decision:** Add a `spawn-agent` command to the CLI.
   - **Rationale:** Allows users to start an agent and observe its streaming
     output.
   - **Event:** Modification of `agentic_cli/src/main.rs`.
   - **Files Affected:** `agentic_cli/src/main.rs`.

5. **Cargo Aliases:**
   - **Decision:** Add cargo aliases for common tasks (ci, test-cli, demo,
     plugin, agent).
   - **Rationale:** Improves developer experience by providing shortcuts for
     frequent operations.
   - **Event:** Modification of the top-level `Cargo.toml`.
   - **Files Affected:** `Cargo.toml`.

6. **Testing:**
   - **Decision:** Add unit tests for `MockStreamingAgent` and integration tests
     for the `spawn-agent` command.
   - **Rationale:** Verifies the agent produces streaming output correctly and
     the orchestrator handles agent events appropriately.
   - **Event:** Addition of tests to `agent.rs` and modifications to other test
     files, including updates to integration test scripts.
   - **Files Affected:** `agentic_core/src/agent.rs`,
     `agentic_core/src/orchestrator.rs`, `agentic_cli/src/main.rs`,
     `scripts/test_cli.sh`.

7. **Phase 5 Completion:**
   - **Decision:** Tag the commit as `v0.0.1a-phase5`.
   - **Rationale:** Marks the successful implementation of multi-agent
     concurrency and streaming output demonstration.

**Key Design Principles Reinforced Throughout Stage 1:**

- **Event-Driven Architecture:** All major system interactions are modeled as
  events, promoting loose coupling and extensibility.
- **Concurrency Safety:** Core data structures (`Pile`, `Progression`) and the
  orchestrator use appropriate synchronization mechanisms (locks, channels) to
  prevent data races.
- **Modularity:** The codebase is organized into well-defined modules,
  separating concerns and improving maintainability.
- **Testability:** Extensive unit and integration tests ensure correctness and
  stability.
- **Explainability:** Event sourcing and structured logging provide insights
  into system behavior, facilitating debugging and auditing.
- **Ephemeral Approach:** The consistent use of an ephemeral orchestrator and
  store throughout stage 1 allowed for rapid iteration and simplified testing.
