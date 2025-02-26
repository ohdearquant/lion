//! Integration tests for lion_isolation.

use lion_isolation::*;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// Initialize tracing for tests
fn init_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

#[test]
fn test_isolation_workflow() {
    // Initialize tracing
    init_tracing();

    // Create a simple WebAssembly module
    const WASM: &[u8] = include_bytes!("testdata/simple.wasm");

    // Create a WebAssembly engine
    let engine = Arc::new(WasmEngine::default().unwrap());

    // Create a resource limiter
    let resource_limiter = Arc::new(DefaultResourceLimiter::default());

    // Create an isolation manager
    let mut manager =
        IsolationManager::with_default_backend(engine.clone(), resource_limiter.clone()).unwrap();

    // Get the backend to set capabilities
    let backend = manager.backend_mut();

    // Create a capability checker
    let capability_checker = Arc::new(interface::DefaultCapabilityChecker::new());

    // Grant some capabilities
    capability_checker.grant_capability("test-plugin", "read_file");
    capability_checker.grant_capability("test-plugin", "log");

    // Set the capability checker
    backend.set_capability_checker(Box::new(capability_checker.clone()));

    // Load the plugin
    let plugin_id = "test-plugin".to_string();
    manager.load_plugin(&plugin_id, WASM).unwrap();

    // Get the plugin state
    let state = manager.get_plugin_state(&plugin_id).unwrap();
    assert_eq!(state, PluginState::Loaded);

    // Call a function
    info!("Calling add function");
    let result = manager.call_function(&plugin_id, "add", &[1, 2]).unwrap();
    assert_eq!(result, vec![3]);

    // Get the plugin state again - should be Running now
    let state = manager.get_plugin_state(&plugin_id).unwrap();
    assert_eq!(state, PluginState::Running);

    // Get the resource usage
    let usage = manager.get_resource_usage(&plugin_id).unwrap();
    info!("Resource usage: {:?}", usage);

    // Unload the plugin
    manager.unload_plugin(&plugin_id).unwrap();

    // Check that the plugin is gone
    assert!(manager.get_plugin_state(&plugin_id).is_err());
}
