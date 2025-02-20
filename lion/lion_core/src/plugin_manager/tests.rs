use super::*;
use tempfile::tempdir;
use tokio;

#[tokio::test]
async fn test_plugin_lifecycle() {
    // Create a temporary directory for testing
    let temp_dir = tempdir().unwrap();
    let manifest_dir = temp_dir.path();

    // Create a plugin manager
    let manager = PluginManager::with_manifest_dir(manifest_dir);

    // Create a test manifest
    let manifest = toml::toml! {
        name = "test-plugin"
        version = "1.0.0"
        description = "A test plugin for integration testing"
        wasm_path = "test.wasm"
        config = {
            setting1 = "value1"
            setting2 = 42
        }
        capabilities = [
            "network",
            "storage"
        ]
    };

    // Load the plugin
    let plugin_id = manager
        .load_plugin_from_string(manifest.to_string(), Some("test.toml".into()))
        .await
        .unwrap();

    // Verify plugin was loaded
    let metadata = manager.get_plugin(plugin_id).unwrap();
    assert_eq!(metadata.manifest.name, "test-plugin");
    assert_eq!(metadata.manifest.version, "1.0.0");
    assert!(matches!(metadata.state, PluginState::Ready));

    // List plugins
    let plugins = manager.list_plugins();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].id, plugin_id);

    // Invoke plugin
    let result = manager.invoke_plugin(plugin_id, "test input").await.unwrap();
    assert!(result.contains("test-plugin"));
    assert!(result.contains("test input"));

    // Remove plugin
    manager.remove_plugin(plugin_id).await.unwrap();
    assert!(manager.get_plugin(plugin_id).is_err());
    assert_eq!(manager.list_plugins().len(), 0);
}

#[tokio::test]
async fn test_invalid_plugin_operations() {
    let manager = PluginManager::new();

    // Try to load invalid manifest
    let invalid_manifest = toml::toml! {
        name = ""  // Empty name should fail validation
        version = "1.0.0"
        description = "Invalid plugin"
    };

    let result = manager
        .load_plugin_from_string(invalid_manifest.to_string(), None)
        .await;
    assert!(matches!(result, Err(PluginError::InvalidManifest(_))));

    // Try to get non-existent plugin
    let non_existent_id = Uuid::new_v4();
    assert!(matches!(
        manager.get_plugin(non_existent_id),
        Err(PluginError::NotFound(_))
    ));

    // Try to invoke non-existent plugin
    let result = manager.invoke_plugin(non_existent_id, "test").await;
    assert!(matches!(result, Err(PluginError::NotFound(_))));

    // Try to remove non-existent plugin
    let result = manager.remove_plugin(non_existent_id).await;
    assert!(matches!(result, Err(PluginError::NotFound(_))));
}

#[tokio::test]
async fn test_plugin_with_dependencies() {
    let manager = PluginManager::new();

    // Create a plugin with dependencies
    let manifest = toml::toml! {
        name = "dependent-plugin"
        version = "1.0.0"
        description = "A plugin with dependencies"
        dependencies = [
            { name = "base-plugin", version_req = "^1.0.0" }
        ]
        capabilities = ["storage"]
    };

    // Load should succeed even with missing dependencies (for now)
    let plugin_id = manager
        .load_plugin_from_string(manifest.to_string(), None)
        .await
        .unwrap();

    let metadata = manager.get_plugin(plugin_id).unwrap();
    assert_eq!(metadata.manifest.name, "dependent-plugin");
    assert!(matches!(metadata.state, PluginState::Ready));
}

#[tokio::test]
async fn test_plugin_manifest_validation() {
    let manager = PluginManager::new();

    // Test various invalid manifests
    let test_cases = vec![
        (
            toml::toml! {
                name = ""
                version = "1.0.0"
                description = "Empty name"
            },
            "empty name",
        ),
        (
            toml::toml! {
                name = "test"
                version = ""
                description = "Empty version"
            },
            "empty version",
        ),
        (
            toml::toml! {
                name = "test"
                version = "1.0.0"
                description = ""
            },
            "empty description",
        ),
    ];

    for (manifest, case) in test_cases {
        let result = manager
            .load_plugin_from_string(manifest.to_string(), None)
            .await;
        assert!(
            matches!(result, Err(PluginError::InvalidManifest(_))),
            "Failed to catch {}",
            case
        );
    }
}