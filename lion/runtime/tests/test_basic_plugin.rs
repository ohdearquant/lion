use std::path::PathBuf;
use core::error::Result;
use runtime::{Runtime, RuntimeConfig};

#[test]
fn test_basic_plugin_load_and_call() -> Result<()> {
    // Create a temporary directory for testing
    let temp_dir = tempfile::tempdir()?;
    let config_path = temp_dir.path().join("lion.toml");
    
    // Create a basic configuration
    let config = RuntimeConfig {
        max_memory_bytes: 10 * 1024 * 1024, // 10 MB
        ..Default::default()
    };
    
    // Save the configuration
    runtime::save_config(&config, &config_path)?;
    
    // Load the configuration
    let config = runtime::load_config(&config_path)?;
    
    // Create a runtime
    let runtime = Runtime::new(config)?;
    
    // Create a simple WebAssembly module
    let wasm_bytes = create_test_wasm()?;
    let wasm_path = temp_dir.path().join("test.wasm");
    std::fs::write(&wasm_path, &wasm_bytes)?;
    
    // Load the plugin
    let plugin_id = runtime.load_plugin_from_file(
        &wasm_path,
        "test-plugin",
        "1.0.0",
        "Test plugin for basic functionality",
    )?;
    
    // Check that the plugin was loaded
    let metadata = runtime.plugin_manager().get_metadata(&plugin_id)
        .expect("Plugin should be loaded");
    
    assert_eq!(metadata.name, "test-plugin");
    assert_eq!(metadata.version, "1.0.0");
    
    // Call a function
    let params = serde_json::json!({
        "name": "world"
    }).to_string().into_bytes();
    
    let result = runtime.plugin_manager().call_function(
        &plugin_id,
        "hello",
        &params,
    )?;
    
    // Parse the result
    let result_str = String::from_utf8(result)?;
    let result_json: serde_json::Value = serde_json::from_str(&result_str)?;
    
    assert_eq!(result_json["message"], "Hello, world!");
    
    // Unload the plugin
    runtime.plugin_manager().unload_plugin(&plugin_id)?;
    
    // Shut down the runtime
    runtime.shutdown()?;
    
    Ok(())
}

/// Create a test WebAssembly module for testing.
fn create_test_wasm() -> Result<Vec<u8>> {
    // In a real implementation, we would compile a test module.
    // For simplicity, we'll use a pre-compiled module or placeholder.
    
    // This is just a placeholder - in a real test you would compile or load a real WASM file
    let wasm_bytes = include_bytes!("fixtures/test_plugin.wasm").to_vec();
    
    Ok(wasm_bytes)
}