use std::path::Path;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

// Import directly from the crate
// In integration tests we need to use the crate name
extern crate lion_ui;
use lion_ui::state::AppState;
use lion_ui::wasm;

#[tokio::test]
async fn test_wasm_load() {
    // Initialize similar to the main application
    let (logs_tx, _) = broadcast::channel(100);
    let log_buffer = Arc::new(RwLock::new(Vec::with_capacity(100)));
    let app_state = Arc::new(AppState::new(logs_tx, log_buffer));

    // Path to the test WASM file
    let wasm_path = Path::new("tests/mock.wasm");
    if !wasm_path.exists() {
        panic!("Test WASM file not found. Run 'wat2wasm tests/mock.wat -o tests/mock.wasm' first.");
    }

    // Load the WASM plugin
    let result = wasm::load_plugin(
        &app_state,
        "tests/mock.wasm",
        Some("TestPlugin".to_string()),
    )
    .await;

    // Verify successful loading
    assert!(
        result.is_ok(),
        "Failed to load WASM plugin: {:?}",
        result.err()
    );

    let plugin_info = result.unwrap();
    assert_eq!(plugin_info.name, "TestPlugin");

    println!("Successfully loaded WASM plugin: {}", plugin_info.name);
}
