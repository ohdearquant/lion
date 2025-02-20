mod agents;
mod events;
mod plugins;

use crate::{
    agents::{list_agents, spawn_agent},
    events::sse_handler,
    plugins::{invoke_plugin_handler, list_plugins_handler, load_plugin_handler},
};
use agentic_core::{Orchestrator, SystemEvent};
use axum::{
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use events::AppState;
use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::TraceLayer;
use tracing::{debug, info, Level};
use tracing_subscriber::FmtSubscriber;

pub async fn index_handler() -> impl IntoResponse {
    debug!("Serving index.html");
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <title>lion UI</title>
    <style>
        #logs {
            height: 300px;
            overflow-y: auto;
            border: 1px solid #ccc;
            padding: 10px;
            margin: 10px 0;
            font-family: monospace;
            white-space: pre-wrap;
        }
        .agent-list, .plugin-list {
            margin: 20px 0;
        }
        .agent-item, .plugin-item {
            margin: 5px 0;
            padding: 5px;
            border: 1px solid #eee;
        }
        .section {
            margin-bottom: 30px;
            padding: 15px;
            background: #f9f9f9;
            border-radius: 5px;
        }
        textarea {
            width: 100%;
            min-height: 100px;
            margin: 10px 0;
            font-family: monospace;
        }
    </style>
</head>
<body>
    <h1>lion UI - System Management</h1>
    
    <div class="section">
        <h2>Agents</h2>
        <div>
            <input type="text" id="promptInput" placeholder="Enter agent prompt" />
            <button id="spawnBtn">Spawn Agent</button>
        </div>
        <div class="agent-list">
            <h3>Active Agents</h3>
            <div id="agentList"></div>
        </div>
    </div>

    <div class="section">
        <h2>Available Plugins</h2>
        <div class="plugin-list">
            <div id="pluginList"></div>
        </div>
    </div>

    <div class="section">
        <h2>Real-time Logs</h2>
        <div id="logs"></div>
    </div>

    <script>
        // Set up SSE for real-time logs
        const evtSource = new EventSource("/events");
        const logsDiv = document.getElementById("logs");
        
        evtSource.onmessage = (event) => {
            const newLog = document.createElement("div");
            newLog.textContent = event.data;
            logsDiv.appendChild(newLog);
            logsDiv.scrollTop = logsDiv.scrollHeight;
        };

        // Function to spawn a new agent
        async function spawnAgent() {
            const promptInput = document.getElementById("promptInput");
            const prompt = promptInput.value.trim();
            
            if (!prompt) {
                alert("Please enter a prompt");
                return;
            }

            try {
                const res = await fetch("/api/agents", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ prompt })
                });
                
                const data = await res.json();
                if (data.error) {
                    alert(data.error);
                } else {
                    promptInput.value = "";
                    fetchAgents(); // Refresh agent list
                }
            } catch (e) {
                alert("Error spawning agent: " + e);
            }
        }

        // Function to fetch and display active agents
        async function fetchAgents() {
            try {
                const res = await fetch("/api/agents");
                const agents = await res.json();
                
                const agentList = document.getElementById("agentList");
                agentList.innerHTML = agents.map(agent => `
                    <div class="agent-item">
                        Agent ${agent.id} - Status: ${agent.status}
                    </div>
                `).join("");
            } catch (e) {
                console.error("Error fetching agents:", e);
            }
        }

        // Plugin Management Functions
        async function fetchPlugins() {
            try {
                const res = await fetch("/api/plugins");
                const plugins = await res.json();
                
                const pluginList = document.getElementById("pluginList");
                pluginList.innerHTML = plugins.map(plugin => {
                    // Sort functions by name for consistent order
                    const sortedFunctions = Object.entries(plugin.functions)
                        .sort(([a], [b]) => a.localeCompare(b))
                        .map(([name, description]) => `
                            <div class="function-item">
                                <button onclick="invokePlugin('${plugin.id}', '${name}')">${name}</button>
                                <span class="function-description">${description}</span>
                            </div>
                        `).join("");

                    return `
                        <div class="plugin-item">
                            <h3>${plugin.name} v${plugin.version}</h3>
                            <p class="plugin-description">${plugin.description}</p>
                            <div class="plugin-functions">
                                <h4>Available Functions:</h4>
                                ${sortedFunctions}
                            </div>
                        </div>
                    `;
                }).join("");
            } catch (e) {
                console.error("Error fetching plugins:", e);
            }
        }

        async function invokePlugin(pluginId, functionName) {
            // Get function arguments based on the plugin and function
            let args = {};
            try {
                const res = await fetch("/api/plugins");
                const plugins = await res.json();
                const plugin = plugins.find(p => p.id === pluginId);
                
                if (!plugin) {
                    alert("Plugin not found");
                    return;
                }

                // Create a form for the function's arguments
                const argsInput = prompt(`Enter arguments for ${functionName} (as JSON):`);
                if (argsInput === null) return; // User cancelled

                try {
                    args = JSON.parse(argsInput);
                } catch (e) {
                    alert("Invalid JSON format");
                    return;
                }

                const res2 = await fetch(`/api/plugins/${pluginId}/invoke`, {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({
                        function: functionName,
                        args: args
                    })
                });
                
                const data = await res2.json();
                if (data.status === "error") {
                    alert(data.message);
                }
            } catch (e) {
                alert("Error invoking plugin: " + e);
            }
        }

        // Add some additional styles
        const style = document.createElement('style');
        style.textContent = `
            .plugin-item {
                background: white;
                padding: 15px;
                margin-bottom: 20px;
                border-radius: 5px;
                box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            }
            .plugin-description {
                color: #666;
                margin: 10px 0;
            }
            .plugin-functions {
                margin-top: 15px;
            }
            .function-item {
                display: flex;
                align-items: center;
                margin: 5px 0;
                gap: 10px;
            }
            .function-item button {
                min-width: 100px;
            }
            .function-description {
                color: #666;
                font-size: 0.9em;
            }
        `;
        document.head.appendChild(style);

        // Set up event listeners
        document.getElementById("spawnBtn").onclick = spawnAgent;
        
        // Initial data fetch
        fetchAgents();
        fetchPlugins();
        
        // Refresh lists periodically
        setInterval(fetchAgents, 5000);
        setInterval(fetchPlugins, 5000);
    </script>
</body>
</html>"#;
    Html(html)
}

pub async fn ping_handler() -> &'static str {
    debug!("Handling ping request");
    "Pong from lion_ui microkernel!"
}

#[tokio::main]
async fn main() {
    // Initialize logging with more verbose output
    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting lion_ui server...");

    // Initialize orchestrator
    let orchestrator = Orchestrator::new(100);
    let orchestrator_sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator in the background
    tokio::spawn(orchestrator.run());

    // Create shared state
    let state = Arc::new(AppState::new(orchestrator_sender, 100));
    let state_clone1 = Arc::clone(&state);
    let state_clone2 = Arc::clone(&state);

    // Spawn a task to forward orchestrator completion events to UI
    tokio::spawn(async move {
        while let Ok(event) = completion_rx.recv().await {
            match &event {
                SystemEvent::AgentPartialOutput {
                    agent_id, chunk, ..
                } => {
                    let _ = state_clone1
                        .logs_tx
                        .send(format!("Agent {}: {}", agent_id, chunk));
                }
                SystemEvent::AgentCompleted {
                    agent_id, result, ..
                } => {
                    let _ = state_clone1
                        .logs_tx
                        .send(format!("Agent {} completed: {}", agent_id, result));
                    let mut agents = state_clone1.agents.write().await;
                    agents.insert(*agent_id, "completed".to_string());
                }
                SystemEvent::AgentError {
                    agent_id, error, ..
                } => {
                    let _ = state_clone1
                        .logs_tx
                        .send(format!("Agent {} error: {}", agent_id, error));
                    let mut agents = state_clone1.agents.write().await;
                    agents.insert(*agent_id, "error".to_string());
                }
                SystemEvent::PluginInvoked {
                    plugin_id, input, ..
                } => {
                    let _ = state_clone1.logs_tx.send(format!(
                        "Plugin {} invoked with input: {}",
                        plugin_id, input
                    ));
                }
                SystemEvent::PluginResult {
                    plugin_id, output, ..
                } => {
                    let _ = state_clone1
                        .logs_tx
                        .send(format!("Plugin {} result: {}", plugin_id, output));
                }
                SystemEvent::PluginError {
                    plugin_id, error, ..
                } => {
                    let _ = state_clone1
                        .logs_tx
                        .send(format!("Plugin {} error: {}", plugin_id, error));
                }
                _ => {}
            }
        }
    });

    // Build our application router with request tracing
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/events", get(sse_handler))
        .route("/api/agents", post(spawn_agent).get(list_agents))
        .route(
            "/api/plugins",
            post(load_plugin_handler).get(list_plugins_handler),
        )
        .route(
            "/api/plugins/{plugin_id}/invoke",
            post(invoke_plugin_handler),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Run it on localhost:8080
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Server started successfully");

    // Send a test message to verify SSE
    let _ = state_clone2
        .logs_tx
        .send("Server initialized and ready".to_string());

    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_ping_endpoint() {
        let app = Router::new().route("/ping", get(ping_handler));

        let response = app
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();

        assert_eq!(body, "Pong from lion_ui microkernel!");
    }

    #[tokio::test]
    async fn test_index_endpoint() {
        let app = Router::new().route("/", get(index_handler));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();

        // Check that the response contains our expected HTML elements
        assert!(body.contains("lion UI - System Management"));
        assert!(body.contains("Plugins"));
        assert!(body.contains("Real-time Logs"));
    }
}
