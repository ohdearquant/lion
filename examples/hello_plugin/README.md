# Hello Plugin Example

This is a simple example plugin for the lion microkernel system that demonstrates the basic plugin architecture.

## Structure

- `hello_plugin.wasm`: A mock WASM module (for Phase 4, this is just a placeholder)
- `manifest.toml`: Plugin manifest describing the plugin's metadata and permissions

## Manifest Details

```toml
name = "hello_plugin"
version = "0.1.0"
entry_point = "examples/hello_plugin/hello_plugin.wasm"
permissions = ["net"]
```

## Usage

1. Load the plugin:
```bash
cargo run -p lion_cli -- load-plugin --manifest examples/hello_plugin/manifest.toml
```

2. Note the plugin ID (UUID) from the output.

3. Invoke the plugin:
```bash
cargo run -p lion_cli -- invoke-plugin --plugin-id <UUID> --input "test message"
```

The plugin will respond with a greeting containing your input message.

## Implementation Notes

For Phase 4, this is a demonstration plugin that simply echoes back input. In future phases, this will be replaced with a real WASM module that can be executed in a sandbox.

The plugin demonstrates:
- Basic manifest structure
- Permission declaration
- Plugin loading and invocation
- Integration with the orchestrator's event system