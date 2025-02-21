use super::core::PluginManager;
use super::manifest::PluginManifest;
use super::test_utils::init_test_logging;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn test_discover_plugins() {
    init_test_logging();
    let manager = PluginManager::new();
    let discovered = manager.discover_plugins().unwrap();

    // Should find at least the calculator plugin
    assert!(!discovered.is_empty(), "No plugins discovered");
    let calculator = discovered
        .iter()
        .find(|m| m.name == "calculator")
        .expect("Calculator plugin not found");

    assert_eq!(calculator.name, "calculator");
    assert_eq!(calculator.version, "0.1.0");
    assert_eq!(calculator.entry_point, "../target/debug/calculator_plugin");
    assert!(calculator.functions.contains_key("add"));
    assert!(calculator.functions.contains_key("subtract"));
    assert!(calculator.functions.contains_key("multiply"));
    assert!(calculator.functions.contains_key("divide"));
}

#[test]
fn test_load_plugin_nonexistent() {
    init_test_logging();
    let mut manager = PluginManager::new();

    // Create a manifest for a non-existent plugin
    let manifest = PluginManifest {
        name: "nonexistent".to_string(),
        version: "0.1.0".to_string(),
        description: "Test plugin".to_string(),
        entry_point: "nonexistent".to_string(),
        permissions: vec![],
        driver: None,
        functions: HashMap::new(),
    };

    let result = manager.load_plugin(manifest);
    assert!(result.is_err());
}

#[test]
fn test_plugin_not_found() {
    init_test_logging();
    let manager = PluginManager::new();
    let id = Uuid::new_v4();
    let result = manager.invoke_plugin(id, "test");
    assert!(matches!(
        result,
        Err(super::error::PluginError::NotFound(_))
    ));
}

#[test]
fn test_list_empty_plugins() {
    init_test_logging();
    let manager = PluginManager::new();
    manager.clear(); // Clear any existing plugins
    assert!(manager.list_plugins().is_empty());
}

#[test]
fn test_load_calculator_plugin() {
    init_test_logging();
    let mut manager = PluginManager::new();
    let manifests = manager.discover_plugins().unwrap();
    let calculator = manifests
        .iter()
        .find(|m| m.name == "calculator")
        .expect("Calculator plugin not found");

    let result = manager.load_plugin(calculator.clone());
    assert!(
        result.is_ok(),
        "Failed to load calculator plugin: {:?}",
        result
    );

    // Verify the plugin was loaded correctly
    let plugins = manager.list_plugins();
    assert_eq!(plugins.len(), 1, "Should have one loaded plugin");
    assert_eq!(plugins[0].1.name, "calculator");
    assert!(plugins[0].1.functions.contains_key("add"));
}
