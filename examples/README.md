# Plugin System Examples

This directory contains example plugins demonstrating how to use the plugin system in lion.

## Available Examples

### [Hello Plugin](hello_plugin/)
A basic example showing plugin structure, manifest format, and usage. See the [hello_plugin/README.md](hello_plugin/README.md) for details.

## Plugin Manifest Fields

Every plugin requires a manifest file (TOML format) with these fields:

- `name`: A unique identifier for your plugin
- `version`: Semantic version of your plugin
- `entry_point`: Path to the plugin's executable (WASM module or script)
- `permissions`: List of permissions the plugin requires (e.g., "net" for network access)

## Notes

For Phase 4, these are demonstration plugins using mock WASM files. In future phases, this will be extended to support real WASM modules or sandboxed processes.