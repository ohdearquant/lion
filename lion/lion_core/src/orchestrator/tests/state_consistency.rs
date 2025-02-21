use super::*;
use crate::plugin_manager::PluginManifest;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;
use tokio::time::timeout;
use tracing::debug;

#[tokio::test]
async fn test_state_consistency() {
    // Initialize test logging
    init_test_logging();
    debug!("Starting state consistency test");

    // Create temporary directories
    let temp_dir = tempdir().unwrap();
    let plugins_dir = temp_dir.path().join("plugins");
    std::fs::create_dir_all(&plugins_dir).unwrap();

    // Create a mock plugin executable
    let plugin_dir = plugins_dir.join("test_plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    let plugin_path = plugin_dir.join("test_plugin");

    // Write a simple shell script as the plugin
    let plugin_content = r#"#!/bin/sh
echo '{"result": "test output"}'
"#;
    fs::write(&plugin_path, plugin_content).unwrap();

    // Make the plugin executable
    let mut perms = fs::metadata(&plugin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&plugin_path, perms).unwrap();

    // Create a test manifest
    let manifest = PluginManifest {
        name: "test_plugin".to_string(),
        version: "0.1.0".to_string(),
        description: "Test plugin".to_string(),
        entry_point: plugin_path.to_str().unwrap().to_string(),
        permissions: vec![],
        driver: Some("native".to_string()),
        functions: HashMap::new(),
    };

    // Write manifest to file
    let manifest_path = plugin_dir.join("manifest.toml");
    let manifest_content = toml::to_string(&manifest).unwrap();
    std::fs::write(&manifest_path, manifest_content).unwrap();

    // Create orchestrator with the test directory
    let mut orchestrator =
        Orchestrator::with_plugin_manager(100, PluginManager::with_manifest_dir(&plugins_dir));
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Get initial state
    let initial_plugins = orchestrator.plugin_manager().list_plugins();
    assert!(initial_plugins.is_empty(), "Should start with no plugins");

    // Discover and load plugin
    let manifests = orchestrator.plugin_manager().discover_plugins().unwrap();
    assert_eq!(manifests.len(), 1, "Should discover one plugin");

    let plugin_id = orchestrator
        .plugin_manager()
        .load_plugin(manifests[0].clone())
        .unwrap();

    // Verify plugin state
    let loaded_plugins = orchestrator.plugin_manager().list_plugins();
    assert_eq!(loaded_plugins.len(), 1, "Should have one loaded plugin");
    assert_eq!(loaded_plugins[0].0, plugin_id, "Plugin ID should match");

    // Get event log reference before spawning orchestrator
    let event_log = orchestrator.event_log().clone();

    // Start orchestrator
    tokio::spawn(orchestrator.run());

    // Send plugin invocation
    let event = SystemEvent::new_plugin_invocation(
        plugin_id,
        serde_json::json!({"test": "data"}).to_string(),
        None,
    );
    sender.send(event.clone()).await.unwrap();

    // Wait for completion
    let _ = timeout(std::time::Duration::from_secs(1), completion_rx.recv())
        .await
        .expect("Timeout waiting for completion")
        .expect("Channel closed");

    // Give time for events to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify event log consistency
    let records = event_log.all();
    assert_eq!(
        records.len(),
        2,
        "Should have invocation and completion events"
    );

    // Verify event sequence
    assert!(
        matches!(records[0].event, SystemEvent::PluginInvoked { .. }),
        "First event should be PluginInvoked"
    );
    match &records[1].event {
        SystemEvent::PluginResult {
            plugin_id: completed_id,
            output,
            ..
        } => {
            assert_eq!(
                *completed_id, plugin_id,
                "Plugin ID should match in completion"
            );
            assert!(
                output.contains("test output"),
                "Output should contain expected result"
            );
        }
        _ => panic!("Second event should be PluginResult"),
    }

    debug!("State consistency test completed successfully");
}
