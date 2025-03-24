Below is an **exhaustively detailed** plan for **Stage 2, Phase 3**. In this
phase, you will expand the **web-based UI** to manage plugins—loading new plugin
manifests, listing and invoking existing plugins—and further refine your
**real-time logs** (or event streaming) so that plugin operations also appear in
the interface. This step integrates the microkernel’s **plugin manager**
directly into the front end, allowing users to see which plugins are loaded,
load new ones, and invoke them, all while viewing partial logs or plugin
invocation results in real time.

---

# **Stage 2, Phase 3 – Plugin Management UI & Extended Real-Time Logging**

## 1. Objectives & Scope

1. **Plugin Management UI**
   - Provide front-end controls for loading a new plugin from a manifest file or
     URL.
   - Show a list of **currently loaded** plugins (with basic details like name,
     version, permissions).
   - Allow **invoking** a plugin function, if the microkernel supports direct
     invocation.

2. **Extended Real-Time Logs**
   - Integrate plugin events (like `PluginInvoked`, `PluginResult`, or
     `PluginError`) into your real-time SSE or WebSocket feed.
   - Display them in your UI’s log console or a dedicated plugin-events area, so
     users see each plugin’s invocation and results as they happen.

3. **Refine the Existing Agent/Logs**
   - Continue building on the real-time streaming from **Phase 2**: The UI
     should handle not only agent partial outputs but also plugin events in the
     same feed (or separate feed).

4. **Docker & Local**
   - The user can still run `docker run -p 8080:8080 lion_ui` or `cargo run`
     locally, then open the UI to load plugins, invoke them, and watch logs.
   - Validate the entire system works in both environments.

**Success** at the end of Phase 3 means that you can fully manage
plugins—listing, loading, invoking them—from the UI, and see all relevant plugin
logs or errors in real time.

---

## 2. High-Level Tasks

1. **Add Plugin Management Endpoints in the UI Server**
   - E.g., `POST /api/plugins` to load a plugin manifest.
   - `GET /api/plugins` to list loaded plugins.
   - Possibly `POST /api/plugins/:id/invoke` or something similar if the
     orchestrator supports it.

2. **Front-End Plugin Pages**
   - A new “Plugins” tab or page listing loaded plugins.
   - A form or button to “Load New Plugin” by providing a manifest file or JSON
     data.
   - If relevant, an “Invoke Plugin” button or form where a user can input
     arguments.

3. **Extend SSE or WebSocket to Include Plugin Events**
   - The orchestrator sends `PluginInvoked`, `PluginResult`, `PluginError` to
     your broadcast or event feed.
   - The UI logs or displays these events in real time.

4. **UI Enhancements**
   - Possibly separate logs into categories: “Agent logs,” “Plugin logs,” or
     unify them.
   - Additional user feedback for plugin loads, errors, or success.

5. **Testing & Docker**
   - Confirm you can load a plugin in Docker.
   - Confirm real-time plugin logs appear in the UI.

---

## 3. Step-by-Step Instructions

### Step 1: **Add Plugin Management Routes**

1. In your `lion_ui` server code, define routes for plugin operations:
   ```rust
   // lion_ui/src/plugins.rs
   use axum::{
       extract::{State, Path},
       Json,
   };
   use crate::MyAppState;
   use std::sync::Arc;
   use serde::Deserialize;

   // A payload for loading a plugin
   #[derive(Deserialize)]
   pub struct LoadPluginRequest {
       pub manifest: String,  // or path, or entire TOML
   }

   pub async fn load_plugin_handler(
       State(app_state): State<Arc<MyAppState>>,
       Json(req): Json<LoadPluginRequest>,
   ) -> String {
       // Parse the manifest string
       // Possibly call your orchestrator plugin manager
       match app_state.orchestrator.load_plugin_from_str(&req.manifest) {
           Ok(plugin_id) => format!("Loaded plugin: {}", plugin_id),
           Err(e) => format!("Error loading plugin: {:?}", e),
       }
   }

   pub async fn list_plugins_handler(
       State(app_state): State<Arc<MyAppState>>,
   ) -> Json<Vec<PluginInfo>> {
       let list = app_state.orchestrator.list_plugins(); // or something
       Json(list)
   }

   #[derive(Deserialize)]
   pub struct InvokePluginRequest {
       pub input: String,
   }

   pub async fn invoke_plugin_handler(
       Path(plugin_id): Path<String>,
       State(app_state): State<Arc<MyAppState>>,
       Json(req): Json<InvokePluginRequest>,
   ) -> String {
       // Convert plugin_id to Uuid
       // call orchestrator plugin invocation
       // return success/error
       "Plugin invoked successfully".to_string()
   }
   ```
2. Insert these routes into your `main.rs` or wherever you define `Router`:
   ```rust
   let app = Router::new()
       .route("/api/plugins", post(load_plugin_handler).get(list_plugins_handler))
       .route("/api/plugins/:plugin_id/invoke", post(invoke_plugin_handler))
       // other routes...
       ;
   ```
3. In your orchestrator, you might implement:
   - `fn load_plugin_from_str(&mut self, manifest: &str) -> Result<Uuid, PluginError>`
   - `fn list_plugins(&self) -> Vec<PluginInfo>`
   - `fn invoke_plugin(&mut self, plugin_id: Uuid, input: &str) -> Result<String, PluginError>`

### Step 2: **UI: “Plugins” Page**

1. In your `frontend/index.html` (or separate routes in a single-page app):
   ```html
   <h2>Plugins</h2>
   <div>
     <textarea
       id="pluginManifest"
       rows="5"
       cols="40"
       placeholder="Paste plugin manifest here"
     ></textarea>
     <button id="loadPluginBtn">Load Plugin</button>
   </div>
   <div id="pluginList"></div>
   <script>
     async function loadPlugin() {
       const manifestText = document.getElementById("pluginManifest").value;
       const res = await fetch("/api/plugins", {
         method: "POST",
         headers: { "Content-Type": "application/json" },
         body: JSON.stringify({ manifest: manifestText }),
       });
       const txt = await res.text();
       alert(txt);
       // Then refresh plugin list
       fetchPlugins();
     }

     async function fetchPlugins() {
       const res = await fetch("/api/plugins");
       const plugins = await res.json();
       const pluginDiv = document.getElementById("pluginList");
       pluginDiv.innerHTML = "";
       plugins.forEach((pl) => {
         const p = document.createElement("div");
         p.innerText = `Plugin ${pl.id}: ${pl.name} v${pl.version}`;
         // Possibly add an invoke button
         const btn = document.createElement("button");
         btn.innerText = "Invoke";
         btn.onclick = () => invokePlugin(pl.id);
         p.appendChild(btn);
         pluginDiv.appendChild(p);
       });
     }

     async function invokePlugin(pluginId) {
       const input = prompt("Enter invocation input:");
       if (!input) return;
       const res = await fetch(`/api/plugins/${pluginId}/invoke`, {
         method: "POST",
         headers: { "Content-Type": "application/json" },
         body: JSON.stringify({ input }),
       });
       const txt = await res.text();
       alert(txt);
     }

     document.getElementById("loadPluginBtn").onclick = loadPlugin;
     // Maybe call fetchPlugins() on page load or a "Refresh" button
   </script>
   ```
2. This way, a user can **paste a plugin manifest** (e.g., your TOML), click
   “Load Plugin,” see the list, and optionally “Invoke” them.

### Step 3: **Add Plugin Events to Real-Time Logs**

1. In the orchestrator, whenever a plugin is invoked, you might do something
   like:
   ```rust
   let msg = format!("PluginInvoked: plugin={}, input={}", plugin_id, input);
   logs_tx.send(msg).ok(); 
   // Then after success or error:
   let msg = format!("PluginResult: plugin={}, output={}", plugin_id, output);
   logs_tx.send(msg).ok();
   ```
2. The SSE feed already passes these lines to the UI. So in the UI’s
   `evtSource.onmessage`, you’ll see them appear as typical logs.
3. (Optional) If you want more structured data, you can define
   `Event::default().json(...)` in SSE so the front end can parse JSON. But for
   Phase 3, a simple string is enough.

### Step 4: **Refining the Log Display & Docker Setup**

1. If your SSE feed now has agent logs **and** plugin logs, you might want to
   separate them in the UI:
   ```javascript
   evtSource.onmessage = (event) => {
     const logsDiv = document.getElementById("logs");
     // A naive approach: parse event.data
     if (event.data.startsWith("PluginInvoked:")) {
       // show in plugin area or highlight
     } else if (event.data.startsWith("Agent")) {
       // show in agent area
     }
     logsDiv.innerText += event.data + "\n";
   };
   ```
2. Docker remains mostly unchanged. If you introduced a bundler or separate
   front-end build steps, ensure your Dockerfile copies your final `dist/` or
   runs the build.
3. Test again with:
   ```bash
   docker build -t lion_ui .
   docker run -p 8080:8080 lion_ui
   ```
   Then open the UI in the browser, load a plugin, watch for real-time logs as
   you invoke it.

### Step 5: **Local Validation & Demo**

1. Locally: `cargo run -p lion_ui`.
2. Open the UI:
   - Paste a minimal plugin manifest (like a “hello plugin”).
   - Click load plugin → see success.
   - The plugin appears in your listing. Click “Invoke” → logs appear in real
     time.
3. If partial outputs or advanced plugin logic exist, confirm the SSE feed shows
   those lines too.

---

## 4. Potential Enhancements & Pitfalls

1. **Plugin Manifest Handling**:
   - For a large TOML manifest, you might want a file upload approach or
     advanced editor. For Phase 3, a simple textarea is enough.
   - You might parse the manifest in the UI side or send it raw. Just ensure
     your orchestrator can parse it.

2. **Plugin Output Format**:
   - If a plugin can produce partial lines, you can treat it similarly to agent
     partial lines, sending them over SSE. Possibly store them in a
     plugin-specific channel.

3. **UI Layout**:
   - Consider a simple tab-based layout: “Agents” tab, “Plugins” tab, “Logs”
     tab. In Phase 3, a single page might suffice, but if the UI grows, a small
     front-end framework might help.

4. **Security**:
   - Right now, it’s local or Docker-based with no auth. If you open it on a
     network, consider some authentication or token to avoid unauthorized plugin
     loads.

5. **Testing**:
   - Write an integration test that:
     1. Loads a plugin via `POST /api/plugins`.
     2. Invokes it via `POST /api/plugins/:id/invoke`.
     3. Subscribes to SSE and ensures the test sees a plugin result message.

---

## 5. Success Criteria

1. **UI Manages Plugins**:
   - You can load a plugin by copying/pasting a manifest in a text area.
   - The UI lists currently loaded plugins with name, version, ID.
   - Clicking “Invoke” triggers the orchestrator, which logs an event, possibly
     returning an output.

2. **Logs Stream**:
   - Real-time logs now include plugin-specific events (`PluginInvoked`,
     `PluginResult`), visible in the UI’s SSE feed or log console.
   - Agent events are also displayed, ensuring both agent partial outputs and
     plugin logs appear seamlessly.

3. **Docker**:
   - The same scenario works in Docker with no extra config.
   - A user can do `docker run -p 8080:8080 lion_ui`, open the browser, load and
     invoke plugins, see logs.

---

## 6. Expected Outcome of Phase 3

By the end of Stage 2, Phase 3, you have:

- A **Plugin Management UI** that can load new plugin manifests, list them, and
  invoke them from the front end.
- **Extended real-time logging** capturing both agent and plugin events,
  displayed in the same SSE-based console (or separate plugin area).
- A robust approach to orchestrator bridging, ensuring all plugin actions (load,
  invoke, error) appear in near real-time for the user.
- A Docker-based environment that seamlessly demonstrates multi-agent
  concurrency **and** plugin interactions, building toward a fully functional,
  user-friendly system.

Next steps in future phases might include advanced plugin usage (like config
pages, deeper error checking), agent orchestration workflows, or a Tauri-based
local app for macOS.
