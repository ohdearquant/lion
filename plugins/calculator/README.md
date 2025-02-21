# Calculator Plugin

A simple calculator plugin for the lion/agentic system that can perform basic arithmetic operations.

## Features

- Basic arithmetic operations:
  - Addition
  - Subtraction
  - Multiplication
  - Division (with division by zero checks)

## Usage

The plugin accepts JSON input via stdin and produces JSON output. Each input should be a single line containing a JSON object with:

- `function`: The operation to perform ("add", "subtract", "multiply", or "divide")
- `args`: An object containing:
  - `a`: First number (float)
  - `b`: Second number (float)

### Example Inputs

Addition:
```json
{"function": "add", "args": {"a": 5.0, "b": 3.0}}
```

Subtraction:
```json
{"function": "subtract", "args": {"a": 10.0, "b": 4.0}}
```

Multiplication:
```json
{"function": "multiply", "args": {"a": 6.0, "b": 7.0}}
```

Division:
```json
{"function": "divide", "args": {"a": 15.0, "b": 3.0}}
```

### Example Outputs

Success:
```json
{"result": 8.0}
```

Error (e.g., division by zero):
```json
{"error": "Division by zero"}
```

## Building

From the repository root:

```bash
cargo build --manifest-path plugins/calculator_plugin/Cargo.toml
```

## Testing

You can test the plugin using the provided test script:

```bash
cargo run --manifest-path scripts/Cargo.toml --bin test_calculator_plugin
```

## Integration

To use this plugin with the lion/agentic system, use the following manifest:

```toml
[plugin]
name = "calculator"
version = "0.1.0"
description = "A simple calculator plugin that can perform basic arithmetic operations"
entry_point = "target/debug/calculator_plugin"
driver = "subprocess"

[functions]
add = "Add two numbers"
subtract = "Subtract two numbers"
multiply = "Multiply two numbers"
divide = "Divide two numbers"
```

Then load the plugin through the UI or API, and invoke functions with appropriate arguments.
