# Lion Isolation

`lion_isolation` provides secure sandboxing and resource control for plugins in
the Lion microkernel system, primarily through WebAssembly isolation.

## Features

- **Plugin Sandboxing**: Secure execution environment for untrusted code
- **WebAssembly Integration**: Support for WASM modules and runtimes
- **Resource Limiting**: Memory, CPU, and I/O constraints for plugins
- **Lifecycle Management**: Loading, unloading, and reloading plugins
- **Host Function Calls**: Secure bridging between host and plugin
- **Memory Management**: Safe memory sharing and isolation

## Architecture

The isolation system is built around several key components:

1. **Manager**: Coordinates plugin lifecycle and resources
   - Backend: Abstraction over isolation technologies
   - Lifecycle: Loading, starting, stopping, and unloading plugins
   - Pool: Managing multiple plugin instances

2. **Resource Control**: Limiting and metering resource usage
   - Limiter: Enforcing resource constraints
   - Metering: Tracking resource consumption
   - Usage: Monitoring and reporting resource usage

3. **WebAssembly**: Integration with WASM runtimes
   - Engine: Abstraction over WASM engines (Wasmtime, Wasmer, etc.)
   - Hostcalls: Secure function calls between host and WASM
   - Memory: Safe memory access and sharing
   - Module: WASM module loading and validation

## Usage

### Loading a WASM Plugin

```rust
use lion_isolation::manager::{PluginManager, PluginConfig};
use lion_isolation::wasm::WasmBackend;
use std::path::Path;

// Create a WASM backend
let backend = WasmBackend::new();

// Create a plugin manager
let manager = PluginManager::new(backend);

// Configure a plugin
let config = PluginConfig {
    name: "example-plugin".to_string(),
    path: Path::new("/path/to/plugin.wasm"),
    memory_limit: Some(64 * 1024 * 1024), // 64 MB
    ..Default::default()
};

// Load the plugin
let plugin_id = manager.load_plugin(config)?;

// Start the plugin
manager.start_plugin(plugin_id)?;

// Call a plugin function
let result = manager.call_function(plugin_id, "hello", &["world"])?;
println!("Result: {:?}", result);

// Stop and unload the plugin
manager.stop_plugin(plugin_id)?;
manager.unload_plugin(plugin_id)?;
```

### Resource Limiting

```rust
use lion_isolation::resource::{ResourceLimiter, ResourceConfig};
use std::time::Duration;

// Create resource limits
let limits = ResourceConfig {
    memory_limit: 64 * 1024 * 1024, // 64 MB
    cpu_time_limit: Duration::from_millis(100), // 100ms per invocation
    io_ops_limit: Some(1000), // 1000 I/O operations
    ..Default::default()
};

// Create a resource limiter
let limiter = ResourceLimiter::new(limits);

// Apply limits to a plugin
manager.set_resource_limiter(plugin_id, limiter)?;
```

### Custom Host Functions

```rust
use lion_isolation::wasm::{HostFunction, HostFunctionContext};

// Define a host function
fn host_log(ctx: &HostFunctionContext, args: &[Value]) -> Result<Value, HostCallError> {
    if let [Value::String(message)] = args {
        println!("Plugin log: {}", message);
        Ok(Value::Null)
    } else {
        Err(HostCallError::InvalidArguments)
    }
}

// Register the host function
let host_fn = HostFunction::new("env", "log", host_log);
backend.register_host_function(host_fn);
```

## Integration with Other Lion Crates

The isolation system integrates with other Lion crates:

- **lion_core**: For core types and traits
- **lion_capability**: For capability-based access control
- **lion_policy**: For policy enforcement
- **lion_runtime**: For runtime orchestration

## Security Considerations

- **Memory Isolation**: Preventing access to host memory
- **Resource Exhaustion Prevention**: Limiting CPU, memory, and I/O
- **Capability-Based Access**: Explicit permission for system resources
- **Input Validation**: Sanitizing all inputs from plugins

## License

Licensed under the Apache License, Version 2.0 - see the
[LICENSE](../../LICENSE) file for details.
