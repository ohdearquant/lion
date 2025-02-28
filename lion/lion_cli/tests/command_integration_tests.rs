use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

fn setup_test_script(script_name: &str, content: &str) -> String {
    let tmp_dir = env::temp_dir();
    let script_path = tmp_dir.join(script_name);

    fs::write(&script_path, content).expect("Failed to write test script");

    // Make the script executable
    #[cfg(unix)]
    {
        StdCommand::new("chmod")
            .args(&["+x", script_path.to_str().unwrap()])
            .status()
            .expect("Failed to chmod the test script");
    }

    script_path.to_string_lossy().to_string()
}

#[test]
fn test_ci_command_executes_script() {
    // Skip if not on CI or in certain environments where scripts can't be executed
    if cfg!(not(unix)) {
        return;
    }

    // Create a test script that simulates the CI script
    let script_content = r#"#!/bin/sh
echo "CI script executed successfully"
exit 0
"#;

    let script_path = setup_test_script("test_ci.sh", script_content);

    // Create a symbolic link or copy the script to the expected location
    let scripts_dir = Path::new("scripts");
    if !scripts_dir.exists() {
        fs::create_dir_all(scripts_dir).expect("Failed to create scripts directory");
    }

    let target_path = scripts_dir.join("ci.sh");
    fs::copy(&script_path, &target_path).expect("Failed to copy test script");

    // Make the script executable
    #[cfg(unix)]
    {
        StdCommand::new("chmod")
            .args(&["+x", target_path.to_str().unwrap()])
            .status()
            .expect("Failed to chmod the target script");
    }

    // Run the command and verify it executes the script
    let mut cmd = Command::cargo_bin("lion_cli").unwrap();
    let assert = cmd.arg("ci").assert();

    assert
        .success()
        .stdout(predicate::str::contains("Executing CI script"));

    // Clean up
    let _ = fs::remove_file(target_path);
}

#[test]
fn test_test_cli_command_executes_script() {
    // Skip if not on CI or in certain environments where scripts can't be executed
    if cfg!(not(unix)) {
        return;
    }

    // Create a test script that simulates the test-cli script
    let script_content = r#"#!/bin/sh
echo "Test CLI script executed successfully"
exit 0
"#;

    let script_path = setup_test_script("test_test_cli.sh", script_content);

    // Create a symbolic link or copy the script to the expected location
    let scripts_dir = Path::new("scripts");
    if !scripts_dir.exists() {
        fs::create_dir_all(scripts_dir).expect("Failed to create scripts directory");
    }

    let target_path = scripts_dir.join("test_cli.sh");
    fs::copy(&script_path, &target_path).expect("Failed to copy test script");

    // Make the script executable
    #[cfg(unix)]
    {
        StdCommand::new("chmod")
            .args(&["+x", target_path.to_str().unwrap()])
            .status()
            .expect("Failed to chmod the target script");
    }

    // Run the command and verify it executes the script
    let mut cmd = Command::cargo_bin("lion_cli").unwrap();
    let assert = cmd.arg("test-cli").assert();

    assert
        .success()
        .stdout(predicate::str::contains("Running CLI tests"));

    // Clean up
    let _ = fs::remove_file(target_path);
}
