# Lion Core

`lion_core` is the foundational library for the Lion microkernel system,
providing the core types, traits, and utilities needed by all other components
of the system.

## Features

- **Capability-Based Security**: A unified security model that integrates
  capabilities with policy enforcement
- **Type-Safe IDs**: Strongly-typed identifiers for plugins, capabilities,
  workflows, etc.
- **Error Handling**: Comprehensive error hierarchy for all Lion components
- **Core Traits**: Fundamental interfaces for isolation, concurrency, plugins,
  and workflows
- **Data Types**: Essential data structures for plugin configuration, workflow
  definitions, etc.
- **Utilities**: Logging, configuration, and version management utilities

## Architecture Overview

The Lion microkernel is built on several key architectural principles:

1. **Capability-Based Security**: Access to resources is controlled through
   unforgeable capability tokens following the principle of least privilege.

2. **Unified Capability-Policy Model**: Capabilities and policies work together
   to ensure both capability possession and policy compliance for resource
   access.

3. **Partial Capability Relations**: Capabilities form a partial order based on
   privilege inclusion, enabling operations like capability attenuation,
   composition, and partial revocation.

4. **Actor-Based Concurrency**: Components operate as isolated actors that
   communicate solely through message passing.

5. **WebAssembly Isolation**: Plugins are isolated in WebAssembly sandboxes for
   security and resource control.

6. **Workflow Orchestration**: Complex multi-step processes are orchestrated
   with parallel execution and error handling.

## Installation

Add `lion_core` to your `Cargo.toml`:

```toml
[dependencies]
lion_core = "0.1.0"
```

## Usage

Here are some examples of how to use the core components of this library:

### Working with IDs

```rust
use lion_core::id::{PluginId, CapabilityId};

// Create new random IDs
let plugin_id = PluginId::new();
let capability_id = CapabilityId::new();

// Create from string
let id_str = "550e8400-e29b-41d4-a716-446655440000";
let plugin_id = PluginId::from_str(id_str).unwrap();
```

### Creating Access Requests

```rust
use lion_core::types::AccessRequest;

// Create file access requests
let read_request = AccessRequest::file_read("/path/to/file.txt");
let write_request = AccessRequest::file_write("/path/to/file.txt");

// Create network access requests
let connect_request = AccessRequest::network_connect("example.com", 80);
let listen_request = AccessRequest::network_listen("0.0.0.0", 8080);
```

### Working with Capabilities

```rust
use lion_core::traits::Capability;
use lion_core::types::AccessRequest;
use lion_core::error::CapabilityError;

struct FileReadCapability {
    path: String,
}

impl Capability for FileReadCapability {
    fn capability_type(&self) -> &str {
        "file_read"
    }

    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::File { path, read, write, .. } => {
                if *write {
                    return Err(CapabilityError::PermissionDenied(
                        "Write access not allowed".into()
                    ));
                }
                
                if !path.starts_with(&self.path) {
                    return Err(CapabilityError::PermissionDenied(
                        format!("Access to {} not allowed", path.display())
                    ));
                }
                
                Ok(())
            },
            _ => Err(CapabilityError::PermissionDenied(
                "Only file access is allowed".into()
            )),
        }
    }
}
```

### Creating and Running Workflows

```rust
use lion_core::types::{Workflow, WorkflowNode};

// Create a workflow
let mut workflow = Workflow::new("Data Processing", "Process and analyze data");

// Add nodes to the workflow
let node1 = WorkflowNode::new_plugin_call("Load Data", "data_loader", "load_csv");
let node1_id = node1.id;
workflow.add_node(node1);

let node2 = WorkflowNode::new_plugin_call("Process Data", "data_processor", "process");
let node2_id = node2.id;
workflow.add_node(node2);

let mut node3 = WorkflowNode::new_plugin_call("Visualize Results", "visualizer", "create_chart");
node3.add_dependency(node2_id);
workflow.add_node(node3);

// Validate the workflow
if let Err(error) = workflow.validate() {
    println!("Workflow validation failed: {}", error);
}
```

### Using Configuration Utilities

```rust
use lion_core::utils::Config;

// Create a configuration
let mut config = Config::new_map();

// Set configuration values
config.set("app.name", "Lion Application").unwrap();
config.set("app.version", "1.0.0").unwrap();
config.set("plugin.timeout_ms", 5000).unwrap();

// Get configuration values
let name = config.get_as::<String>("app.name").unwrap();
let timeout = config.get_as::<i64>("plugin.timeout_ms").unwrap();

println!("Application: {} (timeout: {}ms)", name, timeout);
```

## License

Licensed under either of:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
