use agentic_core::{
    orchestrator::{
        events::{PluginEvent, SystemEvent},
        metadata::EventMetadata,
        Orchestrator,
    },
    plugin_manager::{PluginManager, PluginManifest, PluginState},
};
use std::sync::Arc;
use tempfile::tempdir;
use tokio;
use uuid::Uuid;

#[tokio::test]
async fn test_plugin_lifecycle() {
    // Create temporary directories for testing
    let plugin_dir = tempdir().unwrap();
    let data_dir = tempdir().unwrap();

    // Initialize plugin manager
    let manager = Arc::new(PluginManager::with_manifest_dir(plugin_dir.path()));

    // Create test manifest
    let manifest = PluginManifest::new(
        "test-plugin".to_string(),
        "1.0.0".to_string(),
        "A test plugin for integration testing".to_string(),
    );

    // Create test WASM file
    let wasm_path = plugin_dir.path().join("test.wasm");
    std::fs::write(&wasm_path, b"mock wasm content").unwrap();

    // Register plugin
    let plugin_id = manager
        .load_plugin_from_string(
            toml::to_string(&manifest).unwrap(),
            Some(wasm_path.to_string_lossy().into_owned()),
        )
        .await
        .unwrap();

    // Verify plugin registration
    let metadata = manager.get_plugin(plugin_id).unwrap();
    assert_eq!(metadata.manifest.name, "test-plugin");
    assert_eq!(metadata.manifest.version, "1.0.0");
    match metadata.state {
        PluginState::Ready => (),
        _ => panic!("Plugin should be in Ready state"),
    }

    // Test plugin invocation
    let result = manager
        .invoke_plugin(plugin_id, "test input")
        .await
        .unwrap();
    assert!(result.contains("test-plugin"));
    assert!(result.contains("test input"));

    // Test plugin listing
    let plugins = manager.list_plugins();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].id, plugin_id);

    // Test plugin removal
    manager.remove_plugin(plugin_id).await.unwrap();
    assert!(manager.get_plugin(plugin_id).is_err());
}

#[tokio::test]
async fn test_plugin_orchestration() {
    // Create an orchestrator
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn the orchestrator
    tokio::spawn(orchestrator.run());

    // Create a test manifest
    let manifest = PluginManifest::new(
        "test-plugin".to_string(),
        "1.0.0".to_string(),
        "A test plugin".to_string(),
    );

    // Send plugin load event
    let plugin_id = Uuid::new_v4();
    let load_event = SystemEvent::Plugin(PluginEvent::Load {
        plugin_id,
        manifest: manifest.clone(),
        manifest_path: None,
        metadata: EventMetadata::new(None),
    });

    sender.send(load_event).await.unwrap();

    // Wait for completion
    if let Ok(SystemEvent::Plugin(PluginEvent::Result {
        plugin_id: completed_id,
        ..
    })) = completion_rx.recv().await
    {
        assert_eq!(completed_id, plugin_id);
    } else {
        panic!("Expected plugin load completion");
    }

    // Send plugin invocation event
    let invoke_event = SystemEvent::Plugin(PluginEvent::Invoked {
        plugin_id,
        input: "test input".to_string(),
        metadata: EventMetadata::new(None),
    });

    sender.send(invoke_event).await.unwrap();

    // Wait for completion
    if let Ok(SystemEvent::Plugin(PluginEvent::Result {
        plugin_id: completed_id,
        ..
    })) = completion_rx.recv().await
    {
        assert_eq!(completed_id, plugin_id);
    } else {
        panic!("Expected plugin invocation completion");
    }
}

#[tokio::test]
async fn test_plugin_error_handling() {
    let manager = Arc::new(PluginManager::new());

    // Test invalid manifest
    let result = manager
        .load_plugin_from_string(
            r#"
            name = ""  # Empty name should fail validation
            version = "1.0.0"
            description = "Invalid plugin"
            "#
            .to_string(),
            None,
        )
        .await;
    assert!(result.is_err());

    // Test non-existent plugin
    let non_existent_id = Uuid::new_v4();
    assert!(manager.get_plugin(non_existent_id).is_err());
    assert!(manager
        .invoke_plugin(non_existent_id, "test")
        .await
        .is_err());
    assert!(manager.remove_plugin(non_existent_id).await.is_err());
}

#[tokio::test]
async fn test_plugin_concurrent_operations() {
    let manager = Arc::new(PluginManager::new());
    let manifest = PluginManifest::new(
        "test-plugin".to_string(),
        "1.0.0".to_string(),
        "Test plugin".to_string(),
    );

    // Load plugin
    let plugin_id = manager
        .load_plugin_from_string(toml::to_string(&manifest).unwrap(), None)
        .await
        .unwrap();

    // Spawn multiple concurrent invocations
    let mut handles = Vec::new();
    for i in 0..10 {
        let manager = Arc::clone(&manager);
        let plugin_id = plugin_id;
        handles.push(tokio::spawn(async move {
            let input = format!("input {}", i);
            manager.invoke_plugin(plugin_id, &input).await.unwrap();
        }));
    }

    // Wait for all invocations to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Plugin should still be accessible
    let metadata = manager.get_plugin(plugin_id).unwrap();
    assert_eq!(metadata.manifest.name, "test-plugin");
}
