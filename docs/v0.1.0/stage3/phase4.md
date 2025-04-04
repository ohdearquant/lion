**Phase 4: External Agent Integration, MCP Security, Auditing & Advanced
Monitoring**.

- **Goal:** Bridge the gap to existing agent ecosystems (Python, JS) by enabling
  secure execution via proxies. Implement robust audit trails. Provide specific
  controls for MCP communication. Offer deeper performance insights through
  metrics visualization.
- **Core Principle:** Extend Lion's security umbrella over external processes.
  Make security actions transparent and auditable. Provide operational insights.

---

**1. Look and Feel (Vibe):**

- **Agent Editor (External):** The form for defining external agents should
  clearly delineate between Lion's configuration (like assigned `PluginId`,
  capabilities to request) and the external process configuration (script path,
  interpreter, arguments, environment variables). It should feel like
  configuring a managed service within LionForge.
- **Security Audit Log:** This view needs to inspire confidence and clarity. A
  clean, dense table format is likely best. Use clear icons or color-coding for
  Allow/Deny outcomes. Timestamps should be precise. Provide powerful filtering
  and search capabilities without feeling cluttered. Clicking an entry should
  reveal all associated details (full capability checked, request parameters if
  logged, policy rule involved if applicable).
- **MCP Security View:** This could be a specialized view within the Security
  Pane. It might use a graph visualization showing Agent nodes and external
  Endpoint nodes, with connections indicating allowed `mcp:send` or
  `network:connect` capabilities. Alternatively, a structured table filtering
  capabilities relevant to communication could work. The key is making _allowed
  communication paths_ explicit and easy to manage.
- **Metrics View:** Should feel like a standard monitoring dashboard. Use clear
  line charts for time-series data (CPU, memory), gauges for current states
  (active agents), and potentially histograms or heatmaps for latency
  distributions. Allow users to select time ranges and specific agents/workflows
  to focus on.
- **Python SDK:** (Developer Experience, not UI) The SDK's API should feel
  pythonic. Clear function names (`lion.read_file(...)`,
  `lion.send_mcp_message(...)`), good error handling (custom Python exceptions
  mapping to Lion/Proxy errors), and comprehensive docstrings/examples are
  essential.

---

**2. User Experience (How User Will Use It):**

1. **Defining & Configuring External Agents:**
   - User right-clicks `agents/` -> "New External Agent Definition".
   - A file (`agent_name.agent.json` or `.toml`) is created and opened in the
     Editor Area (Center Pane).
   - The editor presents a form (or structured text view) with fields:
     - `name`: Human-readable name.
     - `agent_type`: Dropdown (Python, JavaScript, Other...).
     - `interpreter`: Path to interpreter (e.g., `/usr/bin/python3`, `node`).
     - `script_path`: Path (relative to project root) to the agent's main
       script.
     - `arguments`: (Optional) List of command-line arguments for the script.
     - `environment_variables`: (Optional) Key-value pairs. Crucially, LionForge
       will inject the `LION_PROXY_IPC_INFO` (socket path/port/token) here.
   - User saves the definition file.
2. **Loading/Running External Agents:**
   - User right-clicks the agent definition file in the Project Explorer ->
     "Load Agent" OR selects the (unloaded) agent in the Agent Status View ->
     "Load".
   - UI shows "Loading Agent..." / "Starting Proxy..." / "Starting Process...".
   - The corresponding Trusted Proxy Plugin starts within Lion.
   - The external agent process is launched by the Lion backend.
   - The agent appears in the Agent Status view, possibly indicating both proxy
     and external process health.
3. **Developing with Python SDK:**
   - Developer installs the `lion-agent-sdk` package
     (`pip install lion-agent-sdk`).
   - In their Python script (`agent_script.py`), they import the SDK:
     `from lion_agent_sdk import LionAgent`.
   - They initialize the agent: `lion = LionAgent()` (SDK reads connection info
     from env vars set by LionForge).
   - They make calls: `file_content = lion.read_file("/data/shared.csv")`,
     `lion.send_mcp_message(target_agent_id="data_analyzer", payload={"task": "..."})`.
   - The SDK handles communication with the proxy and returns results or raises
     exceptions (e.g., `CapabilityError`, `IPCError`).
4. **Viewing Security Audit Logs:**
   - User clicks the "Audit Log" tab in the Bottom Panel.
   - A table displays recent security events chronologically (newest first).
     Columns: Timestamp, Agent ID, Action Type (e.g., `file:read`, `mcp:send`),
     Resource/Target, Capability Checked, Outcome (Allow/Deny), Policy Rule (if
     applicable).
   - User uses filter controls (dropdowns, text search) to narrow down events
     (e.g., show only "Deny" events for "Agent X").
5. **Configuring MCP Security:**
   - User navigates to the Security View -> "MCP Communication" section.
   - They see a list/graph of agents and potentially defined external endpoints.
   - To allow Agent A to send MCP messages to Agent B: Select Agent A, click
     "Grant Capability", choose type `mcp:send`, enter `AgentB` as the target
     parameter, click "Grant".
   - To allow Agent C to connect to an external MCP service: Select Agent C,
     click "Grant Capability", choose type `network:connect`, enter
     `mcp.openai.com:443` (example) as the resource parameter, click "Grant".
6. **Viewing Metrics:**
   - User clicks the "Metrics" tab in the Bottom Panel (or a dedicated
     "Monitoring" view).
   - Default charts show system-level metrics (e.g., Lion Runtime CPU/Memory,
     Total Active Agents, Total Workflow Instances).
   - Dropdowns allow selecting specific agents or workflows to view their
     metrics (if available), such as function call counts, average execution
     times, agent-specific memory/CPU.
   - Time range selector allows viewing historical data (requires
     `lion_observability` backend to store metrics over time).

---

**3. Lion Integration (How It Should Use Lion):**

- **External Agent Definition Parsing (Tauri Backend):** Needs logic to read and
  interpret the `*.agent.json` files.
- **External Process Lifecycle (`lion_runtime::plugin::manager`):**
  - Extend `PluginManager::load_plugin` (or add `load_external_agent`) to handle
    the sequence: start proxy plugin, start external process
    (`tokio::process::Command`), pass IPC details (env var like
    `LION_PROXY_IPC_INFO` containing socket path/token), store association
    between `PluginId` and the external `Child` process handle.
  - Extend `PluginManager::unload_plugin` to terminate the external process
    (`child.kill().await`) and unload the proxy plugin.
  - Implement monitoring: Spawn a Tokio task that waits on the
    `child.wait().await`. When the external process exits, this task updates the
    agent's state (via `PluginManager` API) to `Failed` or `Terminated` and
    emits an `agent_status_changed` event.
- **Trusted Proxy Plugin (Rust - `lion_proxy_plugin`):**
  - **IPC Server:** Implement the chosen IPC listener (e.g.,
    `tokio::net::UnixListener` or WebSocket server). Handle connections,
    authenticate if using tokens. Associate incoming connections with the
    correct external agent `PluginId`.
  - **Request Loop:** Listen for incoming requests, deserialize them.
  - **Capability Enforcement:** For _every_ request type (`send_mcp_message`,
    `read_file`, `make_network_request`, etc.):
    - Construct the appropriate `lion_core::AccessRequest` or capability string
      identifier (e.g., `file:read:/path/to/file`, `mcp:send:TargetAgentID`,
      `network:connect:host:port`).
    - Use the injected `CapabilityResolver` (from
      `lion_runtime::capabilities::resolution`) to call
      `resolver.authorize(agent_plugin_id, resource_string, action_string).await`.
  - **Auditing:** Before returning success/failure, use the injected
    `PluginObservability` instance (from `lion_observability`) to log an audit
    event:
    `plugin_obs.log(LogLevel::Info, "CapabilityCheck", attributes={"agent_id": ..., "operation": ..., "resource": ..., "outcome": "Allow/Deny"})`.
  - **Action Execution:** If authorized, perform the action:
    - `read_file`/`write_file`: Use `tokio::fs`.
    - `make_network_request`: Use `reqwest` or `hyper`.
    - `send_mcp_message`: Forward the request to the appropriate target proxy
      (mechanism TBD - maybe a central Lion message bus plugin, or direct IPC
      lookup managed by `lion_runtime`).
  - **Response:** Serialize the result or error and send back over IPC.
- **Python SDK (`lion-agent-sdk`):**
  - Implement the IPC client logic. Read `LION_PROXY_IPC_INFO` env var to know
    where/how to connect.
  - Implement the public API functions. They should block (or be `async`) while
    waiting for the response from the proxy via IPC. Translate proxy errors into
    Python exceptions.
- **Security Audit Log (`lion_observability` + Tauri Backend):**
  - The `lion_observability` system needs a sink configured to store structured
    audit logs (e.g., to a file, a database, or keep in memory with rotation).
    The proxy plugin sends structured logs to this system.
  - **Tauri Command:** `get_audit_log(filter: AuditFilter)` needs to query this
    sink. Define `AuditFilter` (time range, agent ID, outcome) and `AuditEntry`
    (matching proxy log structure) structs.
  - **Backend Event:** `new_audit_entry(entry: AuditEntry)` requires the log
    sink to push events (e.g., via a channel) to the Tauri runtime listener.
- **Metrics (`lion_observability` + Tauri Backend):**
  - `lion_observability::metrics` needs to be configured to collect metrics from
    the runtime and potentially instrument the proxy plugin.
  - **Tauri Command:** `get_metrics(query: MetricsQuery)` needs to query the
    `MetricsRegistry`, aggregate data as needed (e.g., sum memory across all
    agent proxies), and format it for the frontend charting library. Define
    `MetricsQuery` (time range, metric names, agent filter) and `MetricsData`
    (chart-ready format).
  - **Backend Event/Polling:** Decide if the backend pushes metrics updates
    (`metrics_update`) periodically or if the frontend polls `get_metrics`.
    Polling might be simpler initially.

---

**4. Tauri Backend Implementation Details (Rust - Phase 4):**

- **`plugin/manager.rs`:** Modify `load_plugin` or add `load_external_agent`.
  Add logic using `tokio::process::Command` and monitoring the child task. Add
  association between `PluginId` and `Child` handle. Modify `unload_plugin`.
- **`proxy_plugin/` (New Crate):** Implement the trusted proxy plugin, including
  IPC server, request handling, capability checks, auditing, and action
  execution. Define its own minimal dependencies.
- **`commands.rs`:** Add `get_audit_log`, `get_metrics`. Update agent commands.
- **`events.rs`:** Add `struct AuditEntry`, `struct MetricsData`.
- **`runtime_listener.rs`:** Add listener for audit logs from observability
  sink. Optionally add periodic metrics polling/pushing.
- **`main.rs`:** Ensure the observability sink for audit logs is configured
  correctly. Ensure proxy plugins are started correctly when external agents are
  loaded.

---

**5. Frontend Implementation Details (React/TS - Phase 4):**

- **API Layer (`src/lib/api.ts`):** Add `getAuditLog`, `getMetrics`. Update
  agent types/commands.
- **Components:**
  - `AgentEditor.tsx`: Add conditional rendering for "External Agent" type,
    showing fields for interpreter, script path, etc.
  - `SecurityAuditLogView.tsx`: New component. Fetches data with filters. Uses a
    performant table component (e.g., `react-table` with virtualization).
    Listens for `new_audit_entry`.
  - `SecurityView.tsx`/`CapabilityViewer.tsx`: Enhance to include the "MCP
    Communication" section/filter.
  - `MetricsView.tsx`: New component. Uses charting library (`recharts`,
    `nivo`). Fetches data via `getMetrics`. Implements selectors for
    metrics/time range/agent.
- **State Management:** Add state for audit log entries (with filtering state),
  metrics data, potentially the status of external processes.

---

**6. Phase 4 Acceptance Criteria:**

- User can define an external Python agent in the UI and save the configuration.
- Loading the external agent successfully starts both the Lion proxy plugin and
  the external Python process.
- The Python agent, using the SDK, can successfully request `read_file`. The
  request is correctly allowed/denied by the proxy based on Lion capabilities
  granted via the UI.
- The Security Audit Log accurately displays entries for the `read_file`
  capability checks, including the outcome.
- User can configure allowed MCP communication paths using capabilities in the
  Security View. A Python agent attempting `send_mcp_message` succeeds/fails
  based on these grants. Audit log reflects `mcp:send` checks.
- The Metrics view displays basic runtime metrics fetched from the backend.
- Unloading the external agent correctly terminates both the Python process and
  the proxy plugin.

Phase 4 brings LionForge closer to its vision by bridging to common agent
ecosystems and making the security and monitoring aspects highly visible and
interactive, directly showcasing the value proposition over less secure
alternatives.
