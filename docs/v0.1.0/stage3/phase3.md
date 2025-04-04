**Phase 3: Workflow Editing, Security Configuration & Enhanced Monitoring**.

- **Goal:** Empower the user to define and modify system behavior. Introduce
  visual editing for workflows, and provide interfaces for managing Lion's
  security model (Capabilities and Policies). Enhance monitoring to show more
  workflow execution details.
- **Core Principle:** Shift from observation to creation and configuration. Make
  security a tangible, configurable aspect of the agent system within the IDE.

---

**1. Look and Feel (Vibe):**

- **Editing Interfaces:**
  - **Workflow Editor:** Should feel intuitive and powerful. Nodes should be
    clearly distinguishable by type (icon, color). Selecting a node or edge
    brings up its configuration in the Right Sidebar Inspector, which should use
    clear forms (dropdowns, text inputs, toggles) for parameters. Connections
    should snap easily. Validation errors (e.g., cycles, missing dependencies)
    should be visually highlighted directly on the graph.
  - **Security Views:** These need to feel trustworthy and clear. Visualizing
    capability grants (e.g., graph-based Agent <-> Resource) should make
    relationships obvious. Policy editors should present rules logically. Use
    distinct visual cues (colors, icons) for Allow vs. Deny
    policies/capabilities.
- **Feedback & Validation:** Provide immediate visual feedback during editing
  (e.g., highlighting invalid connections in the workflow editor). Saving
  actions should provide clear success or detailed error messages (e.g.,
  "Workflow saved", "Validation failed: Cycle detected between NodeA and
  NodeB").
- **Density vs. Clarity:** Security configurations can become complex. Strive
  for clarity, but allow for information density in tables (policies,
  capabilities). Use tooltips and expandable sections.
- **Consistency:** Forms and controls used in the Inspector Panel for workflows
  should resemble those used for editing capabilities or policies where
  applicable (e.g., selecting Plugin IDs).

---

**2. User Experience (How User Will Use It):**

1. **Workflow Editing:**
   - User opens a workflow file (`.json`/`.yaml`) from the Project Explorer.
   - The visual editor displays the DAG in the Center Pane.
   - **Adding Nodes:** User drags node types (Plugin Call, Condition, Map, etc.)
     from a palette (perhaps in the Left Sidebar or a toolbar) onto the canvas.
   - **Connecting Nodes:** User drags from an output handle on one node to an
     input handle on another to create an edge/dependency. The UI might prevent
     creating cycles visually or highlight them immediately.
   - **Configuring Nodes:** User clicks a node. The Right Sidebar Inspector
     populates with configuration options specific to the node type (e.g., for
     Plugin Call: dropdowns/search to select Agent ID, text input for function
     name, JSON editor for input parameters mapping).
   - **Configuring Edges:** User clicks an edge. Inspector shows options for
     adding conditions (e.g., dropdown for condition type, input for
     JSONPath/Expression).
   - **Saving:** User clicks a "Save" button (or uses Ctrl+S). The frontend
     sends the updated workflow structure (e.g., as JSON representing nodes and
     edges) to the backend. The backend validates and saves the file.
     Success/error notification shown.
2. **Capability Management (Security View):**
   - User navigates to the "Security" or "Capabilities" view (dedicated tab or
     section).
   - **Viewing:** Selects an agent from a list/tree. The main area displays the
     capabilities currently granted to that agent (e.g., `file:read:/data/*`,
     `network:connect:api.example.com:443`).
   - **Granting:** User clicks "Grant Capability". A form/wizard appears:
     - Select Agent ID.
     - Select Capability Type (dropdown: `file`, `network`, `plugin_call`,
       `mcp:send`, etc.).
     - Fill in type-specific parameters (e.g., for `file`: path pattern,
       read/write toggles; for `mcp:send`: target agent ID).
     - Click "Grant". The backend attempts to grant the capability via
       `lion_capability`, and the UI updates the agent's capability list.
   - **Revoking:** User selects an existing capability grant in the
     list/visualizer. Clicks "Revoke". Confirmation dialog appears. Backend
     revokes the capability via `lion_capability`. UI updates.
3. **Policy Management (Security View or Editor):**
   - User navigates to the "Policies" section.
   - **Viewing:** A table displays existing policy rules (ID, Subject, Object,
     Action, Priority).
   - **Adding:** User clicks "Add Policy Rule". A form appears:
     - Inputs/Dropdowns for Rule ID, Name, Description.
     - Dropdowns/Inputs for Subject (Any, Plugin ID, Tag), Object (Any, File
       path, Network host/port), Action (Allow, Deny, AllowWithConstraints).
     - (Optional) Text area for Conditions or Constraint strings.
     - Input for Priority.
     - Click "Save". Backend creates and stores the policy rule via
       `lion_policy`. UI table refreshes.
   - **Editing/Deleting:** Select a rule in the table, click "Edit" or "Delete".
     Forms similar to "Add" or confirmation dialogs appear. Backend
     updates/removes the rule.
4. **Enhanced Workflow Monitoring:**
   - User selects a _running_ or _completed_ instance in the "Workflow
     Instances" view (Bottom Panel).
   - The Workflow Editor view in the Center Pane now overlays status onto the
     graph:
     - Nodes change color based on status (e.g., Green=Completed, Red=Failed,
       Blue=Running, Gray=Pending/Skipped).
     - Edges might change color based on whether they were traversed.
   - Clicking a completed/failed node in the monitoring view shows its final
     output data or error message in the Right Sidebar Inspector.

---

**3. Lion Integration (How It Should Use Lion):**

- **Workflow Definition Handling (`lion_workflow`):**
  - **Tauri Command:**
    `save_workflow_definition(path: String, definition: WorkflowDefJson) -> Result<(), String>`:
    Backend receives JSON from the frontend editor, parses it into
    `lion_workflow::model::WorkflowDefinition`, calls `definition.validate()`,
    and if valid, serializes and writes back to the specified file path.
  - **Tauri Command:**
    `validate_workflow_definition(definition: WorkflowDefJson) -> Result<(), Vec<ValidationErrorJson>>`:
    Parses definition, calls `validate()`, returns structured validation errors
    if any.
- **Capability Management (`lion_runtime::capabilities::manager` +
  `lion_capability`):**
  - **Tauri Command:**
    `get_agent_capabilities(agent_id: String) -> Result<Vec<CapabilityInfo>, String>`:
    Backend calls `runtime.capabilities` (needs a method to list capabilities
    for a subject, likely querying the underlying `lion_capability::store`).
    Maps the raw capabilities into a frontend-friendly `CapabilityInfo` struct
    (ID, type, description, parameters).
  - **Tauri Command:**
    `grant_capability(agent_id: String, cap_def: CapabilityDefinitionJson) -> Result<CapabilityId, String>`:
    Backend parses `cap_def` into the correct `lion_capability` enum variant
    (e.g., `FileCapability`, `NetworkCapability`). Calls
    `runtime.capabilities.grant_capability(agent_id, object, rights).await`.
    Returns the new `CapabilityId`.
  - **Tauri Command:**
    `revoke_capability(agent_id: String, capability_instance_id: String) -> Result<(), String>`:
    Parses IDs. Calls `runtime.capabilities.revoke_capability(cap_id).await`.
  - **Tauri Command:**
    `list_available_capability_types() -> Result<Vec<CapabilityTypeDesc>, String>`:
    Backend needs a way (possibly hardcoded initially, or dynamic registration
    later) to know what capability types exist and what parameters they take, so
    the UI can build the "Grant Capability" form dynamically. Returns
    descriptions like
    `{ name: "file", params: [{ name: "path", type: "string"}, { name: "read", type: "boolean" }, ...] }`.
- **Policy Management (`lion_policy` via `lion_runtime`):**
  - **Tauri Command:** `list_policies() -> Result<Vec<PolicyRuleJson>, String>`:
    Calls `runtime.policies.list_rules()` (needs adding to runtime facade), maps
    `PolicyRule` structs to a JSON-friendly format.
  - **Tauri Command:** `add_policy(rule: PolicyRuleJson) -> Result<(), String>`:
    Parses JSON into `PolicyRule`, calls `runtime.policies.add_rule(rule)`.
  - **Tauri Command:** `remove_policy(rule_id: String) -> Result<(), String>`:
    Calls `runtime.policies.remove_rule(&rule_id)`.
  - **Tauri Command:**
    `update_policy(rule: PolicyRuleJson) -> Result<(), String>`: Parses JSON
    into `PolicyRule`, calls `runtime.policies.update_rule(rule)`.
- **Enhanced Workflow Monitoring (`lion_workflow::state::machine`):**
  - **Tauri Command:**
    `get_workflow_instance_details(instance_id: String) -> Result<InstanceDetails, String>`:
    Fetches the full `WorkflowState` from the `StateMachineManager`. Includes
    `node_status`, `node_results`, `edge_conditions`. Maps this to a detailed
    `InstanceDetails` struct for the frontend.
  - **Backend Events:** `workflow_status_changed` events (from Phase 2) may need
    to be enhanced to include
    `node_status_changed(instance_id, node_id, new_status, result/error)`
    payloads to allow the UI to update the visual graph overlay dynamically.

---

**4. Tauri Backend Implementation Details (Rust - Phase 3):**

- **Serialization:** Define robust `struct`s (`WorkflowDefJson`,
  `CapabilityInfo`, `CapabilityDefinitionJson`, `PolicyRuleJson`,
  `InstanceDetails`, etc.) with `Serialize, Deserialize` that map cleanly
  between Rust types and the JSON the frontend expects/sends. Use `serde_json`.
- **`commands.rs`:** Add commands for saving/validating workflows,
  listing/granting/revoking capabilities, listing/adding/updating/removing
  policies, getting detailed instance state. Implement the logic using
  `tauri::State<'_, Arc<Runtime>>`. Add more specific error mapping.
- **`events.rs`:** Define `struct NodeStatusUpdate` (or similar) payload for
  backend-to-frontend events indicating node status changes within a workflow
  instance.
- **`runtime_listener.rs`:** Subscribe to finer-grained events from
  `lion_workflow`'s state machine (if available) regarding node status changes
  and emit them to the frontend.
- **Capability Definition:** Implement the logic for
  `list_available_capability_types` - this might involve inspecting the
  `lion_capability::model` enums or having a registration mechanism.

---

**5. Frontend Implementation Details (React/TS - Phase 3):**

- **API Layer (`src/lib/api.ts`):** Add typed functions for all new backend
  commands. Update payload types.
- **Components:**
  - `WorkflowEditor.tsx`: Enhance with:
    - Node/Edge selection handling.
    - Integration with a `PropertiesInspector.tsx` component (likely in Right
      Sidebar).
    - Displaying validation errors visually on the graph.
    - Handling state updates from `node_status_changed` events to apply visual
      overlays (colors, icons).
    - "Save" button triggering `save_workflow_definition`.
    - Toolbar for adding nodes (drag-and-drop).
  - `PropertiesInspector.tsx`: Dynamically renders forms based on selected
    node/edge type. Uses form libraries (e.g., `react-hook-form`) for state
    management and validation within the form.
  - `SecurityView.tsx`: New top-level view/tab. Contains sub-components:
    - `AgentSelector.tsx`: List/tree of agents.
    - `CapabilityViewer.tsx`: Displays granted capabilities for the selected
      agent (table or graph). Buttons trigger `grantCapability`,
      `revokeCapability`.
    - `GrantCapabilityForm.tsx`: Modal or form displayed when granting.
      Dynamically builds fields based on data from
      `list_available_capability_types`.
    - `PolicyListViewer.tsx`: Table of policies. Buttons trigger
      add/edit/delete.
    - `PolicyEditorForm.tsx`: Modal/form for adding/editing policies.
  - `WorkflowInstancesView.tsx`: Enhance selection logic. Clicking an instance
    now loads its details (`get_workflow_instance_details`) and potentially
    signals the `WorkflowEditor` to display the corresponding definition with
    status overlays.
  - `KnowledgeEditor.tsx`: Basic text editing implemented using Monaco editor
    component. Save triggers `write_file`.
- **State Management:** Add state slices for:
  - Current security view selection (selected agent, selected capability).
  - List of policies.
  - Available capability type definitions.
  - Detailed state of the currently viewed workflow instance (including node
    statuses/results).

---

**6. Phase 3 Acceptance Criteria:**

- User can open a workflow file, see the visual DAG, add/remove nodes, connect
  nodes with edges, and configure basic node properties (e.g., plugin ID,
  function name) in an inspector panel.
- User can save the modified workflow definition back to its file, with backend
  validation preventing saving of invalid workflows (e.g., cycles).
- A dedicated Security view exists.
- User can select an agent and view its currently granted Lion capabilities.
- User can use a form to grant a new capability (e.g., file read for `/data`) to
  an agent, and it appears in the agent's capability list.
- User can select a granted capability and revoke it.
- User can view a list of existing policy rules.
- User can add a new policy rule (e.g., Deny Agent X access to Network Y) via a
  form, and it appears in the list.
- User can delete an existing policy rule.
- When viewing a running/completed workflow instance, the visual workflow editor
  overlays node statuses (e.g., color-coding).
- Clicking a completed/failed node in the monitored workflow view displays its
  output/error in the inspector.
- Basic text editing and saving works for files in the `knowledge/` directory.

Phase 3 significantly expands the IDE's capabilities, turning it into a tool for
_defining_ behavior and security, not just observing it. The visual workflow
editor and the security configuration views are the core deliverables here.
