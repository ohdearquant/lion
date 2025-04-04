**Phase 5: Advanced Features, Usability Enhancements & Polish**.

- **Goal:** Transition from a functional tool to a polished, productive, and
  more powerful IDE. Enhance debugging capabilities, improve workflow
  management, add usability features, and refine the overall user experience.
  Introduce more advanced Lion features if applicable.
- **Core Principle:** Increase developer velocity, improve insight into running
  systems, and add layers of convenience and power over the core functionality
  established in previous phases.

---

**1. Look and Feel (Vibe):**

- **Refinement & Polish:** Focus on smoother animations, consistent
  padding/margins, clearer visual hierarchy, improved icon design. Address any
  UI inconsistencies identified in earlier phases. Optimize rendering
  performance, especially for large workflows or long log lists.
- **Contextual Information:** Enhance tooltips across the application (e.g.,
  hovering over a capability grant shows its full description, hovering over a
  workflow node shows its recent status/output snippet).
- **Theming:** Introduce theme support (Light/Dark modes at a minimum). Ensure
  syntax highlighting in editors respects the selected theme.
- **Error Presentation:** Improve how errors are shown. Instead of just toasts,
  maybe link errors back to the specific node/agent/configuration that caused
  them. Validation errors in editors should be more descriptive.
- **Density Control:** Allow users some control over information density
  (compact vs. comfortable views) in tables and lists.
- **Professionalism:** The application should feel robust, stable, and
  trustworthy, reflecting the security focus of the underlying Lion framework.

---

**2. User Experience (How User Will Use It):**

1. **Workflow Debugging & Inspection:**
   - **Step-Through (Conceptual):** While true step-through debugging of
     WASM/external processes is complex, the UI could _simulate_ it at the
     workflow level. Add "Pause Before Node" and "Step Over Node" controls to
     the Workflow Instance view. When stepping, the executor pauses, the UI
     highlights the next node, and the Inspector shows its expected inputs based
     on completed parent outputs.
   - **State Inspection:** When a workflow instance is paused (manually or via
     simulated step-through), selecting a node allows inspection of its
     _current_ input data (derived from completed dependencies) in the Inspector
     Panel. View overall workflow variables.
   - **Retry/Skip Node (UI):** In the Workflow Instance view, if a node is in
     the "Failed" state, enable "Retry" and "Skip" buttons. Clicking them
     triggers the corresponding backend commands (`retry_node`, `skip_node`).
2. **Enhanced Agent Management:**
   - **Resource Limit Configuration:** Add a section to the Agent Editor to view
     and _request changes_ to resource limits (Memory, CPU). This request might
     need admin approval in a deployed scenario, but the IDE can provide the
     interface.
   - **Hot Reload (WASM):** Add a "Reload Code" button for WASM agents. This
     triggers the `runtime.plugins.update_plugin` command in the backend. UI
     provides feedback on success/failure.
   - **Agent Templates:** "File > New Agent..." could offer templates (e.g.,
     "Basic Python MCP Agent", "WASM Data Processor") that create the necessary
     config file and maybe a minimal code stub.
3. **Knowledge Base Interaction:**
   - **Basic Search:** Add a simple search bar within the Knowledge Base
     view/folder to search filenames or potentially basic content search within
     text files.
   - **(Advanced - Future):** Integrate viewing/querying for structured
     knowledge (e.g., if using a vector store managed by a dedicated plugin, the
     UI could provide an interface to query it, gated by capabilities).
4. **Capability & Policy Usability:**
   - **Capability Templates/Presets:** Offer predefined sets of capabilities for
     common agent types (e.g., "Web Service Client", "File Processor").
   - **Policy Validation:** The Policy Editor could provide basic validation or
     linting for policy syntax/logic (if the policy language allows).
   - **Impact Analysis (Conceptual):** Selecting a capability or policy rule
     could highlight _which_ agents/workflows might be affected by changing or
     removing it (requires backend analysis).
5. **Project Management:**
   - **Templates:** "File > New Project..." offers project templates with basic
     folder structures and example files.
   - **Dependency Management (Conceptual):** If agents have dependencies (e.g.,
     Python packages), the IDE could potentially integrate with package managers
     (highly complex, likely future).
   - **Build Integration (WASM):** If Rust/C++ WASM agents are developed within
     the project structure, add a "Build Agent" command that triggers the
     necessary `cargo build --target wasm32-wasi` (or similar) process. Output
     shown in Console.
6. **General Usability:**
   - **Global Search:** Search across agents, workflows, knowledge files (Ctrl+P
     / Cmd+P).
   - **Tab Management:** Improved handling for many open editor tabs (scrolling,
     closing).
   - **Keyboard Shortcuts:** Define standard shortcuts for common actions (Save,
     Open, Run, etc.).
   - **Settings/Preferences:** UI for configuring IDE appearance (theme),
     potentially paths to interpreters (Python, Node), Lion runtime overrides.

---

**3. Lion Integration (How It Should Use Lion):**

- **Workflow Debugging (`lion_workflow::engine::executor` / `state::machine`):**
  - **Tauri Commands:** Need `pause_workflow_instance`,
    `resume_workflow_instance`,
    `step_workflow_instance(instance_id, node_id_to_execute_next)`,
    `get_node_inputs(instance_id, node_id) -> Result<JsonValue, String>`,
    `retry_node(instance_id, node_id)`, `skip_node(instance_id, node_id)`.
  - **Backend Logic:** The `WorkflowExecutor` needs logic to handle pause/resume
    signals. Step-through requires careful state management – pausing execution
    _before_ scheduling a specific node, getting its calculated inputs from the
    `WorkflowState`, and only proceeding when commanded. Retry/Skip involves
    updating the `NodeStatus` in the `WorkflowState` and potentially
    re-triggering scheduling logic.
- **Agent Resource Limits (`lion_isolation::traits::IsolationBackend`):**
  - **Tauri Command:**
    `set_agent_resource_limit(agent_id: String, limit_type: String, value: u64) -> Result<(), String>`.
  - **Backend Logic:** Parses `limit_type` to `ResourceLimitType`. Calls the
    underlying `isolation_backend.set_resource_limit`. Requires the backend
    trait/implementation to support this.
- **Agent Hot Reload (`lion_runtime::plugin::manager`):**
  - **Tauri Command:**
    `hot_reload_agent(agent_id: String, wasm_path: String) -> Result<(), String>`.
  - **Backend Logic:** Reads new WASM bytes. Calls
    `runtime.plugins.update_plugin(agent_id, &bytes).await`.
- **Advanced Capability/Policy Features:**
  - **Impact Analysis:** Requires backend logic in `lion_runtime` (or dedicated
    services) that can query the `CapabilityStore` and `PolicyStore` and analyze
    relationships between agents, capabilities, policies, and potentially
    workflow definitions.
- **Build Integration:** The backend command would simply execute the
  appropriate build tool (`cargo`, `npm`, etc.) as a subprocess
  (`tokio::process::Command`) in the project directory, streaming output back to
  the frontend Console view via events.

---

**4. Tauri Backend Implementation Details (Rust - Phase 5):**

- **`commands.rs`:** Add commands for pause/resume/step/retry/skip workflow
  nodes, getting node inputs, setting resource limits, hot reloading, build
  tasks.
- **`runtime_listener.rs` / State Management:** Enhance state management to
  handle more complex UI needs, potentially caching detailed instance states
  requested by the UI. Refine event payloads for more granular updates.
- **Workflow Engine Interaction:** Implement the backend logic for the new
  workflow control commands, carefully interacting with the `WorkflowExecutor`
  and `StateMachineManager` APIs. This might require adding
  pause/step/retry/skip methods to those core components if they don't exist.
- **Settings:** Implement loading/saving IDE preferences (e.g., using
  `tauri-plugin-store` or simple file I/O).
- **Background Tasks:** Use `tokio::spawn` for potentially long-running tasks
  like builds or complex analysis, reporting progress/completion via events.

---

**5. Frontend Implementation Details (React/TS - Phase 5):**

- **API Layer (`src/lib/api.ts`):** Add functions for all new commands.
- **Components:**
  - `WorkflowEditor.tsx`: Add visual indicators for breakpoints/paused state.
    Handle UI interactions for step/retry/skip controls, calling backend
    commands. Display node input/output data in the Inspector when a node is
    selected in a paused/completed instance.
  - `AgentEditor.tsx`: Add form section for viewing/editing resource limits. Add
    "Hot Reload" button for WASM agents.
  - `SecurityView.tsx`: Add UI elements for capability presets/templates. Add
    validation/linting display for policies.
  - `KnowledgeView.tsx`: Add a search input field.
  - `BuildOutputView.tsx`: (Potentially part of Console) Display output from
    build commands.
  - `SettingsModal.tsx`: UI for application preferences.
- **State Management:** Manage state for workflow debugging (paused instance ID,
  current step), IDE settings.
- **Performance:** Optimize rendering of large lists/graphs. Use memoization
  (`React.memo`), virtualization (`react-window`), and efficient state updates.
  Debounce frequent events if necessary.
- **UX Refinements:** Implement better loading states, clearer error messages,
  consistent keyboard navigation, tooltips.

---

**6. Phase 5 Acceptance Criteria:**

- User can set a "breakpoint" (pause before node) on a workflow node via the UI.
- When a workflow instance hits a breakpoint, execution pauses, and the UI
  indicates the paused node and allows inspection of its calculated input data.
- User can "step over" a paused node, executing it and pausing before the next
  node(s).
- User can "resume" a paused workflow instance.
- User can trigger a "retry" or "skip" action on a failed node via the UI, and
  the workflow state updates accordingly.
- User can view and request changes to agent resource limits via the Agent
  Editor.
- User can trigger a "Hot Reload" for a WASM agent, and the backend attempts the
  update.
- Basic file content search is available for the Knowledge Base section.
- Basic project and agent templates are available under "File > New...".
- UI offers Light/Dark themes.
- Common usability issues (e.g., slow rendering of large lists, inconsistent
  interactions) identified in previous phases are addressed.

Phase 5 transforms LionForge from a functional tool into a more mature IDE,
focusing on developer productivity, debugging, and refining the core workflows
for managing secure multi-agent systems. It adds layers of intelligence and
convenience on top of the secure Lion foundation.
