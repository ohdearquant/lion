use std::path::PathBuf;
use tokio::time::Duration;

#[tokio::test]
async fn test_calculator_plugin_cli() {
    // Load the calculator plugin
    let manifest_path = PathBuf::from("plugins/calculator/manifest.toml");
    assert!(manifest_path.exists(), "Calculator plugin manifest not found");

    // Load plugin and capture its ID
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_lion_cli"))
        .args(["load-plugin", "--manifest"])
        .arg(manifest_path)
        .output()
        .expect("Failed to execute load-plugin command");

    assert!(output.status.success(), "Failed to load plugin");

    let output_str = String::from_utf8(output.stdout).unwrap();
    let plugin_id = output_str
        .lines()
        .find(|line| line.starts_with("Plugin ID: "))
        .map(|line| line.trim_start_matches("Plugin ID: "))
        .expect("Plugin ID not found in output");

    // Test addition
    let add_output = std::process::Command::new(env!("CARGO_BIN_EXE_lion_cli"))
        .args([
            "invoke-plugin",
            "--plugin-id",
            plugin_id,
            "--input",
            r#"{"function": "add", "args": {"a": 5.0, "b": 3.0}}"#,
        ])
        .output()
        .expect("Failed to execute invoke-plugin command");

    assert!(add_output.status.success(), "Addition operation failed");
    let add_output_str = String::from_utf8(add_output.stdout).unwrap();
    assert!(
        add_output_str.contains(r#""result":8.0"#),
        "Unexpected addition result"
    );

    // Test division by zero error handling
    let div_output = std::process::Command::new(env!("CARGO_BIN_EXE_lion_cli"))
        .args([
            "invoke-plugin",
            "--plugin-id",
            plugin_id,
            "--input",
            r#"{"function": "divide", "args": {"a": 1.0, "b": 0.0}}"#,
        ])
        .output()
        .expect("Failed to execute invoke-plugin command");

    assert!(div_output.status.success(), "Division operation failed");
    let div_output_str = String::from_utf8(div_output.stdout).unwrap();
    assert!(
        div_output_str.contains("Division by zero"),
        "Expected division by zero error"
    );

    // Test invalid function
    let invalid_output = std::process::Command::new(env!("CARGO_BIN_EXE_lion_cli"))
        .args([
            "invoke-plugin",
            "--plugin-id",
            plugin_id,
            "--input",
            r#"{"function": "invalid", "args": {"a": 1.0, "b": 2.0}}"#,
        ])
        .output()
        .expect("Failed to execute invoke-plugin command");

    assert!(invalid_output.status.success(), "Invalid function call failed");
    let invalid_output_str = String::from_utf8(invalid_output.stdout).unwrap();
    assert!(
        invalid_output_str.contains("Unknown function"),
        "Expected unknown function error"
    );

    // Test invalid plugin ID
    let invalid_id = "00000000-0000-0000-0000-000000000000";
    let invalid_id_output = std::process::Command::new(env!("CARGO_BIN_EXE_lion_cli"))
        .args([
            "invoke-plugin",
            "--plugin-id",
            invalid_id,
            "--input",
            r#"{"function": "add", "args": {"a": 1.0, "b": 2.0}}"#,
        ])
        .output()
        .expect("Failed to execute invoke-plugin command");

    let invalid_id_output_str = String::from_utf8(invalid_id_output.stdout).unwrap();
    assert!(
        invalid_id_output_str.contains("Plugin not found"),
        "Expected plugin not found error"
    );
}

#[tokio::test]
async fn test_plugin_load_errors() {
    // Test loading non-existent plugin
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_lion_cli"))
        .args(["load-plugin", "--manifest", "nonexistent.toml"])
        .output()
        .expect("Failed to execute load-plugin command");

    assert!(!output.status.success(), "Expected load to fail");
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("Failed to read manifest file"));

    // Test loading invalid manifest
    let temp_dir = tempfile::tempdir().unwrap();
    let invalid_manifest = temp_dir.path().join("invalid.toml");
    std::fs::write(&invalid_manifest, "invalid = toml [ content").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_lion_cli"))
        .args(["load-plugin", "--manifest"])
        .arg(&invalid_manifest)
        .output()
        .expect("Failed to execute load-plugin command");

    assert!(!output.status.success(), "Expected load to fail");
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("Failed to parse manifest"));
}