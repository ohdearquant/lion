use crate::{events::AppState, plugins::PluginInfo};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use lion_core::{plugin_manager::PluginManager, Orchestrator, SystemEvent};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::broadcast;
use tower::ServiceExt;
use tracing::{debug, info};
use tracing_subscriber::fmt::format::FmtSpan;

fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .try_init();
}

fn get_project_root() -> PathBuf {
    let current_dir = std::env::current_dir().unwrap();
    if current_dir.ends_with("lion_ui") {
        current_dir.join("..").join("..").canonicalize().unwrap()
    } else if current_dir.ends_with("lion") {
        current_dir.canonicalize().unwrap()
    } else {
        current_dir
    }
}

async fn setup_test_app() -> (Router, broadcast::Receiver<String>) {
    init_test_logging();
    info!("Setting up test app");

    // Get the project root and construct the plugins path
    let project_root = get_project_root();
    let plugins_dir = project_root.join("plugins");
    info!("Using plugins directory: {:?}", plugins_dir);

    // Initialize plugin manager with plugins directory
    let mut plugin_manager = PluginManager::with_manifest_dir(&plugins_dir);
    info!("Created plugin manager");

    // Discover and load available plugins
    match plugin_manager.discover_plugins() {
        Ok(manifests) => {
            debug!("Discovered {} plugins", manifests.len());
            for manifest in manifests {
                debug!("Found plugin manifest: {:?}", manifest);
                if let Err(e) = plugin_manager.load_plugin(manifest) {
                    eprintln!("Warning: Failed to load plugin: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to discover plugins: {}", e);
        }
    }

    // Create orchestrator with the plugin manager
    let orchestrator = Orchestrator::with_plugin_manager(100, plugin_manager);
    let orchestrator_sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    let (logs_tx, logs_rx) = broadcast::channel::<String>(100);
    let logs_tx_clone = logs_tx.clone();

    // Spawn orchestrator and ensure it's running
    tokio::spawn(orchestrator.run());

    // Forward completion events to logs
    tokio::spawn(async move {
        while let Ok(event) = completion_rx.recv().await {
            match &event {
                SystemEvent::PluginInvoked {
                    plugin_id,
                    input,
                    metadata: _,
                } => {
                    let _ = logs_tx_clone.send(format!("Plugin {} invoked: {}", plugin_id, input));
                }
                SystemEvent::PluginResult {
                    plugin_id,
                    output,
                    metadata: _,
                } => {
                    let _ = logs_tx_clone.send(format!("Plugin {} result: {}", plugin_id, output));
                }
                SystemEvent::PluginError {
                    plugin_id: _,
                    error,
                    metadata: _,
                } => {
                    // Forward error directly from plugin manager
                    let _ = logs_tx_clone.send(error.to_string());
                }
                _ => {}
            }
        }
    });

    // Create state with the same logs channel
    let state = Arc::new(AppState::new_with_logs(
        orchestrator_sender,
        logs_tx.clone(),
    ));

    let app = Router::new()
        .nest("/api", crate::plugins::create_plugin_router())
        .with_state(state);

    // Give orchestrator time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    (app, logs_rx)
}

#[tokio::test]
async fn test_calculator_plugin() {
    let (app, mut logs_rx) = setup_test_app().await;

    // Wait for plugin discovery and loading
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // List plugins to get calculator plugin ID
    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plugins")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);

    let body = String::from_utf8(
        list_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    let plugins: Vec<PluginInfo> = serde_json::from_str(&body).unwrap();
    debug!("Found plugins: {:?}", plugins);

    // Find calculator plugin
    let calculator = plugins
        .iter()
        .find(|p| p.name == "calculator")
        .expect("Calculator plugin should be discovered");

    // Test addition
    let invoke_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plugins/{}/invoke", calculator.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "function": "add",
                        "args": {
                            "a": 5.0,
                            "b": 3.0
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(invoke_response.status(), StatusCode::OK);

    // Verify result in logs
    let mut saw_result = false;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    while let Ok(log) = logs_rx.try_recv() {
        if log.contains(r#""result":8.0"#) {
            saw_result = true;
            break;
        }
    }

    assert!(saw_result, "Should see correct addition result in logs");

    // Test division by zero error
    let invoke_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plugins/{}/invoke", calculator.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "function": "divide",
                        "args": {
                            "a": 1.0,
                            "b": 0.0
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(invoke_response.status(), StatusCode::OK);

    // Verify error in logs
    let mut saw_error = false;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    while let Ok(log) = logs_rx.try_recv() {
        if log.contains("Division by zero") {
            saw_error = true;
            break;
        }
    }

    assert!(saw_error, "Should see division by zero error in logs");
}
