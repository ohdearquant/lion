# Lion Capability

`lion_capability` is a capability-based security framework for the Lion
microkernel system, providing fine-grained access control with attenuation,
filtering, and partial revocation.

## Features

- **Capability Tokens**: Unforgeable tokens representing permissions to access
  resources
- **Capability Attenuation**: Narrowing capabilities to restrict access
- **Capability Composition**: Combining multiple capabilities into a single
  composite
- **Partial Revocation**: Selectively revoking specific aspects of a capability
- **Capability Filtering**: Applying constraints to capabilities
- **In-Memory Store**: Efficient storage and retrieval of capabilities

## Architecture

The capability system is built around several key components:

1. **Access Requests**: Structured requests for resource access (files, network,
   etc.)
2. **Capability Types**: Specialized capabilities for different resource types
   - `FlCap` (File Capability): Controls access to file system resources
   - `NCap` (Network Capability): Controls network access
   - `CCap` (Composite Capability): Combines multiple capabilities
3. **Attenuation Mechanisms**: Ways to restrict capabilities
   - Constraints: Additional restrictions on capability usage
   - Filters: Dynamic filtering of access requests
   - Proxies: Mediated access through capability proxies

## Usage

### Creating File Capabilities

```rust
use lion_capability::file::FileCapability;
use lion_capability::types::AccessMode;

// Create a file capability for read-only access to a specific path
let file_cap = FileCapability::new("/tmp/example", AccessMode::READ);

// Create a file capability for read-write access to a directory
let dir_cap = FileCapability::new("/var/data", AccessMode::READ | AccessMode::WRITE);
```

### Attenuating Capabilities

```rust
use lion_capability::attenuation::{Attenuator, PathConstraint};

// Create a more restricted capability
let restricted_cap = file_cap.attenuate(
    PathConstraint::new("/tmp/example/logs")
);

// Check if the capability allows access
let request = AccessRequest::file_read("/tmp/example/logs/app.log");
assert!(restricted_cap.check(&request).is_ok());

// This would fail - outside the constrained path
let request = AccessRequest::file_read("/tmp/example/config.json");
assert!(restricted_cap.check(&request).is_err());
```

### Combining Capabilities

```rust
use lion_capability::combine::CompositeCapability;

// Create a composite capability
let composite = CompositeCapability::new()
    .add(file_cap)
    .add(network_cap);

// Check if the composite allows access
let request = AccessRequest::file_read("/tmp/example.txt");
assert!(composite.check(&request).is_ok());
```

## Integration with Other Lion Crates

The capability system integrates with other Lion crates:

- **lion_core**: For core types and traits
- **lion_policy**: For policy-based capability enforcement
- **lion_runtime**: For capability management at runtime

## License

Licensed under the Apache License, Version 2.0 - see the
[LICENSE](../../LICENSE) file for details.
