Below is a **proposed architecture** and **refactoring plan** to help you reorganize this codebase into a **highly modular, robust, and extensible** microkernel for multi-agent AI workflows. The overarching goal is to impose a **clear separation of concerns** (Orchestrator, Plugin Manager, Agents, Storage, etc.), **streamline the directory structure**, and **eliminate duplications** or confusion in event, state, and data handling. Each step includes **rationale**, **specific recommendations**, and **practical fixes** for your existing modules.

---

# 1. Core Design Goals

1. **Single, Unified Orchestrator** that processes:
   - Agent events (`AgentEvent`)
   - Task events (`TaskEvent`)
   - Plugin events (`PluginEvent`)
   - Possibly “System events” or “Network events” if needed

2. **Plugin Manager** that handles:
   - Loading & registering plugin manifests
   - Resource usage monitoring (WASM/subprocess if appropriate)
   - Providing “invoke”/“list”/“remove” flows

3. **Multi-Agent Modules** that define:
   - Agent protocol (trait-based)
   - Agent data structures and concurrency patterns
   - Possibly separate module for advanced “progression” or conversation states

4. **Shared Utility Modules** for:
   - Storage layer (FileStorage or other)
   - Collections (Pile, Progression) if you want custom concurrency
   - Event Log (append-only record, event stats/summary)

5. **UI or “Presentation”** (lion_ui):
   - Minimal Axum-based server exposing endpoints
   - SSE/WebSocket or LiveView for real-time updates
   - Docker integration for easy distribution
   - Tauri or other local packaging if desired

6. **Integration of Tools** (like the CLI):
   - lion_cli: Contains user commands for orchestrator interactions (demo tasks, spawn agents, plugin ops)

---

# 2. High-Level Directory & Module Structure

Here’s a recommended structure, tailored to your code. It consolidates related functionalities and clarifies boundaries:

```
lion/
├── lion_core/                         # Primary library crate
│   ├── src/
│   │   ├── orchestrator/
│   │   │   ├── mod.rs                # orchestrator::Orchestrator, orchestrator::Config
│   │   │   ├── events/
│   │   │   │   ├── mod.rs            # SystemEvent + enumerations
│   │   │   │   ├── agent.rs          # AgentEvent
│   │   │   │   ├── plugin.rs         # PluginEvent
│   │   │   │   ├── task.rs           # TaskEvent
│   │   │   ├── metadata/
│   │   │   ├── processor.rs          # Orchestrator's main event loop
│   │   │   ├── handlers.rs           # EventHandler (internal orchestrator operations)
│   │   │   ├── types.rs              # Orchestrator-specific types (EventSender, error definitions, etc.)
│   │   │   └── ...
│   │   ├── agent/                    # agent protocol, agent events, mock agent, ...
│   │   ├── plugin_manager/           # plugin loader, plugin registry, manifest, manager
│   │   ├── event_log/                # event log, stats, summary
│   │   ├── collections/              # pile, progression, ...
│   │   ├── storage/                  # file storage, other storages
│   │   ├── types/                    # shared data structures (element, plugin, agent, traits, etc.)
│   │   ├── store.rs                  # optional: simple wrapper around storage
│   │   ├── lib.rs                    # exports major modules
│   │   └── ...
│   └── Cargo.toml
├── lion_ui/                          # Axum-based web UI
│   ├── src/
│   │   ├── main.rs                   # runs the Axum server
│   │   ├── events.rs                 # SSE logic
│   │   ├── agents.rs                 # agent endpoints
│   │   ├── plugins.rs                # plugin endpoints
│   │   ├── state.rs                  # AppState definition
│   │   └── lib.rs                    # re-exports, etc.
│   ├── frontend/                     # optional compiled or static front-end assets
│   ├── build.rs                      # optional build script
│   └── Cargo.toml
├── lion_cli/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands.rs
│   │   ├── handlers.rs
│   │   └── logging.rs
│   ├── scripts/                      # CI, test scripts
│   └── Cargo.toml
├── Cargo.toml                        # workspace-level
└── README.md
```

**Key** changes:

- **`orchestrator`**: Keep all orchestrator logic in one place. That includes the “processor” loop, “handler” logic, and `SystemEvent` definitions (with submodules for `AgentEvent`, `TaskEvent`, `PluginEvent`).  
- **`agent`**: If needed, keep your agent protocol & mock agent here.  
- **`plugin_manager`**: Keep `loader`, `manager`, `manifest`, and `registry` in one cohesive folder.  
- **`event_log`**: All record/stats/summary logic in a single folder.  
- **`collections`**: Keep `pile`, `progression`, etc. in a single place.  
- **`storage`**: The file-based and other storages (like `InMemoryStore` if you want) are together.

---

# 3. Specific Suggestions & Fixes

### 3.1. Consolidate SystemEvent Variants

- **Right Now**: You have some duplication with `AgentEvent`, `PluginEvent`, `TaskEvent` in multiple places.  
- **Proposed**: Keep them in `orchestrator/events/agent.rs`, `orchestrator/events/plugin.rs`, `orchestrator/events/task.rs`, then unify them in `SystemEvent` (like you already do).  
- **Simplify**: Instead of many re-exported “constructor functions,” prefer the direct pattern:
  ```rust
  let event = SystemEvent::Agent(
      AgentEvent::Spawned {
          agent_id,
          prompt,
          metadata: ...
      }
  );
  ```
  Or keep your “constructor” approach but put them in one place (like you do under `mod events`).

### 3.2. Merge or Remove Duplicated Orchestrator Logic

- **Possible Overlap**: 
  - `orchestrator::handlers::EventHandler` and `orchestrator::processor::Orchestrator` do similar event logic.  
  - The “EventHandler” can be merged into the orchestrator’s loop or kept as a small trait. 
- **Recommendation**: If you want to keep them separate, rename `EventHandler` to `OrchestratorCore` or `OrchestratorEngine` to clarify it’s the orchestrator’s internal operation, then the `Orchestrator` struct just calls it. Otherwise, combine them so that the main loop & event logic live in the same file.

### 3.3. Cleanup “metadata/” Modules

- You have `metadata/mod.rs` with `EventMetadata`, plus “helpers.rs.” 
- This is fine, but consider if “helpers.rs” is too small. Possibly just inline it in `mod.rs`.

### 3.4. Rethink “progression” vs “pile”

- They are interesting data structures:
  - `Pile` is basically an ID→T map with insertion order. 
  - `Progression` is for steps/branching.  
- **If** these are used widely, keep them in `collections/`. That is correct. Ensure you do not keep half in `storage/` or half in `store.rs` to avoid confusion.

### 3.5. Storage vs. Store

- You have `storage` for `FileStorage`, plus a `store.rs` that references `InMemoryStore` or `FileStorage`.  
- If your `store.rs` is basically a “facade” around `FileStorage`, that’s fine. But keep them consistent: 
  - `lion_core/src/storage/` = base storage logic  
  - `lion_core/src/store.rs` = simple “InMemoryStore” or unify them under `storage`.

### 3.6. Plugin Manager

- You have a robust plugin manager: loader, manager, registry, manifest. That’s good. 
- Possibly rename `loader.rs` → `loader_wasm.rs` or something if you want to handle WASM logic or plugin processes. 
- The important part is that `manager.rs` is your single interface for external calls: `manager.invoke_plugin(...)`, etc. The rest is internal.  

### 3.7. Event Log & Stats

- `event_log` with `record.rs`, `stats.rs`, `summary.rs`. Good separation. 
- The code looks consistent. Just ensure you do not replicate event stats logic in multiple places.

### 3.8. UI Fixes

- For `lion_ui`, you have a decent structure:
  - `main.rs` to start the Axum server  
  - `state.rs` for `AppState`  
  - `agents.rs`, `plugins.rs`, `events.rs` for SSE, etc.  
- **If** you’re mixing “orchestrator” references here, keep them minimal, just “we have a channel to the orchestrator.”  
- Consider naming routes consistently (`/api/tasks`, `/api/plugins`, `/api/agents`, etc.).

### 3.9. Avoid Over-Duplication of Test Scenarios

- You have a lot of test files. That’s great for coverage but watch out for duplication:
  - The “integration tests” for plugins might be repeated in `tests/plugin_integration_tests.rs` and also in `plugin_manager/tests.rs`. If they differ, that’s fine. If it’s the same, unify them.  
- Similar with orchestrator tests in `tests/orchestrator_integration_tests.rs` vs in the `orchestrator` folder.

### 3.10. Prune Old or Redundant Constructors

- For instance, `SystemEvent` has many “create a new foo” functions, and `AgentEvent` also has them. 
- You can unify them: Let the user do `SystemEvent::Agent(AgentEvent::spawn(...))`, etc. Or keep the simpler “SystemEvent::new_agent_spawn(...)” approach, but not both.  
- This removes confusion in the code.

---

# 4. Step-by-Step Refactoring Process

**Step 1: Directory Cleanup**

1. **Move** all orchestrator event definitions to `lion_core/src/orchestrator/events/`.  
2. **Move** `Progression`, `Pile`, etc. into `lion_core/src/collections/`.  
3. **Move** all plugin logic (manifest, manager, loader, registry) into `lion_core/src/plugin_manager/`.

**Step 2: Consolidate Orchestrator**

1. In `lion_core/src/orchestrator/`, keep:
   - `mod.rs` (export `Orchestrator`, `SystemEvent`, `OrchestratorConfig`, `OrchestratorError`, etc.)
   - `processor.rs` (the main event loop code)
   - `handlers.rs` or unify that logic inside `processor.rs` if you prefer fewer files
2. Remove duplicate constructor methods from `SystemEvent`. Possibly place them in `AgentEvent::spawn(...)`, `TaskEvent::submit(...)`, etc.

**Step 3: Clean Up “metadata/”**

- Inline the “helpers.rs” into `metadata/mod.rs` if it’s short.  
- Simplify the creation of `EventMetadata` so that `EventMetadata::new(...)` is all you need (the “create_metadata” function is effectively the same).

**Step 4: Clarify the UI State**

1. The `AppState` in `lion_ui` has:
   - `orchestrator_tx`: an `mpsc::Sender<SystemEvent>`  
   - `plugins`: a local store of `PluginInfo`  
   - `logs_tx`: a broadcast for SSE logs  
2. That’s fine. Make sure all the agent info is derived from the orchestrator’s event completions, or store it in the orchestrator. 
3. Possibly unify how you track “active agents” in the orchestrator vs. in the UI `AppState.agents`.

**Step 5: Re-check “store.rs” vs. “storage/”**

- If `store.rs` is basically an older approach, unify it or remove duplicates. 
- Possibly rename `store.rs` → `InMemoryStore` or something if it’s just an in-memory approach.

**Step 6: Test & Migrate**

1. Move modules around in your directory structure.  
2. Update `mod` references (`mod agent`, `mod plugin`, etc.).  
3. Fix `use` paths to reflect the new layout.  
4. Re-run `cargo test`, fix any broken references.  
5. Incrementally push each structural change (to keep diffs smaller and merges simpler).

---

# 5. Conclusion and Next Steps

- **Your code** is already quite comprehensive: agents, plugins, tasks, SSE, etc. The main challenge is that it’s spread out and has some redundancy. 
- By **grouping** related submodules (`orchestrator`, `agent`, `plugin_manager`, `event_log`, `collections`, `storage`), you’ll get a more coherent architecture. 
- **Naming** your modules carefully (like `orchestrator::processor.rs`, `orchestrator::events::task.rs`) clarifies responsibilities. 
- If you fix the “two ways of creating the same event,” you’ll avoid developer confusion. 
- The `lion_ui` crate can remain an Axum front end; just keep it minimal and reference the orchestrator through channels. 
- The `lion_cli` crate can remain a separate CLI that calls orchestrator commands (spawn agent, load plugin, etc.) for local usage or debugging.

With these refactoring steps, you’ll achieve a **modular** structure that is **easier to maintain and extend**. Good luck with the cleanup! Once reorganized, new features (like advanced branching logic, distributed orchestrator, or advanced UI dashboards) will be much simpler to implement.