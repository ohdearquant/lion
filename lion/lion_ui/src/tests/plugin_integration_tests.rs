use crate::{events::AppState, plugins::PluginInfo};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use lion_core::{Orchestrator, SystemEvent, plugin_manager::PluginManager};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceExt;
use tracing::debug;

async fn setup_test_app() -> (Router, broadcast::Receiver<String>) {
    // Initialize plugin manager with plugins directory
    let plugin_manager = PluginManager::with_manifest_dir("plugins");
    
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
        .route(
            "/api/plugins",
            axum::routing::post(crate::plugins::load_plugin_handler)
                .get(crate::plugins::list_plugins_handler),
        )
        .route(
            "/api/plugins/{plugin_id}/invoke",
            axum::routing::post(crate::plugins::invoke_plugin_handler),
        )
        .with_state(state);

    // Give orchestrator time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    (app, logs_rx)
}

#[tokio::test]
async fn test_plugin_lifecycle() {
    let (app, mut logs_rx) = setup_test_app().await;

    // Create test manifest
    let manifest = r#"name = "test_plugin"
version = "0.1.0"
entry_point = "test.wasm"
permissions = ["test"]
"#;

    // 1. Load plugin
    let load_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plugins")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "manifest": manifest
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(load_response.status(), StatusCode::OK);

    // Get plugin ID from response
    let body = String::from_utf8(
        load_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    let plugin_info: PluginInfo = serde_json::from_str(&body).unwrap();

    // 2. List plugins
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
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].id, plugin_info.id);

    // 3. Invoke plugin
    let invoke_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/plugins/{}/invoke", plugin_info.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "input": "test input"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(invoke_response.status(), StatusCode::OK);

    // 4. Verify events in logs
    let mut saw_invocation = false;

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    while let Ok(log) = logs_rx.try_recv() {
        if log.contains("invoked: test input") {
            saw_invocation = true;
        }
    }

    assert!(saw_invocation, "Should see plugin invocation in logs");
}

#[tokio::test]
async fn test_plugin_error_handling() {
    let (app, mut logs_rx) = setup_test_app().await;

    // Try to load invalid manifest
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plugins")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "manifest": "invalid = toml [ content"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

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
    assert!(
        body.contains("Invalid manifest format"),
        "Should fail with invalid manifest error"
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Should see error in logs
    let mut saw_error = false;
    while let Ok(log) = logs_rx.try_recv() {
        println!("Checking log: {}", log);
        if log.contains("Invalid manifest format") {
            saw_error = true;
            break;
        }
    }

    assert!(saw_error, "Should see error in logs for invalid manifest");
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
                .uri(&format!("/api/plugins/{}/invoke", calculator.id))
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
                .uri(&format!("/api/plugins/{}/invoke", calculator.id))
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
