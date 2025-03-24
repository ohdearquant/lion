# Lion UI

## Stage 2 - Phase 4: Advanced Logging & Tauri Integration

This is the user interface component for the Lion runtime system. It provides
both a web-based interface and a desktop application (via Tauri) for interacting
with the Lion system.

## Features

- **Advanced Logging System**
  - Real-time log streaming via Server-Sent Events (SSE)
  - Structured log entries with metadata
  - Powerful filtering and search capabilities
  - Log correlation tracking

- **Agent Management**
  - Spawn and monitor agents
  - View agent status and logs
  - Configure agent parameters

- **Plugin System**
  - Load and manage plugins
  - Invoke plugin methods
  - Monitor plugin activity

- **Desktop Application**
  - System tray integration
  - Multiple windows support
  - Native OS integration via Tauri

## Architecture

The Lion UI consists of two main components:

1. **Web Server & API** (`lion_ui/src/`)
   - Built with Axum for high-performance async handling
   - RESTful API for interacting with the Lion runtime
   - Server-Sent Events for real-time updates
   - In-memory log buffer for searching

2. **Desktop Application** (`lion_ui/src-tauri/`)
   - Built with Tauri for native desktop integration
   - System tray support
   - Multiple windows for different functions (logs, agents, plugins)
   - Bridge to the web server API

## Development

### Prerequisites

- Rust 1.75+
- Node.js 18+ (for frontend development)
- Tauri CLI

### Running the Web Server

```bash
cargo run --bin lion_ui
```

This will start the web server on `localhost:8080`.

### Building the Desktop Application

```bash
cd lion_ui
cargo tauri build
```

### Development Mode

```bash
cd lion_ui
cargo tauri dev
```

## Integration with Lion Runtime

The Lion UI integrates with the core Lion runtime system through:

1. Direct function calls via the Lion crate dependencies
2. Event subscription for real-time updates
3. Plugin and agent management interfaces

## Future Improvements

- Full-featured dashboard with metrics visualization
- Remote server management
- Enhanced security controls
- Theme support and customization options
