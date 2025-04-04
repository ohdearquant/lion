# LionForge IDE Developer Guide v1.0

**Date:** [Current Date] **Status:** Initial Version for LionForge IDE
Development **Audience:** Human Developers & LLM Collaborators (Designer,
Implementer, Orchestrator, QA)

**Mission:** Build LionForge IDE, a secure, integrated desktop application
(Tauri + Rust) for developing, testing, managing, and monitoring multi-agent
systems leveraging the Lion Rust framework's capabilities.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Core Architecture (LionForge IDE)](#2-core-architecture-lionforge-ide)
3. [Phased Development Workflow](#3-phased-development-workflow)
4. [Coding Conventions & Project Structure](#4-coding-conventions--project-structure)
5. [Testing Strategy & Automated Feedback](#5-testing-strategy--automated-feedback)
6. [Lion Runtime Integration](#6-lion-runtime-integration)
7. [Security Model Integration (Capabilities & Policy)](#7-security-model-integration-capabilities--policy)
8. [Workflow Integration](#8-workflow-integration)
9. [External Agent (Proxy) Integration](#9-external-agent-proxy-integration)
10. [UI Development (Tauri Frontend)](#10-ui-development-tauri-frontend)
11. [Documentation Standards](#11-documentation-standards)
12. [LLM Team Collaboration Workflow](#12-llm-team-collaboration-workflow)
13. [Commit & Branching Strategy](#13-commit--branching-strategy)

---

## 1. Introduction

**LionForge IDE** is the Tauri-based desktop application providing a user
interface for the **Lion Rust Framework**. Its purpose is to simplify the
creation, management, security configuration, and monitoring of multi-agent
systems that run on the Lion framework.

- **Rust Backend (Tauri Core Process):** Integrates directly with `lion_runtime`
  and its sub-components (`lion_capability`, `lion_workflow`, etc.). Exposes
  functionality to the frontend via Tauri commands. Manages the lifecycle of the
  Lion runtime and potentially external agent processes.
- **Frontend (Tauri WebView):** Built using modern web technologies
  (React+TypeScript specified for now) to provide an interactive user
  experience. Communicates with the Rust backend via Tauri's API.
- **Lion Framework:** The underlying engine providing secure execution (via
  capabilities, policies, isolation), workflow orchestration, and concurrency
  management. LionForge is the user's window _into_ this engine.

---

## 2. Core Architecture (LionForge IDE)

1. **Tauri Application:** A single binary containing the Rust backend and the
   frontend web assets.
2. **Rust Backend:**
   - Holds the primary `Arc<lion_runtime::Runtime>` instance, managed via Tauri
     state.
   - Exposes specific, secured actions via `#[tauri::command]` functions. These
     commands are the _only_ way the frontend interacts with the Lion core.
   - Listens for internal events from `lion_runtime` (e.g., agent status, logs,
     workflow progress) and emits simplified, serializable events to the
     frontend using `AppHandle::emit_all`.
   - Manages application-level state (e.g., current project path).
   - Handles project file system operations (read/write configs, workflows,
     etc.).
   - If supporting external agents, manages the lifecycle of the **Trusted Proxy
     Plugin** (Rust/WASM within Lion) and the external agent process
     (Python/JS), including secure IPC setup.
3. **Frontend (React + TypeScript):**
   - Standard component-based architecture (`src/components`, `src/hooks`,
     `src/lib/api.ts`, `src/state`).
   - Uses Tauri's JS API (`@tauri-apps/api`) for `invoke` (calling backend
     commands) and `listen` (receiving backend events).
   - Manages UI state (e.g., using Zustand or Redux Toolkit).
   - Renders visualisations (workflow graphs via React Flow), tables, forms,
     logs, etc.
4. **Communication:** Asynchronous Tauri command invocations (FE -> BE) and
   event emissions (BE -> FE). JSON is the primary serialization format over the
   boundary.

---

## 3. Phased Development Workflow

LionForge IDE will be developed incrementally based on the 5 Phases previously
defined:

1. **Phase 1: Foundation & Read-Only Views:** Basic Tauri app, runtime
   integration, status bars, agent list viewer, log viewer.
2. **Phase 2: Core Execution & Workflow Viewing:** Agent load/unload, visual
   workflow graph viewer, start workflow instance, instance list.
3. **Phase 3: Workflow Editing & Security Configuration:** Visual workflow
   editor (nodes, edges, properties), visual capability/policy assignment UI,
   knowledge file editing.
4. **Phase 4: External Agent Integration & Auditing:** Define/load/run external
   agents via proxies, Python SDK, secure MCP communication configuration,
   security audit log view, basic metrics view.
5. **Phase 5: Advanced Features & Polish:** Workflow debugging simulation
   (pause/step), enhanced agent management (hot reload, resource limits UI),
   usability features (search, templates), UI polish.

Each phase will be broken down into specific tasks delegated by the
**@Orchestrator**.

---

## 4. Coding Conventions & Project Structure

1. **Workspace:** Monorepo containing the Tauri app (`src-tauri/`) and the
   frontend (`src/` or `frontend/`). Assumes core Lion crates are available
   (e.g., via path dependencies in `src-tauri/Cargo.toml`).
2. **Rust (Backend - `src-tauri/src/`):**
   - Follow conventions defined in `CODING_STYLE_RUST.md`.
   - Organize code into modules (e.g., `commands`, `state`, `events`,
     `runtime_integration`, `project`).
   - Use `anyhow::Result` for internal error handling within commands; map
     errors to `Result<T, String>` for Tauri command return types.
   - Use `tracing` for backend logging.
3. **TypeScript/React (Frontend - `src/`):**
   - Follow conventions defined in `CODING_STYLE_FRONTEND.md`.
   - Structure components logically. Use hooks for state and effects. Define
     clear types/interfaces for API payloads and events.
4. **File Naming:** Use `snake_case.rs` for Rust files, `PascalCase.tsx` for
   React components, `camelCase.ts` for utility/API modules.

---

## 5. Testing Strategy & Automated Feedback

**Goal:** Ensure correctness, prevent regressions, and provide rapid feedback,
especially for LLM implementers.

1. **Rust Backend Unit Tests (`#[test]`):**
   - **Focus:** Test logic within Tauri commands (`commands.rs`).
   - **Technique:** Mock the `lion_runtime::Runtime` interface (and its
     sub-managers) using `mockall` or simple test implementations. Verify
     command inputs are parsed correctly, expected runtime methods are called,
     return values are formatted correctly, and errors are mapped properly. Test
     edge cases and error paths.
   - **Requirement:** Implementer **MUST** write unit tests covering the core
     logic of any new or modified command. QA **MUST** verify coverage against
     the design.
2. **Rust Backend Integration Tests (`tests/`):**
   - **Focus:** Test the interaction between Tauri commands and a _real_ (or
     near-real, potentially slightly simplified) `lion_runtime` instance.
   - **Technique:** Set up a minimal `Runtime` in the test, invoke commands
     programmatically (simulating `invoke`), assert on results and runtime state
     changes. Test sequences of commands.
   - **Requirement:** Implementer writes integration tests for key user flows
     (e.g., load agent -> grant capability -> check status). QA verifies these
     flows.
3. **Frontend Component Tests (Vitest/Jest + RTL):**
   - **Focus:** Verify individual React components render correctly based on
     props and handle basic user interactions (button clicks, form inputs).
   - **Technique:** Use React Testing Library (`@testing-library/react`). Mock
     Tauri API calls (`invoke`, `listen`). Assert on rendered output and state
     changes.
   - **Requirement:** Implementer **SHOULD** write component tests for complex
     UI elements (editors, forms, complex views). QA **MAY** request specific
     component tests.
4. **(Future) End-to-End Tests:** Use Tauri's WebDriver support (`tauri-driver`)
   or Playwright/Cypress (if exposing a dev server) to test full user flows
   through the UI. (Likely Phase 5 or later).
5. **Automated Feedback Loop:**
   - **CI:** `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`,
     frontend lint/build/test commands **MUST** pass.
   - **Implementer LLM:** Should be instructed to run `cargo test` after making
     changes and include test results/failures in its output/commit summary.
   - **QA LLM:** **MUST** execute `cargo test` and analyze the output and test
     coverage as a primary validation step.

Refer to `TESTING_STRATEGY.md` for more details.

---

## 6. Lion Runtime Integration

- The Tauri backend owns the `Arc<Runtime>`.
- Lifecycle is managed in `main.rs` (`setup`, `on_window_event`).
- Tauri commands access the `Runtime` via `tauri::State`.
- Internal Lion events (logging, status changes) need to be bridged to Tauri
  frontend events (`AppHandle::emit_all`). This requires either:
  - Modifying Lion components to use broadcast channels or callbacks that the
    Tauri backend can hook into.
  - Implementing custom `tracing::Subscriber` layers or log appenders in the
    Tauri backend.

---

## 7. Security Model Integration (Capabilities & Policy)

- The UI provides interfaces for _configuring_ security, but _enforcement_
  happens entirely within the Lion backend (`lion_capability`, `lion_policy`),
  typically triggered by the **Trusted Proxy Plugin** for external agents or
  directly by runtime operations for internal actions.
- **Capabilities:**
  - Tauri commands (`grant_capability`, `revoke_capability`,
    `get_agent_capabilities`) interact with `runtime.capabilities` manager.
  - The UI visually represents granted capabilities. Special focus on
    MCP/Network capabilities.
- **Policies:**
  - Tauri commands (`add_policy`, `remove_policy`, `list_policies`) interact
    with `runtime.policies` manager (needs adding to `Runtime` facade if not
    already present).
  - UI provides editor for rules.
- **Audit Log:**
  - The Trusted Proxy Plugin (and potentially other core Lion components)
    **MUST** log security decisions using `lion_observability`.
  - Tauri backend command `get_audit_log` retrieves these logs for display.

---

## 8. Workflow Integration

- Workflow definitions are stored as project files (JSON/YAML).
- **UI:** Visual editor (React Flow) allows DAG creation/modification.
  Properties panel configures nodes/edges.
- **Tauri Commands:**
  - `load_workflow_definition`: Reads file, parses to `WorkflowDefinition`,
    returns frontend-compatible JSON.
  - `save_workflow_definition`: Receives frontend JSON, parses to
    `WorkflowDefinition`, validates using `definition.validate()`, saves to
    file.
  - `start_workflow_instance`: Calls `runtime.workflows.start_workflow`.
  - `list_workflow_instances`, `get_workflow_instance_details`: Query
    `runtime.workflows` manager (needs methods to list/get details).
  - `pause/resume/cancel_workflow_instance`: Call corresponding
    `runtime.workflows` methods.
  - `retry/skip_node`: Call corresponding `runtime.workflows` methods (needs
    API).
- **Backend Events:** `workflow_status_changed`, `node_status_changed` emitted
  to UI for real-time updates.

---

## 9. External Agent (Proxy) Integration

- **Agent Definition:** UI allows defining external agents (interpreter, script
  path). Saved to a config file.
- **Runtime (`PluginManager`):** Reads config, launches trusted proxy plugin
  instance, launches external process, establishes IPC, monitors process.
- **Proxy Plugin (Rust):** Runs within Lion, listens on IPC, receives requests
  from external agent, **performs capability checks using `lion_capability`**,
  executes allowed actions (file IO, network) or forwards requests (MCP), logs
  audits via `lion_observability`, sends results back via IPC.
- **Agent SDK (Python/TS):** Library used by external agent code. Handles IPC
  communication with proxy, provides simple API (`lion.read_file()`), translates
  errors.

---

## 10. UI Development (Tauri Frontend)

- **Framework:** React + TypeScript (unless decided otherwise).
- **Communication:** Use `@tauri-apps/api` for `invoke` and `listen`. Create a
  typed API wrapper (`src/lib/api.ts`).
- **State Management:** Use Zustand (recommended) or Redux Toolkit for managing
  application state fetched from backend or updated via events.
- **Components:** Build reusable components for UI elements (tables, forms,
  graph nodes, log lines).
- **Styling:** Use a consistent system (e.g., Tailwind CSS, CSS Modules,
  Emotion).
- **Conventions:** Follow `CODING_STYLE_FRONTEND.md`.

---

## 11. Documentation Standards

- **Rust Backend:** Standard Rust doc comments (`///`) for all public items.
  `cargo doc` should produce useful documentation. Module-level docs where
  needed.
- **Frontend:** JSDoc/TSDoc comments for components, props, hooks, API
  functions.
- **Design Docs:** Use `DESIGN_DOC_TEMPLATE.md` for significant features.
- **READMEs:** Maintain project `README.md` and potentially READMEs for major
  components/crates.
- **Code Comments:** Use `//` for explaining complex logic, assumptions, or
  workarounds.

---

## 12. LLM Team Collaboration Workflow

1. **Orchestrator:** Defines phase goals, breaks down into tasks, assigns to
   Designer/Implementer using `ROO_SUBTASK::ASSIGN`. Specifies inputs (design
   docs, code paths, goals) and required outputs/formats. Manages dependencies.
2. **Designer:** Receives task, creates/updates **Markdown Design Doc**
   (`DESIGN_DOC_TEMPLATE.md`). Focuses on API contracts (Tauri
   commands/events/payloads), UI component structure, data flow. Signals
   completion with path to design doc.
3. **Orchestrator:** Reviews design. If approved, assigns to Implementer.
4. **Implementer:** Receives task (referencing Design Doc section). Writes
   **Rust backend code**, **Frontend code**, and **Automated Tests**
   (`cargo test`, frontend tests). Uses diff edits where appropriate. Runs
   tests. Signals completion with summary (branch, commits, PR info, **test
   results**).
5. **Orchestrator:** Reviews summary. If looks reasonable, assigns to QA.
6. **QA:** Receives task (referencing Design Doc section, code branch). **Runs
   `cargo test`**. Reviews tests for coverage/correctness against design. Checks
   code standards (`fmt`, `clippy`). Provides **QA Review Summary** (Markdown)
   including test results and required fixes/missing tests. Signals completion.
7. **Orchestrator:** Reviews QA Summary. If Pass -> Merge. If Fail -> Assign
   fixes back to Implementer (Loop).

**Key Changes:** Reduced reliance on intermediate documents (Research Summaries
less critical if Orchestrator holds context). Direct handoff via design doc and
code branch. **QA MUST run automated tests.** Implementer MUST include test
results.

---

## 13. Commit & Branching Strategy

- Follow guidelines in `COMMIT_GUIDE.md`.
- **Branches:** Use feature branches named like `feat/phaseX-<short_desc>`
  (e.g., `feat/phase2-workflow-viewer`).
- **Commits:** Use Conventional Commits (e.g.,
  `feat(ui): Add workflow graph component`,
  `fix(runtime): Correct agent state propagation`,
  `test(backend): Add unit tests for load_agent command`).
- **PRs:** Use clear titles and descriptions linking to the original task/goal.
  Include summary of changes and **confirmation that all automated tests pass**.
