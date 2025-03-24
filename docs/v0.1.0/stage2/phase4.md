Below is an **exhaustively detailed** plan for **Stage 2, Phase 4**, focusing on
**Advanced Logging (search, filter) and Initial Tauri Setup** for a local
desktop experience—particularly on macOS. In this phase, you enhance the
existing web-based UI with better log management capabilities (filtering by
agent/plugin, searching partial outputs), while also **laying the groundwork**
to wrap the same front end into a **Tauri** desktop application, providing a
native-like environment on macOS.

---

# **Stage 2, Phase 4 – Advanced Logging & Initial Tauri Setup**

## 1. Objectives & Scope

1. **Advanced Logging & Search**
   - Extend your existing real-time logs (SSE or WebSocket) with **filtering**
     or **search** (by agent ID, plugin ID, correlation ID, log text, etc.).
   - Provide **UI controls** so a user can quickly find a particular output line
     among many.

2. **Initial Tauri Setup** (for local desktop usage)
   - Reuse the same front-end code (HTML/JS or React/Svelte) within Tauri,
     producing a `.app` for macOS (and other OSes if desired).
   - Confirm that orchestrator + UI runs **fully local** with Tauri (no Docker
     needed).

3. **Refine Docker & Local**
   - The web-based version (Phase 1–3) remains intact; Tauri is an
     **additional** packaging approach.
   - Docker usage is unchanged, except you might not need it for Tauri
     distribution.

**Success** at the end of Phase 4 means you can run **two** approaches to the
UI:

- The existing web-based server (with advanced log search).
- A Tauri `.app` bundling the same front end, giving local macOS usage and an
  integrated orchestrator, with the same advanced log features.

---

## 2. High-Level Tasks

1. **Implement Log Storage & Search** in Orchestrator or a local buffer to
   support filtering.
2. **UI: Search & Filter Controls** on the logs page (by agent, plugin,
   correlation ID, text).
3. **Set Up Tauri** with a minimal `tauri.conf.json`, pointing to your built
   front-end.
4. **Embed Orchestrator** in Tauri (the same code that was run in the web server
   context).
5. **Package & Test** on macOS, ensuring a `.app` can be launched to see the
   same UI offline.

---

## 3. Step-by-Step Instructions

### Step 1: **Advanced Logging & Search in Orchestrator**

1. **In-Memory Log Buffer**
   - If you only have a real-time SSE broadcast, you might not store logs. For
     search, you need a local store.
   - Example: Keep a ring buffer or vector of the last X lines in the
     orchestrator or UI server’s `MyAppState`.
   - Each log line can be a struct, e.g.:
     ```rust
     pub struct LogLine {
       pub timestamp: chrono::DateTime<chrono::Utc>,
       pub agent_id: Option<Uuid>,
       pub plugin_id: Option<Uuid>,
       pub correlation_id: Option<Uuid>,
       pub message: String,
     }
     ```
   - If you want truly advanced searching or older logs, you could integrate a
     small local DB or event-sourcing. Phase 4 typically uses a memory approach.

2. **Update SSE**
   - You still broadcast new lines as they arrive. But for search or filter, the
     UI can do an HTTP call to a new endpoint, e.g.
     `GET /api/logs?agent=...&plugin=...&text=...`. That endpoint queries your
     in-memory list or ring buffer.

### Step 2: **Create Log Filter Endpoints**

1. A route like `GET /api/logs`:
   ```rust
   // GET /api/logs?agent=...&plugin=...&text=...
   use axum::{
       extract::{Query, State},
       Json,
   };
   use std::sync::Arc;

   #[derive(Deserialize)]
   pub struct LogFilter {
       pub agent: Option<String>,
       pub plugin: Option<String>,
       pub text: Option<String>,
   }

   pub async fn search_logs_handler(
       State(app_state): State<Arc<MyAppState>>,
       Query(params): Query<LogFilter>,
   ) -> Json<Vec<LogLine>> {
       // Filter in-memory log lines by agent, plugin, text
       let lines = app_state.log_buffer.filter(params);
       Json(lines)
   }
   ```
2. The `filter(params)` can convert agent or plugin string to `Uuid`, etc.
3. This endpoint returns JSON array of matching lines. The UI can show them or
   integrate them with SSE.

### Step 3: **UI: Searching & Filtering Logs**

1. In your front end, create a new “Advanced Logs” page or some search form:
   ```html
   <h2>Advanced Log Search</h2>
   <div>
     <input id="agentFilter" placeholder="Agent ID" />
     <input id="pluginFilter" placeholder="Plugin ID" />
     <input id="textFilter" placeholder="Search text" />
     <button id="searchBtn">Search</button>
   </div>
   <div id="searchResults"></div>

   <script>
     async function searchLogs() {
       const agent = document.getElementById("agentFilter").value;
       const plugin = document.getElementById("pluginFilter").value;
       const text = document.getElementById("textFilter").value;
       const params = new URLSearchParams();
       if (agent) params.append("agent", agent);
       if (plugin) params.append("plugin", plugin);
       if (text) params.append("text", text);

       const res = await fetch(`/api/logs?${params.toString()}`);
       const lines = await res.json();
       const container = document.getElementById("searchResults");
       container.innerHTML = lines.map((l) =>
         `<div>[${l.timestamp}] ${l.message}</div>`
       ).join("");
     }

     document.getElementById("searchBtn").onclick = searchLogs;
   </script>
   ```
2. Now a user can input agent ID, plugin ID, or partial text, then get results
   from the orchestrator’s in-memory logs.

### Step 4: **Initial Tauri Setup**

1. **Install Tauri**:
   ```bash
   cargo install create-tauri-app
   ```
   Or if you’re integrating Tauri manually, add `tauri = "1"` to your
   `lion_ui/Cargo.toml`.
2. In your `lion_ui/` folder, create Tauri scaffolding:
   ```bash
   cd lion_ui
   npx create-tauri-app
   ```
   or if you prefer manual, create a `src-tauri/tauri.conf.json` with something
   like:
   ```json
   {
     "build": {
       "beforeDevCommand": "",
       "beforeBuildCommand": "",
       "devPath": "frontend/dist",
       "distDir": "frontend/dist"
     },
     "tauri": {
       "windows": [
         {
           "fullscreen": false,
           "height": 800,
           "resizable": true,
           "title": "lion UI",
           "width": 1200
         }
       ]
     }
   }
   ```
3. **Point Tauri to Your Built Front-End**
   - If your front-end uses `npm run build` to produce `frontend/dist`, ensure
     Tauri loads that as a static asset.
4. **Use Tauri Commands**:
   - Instead of running an HTTP server, Tauri can run your orchestrator code
     in-process. Or you can keep your Axum server, but it’d be local.
   - For a quick approach, you can define Tauri commands to spawn agents, load
     plugins, etc.:
     ```rust
     #[tauri::command]
     fn spawn_agent(prompt: String) -> String {
         // call orchestrator logic
         "spawned agent".to_string()
     }
     ```
   - Then in your front-end code, you do
     `window.__TAURI__.invoke("spawn_agent", { prompt: "my prompt" })`.
   - If you want to keep SSE logs, you might run the Axum server on a local port
     like 127.0.0.1:9000 inside Tauri, or do Tauri event APIs.
5. **Build Tauri**:
   ```bash
   cargo tauri build
   ```
   This produces a `.app` for macOS in `target/release/bundle/` if everything is
   configured correctly.

### Step 5: **Refine Tauri or Keep Axum**

- You can do a “hybrid” approach: Tauri opens a window pointing to
  `http://127.0.0.1:9000/`. This approach reuses your Axum code exactly, but
  it’s all local.
- Or you define Tauri commands for everything. Then SSE might be replaced by
  Tauri’s event system. That’s more refactoring. For Phase 4, a “hybrid”
  approach is simpler: keep Axum for SSE, Tauri loads the front-end from
  `dist/`.

### Step 6: **Local Testing & Docker**

1. **macOS**:
   - `cargo tauri dev` or `cargo tauri build`.
   - Launch the `.app`. You see the same UI. You can do advanced log searching.
2. **Docker**:
   - The Tauri build is typically for a local desktop. Docker usage might remain
     for the web-based approach. If you want to distribute Tauri in Docker,
     that’s less common. Usually Tauri is for local user space.
3. **End-To-End**:
   - Confirm that advanced log searching or filtering works in both:
     - The standard `cargo run -p lion_ui` approach.
     - The Tauri `.app` approach (if you configured a local Axum server or Tauri
       commands).

---

## 4. Potential Enhancements & Pitfalls

1. **Data Retention**:
   - If your user kills Tauri or the Docker container, logs vanish unless you
     store them in a file or DB.
   - For advanced usage, consider an actual event store or database.

2. **Performance**:
   - If searching large logs in memory, you might want indexing or at least a
     limit to how many lines are stored.
   - Tauri: watch out for any blocking code that might freeze the UI thread if
     not carefully handled (the Tauri approach might require sending tasks to a
     Rust background thread).

3. **UI Complexity**:
   - If you want separate pages or a more robust front-end, you might adopt
     React or Svelte for navigation, subcomponents, etc. That can be done
     seamlessly with Tauri as well.
   - Phase 4 is a good time to possibly introduce a small front-end framework if
     you haven’t yet.

4. **macOS Notarization**:
   - If you plan to distribute `.app` widely, Apple requires code signing or
     notarization. That’s beyond the scope of Phase 4 but worth noting.

5. **Security**:
   - Tauri default is local usage with no external network port. If you keep
     Axum or SSE, it’s still local to 127.0.0.1. That’s typically safe for
     single-user scenarios.

---

## 5. Success Criteria

1. **Advanced Log Search**:
   - The user can input search criteria (agent ID, plugin ID, or text), retrieve
     a filtered list of log lines from your memory store.
   - SSE is still live for real-time logs, but search is an additional endpoint
     for historical or advanced filtering.

2. **Tauri Local App**:
   - You can do `cargo tauri build` on macOS, producing a `.app`.
   - Double-clicking it opens a window that shows the same front-end you had in
     the browser. You can spawn agents, load plugins, see partial logs, etc.,
     all offline.

3. **Docker**:
   - The existing approach for a pure web-based server still works. Advanced
     logs are available at `/api/logs`, Tauri is just an extra method for local
     usage.

4. **Both UI Approaches**:
   - “Web-based server + SSE” and “Tauri .app + optional SSE or Tauri commands”
     co-exist in the codebase.
   - Users can choose how they want to run lion’s UI: local `.app` for a desktop
     vibe, or Docker-based server for external access.

---

## 6. Expected Outcome of Phase 4

By the end of **Stage 2, Phase 4**, you have:

- A **robust log search/filter** system in the UI, integrated with your
  microkernel’s log buffer. Real-time SSE streaming remains active for new
  lines, while old lines can be filtered or retrieved via `GET /api/logs` calls.
- An **initial Tauri-based** local desktop approach for macOS that reuses your
  front-end code, letting users run lion **entirely offline** without Docker or
  a separate web server. The `.app` loads your orchestrator code and front-end
  in one packaged environment.
- A UI that’s significantly more powerful—letting advanced users manage large or
  complex multi-agent operations, find relevant log lines quickly, and see
  plugin invocation details in real time.

In **Stage 2, Phase 5**, you might finalize the UI experience (styling, user
experience improvements, more advanced plugin usage) or add more integration
with multi-agent workflows (graph visualization of agent dependencies, for
example). But with Phase 4 complete, you have a strong local macOS Tauri `.app`
plus advanced web-based logging and plugin management.
