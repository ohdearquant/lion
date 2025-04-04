# **Stage 2, Phase 4 – CLI Feature Completeness: Core System Control & Observability**

**(Revised & Expanded: CLI Focus - Builds on Stage 1, Phase 5 Completion)**

## 1. Essence & Value Proposition Refocus

**Goal:** Transform `lion_cli` into a **comprehensive command-line control
plane** for the Lion system's foundational features. This phase extends beyond
the basic agent/plugin demos of Stage 1, providing deep inspection capabilities
for security constructs (Capabilities, Policies), detailed status reporting for
core runtime entities (Plugins, Workflows), enhanced observability through
structured log searching, and initial mechanisms for resource management.

- **Value Proposition Addressed:**
  - **Authoritative Control & Manageability:** Establish the CLI as the primary
    interface for inspecting the state of loaded plugins, active workflow
    instances, and the system's security configuration. Introduce basic agent
    and resource management commands.
  - **Security Transparency & Verification:** Enable users to directly query and
    view granted capabilities for specific plugins and the exact definitions of
    applied policy rules via dedicated CLI commands.
  - **Enhanced Debugging & Monitoring:** Implement powerful, structured log
    searching (`lion logs search`), allowing users to filter runtime events by
    various criteria (level, source, IDs, text, time) for effective diagnostics.
  - **Resource Awareness:** Provide commands to view current resource usage
    metrics for plugins and set fundamental resource limits, offering visibility
    and control over sandbox constraints.
- **Essence Reinforced:** Phase 4 solidifies the CLI as a powerful tool for
  _understanding_ and _verifying_ the Lion system's state and behavior. It
  demonstrates that the core concepts (capabilities, policies, isolation,
  observability) are not just theoretical but practically accessible and
  manageable before adding advanced orchestration or UI layers.

## 2. Redesigned Ideal CLI Interaction Focus for Phase 4

The target CLI experience emphasizes informative output, consistent structure,
and powerful querying for core system elements.

- **Command Structure:** Strict adherence to `lion <NOUN> <VERB> [ARGS/FLAGS]`.
  Key nouns for this phase: `plugin`, `capability`, `policy`, `workflow`,
  `agent`, `resource`, `logs`. Key verbs: `list`, `info`, `search`, `spawn`,
  `limits`, `usage`.
- **Output Formatting:**
  - **Human-Readable (Default):** Use well-aligned tables (via `comfy-table` or
    similar) for `list` commands. Use clear key-value pairs or sectioned text
    for `info`/`status`/`usage` commands. Employ color-coding (via `colored`)
    for statuses (e.g., `PluginState::Running` green, `Failed` red).
  - **Machine-Readable (`--output json`):** Provide a JSON output option for
    commands returning structured data (`list`, `info`, `search`, `usage`). The
    JSON structure should closely mirror the underlying Rust structs
    (`PluginMetadata`, `CapabilityInfo`, `PolicyRule`, `LogEntry`, etc.).
- **Filtering & Searching:** Log searching (`lion logs search`) must perform
  efficient filtering based on multiple criteria. The backend (`lion_runtime`
  interfacing with `lion_observability`) is responsible for handling the
  filtering logic.
- **Error Handling:** CLI commands must exit with non-zero status on error.
  Messages to `stderr` should clearly state the command that failed, the
  specific error encountered (leveraging the core error hierarchy, e.g.,
  `PluginNotFound`, `CapabilityDenied`, `PolicyRuleNotFound`), and potential
  hints if applicable.
- **Argument Consistency:** Use standardized flags: `--plugin-id <UUID>`,
  `--rule-id <STRING>`, `--workflow-id <UUID>`, `--instance-id <STRING>`,
  `--correlation-id <UUID>`, `--output json`, `--limit N`, `--offset N`, etc.

## 3. Objectives & Scope (Expanded Detail)

_(Assumes Stage 1, Phases 1-5 completed, providing basic `agentic_core` with
orchestrator, event log, mock agent/plugin handlers, and a rudimentary CLI)_

1. **Implement Structured Log Querying (`lion logs search`):**
   - **Backend Requirement (`lion_runtime` + `lion_observability`):** Needs an
     accessible log store (in-memory `VecDeque<LogEntry>` or persistent) holding
     structured `LogEntry` { `timestamp`, `level`, `message`, `source_type`:
     String (e.g., "System", "Plugin", "Agent", "Workflow"), `source_id`:
     Option<String> (UUID/Name), `correlation_id`: Option<String>, `metadata`:
     `serde_json::Value` }. Requires
     `search_logs(filter: LogFilterParams) -> Result<Vec<LogEntry>>` function
     implementing filtering by all criteria.
   - **CLI Implementation (`lion_cli`):**
     - Add `search` subcommand to `lion logs`.
     - Define `clap` arguments: `--level <trace|...|error>`,
       `--source-type <type>`, `--source-id <id>`, `--correlation-id <id>`,
       `--text <pattern>`, `--since <timestamp>`, `--until <timestamp>`,
       `--limit <N>`, `--offset <N>`, `--output json`.
     - Implement handler in `commands/logs.rs` (or similar): Parse args into
       `LogFilterParams`, call interface `search_logs`.
     - Format output: Default table (`Timestamp | Level | Source | Msg`), JSON
       array of `LogEntry`.
2. **Refine Plugin Inspection (`lion plugin list/info`):**
   - **Backend Requirement (`lion_runtime` + `PluginManager` +
     `CapabilityManager`):**
     - `PluginManager::list_plugins()` needs to return
       `Vec<PluginSummary { id, name, version, state: PluginState }>`.
     - `PluginManager::get_plugin_details(id)` needs to return
       `PluginDetails { metadata: PluginMetadata, granted_capabilities: Vec<CapabilityInfo> }`.
       This implies `PluginManager` queries `CapabilityManager` for the
       capabilities associated with the plugin ID (as the subject).
       `CapabilityInfo` should contain
       `{ id: CapabilityId, type: String, description: String }`.
   - **CLI Implementation (`lion_cli`):**
     - Modify `list` handler in `commands/plugin.rs`: Add `STATE` column
       (colored). Add `--output json`.
     - Implement `info <plugin_id>` subcommand. Add `--output json`.
     - Implement `handle_plugin_info`: Call interface `get_plugin_details`.
       Display `PluginMetadata` fields. Display `granted_capabilities` in a
       separate section/list. Format JSON output.
3. **Implement Capability Viewing (`lion capability list`):**
   - **Backend Requirement (`lion_runtime` + `CapabilityManager`):** Expose
     `CapabilityManager::list_capabilities_for_subject(subject_id: &str) -> Result<Vec<CapabilityInfo>>`.
   - **CLI Implementation (`lion_cli`):**
     - Add top-level `capability` command group.
     - Add `list` subcommand requiring `--plugin-id <id>`. Add `--output json`.
     - Implement handler in `commands/capability.rs`: Call interface
       `list_capabilities`. Format output table
       (`Capability ID | Type | Details`). JSON is array of `CapabilityInfo`.
4. **Refine Policy Inspection (`lion policy list/info`):**
   - **Backend Requirement (`lion_runtime` + `PolicyStore`):** Expose
     `PolicyStore::get_rule(rule_id: &str) -> Result<PolicyRule>`. Ensure
     `list_rules()` is exposed.
   - **CLI Implementation (`lion_cli`):**
     - Refine `list` handler in `commands/policy.rs`: Use `comfy-table`. Add
       `--output json`.
     - Implement `info <rule_id>` subcommand. Add `--output json`.
     - Implement `handle_policy_info`: Call interface `get_policy_rule`. Display
       all fields of the `PolicyRule` struct in a readable key-value format.
       JSON output is the full struct.
5. **Enhance Workflow Status (`lion workflow info`):**
   - **Backend Requirement (`lion_runtime` + `StateMachineManager`):** Expose
     `get_workflow_instance_details(instance_id: &str) -> Result<WorkflowInstanceDetails>`.
     The `WorkflowInstanceDetails` struct needs
     `{ overall_status: ExecutionStatus, node_statuses: HashMap<NodeId, NodeRuntimeInfo { name: String, status: NodeStatus, start_time: Option<DateTime>, end_time: Option<DateTime> }> }`.
     (Requires StateMachine to potentially access the definition for names).
   - **CLI Implementation (`lion_cli`):**
     - Refine `status <instance_id>` handler output.
     - Implement `info <instance_id>` subcommand. Add `--output json`.
     - Implement `handle_workflow_info`: Call interface, display overall status,
       then list/table of node statuses with names and times. JSON output
       mirrors `WorkflowInstanceDetails`.
6. **Implement Agent Management (`lion agent spawn/list`):**
   - **Backend Requirement (`lion_runtime`/`agentic_core`):** Expose agent
     management interface:
     `spawn_agent(prompt: String, name: Option<String>) -> Result<AgentId>`,
     `list_agents() -> Result<Vec<AgentInfo { id: AgentId, name: String, status: String }>>`.
   - **CLI Implementation (`lion_cli`):**
     - Add top-level `agent` command group.
     - Implement `spawn --prompt <TEXT> [--name <NAME>]`.
     - Implement `list [--output json]`.
     - Implement handlers in `commands/agent.rs`. `spawn` prints the new ID.
       `list` uses a table (`Agent ID | Name | Status`) or JSON.
7. **Implement Resource Management (`lion resource limits/usage`):**
   - **Backend Requirement (`lion_runtime` + `IsolationManager`):** Expose
     `set_resource_limit(plugin_id: PluginId, limit_type: ResourceLimitType, value: u64) -> Result<()>`
     and `get_resource_usage(plugin_id: PluginId) -> Result<ResourceUsage>`.
   - **CLI Implementation (`lion_cli`):**
     - Add top-level `resource` command group.
     - Implement
       `limits --plugin-id <id> [--memory-mb <N>] [--cpu-percent <N>] ...`
       command. Handler parses flags and calls backend `set_resource_limit` for
       each provided limit.
     - Implement `usage --plugin-id <id> [--output json]`. Handler calls backend
       `get_resource_usage`.
     - Implement handlers in `commands/resource.rs`. Format `usage` output
       clearly. JSON output mirrors the `ResourceUsage` struct from `lion_core`.
8. **Interface Layer Implementation (`lion_cli/src/interfaces/`):** Create or
   update functions in the relevant modules (`observability.rs`, `runtime.rs`,
   `capability.rs`, `policy.rs`, `workflow.rs`, `isolation.rs`, `agent.rs`) to
   match the required backend signatures and perform the calls. Define necessary
   intermediate structs (like `PluginSummary`, `CapabilityInfo`,
   `LogFilterParams`, etc.) if they differ from core types.
9. **Testing (`lion_cli/tests/`):** Create new test files (e.g.,
   `test_logs_search.rs`, `test_plugin_info.rs`, etc.) for each new
   command/feature. Use `assert_cmd` to:
   - Verify command execution with various flags.
   - Check for expected stdout (including table formatting).
   - Verify `--output json` produces valid JSON matching expected structures.
   - Check stderr for correct error messages on failure cases (e.g., ID not
     found).
   - Test exit codes.

## 5. Potential Issues & Considerations

- **Backend Interface Availability:** The feasibility of this phase hinges
  entirely on the `lion_runtime` layer exposing the detailed functions needed
  (e.g., getting capabilities per plugin, detailed node statuses, setting
  resource limits). If these backend endpoints aren't ready, they must be
  implemented first.
- **Structured Data Consistency:** Ensuring the data structures
  (`PluginMetadata`, `CapabilityInfo`, `PolicyRule`, `LogEntry`,
  `ResourceUsage`, etc.) are defined consistently in `lion_core` and used
  correctly by the backend, interfaces, and CLI JSON output is crucial.
- **Performance of Backend Queries:** CLI commands like `lion logs search` or
  `lion capability list` might trigger potentially expensive operations on the
  backend. The backend implementation needs to be reasonably performant or the
  CLI might appear sluggish.
- **State Management & Concurrency:** The backend functions accessed by the CLI
  must correctly handle concurrent access to the underlying state (plugin
  registries, capability stores, workflow state machines, log buffers) using
  `RwLock`/`Mutex`.
- **Async Integration:** All CLI handlers interacting with the potentially
  `async` backend must be `async` and use `await` correctly within the `tokio`
  runtime established in `lion_cli/src/main.rs`.

## 6. Expected Outcome

- **Comprehensive Core CLI:** `lion_cli` graduates from a basic tool to a robust
  interface for inspecting and managing the fundamental state of the Lion
  system: detailed plugin info (including capabilities), policy rules, workflow
  instance/node status, basic agent lifecycle, and resource usage/limits.
- **Effective Observability Tool:** `lion logs search` provides powerful,
  structured querying capabilities, making the CLI a primary tool for
  diagnostics and monitoring.
- **Validated Backend API:** The implementation of these CLI commands serves as
  a practical validation that the `lion_runtime` exposes a usable and
  sufficiently detailed API for core system management.
- **Strong Foundation:** The CLI now offers a feature-rich, verifiable interface
  to the core system, simplifying subsequent development of advanced CLI
  features (Phase 5) or any graphical UI.
