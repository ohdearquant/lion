**Phase 2: Core Execution, Agent Management & Workflow Viewing**.

- **Goal:** Transition from a purely observational tool to one that allows basic
  control over core entities. Enable users to load/unload agents, view workflow
  structures, and initiate workflow execution. Introduce the concept of managing
  entities _through_ the IDE, not just observing external state.
- **Core Principle:** Build interactive capabilities on the established
  foundation. Implement the primary user actions for managing agents and
  starting workflows.

---

**1. Look and Feel (Vibe):**

- **Interactivity:** UI elements become more active. Buttons (Load Agent, Start
  Workflow) are enabled. Context menus in the Project Explorer become
  functional. Tables (Agent Status, Workflow Instances) might gain action
  buttons.
- **Visual Feedback:** Clear visual cues for actions (e.g., button states
  changing, loading indicators during agent load, success/error
  notifications/toasts).
- **Workflow Visualization:** The center Editor Area now renders workflow
  definitions as graphs. It should look clean, readable, and professional. Nodes
  should clearly display their names/types. Edges should indicate dependencies.
  Use standard DAG layout algorithms initially. Ensure smooth zooming and
  panning.
- **Consistency:** Maintain the professional, developer-centric aesthetic
  established in Phase 1. New UI elements (forms, modals for input) should match
  the existing style guide.
- **Responsiveness:** As operations like loading agents or starting workflows
  might take time, the UI must provide immediate feedback (e.g., "Loading
  agent...") and remain responsive. Asynchronous operations are key.

---

**2. User Experience (How User Will Use It):**

1. **Agent Loading:**
   - User right-clicks within the `agents/` folder (or a dedicated "Agents"
     view) in the Project Explorer and selects "Load Agent from File..." (or
     similar).
   - A file dialog opens, allowing the user to select a WASM file or potentially
     an agent configuration file (`*.agent.json`?).
   - Alternatively, if agent definitions exist in the project
     (`agents/agent_b.config.json`), the user might right-click the definition
     file and select "Load Agent".
   - The UI shows a "Loading..." status indicator.
   - Upon success: A notification confirms loading, and the agent appears in the
     "Agent Status" view (Bottom Panel) with state "Ready".
   - Upon failure: An error notification is displayed (e.g., "Failed to load
     agent: WASM validation error").
2. **Agent Unloading:**
   - User selects an agent in the "Agent Status" view.
   - Clicks an "Unload" button (or uses a context menu).
   - A confirmation dialog might appear ("Are you sure you want to unload agent
     X?").
   - The UI shows an "Unloading..." status.
   - Upon success: A notification confirms unloading, and the agent is removed
     from the "Agent Status" view.
   - Upon failure: An error notification is displayed.
3. **Workflow Viewing:**
   - User clicks on a workflow definition file (e.g., `main_workflow.json`) in
     the Project Explorer (`workflows/` folder).
   - The Center Editor Area opens a new tab displaying the workflow as a visual
     graph (using React Flow or similar).
   - Nodes and edges are rendered based on the definition file content.
   - Users can pan and zoom the graph. Selecting a node _might_ show basic
     properties in the Right Sidebar Inspector (full editing is Phase 3).
4. **Workflow Execution (Starting):**
   - With a workflow definition open and visible in the Editor Area, a "Start
     Instance" button is available (e.g., in the editor's toolbar or the Right
     Sidebar Inspector).
   - Clicking "Start Instance" might open a simple modal dialog asking for
     initial input data (as JSON). User enters input (or leaves blank) and
     clicks "Start".
   - The UI shows a "Starting workflow..." indicator.
   - Upon success: A notification confirms the start ("Workflow instance 'XYZ'
     started"), and the new instance appears in the "Workflow Instances" view
     (Bottom Panel) with status "Running" or "Pending".
   - Upon failure: An error notification is displayed (e.g., "Failed to start
     workflow: Definition validation failed").
5. **Workflow Instance Monitoring (Basic):**
   - The "Workflow Instances" view (Bottom Panel) now lists instances started
     via the UI (or potentially discovered from the runtime backend if
     persistence exists there).
   - Displayed columns: Instance ID, Workflow Name (from definition), Status
     (Running, Completed, Failed, etc.), Start Time.
   - The view listens for backend events (`workflow_status_changed`) and updates
     instance statuses dynamically.
   - _Interaction:_ Selecting an instance highlights it. Clicking it _might_
     eventually focus the corresponding workflow graph in the editor and overlay
     status (Phase 3/4). Basic "Cancel" button could be added here.

---

**3. Lion Integration (How It Should Use Lion):**

- **Agent Loading (`lion_runtime::plugin::manager`):**
  - **Tauri Command:**
    `load_agent(agent_def_path: String) -> Result<PluginId, String>`.
  - **Backend Logic:** Reads the agent definition file (or directly uses WASM
    path). Determines if it's WASM or External. Calls the appropriate
    `runtime.plugins.load_plugin(...)` (needs to read WASM bytes if necessary)
    or the extended logic for launching external process + proxy (developed in
    Phase 1 foundation). Returns the assigned `PluginId`. Emits
    `agent_status_changed` event.
- **Agent Unloading (`lion_runtime::plugin::manager`):**
  - **Tauri Command:** `unload_agent(agent_id: String) -> Result<(), String>`.
  - **Backend Logic:** Parses `agent_id` to `PluginId`. Calls
    `runtime.plugins.unload_plugin(&plugin_id).await`. Handles potential errors.
    Emits `agent_status_changed` event (or an `agent_removed` event).
- **Workflow Definition Loading/Parsing (`lion_workflow::model::definition`):**
  - **Tauri Command:**
    `load_workflow_definition(path: String) -> Result<WorkflowDefJson, String>`.
  - **Backend Logic:** Reads the file content (JSON or YAML). Parses it into
    `lion_workflow::model::WorkflowDefinition`. Serializes the definition (or a
    simplified version suitable for the UI graph) into a JSON structure
    (`WorkflowDefJson`) that the frontend library (e.g., React Flow) can easily
    consume (list of nodes with positions - initially random/auto-layout, list
    of edges). Returns the JSON.
- **Workflow Starting (`lion_runtime::workflow::manager`):**
  - **Tauri Command:**
    `start_workflow_instance(workflow_id: String, input_json: Option<String>) -> Result<ExecutionId, String>`.
  - **Backend Logic:** Parses `workflow_id`. Parses `input_json` into
    `serde_json::Value`. Calls
    `runtime.workflows.start_workflow(workflow_id, input_value).await`. Returns
    the new `ExecutionId`. Emits `workflow_status_changed` event for the new
    instance (Status: Pending/Running).
- **Workflow Instance Listing (Requires Enhancement in
  `lion_runtime`/`lion_workflow`):**
  - **Tauri Command:**
    `list_workflow_instances(filter: Option<InstanceFilter>) -> Result<Vec<InstanceSummary>, String>`.
  - **Backend Logic:** `lion_runtime::workflow::manager` needs a method to list
    active/recent instances. It would query its internal `WorkflowExecutor` (or
    underlying state manager) for instances and their statuses. Maps internal
    state to a simple
    `InstanceSummary { instance_id: String, workflow_name: String, status: String, start_time: String }`
    for the frontend.
- **Workflow Status Events (Backend Event Listener -> Tauri Event):**
  - The `lion_runtime::workflow::manager` or its `WorkflowExecutor` needs to
    emit events when an instance's overall status changes (Running, Completed,
    Failed, Paused, Cancelled).
  - The Tauri backend runtime listener subscribes to these and emits
    `workflow_status_changed` events (with
    `InstanceStatusUpdate { instance_id: String, new_status: String }` payload)
    to the frontend.

---

**4. Tauri Backend Implementation Details (Rust - Phase 2):**

- **`commands.rs`:**
  - Add new commands: `load_agent`, `unload_agent`, `load_workflow_definition`,
    `start_workflow_instance`, `list_workflow_instances`,
    `cancel_workflow_instance`.
  - Implement the logic using `tauri::State<'_, Arc<Runtime>>` to access the
    Lion runtime managers.
  - Define new serializable structs for command arguments and return payloads
    (`AgentDefinition`, `WorkflowDefJson`, `InstanceFilter`, `InstanceSummary`,
    `InstanceDetails`, etc.).
- **`runtime_listener.rs`:**
  - Add logic to subscribe to _workflow instance status events_ from
    `lion_runtime`.
  - Define `struct InstanceStatusUpdate` (or similar) implementing
    `Clone, Serialize`.
  - Emit `workflow_status_changed` events to the frontend via `AppHandle`.
- **Error Handling:** Enhance error mapping to provide more context to the
  frontend (e.g., distinguish between "file not found" and "WASM validation
  failed" during agent load).
- **File Dialogs:** Use Tauri's dialog API
  (`tauri::api::dialog::blocking::FileDialogBuilder`) within the backend
  commands (like `load_agent`) when triggered by the frontend if a path isn't
  directly provided.

---

**5. Frontend Implementation Details (React/TS - Phase 2):**

- **API Layer (`src/lib/api.ts`):** Add typed functions for the new backend
  commands (`loadAgent`, `unloadAgent`, `loadWorkflowDefinition`,
  `startWorkflowInstance`, `listWorkflowInstances`, `cancelWorkflowInstance`).
- **Components:**
  - `ProjectExplorer.tsx`: Add right-click context menus ("Load Agent", "Open
    Workflow") that call the relevant API functions.
  - `AgentStatusView.tsx`: Add "Unload" button per agent row, triggering
    `unloadAgent`. Update to handle `agent_removed` events if implemented. Add a
    "Load Agent" button somewhere (maybe toolbar) to trigger the load
    command/dialog.
  - `WorkflowEditor.tsx`: New component. Takes `WorkflowDefJson` as prop. Uses
    `react-flow` (or similar) to render nodes and edges. Implements basic
    panning/zooming.
  - `WorkflowInstancesView.tsx`: New component in the bottom panel. Fetches
    initial list via `listWorkflowInstances`. Uses `useTauriEvent` hook for
    `workflow_status_changed` to update instance statuses. Renders a table. Add
    a "Cancel" button per row.
  - `MainArea.tsx`: Modify to handle tabs. When a workflow file is clicked in
    ProjectExplorer, call `loadWorkflowDefinition`, and open a new tab
    containing `WorkflowEditor` with the returned data.
  - `StartWorkflowModal.tsx`: (Optional) A simple modal triggered by the "Start
    Instance" button, containing a JSON editor (e.g., `react-json-editor-ajrm`)
    for input data. Calls `startWorkflowInstance` on submit.
  - `Notifications.tsx`: Implement a toast/notification system (e.g., using
    `react-hot-toast`) to display success/error messages from backend command
    results.
- **State Management:** Extend the state store (e.g., Zustand) to manage:
  - The currently opened workflow definition
    (`currentWorkflowDef: WorkflowDefJson | null`).
  - The list of workflow instances (`workflowInstances: InstanceSummary[]`).
  - Potentially loading states for various actions (`isAgentLoading: boolean`,
    `isWorkflowStarting: boolean`).

---

**6. Phase 2 Acceptance Criteria:**

- User can successfully load a WASM agent via the UI, and it appears in the
  Agent Status view as "Ready".
- User can successfully unload an agent via the UI, and it disappears from the
  Agent Status view.
- Clicking a workflow definition file (`.json`/`.yaml`) in the Project Explorer
  opens a visual representation of the DAG in the main editor area.
- User can click a "Start Instance" button for an open workflow definition.
- A new workflow instance appears in the "Workflow Instances" view with status
  "Running" (or similar initial state).
- If the backend `lion_runtime` updates the status of an agent or workflow
  instance, the change is reflected dynamically in the corresponding UI views
  without manual refresh.
- Error conditions (e.g., trying to load an invalid WASM, starting a
  non-existent workflow) result in user-visible error notifications.
- The basic UI remains responsive while backend operations are in progress.

This phase focuses on enabling the core user actions of managing agents and
workflows, making the IDE significantly more interactive and useful, while still
relying on the robust Lion backend for the actual execution and state
management.
