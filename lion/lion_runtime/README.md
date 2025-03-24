# Lion Runtime

`lion_runtime` is the core runtime environment for the Lion microkernel system,
orchestrating all subsystems including capabilities, plugins, policies, and
workflows.

## Features

- **System Lifecycle**: Bootstrap, configuration, and shutdown
- **Capability Management**: Runtime capability resolution and enforcement
- **Plugin Management**: Plugin lifecycle and registry
- **Workflow Execution**: Workflow orchestration and monitoring
- **Integration**: Unified interface to all Lion subsystems
- **Configuration**: Flexible runtime configuration

## Architecture

The runtime system is built around several key components:

1. **System**: Core system management
   - Bootstrap: System initialization and startup
   - Config: Runtime configuration
   - Shutdown: Graceful system termination

2. **Capabilities**: Runtime capability management
   - Manager: Central capability registry and resolver
   - Resolution: Dynamic capability resolution

3. **Plugin**: Plugin lifecycle management
   - Lifecycle: Loading, starting, stopping, and unloading plugins
   - Manager: Central plugin coordination
   - Registry: Plugin discovery and metadata

4. **Workflow**: Workflow execution
   - Execution: Running workflow instances
   - Manager: Workflow lifecycle and monitoring

## Usage

### System Initialization

```rust
use lion_runtime::system::{System, SystemConfig};
use std::path::Path;

// Create system configuration
let config = SystemConfig {
    plugin_dir: Path::new("/path/to/plugins"),
    capability_policy: true,
    observability_enabled: true,
    ..Default::default()
};

// Initialize the system
let system = System::new(config)?;

// Start the system
system.start()?;

// Use the system
// ...

// Shutdown the system
system.shutdown()?;
```

### Plugin Management

```rust
use lion_runtime::plugin::{PluginManager, PluginConfig};
use std::path::Path;

// Get the plugin manager from the system
let plugin_manager = system.plugin_manager();

// Load a plugin
let plugin_id = plugin_manager.load_plugin(
    Path::new("/path/to/plugin.wasm"),
    PluginConfig::default(),
)?;

// Start the plugin
plugin_manager.start_plugin(plugin_id)?;

// Call a plugin function
let result = plugin_manager.call_function(
    plugin_id,
    "hello",
    &["world"],
)?;

// Stop and unload the plugin
plugin_manager.stop_plugin(plugin_id)?;
plugin_manager.unload_plugin(plugin_id)?;
```

### Capability Management

```rust
use lion_runtime::capabilities::{CapabilityManager, CapabilityRequest};
use lion_core::types::AccessRequest;

// Get the capability manager from the system
let capability_manager = system.capability_manager();

// Grant a capability to a plugin
capability_manager.grant_capability(
    plugin_id,
    "file_read",
    &["/tmp/example.txt"],
)?;

// Check if a capability is allowed
let request = AccessRequest::file_read("/tmp/example.txt");
let allowed = capability_manager.check_capability(
    plugin_id,
    &request,
)?;

if allowed {
    println!("Access allowed");
} else {
    println!("Access denied");
}
```

### Workflow Execution

```rust
use lion_runtime::workflow::{WorkflowManager, WorkflowConfig};
use lion_workflow::model::WorkflowDefinition;

// Get the workflow manager from the system
let workflow_manager = system.workflow_manager();

// Register a workflow
let workflow_id = workflow_manager.register_workflow(
    workflow_definition,
    WorkflowConfig::default(),
)?;

// Start a workflow instance
let instance_id = workflow_manager.start_workflow(
    workflow_id,
    Some(initial_data),
)?;

// Check workflow status
let status = workflow_manager.get_workflow_status(instance_id)?;
println!("Workflow status: {:?}", status);

// Cancel a workflow
workflow_manager.cancel_workflow(instance_id)?;
```

## Integration with Other Lion Crates

The runtime system integrates all other Lion crates:

- **lion_core**: For core types and traits
- **lion_capability**: For capability-based security
- **lion_concurrency**: For actor-based concurrency
- **lion_isolation**: For plugin isolation
- **lion_observability**: For logging, metrics, and tracing
- **lion_policy**: For policy enforcement
- **lion_workflow**: For workflow orchestration

## Configuration

The runtime system is highly configurable:

```rust
use lion_runtime::system::SystemConfig;

let config = SystemConfig {
    // System paths
    plugin_dir: "/path/to/plugins".into(),
    data_dir: "/var/lib/lion".into(),
    
    // Feature flags
    capability_policy: true,
    observability_enabled: true,
    
    // Resource limits
    max_plugins: 100,
    max_workflows: 1000,
    
    // Timeouts
    plugin_startup_timeout: Duration::from_secs(30),
    workflow_execution_timeout: Duration::from_secs(3600),
    
    ..Default::default()
};
```

## License

Licensed under the Apache License, Version 2.0 - see the
[LICENSE](../../LICENSE) file for details.
