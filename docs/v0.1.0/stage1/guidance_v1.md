Below is a **more detailed**, **more comprehensive** developer guide for your
**Rust-based lion** project. It expands on coding standards, project phases,
test strategies, and security considerations—tying them all together into a
single, cohesive reference. This guide is intended for **both human developers
and LLM-based contributors**, ensuring that everyone follows the same high
standards for maintainability, performance, and security throughout the lion
development lifecycle.

---

# lion Developer Guide (Rust Microkernel) v1.0

**Date:** 2025-02-19\
**Status:** Active\
**Audience:** Human developers & LLM collaborators

This guide serves as the **canonical reference** for designing, coding, testing,
and extending the lion platform—a Rust-based microkernel system supporting
multi-agent AI workflows and a secure plugin architecture. By adhering to these
guidelines, we will produce a robust, high-performance, and modular system that
can evolve alongside our user and business needs.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Architecture & Design Principles](#2-architecture--design-principles)
3. [Phase-Driven Development Workflow](#3-phase-driven-development-workflow)
4. [Coding Conventions & Project Structure](#4-coding-conventions--project-structure)
5. [Test Strategy & Quality Assurance](#5-test-strategy--quality-assurance)
6. [Plugin System & Microkernel Approach](#6-plugin-system--microkernel-approach)
7. [Security & Sandbox Guidelines](#7-security--sandbox-guidelines)
8. [Performance & Concurrency Management](#8-performance--concurrency-management)
9. [Documentation & In-Code Comments](#9-documentation--in-code-comments)
10. [Reporting, Tagging & Phase Validation](#10-reporting-tagging--phase-validation)
11. [LLM Integration & Collaboration](#11-llm-integration--collaboration)
12. [Conclusion & Future Roadmap](#12-conclusion--future-roadmap)

---

## 1. Introduction

**lion** is a Rust-based project aiming to build a **minimal microkernel** for
event-driven, multi-agent AI operations, with modular plugins providing extended
functionality. By leveraging **Rust’s** type safety and concurrency features,
plus a **phased** development approach, lion will be secure, maintainable, and
easy to evolve.

- **Why Rust?**
  - Strong compile-time guarantees (memory safety, no data races)
  - High-performance async support (Tokio)
  - Excellent concurrency model for multi-agent systems

- **Why a Microkernel?**
  - Minimal, isolated “core” that delegates advanced features to plugins
  - Strong modularity and security boundaries
  - Clear lines of separation for future enhancements

---

## 2. Architecture & Design Principles

1. **Microkernel Foundation:**
   - The “core” is responsible for orchestrating events, managing concurrency,
     security checks, and minimal resource scheduling.
   - All other functionalities (agent logic, specialized tools, advanced ML
     routines) plug into the microkernel via well-defined interfaces.

2. **Event-Driven & Actor-Like:**
   - System events flow through channels or an actor framework (e.g., Actix or a
     custom Tokio loop).
   - Agents, services, or plugins communicate strictly via messages/events, not
     direct function calls.

3. **Security & Isolation:**
   - Plugins can be sandboxed (WASM or subprocess).
   - Rust’s ownership prevents memory corruption, ensuring robust concurrency.

4. **Phase-Driven Implementation:**
   - The project is organized into sequential phases (Core Primitives,
     Orchestrator, Event Sourcing, Plugins, Multi-Agent, Hardening). Each phase
     yields testable milestones.

5. **Observability & Explainability:**
   - We rely on structured logs (`tracing` crate) and optional event-sourcing.
   - Each significant event is recorded, enabling state reconstruction, replay,
     and debugging.

---

## 3. Phase-Driven Development Workflow

We adopt a **six-phase** plan for v0.0.1a, ensuring each phase is an
incrementally testable slice:

1. **Phase 1: Workspace Setup & Core Primitives**
   - Create `agentic_core` (lib) & `agentic_cli` (bin).
   - Define data structures like `ElementData`, `Pile<T>`, `Progression`.
   - Provide simple CLI commands (`create-element`, `list-elements`).

2. **Phase 2: Orchestrator & System Events**
   - Introduce an event loop or actor-based orchestrator (with `SystemEvent`
     types).
   - Basic concurrency demonstration (submit tasks, produce completed events).

3. **Phase 3: Event Sourcing & Explainability**
   - Append events to an immutable log.
   - Implement replay logic to confirm final system state.
   - Improve logging with correlation IDs in `tracing`.

4. **Phase 4: Secure Plugin System**
   - Plugin Manager loads plugins from manifest.
   - WASM or subprocess sandbox approach, with minimal host privileges.
   - Example “HelloWorld” plugin.

5. **Phase 5: Multi-Agent & Streaming**
   - Multiple agents (actor tasks or custom approach) running concurrently.
   - Demonstrate partial output streaming from an LLM or another external
     service.
   - Integrate concurrency checks.

6. **Phase 6: Hardening & Final Packaging**
   - Security/timeouts for plugin calls.
   - Performance profiling.
   - Document final usage and produce `v0.0.1a`.

---

## 4. Coding Conventions & Project Structure

1. **Workspace Layout**
   ```
   lion/
   ├── agentic_core/
   │   ├── src/
   │   │   ├── element.rs
   │   │   ├── pile.rs
   │   │   ├── progression.rs
   │   │   ├── orchestrator.rs
   │   │   ├── plugin_manager.rs
   │   │   └── store.rs
   │   └── Cargo.toml
   ├── agentic_cli/
   │   └── src/
   │       └── main.rs
   ├── tests/
   ├── docs/
   ├── Cargo.toml
   └── README.md
   ```

2. **Naming & Modules**
   - Use **snake_case** for file names, modules, and `#[cfg(test)]` blocks.
   - Keep modules cohesive: `element.rs` only for `ElementData` logic, `pile.rs`
     for concurrency containers, etc.

3. **Coding Style**
   - **Rust 2021** or higher edition.
   - Run `cargo fmt` and `cargo clippy` before commits.
   - For each public function or struct, use doc comments (`///`) explaining its
     usage and parameters.

4. **Error Handling**
   - Return `Result<T, E>` for fallible operations; avoid panics in library
     code.
   - Group related errors in a domain-specific error enum, e.g., `PluginError`,
     `StoreError`.

5. **Dependency Management**
   - Keep third-party crates minimal.
   - Use `[workspace]` in the top-level `Cargo.toml` to unify versions.

---

## 5. Test Strategy & Quality Assurance

1. **Unit Tests**
   - Each module has its own `#[cfg(test)] mod tests` or a separate file in
     `tests/`.
   - Example: `test_pile_insert_retrieve()`, verifying concurrency correctness.

2. **Integration Tests**
   - Larger scenarios stored in the `tests/` directory.
   - E.g., `orchestrator_tests.rs` verifying “submit-task → orchestrator
     processes → events appended.”

3. **Continuous Integration (CI)**
   - Every commit triggers `cargo build`, `cargo test`, `cargo clippy`,
     `cargo fmt --check`.
   - The build must pass with zero warnings for merges into `main`.

4. **Coverage & Observability**
   - Aim for high coverage in core modules.
   - Consider cargo plugins (like `cargo-tarpaulin`) for coverage reporting.
   - Use `tracing` macros in code for real-time debugging if needed.

---

## 6. Plugin System & Microkernel Approach

1. **Minimal Core, Extended by Plugins**
   - The microkernel orchestrates tasks, enforces permissions, and logs events.
   - Additional functionality or specialized agent logic is loaded as a plugin.

2. **Plugin Manager**
   - Loads manifests describing each plugin (permissions, entry point).
   - Invokes plugin code via WASM or subprocess calls.
   - Maintains references to active plugin instances, tracking security
     constraints.

3. **Sandboxing**
   - **WASM**: Restrict host functions to a minimal set.
   - **Subprocess**: Launch plugins with limited OS privileges or containers.
   - Enforce timeouts, resource usage checks (e.g., CPU/memory limits) in the
     orchestrator.

4. **Plugin Lifecycle**
   - **Load**: Parse manifest, check permissions, load code.
   - **Initialize**: Provide plugin with references to the orchestrator or store
     if safe.
   - **Execute**: On each invocation event, the plugin runs in a restricted
     environment.
   - **Unload**: Gracefully shut down or forcibly remove if it misbehaves.

---

## 7. Security & Sandbox Guidelines

1. **Least Privilege**
   - If a plugin needs file access, explicitly allow only the relevant
     directories or host functions.
   - Deny or restrict network access unless manifest declares it.

2. **Process Isolation**
   - For untrusted or third-party plugins, prefer spawning a separate process
     with OS-level isolation (e.g., seccomp, user namespaces on Linux).
   - Communicate over a local socket or channel.

3. **Error Logging & Auditing**
   - Log plugin load/unload actions and any security-related events (permission
     denials) using `tracing::error!` or `info!`.

4. **Timeout & Resource Limits**
   - Wrap plugin calls in `tokio::time::timeout(...)`.
   - If using Actix or an actor-based system, implement a supervisor that can
     restart or kill misbehaving plugins.

---

## 8. Performance & Concurrency Management

1. **Tokio Async Runtime**
   - Use `#[tokio::main]` or a custom runtime in your orchestrator.
   - For heavy CPU tasks, consider separate thread pools or spawn blocking tasks
     with `tokio::task::spawn_blocking`.

2. **Concurrent Data Structures**
   - For shared data like `Pile<T>`, wrap in `Arc<Mutex<...>>` or adopt an actor
     approach to isolate writes.
   - Keep critical sections short and prefer message-passing to limit locking
     overhead.

3. **Profiling & Metrics**
   - Integrate Rust-based metrics (e.g., `metrics` crate) or logs with
     timestamps to measure throughput.
   - Evaluate concurrency under load to ensure no bottlenecks.

4. **Scalability**
   - The microkernel approach allows multiple orchestrator instances if needed
     (though advanced distributed logic is out of scope for initial v0.0.1a).

---

## 9. Documentation & In-Code Comments

1. **Doc Comments**
   - Use triple-slash `///` for every public item.
   - Provide at least one usage example if feasible.

2. **Module-level README**
   - If a module is complex (e.g. `plugin_manager`), include a local `README.md`
     summarizing design, usage, and extension patterns.

3. **Design Rationale**
   - For concurrency or security logic, add line comments clarifying how data is
     protected or how certain boundaries are enforced.

4. **docs/ Directory**
   - Keep high-level architecture docs, phase-level reports, and reference
     material here.
   - This developer guide (in a file like `docs/dev_guide.md`) should be kept
     updated as practices evolve.

---

## 10. Reporting, Tagging & Phase Validation

1. **Phase Reports**
   - At the end of each phase, produce a short report:
     - **Objectives**: The tasks or goals for that phase.
     - **Work Done**: Summaries of new modules, tests, or features.
     - **Validation**: Outline of tests, how you verified correctness.
     - **Next Steps**: Any open issues or tasks for future phases.

2. **Version & Tagging**
   - Use tags like `v0.0.1a-phase1` once the entire Phase 1 tasks pass
     acceptance tests.
   - Summarize commits in a “commit report” file if desired.

3. **Commit Guidelines**
   - Keep commits small and focused.
   - Reference the phase in commit messages (e.g.,
     `[phase1] Add Pile concurrency tests`).

---

## 11. LLM Integration & Collaboration

1. **Contextual Prompts**
   - When requesting code from an LLM, provide relevant context (module name,
     function signatures, constraints from this guide).
   - Example prompt snippet:\
     “We are modifying `agentic_core/src/orchestrator.rs`. Please add a
     `SystemEvent` enum variant `LLMChunk` with a `text: String` field.”

2. **Validate Generated Code**
   - Even if code is LLM-generated, it must pass `cargo fmt`, `cargo clippy`,
     and `cargo test`.
   - Perform a code review to ensure compliance with concurrency and security
     guidelines.

3. **LLM Assist in Code Review**
   - The LLM can provide feedback or suggestions on PRs, but a human must do
     final checks.
   - The LLM can also generate doc comments, but ensure they reflect actual
     logic.

---

## 12. Conclusion & Future Roadmap

By following these guidelines, you ensure that every contribution—be it from a
human developer or an LLM-based helper—adheres to the **same high standards** of
code quality, security, and performance. lion’s microkernel approach and
phase-based development guarantee that each piece of functionality is
well-tested, auditable, and extensible.

- **In the near term**, we complete **Phase 1** (core primitives, CLI) through
  **Phase 5** (multi-agent concurrency, partial streaming).
- **Eventually**, we refine security (Phase 6), add distributed or advanced
  scheduling logic, and possibly integrate new plugin frameworks.

**The result** is a **highly modular, future-proof Rust platform** that can
orchestrate complex multi-agent AI workflows while remaining stable, secure, and
straightforward to maintain.

---

**Thank you for contributing to lion**—let’s build a powerful, next-generation
Rust-based microkernel for AI operations!
