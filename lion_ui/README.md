# Lion UI

`lion_ui` provides a user interface for interacting with the Lion microkernel
system, offering visualization and control of plugins, workflows, and system
state.

## Features

- **Plugin Management**: Load, configure, and monitor plugins
- **Workflow Visualization**: Interactive workflow graphs and status monitoring
- **Event Monitoring**: Real-time event stream visualization
- **Log Viewing**: Structured log viewing and filtering
- **Agent Interaction**: Interface for interacting with Lion agents
- **System Status**: System health and resource usage monitoring

## Architecture

The UI system is built around several key components:

1. **Frontend**: User interface components
   - React-based UI with TypeScript
   - Interactive visualizations
   - Responsive design

2. **Backend**: Tauri-based native application
   - Rust backend for system integration
   - IPC bridge between UI and Lion system
   - File system access and management

3. **Core Components**:
   - Agents: Agent management and visualization
   - Events: Event stream processing and display
   - Logs: Log collection and structured viewing
   - Plugins: Plugin management interface
   - State: Application state management
   - WASM: WebAssembly integration for plugin visualization

## Installation

### From Releases

Download the latest release for your platform from the releases page.

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/lion.git
cd lion

# Install dependencies
npm install

# Build the UI
npm run build

# Build the Tauri application
cargo tauri build
```

## Usage

### Starting the UI

```bash
# Development mode
npm run tauri dev

# Run the built application
./target/release/lion-ui
```

### Plugin Management

The UI allows you to:

- Browse available plugins
- Load plugins into the system
- Configure plugin parameters
- Monitor plugin status and resource usage
- Call plugin functions and view results

### Workflow Visualization

The workflow view provides:

- Interactive graph visualization of workflows
- Real-time status updates for workflow nodes
- Ability to start, pause, and cancel workflows
- Detailed view of workflow data flow

### Log Viewing

The log viewer offers:

- Structured viewing of system logs
- Filtering by log level, component, and time
- Search functionality
- Export options for log analysis

## Development

### Project Structure

- `src/`: Rust backend code
  - `main.rs`: Application entry point
  - `agents.rs`: Agent management
  - `events.rs`: Event handling
  - `logs.rs`: Log processing
  - `plugins.rs`: Plugin management
  - `state.rs`: Application state
  - `wasm.rs`: WebAssembly integration

- `src-tauri/`: Tauri configuration and native code
  - `src/`: Rust code for Tauri backend
  - `tauri.conf.json`: Tauri configuration

- `frontend/`: React frontend code
  - `src/`: TypeScript source code
  - `public/`: Static assets
  - `index.html`: Main HTML template

### Building for Development

```bash
# Start the development server
npm run tauri dev
```

### Running Tests

```bash
# Run Rust tests
cargo test -p lion_ui

# Run frontend tests
npm test
```

## Tauri Migration

The UI is currently being migrated to Tauri. See
[TAURI_MIGRATION.md](TAURI_MIGRATION.md) for details on the migration process
and status.

## Integration with Other Lion Crates

The UI integrates with other Lion crates:

- **lion_core**: For core types and interfaces
- **lion_runtime**: For system control and management
- **lion_observability**: For logs and metrics
- **lion_workflow**: For workflow visualization

## License

Licensed under the Apache License, Version 2.0 - see the [LICENSE](../LICENSE)
file for details.
