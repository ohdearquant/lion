# Lion: A Capability-Based Microkernel System

Lion is a secure, extensible microkernel architecture designed for running
isolated plugins with capability-based security, actor-based concurrency, and
workflow orchestration.

## Overview

Lion provides a comprehensive framework for building secure, extensible systems
with the following key features:

- **Capability-Based Security**: Fine-grained access control with capability
  attenuation and partial revocation
- **Actor-Based Concurrency**: Isolated components communicating through message
  passing
- **WebAssembly Isolation**: Secure sandboxing for untrusted code execution
- **Policy Engine**: Unified security policy enforcement
- **Workflow Orchestration**: Complex multi-step processes with error handling
- **Observability**: Comprehensive logging, metrics, and tracing

## Architecture

The Lion microkernel is built on several key architectural principles:

1. **Capability-Based Security**: Access to resources is controlled through
   unforgeable capability tokens following the principle of least privilege.

2. **Unified Capability-Policy Model**: Capabilities and policies work together
   to ensure both capability possession and policy compliance for resource
   access.

3. **Actor-Based Concurrency**: Components operate as isolated actors that
   communicate solely through message passing.

4. **WebAssembly Isolation**: Plugins are isolated in WebAssembly sandboxes for
   security and resource control.

5. **Workflow Orchestration**: Complex multi-step processes are orchestrated
   with parallel execution and error handling.

## Crates

Lion is organized into multiple crates, each with a specific responsibility:

| Crate                                         | Description                                                    |
| --------------------------------------------- | -------------------------------------------------------------- |
| [lion_capability](lion/lion_capability)       | Capability-based security model with attenuation and filtering |
| [lion_cli](lion/lion_cli)                     | Command-line interface for interacting with the Lion system    |
| [lion_concurrency](lion/lion_concurrency)     | Actor-based concurrency primitives and scheduling              |
| [lion_core](lion/lion_core)                   | Core types, traits, and utilities used by all other crates     |
| [lion_isolation](lion/lion_isolation)         | Plugin isolation and WebAssembly integration                   |
| [lion_observability](lion/lion_observability) | Logging, metrics, and distributed tracing                      |
| [lion_policy](lion/lion_policy)               | Policy engine for security enforcement                         |
| [lion_runtime](lion/lion_runtime)             | Runtime environment that orchestrates all subsystems           |
| [lion_workflow](lion/lion_workflow)           | Workflow engine for complex task orchestration                 |
| [lion_ui](lion_ui)                            | User interface for interacting with the Lion system            |

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Cargo
- WebAssembly toolchain (for building plugins)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/lion.git
cd lion

# Build all crates
cargo build

# Run tests
cargo test
```

### Running the CLI

```bash
# Build and run the CLI
cargo run -p lion_cli -- --help

# Start the Lion microkernel
cargo run -p lion_cli -- system start

# Load a plugin
cargo run -p lion_cli -- plugin load --path examples/hello_plugin/hello_plugin.wasm
```

## Examples

The [examples](examples) directory contains sample plugins and workflows
demonstrating Lion's capabilities:

- [Hello Plugin](examples/hello_plugin): A basic example showing plugin
  structure and usage
- [Calculator](plugins/calculator): A simple calculator plugin demonstrating
  function definitions

## Documentation

For more detailed documentation, see:

- [Architecture Overview](docs/v0.1.0/stage2/project_descriptioin.md)
- [Development Style Guide](docs/dev_style.md)
- [API Documentation](https://docs.rs/lion_core) (coming soon)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the Apache License 2.0 - see the
[LICENSE](LICENSE) file for details.
