use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

pub fn test_calculator_plugin() {
    // Build the plugin first
    Command::new("cargo")
        .args(["build"])
        .current_dir("plugins/calculator_plugin")
        .status()
        .expect("Failed to build calculator plugin");

    // Start the plugin process
    let mut child = Command::new("../../target/debug/calculator_plugin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir("plugins/calculator_plugin")
        .spawn()
        .expect("Failed to start calculator plugin");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let stderr = child.stderr.take().expect("Failed to get stderr");

    // Create a reader for stdout
    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);

    // Test cases
    let test_cases = vec![
        (
            r#"{"function": "add", "args": {"a": 5.0, "b": 3.0}}"#,
            r#"{"result":8.0}"#,
        ),
        (
            r#"{"function": "subtract", "args": {"a": 10.0, "b": 4.0}}"#,
            r#"{"result":6.0}"#,
        ),
        (
            r#"{"function": "multiply", "args": {"a": 6.0, "b": 7.0}}"#,
            r#"{"result":42.0}"#,
        ),
        (
            r#"{"function": "divide", "args": {"a": 15.0, "b": 3.0}}"#,
            r#"{"result":5.0}"#,
        ),
        (
            r#"{"function": "divide", "args": {"a": 1.0, "b": 0.0}}"#,
            r#"{"error":"Division by zero"}"#,
        ),
        (
            r#"{"function": "unknown", "args": {"a": 1.0, "b": 1.0}}"#,
            r#"{"error":"Unknown function: unknown"}"#,
        ),
    ];

    // Spawn a thread to read stderr
    std::thread::spawn(move || {
        let mut line = String::new();
        while stderr_reader.read_line(&mut line).unwrap() > 0 {
            eprintln!("Plugin stderr: {}", line);
            line.clear();
        }
    });

    // Run test cases
    for (i, (input, expected)) in test_cases.iter().enumerate() {
        println!("\nTest case {}", i + 1);
        println!("Input:    {}", input);
        println!("Expected: {}", expected);

        // Send input to plugin
        stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
        stdin.write_all(b"\n").expect("Failed to write newline");
        stdin.flush().expect("Failed to flush stdin");

        // Read response
        let mut response = String::new();
        stdout_reader
            .read_line(&mut response)
            .expect("Failed to read from stdout");

        println!("Got:      {}", response.trim());

        // Compare (ignoring whitespace)
        assert_eq!(
            response.trim(),
            expected.trim(),
            "Test case {} failed",
            i + 1
        );
    }

    // Clean up
    drop(stdin); // Close stdin to let the plugin know we're done
    let status = child.wait().expect("Failed to wait for plugin");
    assert!(status.success(), "Plugin exited with error");

    println!("\nAll tests passed!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin() {
        test_calculator_plugin();
    }
}