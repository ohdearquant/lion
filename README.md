# lion

A Rust-based event-driven orchestration system with microkernel architecture.

## Overview

lion is designed to be a secure, explainable, and high-performance orchestration system that can handle multi-agent AI workflows. It follows a microkernel architecture pattern with strong isolation boundaries and comprehensive logging.

## Project Structure

```
lion/
├── agentic_core/       # Core library crate
│   └── src/
│       ├── element.rs      # Base trackable entity
│       ├── pile.rs         # Thread-safe container
│       ├── progression.rs  # Ordered sequence tracker
│       └── store.rs        # Element storage
├── agentic_cli/       # Command-line interface
└── docs/             # Documentation
```

## Development

### Prerequisites

- Rust (stable channel)
- Cargo

### Setup

1. Clone the repository:
```bash
git clone https://github.com/yourusername/lion.git
cd lion
```

2. Build the project:
```bash
cargo build
```

3. Run tests:
```bash
cargo test
```

### Development Guidelines

1. **Code Style**
   - Follow Rust standard formatting (enforced by rustfmt)
   - Run `cargo fmt` before committing
   - Run `cargo clippy` to catch common mistakes

2. **Git Workflow**
   - Create feature branches from `main`
   - Follow conventional commits
   - Submit PRs with clear descriptions
   - Ensure CI passes before merging

3. **Testing**
   - Write unit tests for new functionality
   - Include integration tests where appropriate
   - Ensure all tests pass locally before pushing

4. **Documentation**
   - Document public APIs
   - Keep README updated
   - Include examples for new features

### Running the CLI

```bash
# Create a new element
cargo run -p agentic_cli -- create-element --metadata '{"key": "value"}'

# List elements
cargo run -p agentic_cli -- list-elements
```

## Project Phases

1. **Phase 1** (Current): Core primitives and basic CLI
2. **Phase 2**: Microkernel orchestrator and system events
3. **Phase 3**: Event sourcing and explainability
4. **Phase 4**: Secure plugin system
5. **Phase 5**: Multi-agent support and advanced features

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

[Insert your chosen license here]

## Acknowledgments

- [List any acknowledgments, inspirations, or references]