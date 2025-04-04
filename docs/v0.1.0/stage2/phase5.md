# **Stage 2, Phase 5 – CLI Feature Completeness: Advanced Patterns & Polish**

**(Revised & Expanded: CLI Focus - Builds on Phase 4 Completion)**

## 1. Essence & Value Proposition Refocus

**Goal:** Finalize `lion_cli` by incorporating commands for Lion's **advanced
orchestration patterns (Saga, Event Broker)** and **deep observability features
(Metrics Querying, Trace Inspection)**. This phase also prioritizes
**polishing** the entire CLI for a professional, consistent, and highly
intuitive user experience, making it the definitive tool for interacting with
the Lion system.

- **Value Proposition Addressed:**
  - **Mastery over Complex Orchestration:** Grant users direct command-line
    control over defining, executing, monitoring, and managing Sagas (handling
    distributed consistency) and interacting with the event bus (for
    event-driven architectures).
  - **Deep Operational Insight:** Enable precise querying of performance metrics
    (with labels) and detailed inspection of distributed traces (span
    hierarchies, attributes) via the CLI, providing essential tools for advanced
    performance analysis and debugging complex cross-component interactions.
  - **Professional Developer/Operator Experience:** Deliver a highly polished
    CLI featuring consistent formatting, clear status indicators, helpful error
    messages, robust JSON output for automation, and usability enhancements like
    shell completion.
- **Essence Reinforced:** Phase 5 establishes `lion_cli` as a mature,
  feature-complete interface reflecting the full power and sophistication of the
  Lion system. It demonstrates Lion's capability to handle complex, resilient,
  distributed workflows and provide deep runtime visibility, all accessible
  through a professional command-line tool.

## 2. Redesigned Ideal CLI Interaction Focus for Phase 5

The CLI should now feel complete, powerful, and extremely user-friendly for
advanced tasks.

- **Saga Management (`lion workflow saga ...`):**
  - `define --file <path.yaml|json>`: Loads definition, validates structure (DAG
    check), reports success/failure.
  - `start <definition_id> [--input <json_string>] [--correlation-id <uuid>]`:
    Starts instance, prints instance ID.
  - `list`: Table output (`Definition ID | Name | Version`). Supports
    `--output json`.
  - `instances [--definition-id <id>] [--status <Created|Running|...|Aborted>] [--limit N]`:
    Filterable list of instances. Table output
    (`Instance ID | Def ID | Status | Start Time`). Supports `--output json`.
  - `status <instance_id>`: Detailed view showing overall status (colored),
    start/end times, error (if any), followed by a table of steps
    (`Step ID | Name | Status | Start Time | End Time | Result/Error Preview`).
    Supports `--output json`.
  - `abort <instance_id> [--reason <string>]`: Triggers abort and compensation.
    Reports success/failure of the abort _request_.
- **Event Broker Interaction (`lion event ...`):**
  - `publish --topic <topic> --payload <json_string> [--source <id>] [--correlation-id <id>] [--priority <low|...|critical>]`:
    Publishes event, confirms success. Payload must be valid JSON.
  - `subscribe --topic <topic> [--count <N>] [--timeout <secs>]`: Blocks and
    prints received events (formatted JSON or plain string) until N events
    received or timeout occurs. Handles potential connection errors.
  - `topics list`: Lists known event topics (if discoverable by backend). Table
    output (`Topic Name | Subscribers (Count)`).
  - `broker status`: Displays broker metrics (e.g., total topics, total
    subscribers, in-flight messages, queue depths, if exposed by backend).
- **Metrics Querying (`lion metrics ...`):**
  - `list`: Table output (`Name | Type | Description`). Supports
    `--output json`.
  - `get <metric_name> [--labels <key=value,...>]`: Gets counter/gauge value(s).
    Handles multi-value results for label variations. Clear output format.
    Supports `--output json`.
  - `histogram <metric_name> [--labels <key=value,...>] [--percentiles <p50,p90,p99.9>]`:
    Gets histogram summary. Displays count, sum, average, and requested
    percentiles. Supports `--output json`.
- **Trace Querying (`lion trace ...`):**
  - `get <trace_id>`: Displays trace as a hierarchical tree (using indentation
    or box-drawing characters), showing span names, durations, start times,
    status (Ok/Error), and key attributes. Supports `--output json` (likely as a
    nested structure or flat list of spans with parent IDs).
  - `find [--service <name>] [--span-name <name>] [--min-duration <ms>] [--max-duration <ms>] [--status <Ok|Error>] [--tag <key=value,...>] [--limit N]`:
    Searches traces. Table output
    (`Trace ID | Root Span Name | Start Time | Duration | Span Count | Status`).
    Supports `--output json`. (Requires capable trace backend).
- **CLI Polish:**
  - **Tables:** Consistent use of `comfy-table` for all list outputs, ensuring
    alignment and readability.
  - **Colors:** Standardized colors for all status fields (Running/OK=Green,
    Failed/Error=Red, Pending/Warn=Yellow,
    Completed/Compensated/Aborted=Blue/Cyan).
  - **Errors:** Detailed, contextual error messages printed to `stderr`. Use
    specific exit codes (e.g., `1` general, `2` not found, `3` permission, `4`
    invalid input).
  - **Help:** Comprehensive help text for all commands, subcommands, and flags
    via `clap`.
  - **Shell Completion:** Generate and provide instructions for installing
    completion scripts for Bash, Zsh, Fish via `lion completion <shell>`.

## 3. Objectives & Scope (Expanded Detail)

1. **Implement Saga CLI Commands (`lion workflow saga ...`):**
   - **Backend Req (`lion_runtime` + `SagaOrchestrator`):** Need functions for
     all operations: `define(SagaDefinition)`,
     `start(def_id, input, correlation_id)`, `list_definitions()`,
     `list_instances(filter)`, `get_instance_details(id)` (incl. detailed step
     info: status, times, result/error snippets), `abort(id, reason)`.
   - **CLI Impl:** Implement all subcommands (`define`, `start`, `list`,
     `instances`, `status`, `abort`) in `commands/workflow.rs` or `saga.rs`.
     Parse file for `define`, JSON for `start` input. Use `comfy-table` for
     lists. Format `status` output clearly. Handle `--output json`.
2. **Implement Event Broker CLI (`lion event ...`):**
   - **Backend Req (`lion_runtime` + `EventBroker`):** Need `publish(Event)`,
     `get_status() -> BrokerStatusInfo`, `list_topics()`. `subscribe` needs a
     backend mechanism to stream events back to the potentially long-running CLI
     process.
   - **CLI Impl:** Add `event` command group. Implement `publish` (parses JSON
     payload). Implement `subscribe` (handles blocking loop, printing events,
     respects `--count`/`--timeout`). Implement `topics list`. Implement
     `status`.
3. **Implement Metrics CLI (`lion metrics ...`):**
   - **Backend Req (`lion_runtime` + `MetricsRegistry` + Provider Query):** Need
     `list_metrics() -> Vec<MetricInfo>`,
     `query_metric(name, labels) -> Result<ValueOrMap>`,
     `query_histogram(name, labels, percentiles) -> Result<HistogramSummary>`.
     This likely requires runtime to query Prometheus/OTel backend.
   - **CLI Impl:** Add `metrics` command group. Implement `list`, `get`,
     `histogram`. Implement label parsing (`--labels k=v,k=v`). Implement
     percentile parsing (`--percentiles p1,p2`). Format output. Handle
     `--output json`.
4. **Implement Tracing CLI (`lion trace ...`):**
   - **Backend Req (`lion_runtime` + `Tracer` + Trace Store Query):** Need
     `get_trace_details(trace_id) -> Result<TraceData { root_span_id, spans: Vec<SpanDetail> }>`,
     `find_traces(criteria) -> Result<Vec<TraceSummary>>`. Requires runtime to
     query Jaeger/Tempo/OTel backend.
   - **CLI Impl:** Add `trace` command group. Implement `get <trace_id>`
     (formats span tree). Implement `find` with filter flags. Handle
     `--output json`.
5. **CLI Polish and UX:**
   - **Formatting:** Integrate `comfy-table` across _all_ `list` commands
     (plugin, capability, policy, workflow, saga, event, metrics). Standardize
     column names and widths where possible.
   - **Coloring:** Apply the defined color scheme consistently to all status
     fields in command outputs.
   - **Error Handling:** Review all `Result` handling in command handlers. Use
     `anyhow::Context` or `map_err` to add context. Implement specific exit
     codes in `main.rs` based on error types.
   - **Shell Completion:** Add `clap_complete` dependency. Implement
     `completion` command handler. Add build script step or documentation for
     generation/installation.
6. **Interface Layer (`lion_cli/src/interfaces/`):** Implement all new interface
   functions (for Saga, Event, Metrics, Trace) ensuring they correctly interact
   with the assumed `lion_runtime` backend functions and handle the required
   data structures.
7. **Testing (`lion_cli/tests/`):** Add comprehensive integration tests
   (`test_saga_commands.rs`, `test_event_commands.rs`,
   `test_metrics_commands.rs`, `test_trace_commands.rs`). Test all subcommands,
   flags, success/error paths, table output structure, and JSON output
   structure. Test shell completion generation command.

## 4. Step-by-Step Implementation Plan (Expanded Detail)

_(Focuses on `lion_cli` implementation, assuming backend functions are
available)_

### Step 1: **Implement Saga CLI (`lion workflow saga ...`)**

1. **`lion_runtime` Backend:** Implement functions wrapping `SagaOrchestrator`.
2. **`lion_cli/interfaces/workflow.rs` (or `saga.rs`):** Define structs
   (`SagaDefinitionInfo`, `SagaInstanceSummary`, `StepExecutionInfo`,
   `SagaInstanceDetails`, `InstanceFilter`). Implement interface functions
   (`define_saga`, `start_saga`, `list_definitions`, `list_instances`,
   `get_instance_details`, `abort_instance`). Handle file parsing for `define`,
   JSON parsing for `start --input`.
3. **`lion_cli/main.rs`:** Define `SagaCommands` enum and all `clap` arguments
   precisely.
4. **`lion_cli/commands/workflow.rs` (or `saga.rs`):** Implement handlers. Use
   `comfy-table` for `list` and `instances`. Format `status` with overall
   summary + step table (colored). Handle JSON output.
5. **Testing:** Create `test_saga_commands.rs`. Test all subcommands, filters
   (`instances --status`), error cases (not found, already exists), JSON output.

### Step 2: **Implement Event Broker CLI (`lion event ...`)**

1. **`lion_runtime` Backend:** Implement `EventBroker` wrappers. Devise strategy
   for CLI `subscribe` (e.g., runtime returns a limited stream, or CLI blocks).
2. **`lion_cli/interfaces/workflow.rs` (or `event.rs`):** Implement
   `publish_event`, `get_broker_status`, `list_topics`. Implement
   `subscribe_to_topic` according to chosen strategy. Define `BrokerStatusInfo`,
   `Event`.
3. **`lion_cli/main.rs`:** Define `EventCommands` enum and `clap` args.
4. **`lion_cli/commands/event.rs`:** Implement handlers. `publish` validates
   payload JSON. `subscribe` implements the blocking/streaming logic. Use tables
   for `topics` and `status`.
5. **Testing:** Create `test_event_commands.rs`. Test `publish` with
   valid/invalid JSON. Test `topics`, `status`. Test `subscribe`'s exit
   condition (count or timeout).

### Step 3: **Implement Metrics CLI (`lion metrics ...`)**

1. **`lion_runtime` Backend:** Implement `MetricsRegistry` wrappers, including
   query logic against the metrics provider.
2. **`lion_cli/interfaces/observability.rs`:** Define `MetricInfo`,
   `MetricValue`, `HistogramSummary`. Implement `list_metrics`, `query_metric`,
   `query_histogram`.
3. **`lion_cli/main.rs`:** Define `MetricsCommands` enum and `clap` args.
   Implement `parse_labels` helper.
4. **`lion_cli/commands/metrics.rs`:** Implement handlers. Parse
   labels/percentiles correctly. Call interfaces. Format output (table for list,
   key-value for get/histogram). Handle `--output json`.
5. **Testing:** Create `test_metrics_commands.rs`. Test all subcommands, label
   filtering, percentile requests, error cases, JSON output.

### Step 4: **Implement Tracing CLI (`lion trace ...`)**

1. **`lion_runtime` Backend:** Implement `Tracer`/Trace Store wrappers. Requires
   integration with Jaeger/Tempo/OTel Collector APIs for `get` and `find`.
2. **`lion_cli/interfaces/observability.rs`:** Define `SpanDetail`, `TraceData`,
   `TraceSummary`, `TraceSearchCriteria`. Implement `get_trace_details`,
   `find_traces`.
3. **`lion_cli/main.rs`:** Define `TraceCommands` enum and `clap` args for `get`
   and `find`.
4. **`lion_cli/commands/trace.rs`:** Implement handlers. `get` needs to format
   the span tree (e.g., using indentation). `find` uses a table. Handle
   `--output json`.
5. **Testing:** Create `test_trace_commands.rs`. Test `get` output structure.
   Test `find` with various filters. Test non-existent IDs. Test JSON output.

### Step 5: **CLI Polish and UX Enhancements**

1. **Integrate Tables:** Audit _all_ list commands (`plugin list`,
   `capability list`, `policy list`, `workflow list`, `agent list`, `saga list`,
   `saga instances`, `event topics`, `metrics list`, `trace find`) and ensure
   they use `comfy-table` with consistent column definitions.
2. **Apply Colors:** Audit _all_ commands displaying status (`plugin info`,
   `workflow status`, `workflow info`, `agent list`, `saga status`, `trace get`)
   and apply the standard color scheme using `colored`.
3. **Refine Error Handling:** In `main.rs`'s error handling block (or individual
   handlers), map specific backend `Error` variants (e.g.,
   `Error::Plugin(PluginError::NotFound)`) to distinct exit codes and more
   user-friendly messages on `stderr`.
4. **Implement Shell Completion:**
   - Add `clap_complete` dev-dependency.
   - Add `Completion { shell: clap_complete::Shell }` command to `main.rs`.
   - Implement handler using `clap_complete::generate`.
   - Add documentation for setup (`eval "$(lion completion bash)"`).

### Step 6: **Final Testing and Documentation**

1. **Full Test Suite:** Run `cargo test --all` in `lion_cli`. Ensure all tests
   pass, including the new ones for advanced features and polish checks (e.g.,
   JSON output validation).
2. **Manual E2E:** Perform a comprehensive manual test of all major CLI
   workflows (load plugin -> grant caps -> check policy -> run workflow -> check
   saga -> inspect logs/metrics/traces).
3. **Documentation:** Update all CLI documentation (README, man pages if
   generated, usage examples) to cover every command and flag accurately. Ensure
   shell completion instructions are clear.
4. **Commit:** Finalize commits with `[stage2-phase5]` prefix.

## 5. Potential Issues & Considerations

- **Backend Feature Completeness:** This phase heavily assumes the backend
  (`lion_runtime` and underlying crates) fully implements Sagas, Event Broker
  interactions, and provides queryable APIs for metrics and traces. Any gaps
  there will block CLI implementation.
- **Observability Backend Integration:** The implementation complexity of
  `lion metrics` and `lion trace` commands depends greatly on the ease of
  querying the chosen observability backends (Prometheus, Jaeger, OTel
  Collector, etc.). This might require adding specific client libraries or API
  wrappers in `lion_runtime`.
- **CLI Streaming Complexity:** The `lion event subscribe` command remains the
  most complex due to its streaming nature compared to other request-response
  commands. Careful implementation is needed to handle blocking, timeouts, and
  graceful termination.
- **Polish Scope:** Achieving perfect formatting, error messages, and
  completions across many commands takes significant effort. Prioritize
  consistency and clarity.

## 6. Expected Outcome

- **Fully Featured CLI:** `lion_cli` is now the definitive command-line
  interface for the Lion system, providing control and observability over core
  features _and_ advanced patterns like Sagas and Events, along with deep
  diagnostic capabilities via metrics and trace querying.
- **Professional Developer/Operator Tool:** The CLI boasts a polished user
  experience with consistent table formatting, informative color-coded statuses,
  helpful error messages, comprehensive JSON output for scripting, and
  potentially shell completion.
- **Complete System Validation:** The CLI serves as a powerful end-to-end
  validation tool, demonstrating that all major components and features of the
  Lion system are functional and interact correctly.
- **Ready for UI Development:** With the backend interactions thoroughly defined
  and tested via the comprehensive CLI, building a graphical UI (Web or Tauri)
  becomes significantly more straightforward, focusing primarily on visual
  presentation and user interaction design based on the established CLI commands
  and backend APIs.
