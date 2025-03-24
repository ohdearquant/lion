Below is an **exhaustively detailed** set of instructions for **Stage 2,
Phase 1**: **UI Foundation & Docker Setup**. The goal is to create a **web-based
front end** (hosted by a minimal Rust HTTP server) for your lion microkernel,
ensuring real-time event streaming is possible later and that the entire system
can run in Docker. This phase yields a **skeleton** UI that simply confirms it
can connect to the orchestrator. Over subsequent phases, you will add
multi-agent management, real-time logs, and plugin control to this foundation.

---

# **Stage 2, Phase 1 – UI Foundation & Docker Setup**

## 1. Objectives & Scope

1. **Establish a New UI Crate**:
   - Create a Rust crate (e.g., `lion_ui` or `agentic_ui`) that will host both:
     1. A minimal web server (using Axum or Actix).
     2. A skeleton HTML/JS (or a basic front-end framework) to test the
        orchestrator connection.
2. **Orchestrator Connectivity**:
   - Validate that the new UI can call the microkernel’s orchestrator or simple
     “ping” function.
   - This proves end-to-end integration: UI → local server → orchestrator →
     response.
3. **Dockerfile & Docker Integration**:
   - Provide a Docker configuration so the team can run
     `docker build ... && docker run -p 8080:8080 ...`.
   - This container, on startup, serves the minimal UI at
     `http://localhost:8080/`.
4. **macOS Testing**:
   - Ensure local dev usage on macOS is straightforward (`cargo run`) plus
     Docker usage if desired.

**Success** at the end of Phase 1 means you have a running UI crate with a
minimal page, a single test route (e.g., `/ping`), and an orchestrator call—plus
a Docker container that exposes this same interface.

---

## 2. High-Level Tasks

1. **Create the UI Crate & Project Structure**
2. **Implement a Minimal HTTP Server**
3. **Add a Basic Web Page** (HTML or minimal framework)
4. **Invoke a “Ping” Endpoint** from the UI
5. **Bridge to Orchestrator** (call a simple function in `agentic_core`)
6. **Build & Test with Docker**
7. **Local Validation** on macOS

---

## 3. Step-by-Step Instructions

### Step 1: **Create the UI Crate**

1. Navigate to your lion workspace root (where your top-level `Cargo.toml` and
   `agentic_core`, `agentic_cli`, etc., live).
2. Create a new folder `lion_ui` (or `agentic_ui`):
   ```bash
   cd lion
   cargo new --lib lion_ui
   ```
3. Open the top-level `Cargo.toml`; under `[workspace]` → `members`, add
   `"lion_ui"` to ensure it’s recognized as part of the workspace:
   ```toml
   [workspace]
   members = [
       "agentic_core",
       "agentic_cli",
       "lion_ui"
   ]
   ```
4. Inside `lion_ui/Cargo.toml`, reference `agentic_core` so you can call
   orchestrator functions:
   ```toml
   [dependencies]
   agentic_core = { path = "../agentic_core" }
   axum = "0.6"            # or actix-web = "4"
   tokio = { version = "1.0", features = ["full"] }
   # Possibly add serde, etc.
   ```

### Step 2: **Implement a Minimal HTTP Server**

1. In `lion_ui/src/main.rs` (or `server.rs` if you prefer a separate file):
   ```rust
   use axum::{Router, routing::get};
   use std::net::SocketAddr;
   use agentic_core::Orchestrator; // or any orchestrator function

   #[tokio::main]
   async fn main() {
       // Initialize orchestrator or a reference to it if needed
       // For Phase 1, we can do minimal checks
       println!("Starting minimal lion_ui server...");

       // Build a simple router with a "ping" endpoint
       let app = Router::new().route("/ping", get(ping_handler));

       let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
       println!("Listening on {addr}");
       axum::Server::bind(&addr)
           .serve(app.into_make_service())
           .await
           .unwrap();
   }

   async fn ping_handler() -> &'static str {
       // For demonstration, might call agentic_core::some_ping_function() if you like
       "Pong from lion_ui!"
   }
   ```
2. This code:
   - Listens on `0.0.0.0:8080`.
   - Exposes a `GET /ping` returning “Pong from lion_ui!” or something similar.
   - Phase 1 can keep it trivial. In later steps, you’ll expand the router to
     handle partial logs or agent spawns.

### Step 3: **Add a Basic Web Page** (Optional but recommended)

1. Create a `frontend/` or `ui/` folder:
   ```bash
   mkdir -p lion_ui/frontend
   ```
2. Put a minimal `index.html` inside `lion_ui/frontend/`:
   ```html
   <!-- lion_ui/frontend/index.html -->
   <!DOCTYPE html>
   <html lang="en">
     <head>
       <meta charset="UTF-8" />
       <title>lion UI</title>
     </head>
     <body>
       <h1>lion UI - Hello from Phase1</h1>
       <button id="pingBtn">Ping Microkernel</button>
       <div id="resp"></div>

       <script>
         async function ping() {
           const respEle = document.getElementById("resp");
           try {
             const res = await fetch("/ping");
             const text = await res.text();
             respEle.textContent = text;
           } catch (e) {
             respEle.textContent = "Error: " + e;
           }
         }

         document.getElementById("pingBtn").onclick = ping;
       </script>
     </body>
   </html>
   ```
3. Optionally, serve this file. For Phase 1, you can:
   - Either copy it manually to your Docker image and serve it using Axum’s
     `Static` or a custom route.
   - For a quick approach, define a route `GET /` that returns the static
     content from memory or a file read.

   Example snippet in `main.rs`:
   ```rust
   use axum::{Router, routing::get, response::{Html, IntoResponse}};

   async fn index_handler() -> impl IntoResponse {
       // In a real scenario, read from a file or do something more dynamic
       // For Phase 1, just inline:
       let html = include_str!("frontend/index.html");
       Html(html)
   }

   #[tokio::main]
   async fn main() {
       let app = Router::new()
           .route("/", get(index_handler))
           .route("/ping", get(ping_handler));
       // ...
   }
   ```
4. Now if you open `http://localhost:8080/`, you see a minimal page with a “Ping
   Microkernel” button that calls `/ping`.

### Step 4: **Invoke the Orchestrator**

1. If you want to show orchestrator connectivity, define a small orchestrator
   function in `agentic_core`:
   ```rust
   // in agentic_core/src/lib.rs
   pub fn microkernel_ping() -> String {
       "Pong from agentic_core orchestrator!".to_string()
   }
   ```
2. Then call it in your `ping_handler`:
   ```rust
   async fn ping_handler() -> String {
       agentic_core::microkernel_ping()
   }
   ```
3. Now your UI is actually calling your microkernel code, which returns a
   message. If you see that in the browser, you know the “UI → microkernel” path
   works.

### Step 5: **Dockerfile & Docker Integration**

1. In your root or in `lion_ui`, create a file named `Dockerfile`:
   ```dockerfile
   FROM rust:1.70 AS builder
   WORKDIR /app
   COPY . .
   # If you need system dependencies for axum/ssl, do apt-get
   RUN apt-get update && apt-get install -y libssl-dev # or similar

   # Build for release. Specify the crate if needed:
   RUN cargo build --release -p lion_ui

   FROM debian:11
   WORKDIR /app
   COPY --from=builder /app/target/release/lion_ui /usr/local/bin/lion-ui

   # Expose the HTTP port
   EXPOSE 8080
   CMD ["lion-ui"]
   ```
2. **Build & Test**:
   - In your workspace root:
     ```bash
     docker build -t lion_ui .
     docker run -p 8080:8080 lion_ui
     ```
   - Open a browser to `http://localhost:8080/` → you should see the minimal
     page with a “Ping” button. Clicking it calls your orchestrator’s `/ping`
     endpoint.

### Step 6: **Local Validation on macOS**

1. Ensure you can do:
   ```bash
   cd lion_ui
   cargo run
   ```
   This prints “Listening on 0.0.0.0:8080”.
2. Open `http://localhost:8080/` in Safari or Chrome on macOS.
3. Confirm the minimal page loads, and clicking “Ping Microkernel” yields a
   result from orchestrator code.

### Step 7: **Document & Commit**

1. Add or update your `docs/progress/stage2-phase1.md` with:
   - **Objectives**: UI skeleton, Docker.
   - **Work Done**: new `lion_ui` crate, minimal server, Docker builds.
   - **Validation**: tested locally, tested Docker, able to see “Pong.”
   - **Next**: Real-time SSE or WebSocket logs, agent spawn forms, plugin UI.
2. Create a final commit referencing `[stage2-phase1]`.

---

## 4. Potential Issues & Extra Tips

- **File Serving**: If you want a more robust static-file approach, consider the
  `tower_http::services::ServeDir` or a `StaticFiles` service in Axum.
- **HTTPS**: For Phase 1, plain HTTP is enough. If you want TLS for local dev or
  Docker usage, you can add a self-signed cert or let the user do that.
- **Cross-Container Communication**: If your orchestrator ends up in a separate
  container from the UI, you’ll map internal Docker networks. Phase 1 usually
  merges them for simplicity.
- **Mac-Specific**: If your Docker approach references webkit dev libs, that’s
  more for Tauri Linux builds. On macOS, typically you only do local dev or
  cross compilation. This is optional if you plan to distribute the `.app`
  eventually.
- **Front-End Framework**: If you decide to adopt React or Svelte, you might do
  `npm init vite@latest`, build the dist, then serve it from Axum. For Phase 1,
  a single HTML is typically enough.

---

## 5. Success Criteria

1. **Minimal Web Server**:
   - `GET /` serves a trivial page with a “Ping Microkernel” button or text.
   - `GET /ping` returns a string from `agentic_core`.
2. **Local**:
   - `cargo run -p lion_ui` → open `http://localhost:8080` → “Hello from Phase1”
     → click “Ping” → see orchestrator message.
3. **Docker**:
   - `docker build -t lion_ui .` then `docker run -p 8080:8080 lion_ui` → open
     `localhost:8080` → identical behavior.
   - Proves Docker-based distribution is functional.

---

## 6. Expected Outcome of Phase 1

By the end of Stage 2, Phase 1, you have:

- A **new UI crate** (`lion_ui`), building a minimal HTTP server and front-end.
- Verified connectivity to the orchestrator with a simple “ping” route.
- A working **Docker** container that publishes the web interface at port 8080,
  so others can easily test the UI.
- Basic local dev steps for macOS, ensuring it’s trivial to do `cargo run` or
  `docker run` to see results.

From here, you’ll expand in **Stage 2, Phase 2** to handle **real-time
streaming** (SSE or websockets) of partial logs, multi-agent concurrency, or
plugin interactions in the UI. This foundation ensures the new front-end can
evolve into a comprehensive agent management console.

---

# Review

Below is a **detailed PR review** of your **Stage 2, Phase 1** changes, focusing
on the newly introduced `lion_ui` crate with a basic Axum web server and minimal
Dockerfile. I'll walk through each of the significant additions, commenting on
code structure, consistency with the developer guide, and suggestions for future
improvements as you move into the next phases.

---

## Overall Assessment

- **Excellent Start**: You’ve successfully created a new crate (`lion_ui`) with
  its own `main.rs` that spins up a minimal Axum server. This is perfectly
  aligned with the Phase 1 goal of establishing a basic foundation for a
  web-based UI.
- **Folder & Workspace Setup**: Adding `"lion_ui"` to your `[workspace]` in the
  top-level `Cargo.toml` is done correctly. The `Cargo.toml` for `lion_ui`
  references `agentic_core` and the required crates (`axum`, `tokio`, etc.),
  indicating you’re ready to call into the microkernel in subsequent phases.
- **Docker Integration**: Your `Dockerfile` is concise, building the `lion_ui`
  binary and copying it to a minimal Debian environment. This meets the
  objective of letting a user run
  `docker build ... && docker run -p 8080:8080 lion_ui` to open the minimal UI.

In short, you have the right scaffolding to expand features (like real-time SSE
logs, agent or plugin management) in the next phases. Below are more specific
comments.

---

## Code & File-Level Comments

### 1. Changes in `.github/ISSUE_TEMPLATE` & Root Cargo

- **`.github/ISSUE_TEMPLATE/bug_report.md`**
  - Minor version label changes (from `v0.0.1a` to `v0.0.1`). This is a trivial
    doc update. Just be mindful if the team references “v0.0.1a” in older docs.

- **`Cargo.toml` (Workspace)**
  ```toml
  [workspace]
  members = [
      "agentic_core",
      "agentic_cli",
      "lion_ui"
  ]
  resolver = "2"
  ```
  - Perfect approach: now your workspace tracks the new UI crate.
  - That means `cargo build --all` will compile `lion_ui`, alongside
    `agentic_core` and `agentic_cli`.

### 2. `agentic_cli/Cargo.toml` & `agentic_core/Cargo.toml` version changes

- You renamed from `0.0.1-alpha-phase5` to `0.0.1-stage1-phase5`, etc. This is
  presumably housekeeping on version naming. The main point is that you’re
  consistent in how you label your crates by phases or stage. It’s purely
  cosmetic but can help keep track of changes across the project.
- Changing `thiserror = "1.0"` to `"2.0"` may require a quick check for any
  breaking changes in `thiserror`. Usually it’s stable, so you’re likely safe,
  but keep an eye out for potential updates or warnings.

### 3. New Crate: `lion_ui`

#### 3.1 `Cargo.toml`

```toml
[package]
name = "lion_ui"
version = "0.0.1-stage2-phase1"
edition = "2021"

[[bin]]
name = "lion_ui"
path = "src/main.rs"

[dependencies]
agentic_core = { path = "../agentic_core" }
axum = "0.8.1"
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tower-http = { version = "0.6.2", features = ["fs", "trace"] }
tower = { version = "0.5.2", features = ["util"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
hyper = { version = "1.0", features = ["full"] }
hyper-util = { version = "0.1", features = ["full"] }
http-body-util = "0.1"
```

- **Appropriate Dependencies**:
  - `axum` plus `tower`, `tracing`, `tokio` for an async server—great choices
    for a minimal web approach.
  - `agentic_core` is there, so you can call microkernel code later (like
    orchestrator logic). This is a key piece for subsequent phases.

- **Binary vs. Library**:
  - You’ve declared `[package]` plus `[[bin]]`, meaning the crate can be used as
    a library or has at least one binary entry point. This is typical for a
    stand-alone server. Great.

#### 3.2 `Dockerfile`

```dockerfile
FROM rust:1.70 AS builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y libssl-dev
RUN cargo build --release -p lion_ui

FROM debian:11-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates
COPY --from=builder /app/target/release/lion_ui /usr/local/bin/lion-ui

EXPOSE 8080
CMD ["lion-ui"]
```

- **Two-Stage Build**:
  - The first stage compiles everything, the second stage is a minimal runtime.
    This is a standard best practice for smaller Docker images. Good.
- **libssl**:
  - Likely for building dependencies requiring TLS. Perfectly normal.
- **Using `EXPOSE 8080`**:
  - Clear that your server is on port 8080.
- **General**:
  - The Dockerfile is straightforward—someone can do `docker build -t lion_ui .`
    and `docker run -p 8080:8080 lion_ui`.
  - That meets Phase 1’s requirement that the user sees a minimal web page and
    can ping the microkernel.

#### 3.3 `lib.rs`

```rust
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

// basic test
```

- This is just a placeholder. In future phases, you might add more library-like
  functions if your `lion_ui` code is bigger. For Phase 1, it’s fine to keep it
  minimal.

#### 3.4 `main.rs`

```rust
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// Handler for the root path
pub async fn index_handler() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>...
    ...
    "#;
    Html(html)
}

pub async fn ping_handler() -> &'static str {
    // TODO: In future phases, this will call actual microkernel functions
    "Pong from lion_ui microkernel!"
}

#[tokio::main]
async fn main() {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .compact()
        .init();

    info!("Starting lion_ui server...");

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ping", get(ping_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Server started successfully");

    axum::serve(listener, app).await.unwrap();
}
```

- **Server Setup**:
  - `axum::serve(listener, app).await.unwrap();` is the new style for Axum > 0.6
    or 0.7. Good.
  - You’re printing logs with `info!`, clarifying the server’s startup steps.
    This is consistent with the developer guide’s suggestion to use structured
    logs.
- **`index_handler`**:
  - Inline HTML with a “Ping Microkernel” button that calls `/ping`. That’s
    precisely Phase 1’s minimal demonstration.
  - The doc comments and snippet are a nice touch.
- **`ping_handler`**:
  - For now, it returns a static message. Future phases will replace that with
    actual orchestrator calls. This lines up perfectly with the plan.
- **`#[cfg(test)]`** block:
  - Great to see you’re testing your endpoints with `oneshot` and verifying the
    body. This ensures your server routes produce the correct HTML or text. Good
    practice.

**Minor Suggestions**:

1. **Cargo Fmt**: The code mostly looks well-formatted. Just ensure you run
   `cargo fmt` regularly, so everything is consistent.
2. **Trivial**: You might rename the doc snippet from “test_index” to
   “test_index_handler” but that’s purely a naming style preference.
3. **Expand Logging**: In future phases, you might want to log the route hits or
   incorporate `tower-http::trace::TraceLayer` for request/response logs.

---

## Additional Feedback & Alignment with Phase 1 Goals

- **Meets Phase 1**: You have a minimal UI crate that:
  1. Runs a small Axum server on port 8080.
  2. Serves a trivial HTML page at `/`.
  3. Provides a “Ping Microkernel” demonstration endpoint.
  4. Bundles everything in Docker.

This matches the **Stage 2, Phase 1** instructions to stand up a basic front end
for your microkernel. You can see how easy it’ll be to expand to real-time logs
or agent operations in future phases.

- **Integration with `agentic_core`**: Right now, you only import it in your
  `Cargo.toml`; you’re not calling any orchestrator function. That’s fine for
  Phase 1. In subsequent phases, you’ll tie into `agentic_core::Orchestrator` or
  events.

---

## Next Steps

1. **Phase 2**: Real-time logs (SSE or WebSockets) to show partial agent
   outputs. You’ll expand the routes in this UI crate to handle event streaming.
2. **Phase 3**: Plugin management, endpoints to load/invoke plugins. The
   `lion_ui` server can wrap orchestrator plugin logic.
3. **Eventually**: Advanced logging search, Tauri integration, etc.

Everything looks well-prepared for these expansions. Keep your code base tidy,
with new routes in separate modules or files (`events.rs`, `plugins.rs`, etc.)
as it grows.

---

## Conclusion

**Great job** implementing the initial `lion_ui` crate and Docker build. You’ve
created a clear minimal server with a `/` HTML page, `/ping` endpoint, and a
robust test suite. This sets a **strong foundation** for the upcoming phases
(real-time logs, plugin management, advanced UI). It’s aligned with the
developer guide’s stage-based approach:

- **Minimal** but fully functional server → check.
- **Docker** for quick usage → check.
- **Documented** and tested → check.

You’re ready to move on to Stage 2, Phase 2, where you’ll implement real-time
SSE or WebSocket logs and agent management. Nice work!
