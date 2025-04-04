**Phase 1: Foundation, Runtime Connection & Read-Only Views**

- **Goal:** Establish the basic Tauri application structure, integrate the Lion
  runtime lifecycle, connect the UI to the runtime for essential status
  information, and provide read-only views for core entities (Agents, Logs).
  Prove the core Tauri <-> Rust communication and display live data.
- **Core Principle:** Build the skeleton and nervous system. Functionality is
  primarily observational; editing and creation come later.

---

**1. Look and Feel (Vibe):**

- **Overall Aesthetic:** Aim for a clean, professional, developer-centric look.
  Think functional and informative over flashy. A dark theme is standard for
  developer tools and often preferred. Use a consistent color palette – perhaps
  neutral grays/charcoals with accents inspired by the "Lion" theme (e.g.,
  subtle golds, deep blues, or warm earth tones) for highlights, icons, and
  status indicators.
- **Layout:** Standard IDE structure:
  - **Left Sidebar:** Project Explorer (Tree View). Initially might be simple,
    showing top-level project folders.
  - **Main Area:** Placeholder or welcome screen. Tabs will go here later.
  - **Bottom Panel:** Tabbed area. Start with a "Console/Logs" tab.
  - **Status Bar (Bottom):** Persistent area showing critical info like Lion
    Runtime Status.
- **Typography:** Clear, readable fonts. A good sans-serif for UI elements (like
  Inter, Noto Sans) and a clear monospace font for logs/code (like Fira Code,
  JetBrains Mono).
- **Responsiveness:** The UI should feel snappy. Backend calls via Tauri
  `invoke` are async, so the UI must handle loading states gracefully (e.g.,
  subtle loading spinners or skeleton placeholders) without freezing. Real-time
  updates (logs, status) should appear smoothly.
- **Initial Impression:** The application should launch quickly and immediately
  try to connect to/start the Lion runtime. The status bar should reflect this
  connection attempt. Even with limited features, it should feel like a stable
  foundation connected to a powerful backend.

---

**2. User Experience (How User Will Use It):**

1. **Launch:** User starts the LionForge application (e.g., double-clicking the
   executable).
2. **Initial State:** The main window appears with the basic layout (Sidebar,
   Main Area, Bottom Panel, Status Bar).
3. **Runtime Connection:**
   - The application backend automatically attempts to initialize and start the
     embedded `lion_runtime::Runtime`.
   - The Status Bar displays "Lion Runtime: Starting..." then transitions to
     "Lion Runtime: Running" (with a green indicator) or "Lion Runtime: Error"
     (with red).
4. **Project Selection:**
   - Initially, there might be no project open. A "File > Open Project..." menu
     item is available (or a button on a welcome screen).
   - User selects a directory containing (or intended to contain) their agent
     system files. For Phase 1, the app might just check for the _existence_ of
     standard subfolders (`agents/`, `workflows/`, `knowledge/`, `security/`) or
     a `lionforge.toml` file to identify it as a project root.
5. **Project Explorer:**
   - Once a project is opened, the Left Sidebar populates with a tree view
     showing the recognized project structure (the standard folders).
   - _Interaction:_ Clicking folders might expand them (if they contain files
     the backend can list), but clicking _files_ won't open editors yet. It's
     purely navigational viewing.
6. **Agent Status View (New Tab in Bottom Panel or Dedicated View):**
   - This view automatically queries the Lion runtime (via a Tauri command) upon
     project load or periodically.
   - It displays a table or list of agents currently known to the _backend
     runtime_.
   - _Important:_ These agents might have been loaded via external means (CLI,
     config file) for Phase 1; the UI doesn't _manage_ them yet, it just
     _observes_ them.
   - Displayed columns: Agent ID (`PluginId`), Name (from `PluginMetadata` if
     available), State (`PluginState` enum value, e.g., "Ready", "Running",
     "Failed").
   - The view listens for backend events (`agent_status_changed`) and updates
     the table dynamically if an agent's state changes in the runtime.
7. **Console/Log View (Bottom Panel Tab):**
   - This view is active immediately on startup.
   - It listens for `new_log_entry` events from the backend.
   - As log events arrive (from the runtime itself, observability setup, or
     potentially proxied agent logs later), they are appended to a scrollable
     view.
   - Each log entry shows Timestamp, Level (colored appropriately - ERROR red,
     WARN yellow, INFO green/default), Component/Module, and Message.
   - Basic auto-scrolling should be enabled by default. A "Clear" button could
     be added. Advanced filtering is deferred.
8. **Shutdown:** User closes the application window. The Tauri backend
   intercepts the close request, triggers a graceful shutdown of the
   `lion_runtime::Runtime` (calling `runtime.shutdown().await`), waits for it to
   complete (with a timeout), and then allows the application to exit.

---

**3. Lion Integration (How It Should Use Lion):**

- **Runtime Lifecycle (Tauri Backend - Rust):**
  - Use `tauri::Builder::setup` to initialize the `lion_runtime::Runtime`. Load
    `RuntimeConfig` (potentially looking for a project-specific config or using
    defaults). Store `Arc<Runtime>` in Tauri's managed state.
  - Call `runtime.start().await` during setup. Handle potential errors and
    report them to the frontend (e.g., emit an error event).
  - Use `tauri::Builder::on_window_event` to hook into the window close request
    (`WindowEvent::CloseRequested`). Prevent the default close, call
    `runtime.shutdown().await` asynchronously, and then explicitly close the
    window (`window.close()`).
- **Runtime Status (Tauri Command & Event):**
  - `#[tauri::command] async fn get_runtime_status(runtime: State<'_, Arc<Runtime>>) -> Result<RuntimeStatusPayload, String>`:
    Accesses runtime state (e.g., `runtime.system.is_running()`, needs methods
    in `lion_runtime` to expose status). Returns a serializable payload.
  - The backend needs to monitor the runtime's state (perhaps via a task polling
    or internal events within `lion_runtime`) and use
    `AppHandle::emit_all("runtime_status_changed", ...)` when the status
    changes.
- **Agent Listing (Tauri Command):**
  - `#[tauri::command] async fn list_agents(runtime: State<'_, Arc<Runtime>>) -> Result<Vec<AgentSummary>, String>`:
    Calls `runtime.plugins.get_plugins().await`. Maps the resulting
    `Vec<PluginMetadata>` to a simpler
    `AgentSummary { id: String, name: String, state: String }` struct for the
    frontend.
- **Agent Status Events (Backend Event Listener -> Tauri Event):**
  - The `lion_runtime::plugin::manager` needs an event mechanism (e.g.,
    `tokio::sync::broadcast`, callbacks) to signal state changes.
  - The Tauri backend component responsible for runtime integration listens to
    these internal events.
  - On receiving an internal agent status change event, it constructs an
    `AgentStatusUpdate { id: String, new_state: String }` payload and emits it
    to the frontend: `app_handle.emit_all("agent_status_changed", ...)`.
- **Log Handling (Backend Log Capture -> Tauri Event):**
  - Configure `lion_observability` (used within `lion_runtime`) to output logs.
  - **Option A (Tracing Subscriber):** Implement a custom `tracing::Subscriber`
    layer within the Tauri backend. This layer intercepts tracing events/logs
    generated by Lion components. Inside its `event` method, format the log into
    the `LogEntry` struct and emit it via
    `app_handle.emit_all("new_log_entry", ...)`.
  - **Option B (Channel):** Modify `lion_observability` or the runtime's logging
    setup to push formatted `LogEntry` structs onto a `tokio::sync::mpsc`
    channel. The Tauri backend spawns a task that reads from this channel and
    emits events to the frontend.
- **Project Handling (Tauri Command):**
  - `#[tauri::command] fn identify_project(path: String) -> Result<ProjectStructure, String>`:
    Takes a directory path. Checks for `lionforge.toml` or presence of
    `agents/`, `workflows/` etc. Reads basic project name from TOML if it
    exists. Returns
    `ProjectStructure { name: String, root_path: String, folders: Vec<String> }`.
    _Does not_ load runtime state based on project yet.

---

**4. Tauri Backend Implementation Details (Rust - Phase 1):**

- **Crate Setup:** `Cargo.toml` with `tauri`, `lion_runtime`, `tokio`, `serde`,
  `serde_json`, `thiserror`, `anyhow`, `tracing`.
- **`main.rs`:**
  - `#[tokio::main] async fn main()` entry point.
  - `tauri::Builder::default()`
  - `.manage(Arc<Runtime>)` // Initialize and store runtime here.
  - `.setup(|app| { /* Start runtime, set up event listeners */ Ok(()) })`
  - `.invoke_handler(tauri::generate_handler![...])` // Register commands.
  - `.on_window_event(...)` // Handle close requests for shutdown.
  - `.run(tauri::generate_context!())?`
- **`commands.rs`:** Define the `get_runtime_status`, `list_agents`,
  `identify_project` commands using `#[tauri::command]`. Use `tauri::State` to
  access the managed `Runtime`. Map errors to `String`. Define serializable
  return types (Payloads/Summaries).
- **`events.rs`:** Define `struct RuntimeStatusUpdate`,
  `struct AgentStatusUpdate`, `struct LogEntry` implementing `Clone, Serialize`.
- **`runtime_listener.rs`:** (Or integrated into `main.rs::setup`) Code that
  interacts with the `Runtime` instance to subscribe to internal Lion events
  (agent status, logs) and uses the `AppHandle` (obtained in `setup`) to
  `emit_all`. Requires Lion components to _provide_ these event
  streams/callbacks.
- **`project.rs`:** Implement the `identify_project` logic, including basic
  `lionforge.toml` parsing (if desired for Phase 1).

---

**5. Frontend Implementation Details (React/TS - Phase 1):**

- **Project Setup:** Use `create-vite` or `create-react-app` with TypeScript,
  integrate Tauri (`npm run tauri dev`).
- **API Layer (`src/lib/api.ts`):**
  ```typescript
  import { invoke } from "@tauri-apps/api/tauri";

  interface RuntimeStatusPayload {
    is_running: boolean;
    uptime_seconds: number; /* ... */
  }
  interface AgentSummary {
    id: string;
    name: string;
    state: string;
  }
  // ... other payload types

  export const getRuntimeStatus = async (): Promise<RuntimeStatusPayload> =>
    await invoke("get_runtime_status");

  export const listAgents = async (): Promise<AgentSummary[]> =>
    await invoke("list_agents");

  // ... other API functions
  ```
- **Event Handling (`src/hooks/useTauriEvent.ts`):** Custom hook to simplify
  listening.
  ```typescript
  import { Event, listen } from "@tauri-apps/api/event";
  import { useEffect, useState } from "react";

  export function useTauriEvent<T>(
    eventName: string,
    initialValue: T | null,
  ): T | null {
    const [data, setData] = useState<T | null>(initialValue);
    useEffect(() => {
      const unlisten = listen<T>(eventName, (event: Event<T>) => {
        setData(event.payload);
      });
      return () => {
        unlisten.then((f) => f());
      };
    }, [eventName]);
    return data;
  }
  // Similar hook for streaming/appending events (like logs)
  ```
- **Components:**
  - `ProjectExplorer.tsx`: Displays static folders for now. Fetches project
    structure via `identify_project` when a project is opened.
  - `AgentStatusView.tsx`: Fetches initial list via `listAgents`. Uses
    `useTauriEvent` for `agent_status_changed` to update the state (e.g., held
    in a Zustand store or local state). Renders a table.
  - `LogView.tsx`: Uses a state variable (e.g., `useState<LogEntry[]>([])`) and
    `listen('new_log_entry', ...)` to append new entries. Uses a virtualized
    list component (like `react-window` or `react-virtuoso`) to handle
    potentially many logs efficiently.
  - `StatusBar.tsx`: Uses `getRuntimeStatus` initially and `useTauriEvent` for
    `runtime_status_changed`. Displays text and indicator icon.
- **State Management:** Choose a simple state manager (Zustand is often good for
  Tauri) or React Context for sharing runtime status, agent list, logs across
  components.

---

**Phase 1 Acceptance Criteria (Refined):**

- The Tauri application compiles and launches without errors.
- The Status Bar accurately reflects the Lion runtime starting and running.
- The user can select a directory using "File > Open Project...".
- The Project Explorer displays the standard folder names (`Agents`,
  `Workflows`, etc.) for the selected project.
- The Agent Status view displays a list of Agent IDs, Names, and States fetched
  from the backend `lion_runtime`.
- The Console/Log view displays logs emitted by the `lion_runtime` backend as
  they occur.
- Closing the application window triggers a backend message indicating shutdown
  initiation. (Full graceful shutdown verification might be Phase 2).
- UI remains responsive during backend initialization.

This phase lays a robust foundation by connecting the UI to the core runtime and
enabling essential observability, setting the stage for adding interactive
features in subsequent phases.
