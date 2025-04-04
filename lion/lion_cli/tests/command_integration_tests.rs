use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
// Command integration tests for the Lion CLI
//
// These tests verify that the CLI commands work correctly
// by executing them and checking their output.

use assert_cmd::Command as TestCommand;
use tempfile::tempdir;

// Helper function to get the command
fn cmd() -> TestCommand {
    TestCommand::cargo_bin("lion_cli").unwrap()
}

#[test]
fn test_plugin_commands() {
    // Test plugin load
    let temp_dir = tempdir().unwrap();
    let plugin_path = temp_dir.path().join("test_plugin.wasm");

    // Create an empty file for testing
    std::fs::write(&plugin_path, b"mock wasm content").unwrap();

    let result = cmd()
        .arg("plugin")
        .arg("load")
        .arg("--path")
        .arg(&plugin_path)
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Plugin loaded successfully"));

    // Test plugin list
    let result = cmd().arg("plugin").arg("list").assert();

    result
        .success()
        .stdout(predicate::str::contains("Listing all loaded plugins"));

    // Test plugin call
    let result = cmd()
        .arg("plugin")
        .arg("call")
        .arg("123e4567-e89b-12d3-a456-426614174000")
        .arg("calculate")
        .arg("--args")
        .arg(r#"{"x": 5, "y": 3, "operation": "add"}"#)
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Calling function 'calculate'"));

    // Test plugin unload
    let result = cmd()
        .arg("plugin")
        .arg("unload")
        .arg("123e4567-e89b-12d3-a456-426614174000")
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Unloading plugin"));

    // Test plugin grant-cap
    let result = cmd()
        .arg("plugin")
        .arg("grant-cap")
        .arg("--plugin")
        .arg("123e4567-e89b-12d3-a456-426614174000")
        .arg("--cap-type")
        .arg("file")
        .arg("--params")
        .arg(r#"{"path": "/tmp/*", "read": true}"#)
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Granting capability to plugin"));
}

#[test]
fn test_policy_commands() {
    // Test policy add
    let result = cmd()
        .arg("policy")
        .arg("add")
        .arg("--rule-id")
        .arg("test-rule")
        .arg("--subject")
        .arg("plugin:123e4567-e89b-12d3-a456-426614174000")
        .arg("--object")
        .arg("file:/etc")
        .arg("--action")
        .arg("deny")
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Adding policy rule"));

    // Test policy list
    let result = cmd().arg("policy").arg("list").assert();

    result
        .success()
        .stdout(predicate::str::contains("Listing all policy rules"));

    // Test policy remove
    let result = cmd().arg("policy").arg("remove").arg("test-rule").assert();

    result
        .success()
        .stdout(predicate::str::contains("Removing policy rule"));
}

#[test]
fn test_system_commands() {
    // Test system start
    let result = cmd().arg("system").arg("start").assert();

    result
        .success()
        .stdout(predicate::str::contains("Starting Lion microkernel"));

    // Test system status
    let result = cmd().arg("system").arg("status").assert();

    result
        .success()
        .stdout(predicate::str::contains("Lion System Status"));

    // Test system logs
    let result = cmd()
        .arg("system")
        .arg("logs")
        .arg("--level")
        .arg("INFO")
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Viewing system logs"));

    // Test system shutdown
    let result = cmd().arg("system").arg("shutdown").assert();

    result
        .success()
        .stdout(predicate::str::contains("Shutting down Lion microkernel"));
}

#[test]
fn test_workflow_commands() {
    // Test workflow register
    let temp_dir = tempdir().unwrap();
    let workflow_path = temp_dir.path().join("test_workflow.yaml");

    // Create a mock workflow file
    std::fs::write(
        &workflow_path,
        b"nodes:\n  - id: test\n    plugin_id: test\n",
    )
    .unwrap();

    let result = cmd()
        .arg("workflow")
        .arg("register")
        .arg("--file")
        .arg(&workflow_path)
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Workflow registered with ID"));

    // Test workflow start
    let result = cmd()
        .arg("workflow")
        .arg("start")
        .arg("123e4567-e89b-12d3-a456-426614174000")
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Starting workflow"));

    // Test workflow status
    let result = cmd()
        .arg("workflow")
        .arg("status")
        .arg("123e4567-e89b-12d3-a456-426614174000")
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Checking status of workflow"));

    // Test workflow cancel
    let result = cmd()
        .arg("workflow")
        .arg("cancel")
        .arg("123e4567-e89b-12d3-a456-426614174000")
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("Cancelling workflow"));
}

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
#[ignore = "ci command not implemented yet"]
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
#[ignore = "test-cli command not implemented yet"]
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
