use std::path::PathBuf;

// This test demonstrates the CLI integration without needing the full Tauri UI
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_plugin_integration() {
        let plugin_path = PathBuf::from("../plugins/calculator/calculator_plugin.wasm");

        // Call the CLI library function directly
        let result = lion_cli::commands::plugin::load_plugin(&plugin_path, None);
        assert!(result.is_ok(), "Failed to load plugin: {:?}", result.err());

        let plugin_id = result.unwrap();
        println!("Plugin loaded with ID: {}", plugin_id);

        // List plugins
        let (_status, plugin_ids) = lion_cli::interfaces::runtime::get_runtime_status_and_plugins()
            .expect("Failed to get plugins");

        println!("Found {} plugins", plugin_ids.len());

        // Call plugin function
        if let Some(id) = plugin_ids.first() {
            let result = lion_cli::commands::plugin::call_plugin(
                id,
                "calculate",
                Some(r#"{"x": 5, "y": 3, "operation": "add"}"#),
            );

            assert!(
                result.is_ok(),
                "Failed to call plugin function: {:?}",
                result.err()
            );
            println!("Function result: {:?}", result.unwrap());
        }
    }
}
