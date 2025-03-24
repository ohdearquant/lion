# Lion CLI

The Lion Command Line Interface (CLI) provides a comprehensive set of commands
for interacting with the Lion microkernel system. It allows users to manage
plugins, define security policies, control the system, and orchestrate
workflows.

## Features

- **Plugin Management**: Load, list, call, unload plugins, and grant specific
  capabilities
- **Policy Management**: Define, list, check, and remove security policies
- **System Management**: Start, check status, view logs, and shutdown the
  microkernel
- **Workflow Management**: Register, start, check status, and cancel workflows
- **Enhanced UI**: Colored output for better readability and intuitive status
  indicators
- **Guided Experience**: Suggests relevant next commands after each operation
- **Real Microkernel Integration**: Optional feature flags to connect with the
  actual Lion components

## Installation

### Building from Source

```bash
# Build with default (placeholders) mode
cargo build --release

# Build with actual microkernel integration
cargo build --release --features all-integrations
```

### Feature Flags

| Flag                     | Description                                                |
| ------------------------ | ---------------------------------------------------------- |
| `runtime-integration`    | Enables integration with the actual Lion runtime component |
| `policy-integration`     | Enables integration with the Lion policy engine            |
| `capability-integration` | Enables integration with the Lion capability system        |
| `workflow-integration`   | Enables integration with the Lion workflow orchestrator    |
| `all-integrations`       | Enables all integration features                           |

## Usage

### Plugin Commands

```bash
# Load a WASM plugin
lion-cli plugin load --path /path/to/plugin.wasm [--caps /path/to/capabilities.json]

# List all loaded plugins
lion-cli plugin list

# Call a function in a loaded plugin
lion-cli plugin call <plugin-id> <function-name> [--args '{"param1": "value1"}']

# Unload a plugin
lion-cli plugin unload <plugin-id>

# Grant capabilities to a plugin
lion-cli plugin grant-cap --plugin <plugin-id> --cap-type file --params '{"path": "/tmp/*", "read": true}'
```

### Policy Commands

```bash
# Add a policy rule
lion-cli policy add --rule-id rule1 --subject plugin:<plugin-id> --object file:/etc --action deny

# List all policy rules
lion-cli policy list

# Check if a policy would allow a specific action
lion-cli policy check --subject plugin:<plugin-id> --object file:/etc/passwd --action read

# Remove a policy rule
lion-cli policy remove rule1
```

### System Commands

```bash
# Start the Lion microkernel
lion-cli system start

# Show system status
lion-cli system status

# View system logs
lion-cli system logs [--level INFO] [--component plugin]

# Shutdown the microkernel
lion-cli system shutdown
```

### Workflow Commands

```bash
# Register a new workflow
lion-cli workflow register --file /path/to/workflow.yaml

# List all registered workflows
lion-cli workflow list

# Start a registered workflow
lion-cli workflow start <workflow-id>

# Pause and resume a workflow
lion-cli workflow pause <workflow-id>
lion-cli workflow resume <workflow-id>

# Check workflow status
lion-cli workflow status <workflow-id>

# Cancel a running workflow
lion-cli workflow cancel <workflow-id>
```

## Architecture

The Lion CLI is designed with a modular architecture that separates command
handling from the actual implementation:

1. **Command Layer**: Parses user input and routes to appropriate handlers
2. **Interface Layer**: Connects to the Lion microkernel components
3. **Integration Layer**: Provides testing utilities and integration points

### Integration Modes

The CLI can operate in two modes:

1. **Placeholder Mode (Default)**: Uses mock data and simulates operations for
   development
2. **Real Integration Mode**: Connects to actual Lion microkernel components for
   production use

## Development

### Building

```bash
# Development build with placeholder mode
cargo build

# Production build with real integration
cargo build --release --features all-integrations

# Target specific integrations
cargo build --release --features runtime-integration,policy-integration
```

### Testing

```bash
# Run all tests
cargo test

# Run with specific integration features
cargo test --features runtime-integration
```

### Code Structure

- **Core CLI Structure**:
  - `src/lib.rs`: Library exports and module organization
  - `src/main.rs`: CLI entry point with command structure based on Clap
  - `src/integration.rs`: Integration test utilities

- **Command Handlers**:
  - `src/commands/plugin.rs`: Plugin management commands
  - `src/commands/system.rs`: System management commands
  - `src/commands/workflow.rs`: Workflow management commands
  - `src/commands/policy.rs`: Policy management commands

- **Microkernel Interfaces**:
  - `src/interfaces/runtime.rs`: Interface to the Lion runtime component
  - `src/interfaces/isolation.rs`: Interface to the WASM isolation system
  - `src/interfaces/capability.rs`: Interface to the capability system
  - `src/interfaces/policy.rs`: Interface to the policy engine
  - `src/interfaces/workflow.rs`: Interface to the workflow orchestrator
  - `src/interfaces/observability.rs`: Interface to logging, metrics, and
    tracing

- **Tests**:
  - `tests/command_integration_tests.rs`: End-to-end command tests

## Example Workflows

### Managing Plugins

```bash
# 1. Start the Lion microkernel
lion-cli system start

# 2. Load a calculator plugin
lion-cli plugin load --path plugins/calculator/calculator_plugin.wasm

# 3. Call a function in the plugin
lion-cli plugin call 123e4567-e89b-12d3-a456-426614174000 calculate --args '{"x": 5, "y": 3, "operation": "add"}'

# 4. Grant additional capabilities
lion-cli plugin grant-cap --plugin 123e4567-e89b-12d3-a456-426614174000 --cap-type file --params '{"path": "/tmp/results.txt", "read": true, "write": true, "execute": false}'

# 5. Check system status
lion-cli system status
```

### Working with Workflows

```bash
# 1. Register a data processing workflow
lion-cli workflow register --file examples/workflows/data_processing.json

# 2. Start the workflow
lion-cli workflow start workflow-123

# 3. Check the workflow status
lion-cli workflow status workflow-123

# 4. Pause the workflow if needed
lion-cli workflow pause workflow-123
```

## License

Apache License 2.0
