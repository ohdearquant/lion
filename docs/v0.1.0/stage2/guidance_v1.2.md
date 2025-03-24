Below is an **optimized and extensively detailed** version of the developer
guide, **v1.2**, incorporating your existing microkernel guidelines with new
sections on **Stage 2** UI/UX using a **web-based approach** (potentially
wrapped by Tauri for local desktop usage). It includes specific considerations
for **Docker** deployment, **real-time event streaming**, and **multi-agent
concurrency**. This document is designed to “ground” your LLM (and human
developers) in a precise, comprehensive manner—ensuring consistent, high-quality
implementation across all parts of the lion project.

---

# lion Developer Guide (Rust Microkernel) v1.2

**Date:** 2025-02-19\
**Status:** Updated to Include Stage 2 Web UI & Docker Guidance\
**Audience:** Human Developers & LLM Collaborators

**Mission:** Provide a robust, flexible Rust microkernel (lion) for multi-agent
AI operations, extended by a web-based user interface for real-time management
and logging, plus optional Tauri wrapping for a local macOS desktop experience.

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
12. [UI/UX Integration & Stage 2 Approach](#12-uiux-integration--stage-2-approach)
13. [UI Architecture & Web Framework Considerations](#13-ui-architecture--web-framework-considerations)
14. [Real-Time Event Streaming & Docker Usage](#14-real-time-event-streaming--docker-usage)
15. [Future Tauri Integration & macOS Desktop Deployment](#15-future-tauri-integration--macos-desktop-deployment)
16. [Conclusion & Future Roadmap](#16-conclusion--future-roadmap)

---

## 1. Introduction

The **lion** project is a **Rust-based microkernel** system designed for
event-driven, multi-agent AI workflows. It encourages a **plugin-based**
architecture for extended functionality—enabling specialized AI logic, agent
collaboration, and advanced ML routines.

- **Rust** is chosen for:
  - Compile-time safety (memory, concurrency).
  - High-performance async concurrency (Tokio).
  - Ecosystem synergy for advanced systems and AI integration.

- **Microkernel** ensures:
  - Minimal “core” orchestrating concurrency, security, event distribution.
  - Plugins or separate modules handle specialized tasks, loaded/unloaded as
    needed.
  - Clear isolation boundaries for reliability and security.

**This version of the guide (v1.2)** adds new sections about **Stage 2**:
building a web-based UI (potentially deployable in Docker), with real-time
streaming of logs/agent outputs, plus an optional Tauri-based local app path for
macOS.

---

## 2. Architecture & Design Principles

1. **Microkernel Core**
   - The orchestrator (concurrency, scheduling) plus security checks.
   - Maintains minimal state, delegates advanced tasks to plugins or agents.

2. **Event-Driven**
   - Agents or plugins communicate via events (no direct calls).
   - Our orchestrator processes `SystemEvent` variants, triggers concurrency
     logic, logs outcomes.

3. **Security & Isolation**
   - Rust ensures memory safety.
   - Plugin sandboxing (WASM or OS-level processes) for minimal blast radius if
     a plugin fails.

4. **Phased Implementation**
   - Early phases yield core primitives (piles, orchestrator, event log).
   - Later phases add secure plugins, multi-agent streaming, and final
     packaging.

5. **Observability**
   - Logging with `tracing`, optional event-sourcing for advanced debugging or
     replay.
   - Real-time streaming to a front-end in Stage 2.

---

## 3. Phase-Driven Development Workflow

**lion** 0.0.1 is developed in **six** microkernel-focused phases (1–6). We also
have a **Stage 2** layer focusing on the new UI. Summaries:

1. **Phase 1**: Workspace Setup & Core Primitives (`agentic_core`, basic data
   structures).
2. **Phase 2**: Orchestrator & System Events (submit tasks, concurrency
   demonstration).
3. **Phase 3**: Event Sourcing & Explainability (immutable log, replays).
4. **Phase 4**: Secure Plugin System (manifest-based load, sandboxing).
5. **Phase 5**: Multi-Agent & Streaming (multiple agents concurrency, partial
   output).
6. **Phase 6**: Hardening & Final Packaging (security/timeouts, performance,
   documentation).

**Stage 2** expansions revolve around a new `lion_ui` crate or a web front end
for real-time event streaming. We track those in sub-phases (Stage2-Ph1,
Stage2-Ph2, etc.):

- **Stage2-Ph1**: UI Foundation & Docker Setup.
- **Stage2-Ph2**: Real-Time Logs & Agent Management.
- **Stage2-Ph3**: Plugin Manager UI & Workflow Visualization.
- **Stage2-Ph4**: Advanced Logging, Searching, & Possibly Tauri Wrapping.
- **Stage2-Ph5**: Final Polishing & Packaging for macOS.

---

## 4. Coding Conventions & Project Structure

1. **Workspace Layout**
   ```text
   lion/
   ├── agentic_core/
   │   └── src/
   ├── agentic_cli/
   │   └── src/
   ├── lion_ui/ (Stage 2 UI code)
   │   ├── Cargo.toml
   │   ├── src/
   │   │   └── main.rs (Possible Axum/Actix server)
   │   └── frontend/ (Optional: React, Svelte, or minimal HTML/JS)
   ├── tests/
   ├── docs/
   └── Cargo.toml
   ```

2. **Naming & Modules**
   - Use **snake_case** for files and modules.
   - Each module handles a cohesive domain (e.g., `plugin_manager.rs` just for
     plugin logic).

3. **Coding Style**
   - **Rust 2021 edition**.
   - Enforce `cargo fmt --all` and `cargo clippy -- -D warnings`.
   - Write doc comments for all public items using `///`.

4. **Error Handling**
   - Avoid panics except in truly unrecoverable logic.
   - Use domain-specific error enums (`PluginError`, `StoreError`).

5. **UI-Specific**
   - For the web-based approach, maintain a consistent structure. Possibly use
     `frontend/src/` for the JavaScript/TypeScript code.
   - Keep minimal dependencies if possible. If using React or Svelte, keep your
     `package.json` curated.

---

## 5. Test Strategy & Quality Assurance

1. **Core Unit Tests**
   - Each microkernel module has thorough tests.
   - Example: test concurrency in `pile.rs` with multiple threads.

2. **Integration Tests**
   - `agentic_cli` integration (submitting tasks) or a `tests/` folder that
     spins up orchestrator.
   - Could test partial streaming or plugin loading.

3. **CI Pipeline**
   - Automatic checks for build, test, lint, format. Possibly coverage tools
     like `tarpaulin`.

4. **UI Testing**
   - For Stage 2, early manual testing suffices.
   - If needed, adopt [Playwright](https://playwright.dev/) or
     [Cypress](https://www.cypress.io/) for advanced integration tests once
     real-time streaming is stable.
   - Tauri-based testing can be partial end-to-end with the Tauri dev server.

---

## 6. Plugin System & Microkernel Approach

1. **Core Delegation**
   - The orchestrator manages tasks and concurrency.
   - Specialized tasks are invoked as plugins (e.g., LLM or data fetchers).

2. **Plugin Manager**
   - Reads plugin manifest: name, version, entry_point, permissions.
   - Possibly uses WASM or subprocess calls.
   - Tracks plugin handles in a `HashMap<Uuid, PluginHandle>`.

3. **Security**
   - If WASM, limit host functions.
   - If subprocess, enforce OS-level constraints.
   - Timeout / kill unresponsive plugins.

4. **Lifecycle**
   - Load → Initialize → Execute → Unload.
   - The orchestrator spawns events for each step.

---

## 7. Security & Sandbox Guidelines

1. **Least Privilege**
   - If a plugin requires network or file access, only allow the minimal set.
   - Deny or restrict anything beyond that.

2. **Separate Processes**
   - For untrusted or third-party code, run in a separate user account or
     container.
   - Communicate with the microkernel over local sockets.

3. **Auditing**
   - Log plugin actions in `tracing`.
   - Possibly store them in an event log for replay.

4. **UI Ports**
   - For web usage, consider binding to `127.0.0.1` by default, or ensure users
     know the port is open if Docker runs it.

---

## 8. Performance & Concurrency Management

1. **Tokio & Concurrency**
   - The orchestrator uses an async approach for multiple tasks.
   - Use channels or message passing to avoid large lock contention.

2. **Agent Work**
   - If CPU-bound, consider `spawn_blocking` or a dedicated thread pool.
   - For advanced multi-agent synergy, design each agent as a separate
     `tokio::task` or “actor.”

3. **Profiling**
   - Use `cargo flamegraph` or `perf` for hotspots if performance issues arise.
   - Apply backpressure or throttling if logs produce thousands of lines per
     second.

4. **UI Performance**
   - If using SSE or WebSockets, ensure the front end doesn’t render every
     single event individually in a heavy manner. Possibly batch or virtualize
     logs.

---

## 9. Documentation & In-Code Comments

1. **Doc Comments**
   - `///` summarizing each function’s usage.
   - Provide short examples if it clarifies usage.

2. **Module-Level READMEs**
   - For complex modules (like `orchestrator.rs` or `plugin_manager.rs`), add an
     internal README.

3. **Design Rationale**
   - For concurrency patterns or security constraints, add line comments
     clarifying.
   - Keep architecture docs in `docs/`.

4. **UI Documentation**
   - For Stage 2, describe your chosen web approach. If you do React, note your
     component structure in `docs/ui_design.md`.

---

## 10. Reporting, Tagging & Phase Validation

1. **Phase Reports**
   - Summarize tasks, progress, validations at the end of each phase.
   - Keep them in `docs/progress/` or similar.

2. **Versioning**
   - Tag commits with `v0.0.1-phaseX` or `stage2-phaseY`.
   - Summarize changes in a commit report if desired.

3. **Commit Guidelines**
   - Keep commits small, referencing the phase or feature.
   - Example: `[stage2-phase1] Add minimal Axum server for UI`.

---

## 11. LLM Integration & Collaboration

1. **Contextual Prompts**
   - Provide module paths, function signatures, relevant design constraints in
     your prompt.
   - E.g., “We are editing `lion_ui/src/main.rs` to implement SSE logs from
     `agentic_core::orchestrator`.”

2. **Validating Code**
   - LLM-generated code must pass `cargo test`, `cargo fmt`, `cargo clippy`.
   - Don’t blindly accept large diffs from the LLM; do a thorough review for
     concurrency or security pitfalls.

3. **UI Collaboration**
   - The LLM can propose front-end code (HTML/JS/React) if that’s your approach.
     Provide consistent versioning (like React 18 or Svelte 3).
   - The LLM should reference your real-time streaming approach (SSE or WS).

---

## 12. UI/UX Integration & Stage 2 Approach

The next major milestone is building a **flexible web interface** to manage the
orchestrator and its multi-agent concurrency. By focusing on a web-based UI
first, you can easily host it in Docker or embed it in Tauri later.

### Stage 2 Goals

1. **Real-Time Event Streaming**: Show partial logs from agents and plugin calls
   in real time, so users can follow multi-agent tasks.
2. **Agent & Plugin Management**: A user can spawn new agents, see a list of
   running ones, load/invoke plugins, etc.
3. **Docker Hosting**: Provide a Dockerfile so advanced users or integrators can
   simply `docker run -p 8080:8080 lion_ui` and open the UI in a browser.
4. **Local macOS Option**: Possibly a Tauri wrapper that reuses the same
   front-end code, giving a `.app` for a fully local experience.

### Rationale

- A **web-based** approach is the quickest to share with your dev team or users.
- If you want a local desktop experience on macOS, you can embed the same build
  output in Tauri.
- Real-time logs are easily handled via SSE or WebSockets on the same port.

---

## 13. UI Architecture & Web Framework Considerations

### 13.1 Project Structure for `lion_ui`

1. `lion_ui/Cargo.toml` referencing `agentic_core`.
2. `src/`:
   - `main.rs` or `server.rs` sets up an HTTP server (e.g., Axum).
   - `routes/`, `handlers/`, or `controllers/` define endpoints for logs, agent
     spawns, plugin calls.
3. `frontend/`:
   - Possibly a React or Svelte app, or minimal HTML/JS, with a bundler (Vite,
     Webpack).
   - On build, produce a `dist/` folder that the Rust server can serve at `/`.

### 13.2 Real-Time Data

- **SSE or WebSocket**:
  - SSE simpler for unidirectional partial logs (the orchestrator → UI).
  - WebSockets if you want more two-way (though you can still do two-way with
    standard HTTP for commands, SSE for logs).
- **Agent Outputs**: The orchestrator calls an async channel. The web server
  listens, then pushes lines to the front end.

### 13.3 Agent & Plugin Management

- Provide routes like:
  - `POST /api/agents` → spawn a new agent.
  - `GET /api/plugins` → list loaded plugins.
  - `POST /api/plugins/load` → load a new plugin from a manifest.
- The front end calls these routes. For example, a React page with forms for
  plugin loading or agent creation.

### 13.4 Key UI Pages

1. **Dashboard**: Summaries of active agents, loaded plugins, quick stats.
2. **Agents**: Table or list of active agents. Clicking one shows partial logs
   or console.
3. **Plugins**: Table of loaded plugins. A button or form to load a new one, or
   “Invoke” actions.
4. **Logs**: A real-time console if you want a dedicated log page, or integrated
   with each agent’s detail.

---

## 14. Real-Time Event Streaming & Docker Usage

### 14.1 Event Streaming Implementation

1. **Server-Sent Events**
   - In Axum, define a route returning an `Sse<impl Stream<Item = something>>`.
   - The orchestrator can push
     `SystemEvent::AgentPartialOutput(agent_id, chunk, ...)` into a
     `tokio::sync::broadcast` or `mpsc`, which the SSE route maps into SSE data
     frames.
   - The front end uses an `EventSource` object:
     ```javascript
     const es = new EventSource("/sse-logs");
     es.onmessage = (evt) => {
       console.log("Log event:", evt.data);
     };
     ```
2. **WebSocket**
   - If you prefer two-way communication or advanced interactions, define a `ws`
     route. The front end uses a WebSocket client.
   - For partial logs, push `Text` messages with the chunk data.

### 14.2 Docker Integration

1. **Dockerfile** Example
   ```dockerfile
   FROM rust:1.70-bullseye as builder
   WORKDIR /app
   COPY . .
   RUN apt-get update && apt-get install -y libssl-dev
   RUN cargo build --release -p lion_ui

   FROM debian:11
   COPY --from=builder /app/target/release/lion_ui /usr/local/bin/lion-ui
   EXPOSE 8080
   CMD ["lion-ui", "--http-port=8080"]
   ```

2. **Run**
   - `docker build -t lion_ui .`
   - `docker run --rm -p 8080:8080 lion_ui`
   - Then open `http://localhost:8080` in a browser. The front-end code loads,
     connecting to SSE on the same host.

3. **Local macOS**
   - If devs want a local experience, they can skip Docker and just do
     `cargo run -p lion_ui`. This starts the server on 127.0.0.1:8080.
   - Docker is more for distribution or consistent environments.

---

## 15. Future Tauri Integration & macOS Desktop Deployment

While a pure web-based UI suffices, you may eventually provide a “native-like”
desktop experience with **Tauri**:

1. **Tauri Basics**
   - Tauri wraps a local webview (WKWebView on macOS) plus your Rust code.
   - The same built front-end assets from your `frontend/dist/` folder can be
     placed in Tauri’s `distDir` config.
   - You define Tauri commands in Rust, bridging to your orchestrator.

2. **macOS**
   - Tauri can produce a `.app` bundle for macOS. You can sign or notarize it if
     distributing widely.
   - The orchestrator is compiled into the same binary. The user simply
     double-clicks the app to open a window.
   - Real-time logs are handled by Tauri’s event system or by serving SSE
     internally. The logic is effectively the same, reusing your Stage 2 code.

3. **Distribution**
   - If shipping a `.app`, keep all the logic local. The user doesn’t need
     Docker or a web server.
   - If you only want a dev experience, Docker or a local HTTP approach is
     enough.

Hence, your Stage 2 UI code can be used in both Docker-based server scenarios
and Tauri-based local .app distribution for macOS.

---

## 16. Conclusion & Future Roadmap

**lion** has grown from a pure Rust microkernel into a multi-faceted system with
real-time concurrency and an optional **web-based UI**. By adopting the
guidelines above, you ensure:

- **Consistent coding style and concurrency safety** across the microkernel and
  UI layers.
- **Robust integration** of orchestrator events into a real-time streaming front
  end.
- **Flexible deployment**: local dev (`cargo run`), Docker container
  (`docker run -p 8080:8080 lion_ui`), or Tauri `.app` on macOS.

With this vantage, future expansions include advanced workflow diagrams,
multi-tenant plugin marketplaces, or deeper security if you open the UI to
external networks. For now, you’ll produce a local-only Docker-lized or
Tauri-lized environment that developers can use to orchestrate multi-agent AI
tasks with immediate feedback. This synergy of microkernel reliability and
accessible UI design will help lion stand out as a powerful, next-generation AI
orchestrator.

---

**Happy developing** with lion, as we continue to refine both the microkernel’s
concurrency underpinnings and the new Stage 2 front end for real-time
multi-agent operations!
