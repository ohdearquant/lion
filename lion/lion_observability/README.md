# Lion Observability

A comprehensive observability framework for the Lion microkernel, providing structured logging, distributed tracing, and metrics collection with capability-based security integration.

## Features

- **Structured Logging**: JSON-structured logs with context propagation
- **Distributed Tracing**: OpenTelemetry-compatible distributed tracing
- **Metrics Collection**: Lightweight performance metrics with Prometheus export
- **Security Integration**: Capability-based access to observability features
- **Context Propagation**: Trace context propagation across plugin boundaries
- **Plugin-specific Observability**: Isolated observability for each plugin
- **Performance Focus**: Low overhead (target 5-10%)

## Architecture

The Lion Observability system follows these design principles:

1. **Security First**: All observability features require explicit capabilities
2. **Performance**: Minimal overhead with efficient implementations
3. **Modularity**: Clean separation of logging, metrics, and tracing
4. **Integration**: Seamless integration with Lion's capability and plugin systems

## Example Usage

```rust
// Create observability configuration
let config = ObservabilityConfig::default();

// Create capability checker
let checker = SelectiveCapabilityChecker::new("demo_checker")
    .allow("plugin1", ObservabilityCapability::Log(LogLevel::Info))
    .allow("plugin1", ObservabilityCapability::Metrics);

// Initialize observability system
let obs = Observability::new(config)?.with_capability_checker(checker);

// Create plugin-specific observability
let plugin_obs = obs.create_plugin_observability("plugin1");

// Use plugin-specific observability
plugin_obs.info("Plugin started")?;

// Create a counter metric
let counter = plugin_obs.counter(
    "requests_total",
    "Total number of requests",
    HashMap::new(),
)?;

// Increment the counter
counter.increment(1)?;

// Create a span for tracing
plugin_obs.with_span("process_request", || {
    // Do some work
    plugin_obs.info("Processing request").unwrap();
    
    // Add an event to the span
    let event = TracingEvent::new("request_processed")
        .with_attribute("status", "success").unwrap();
    plugin_obs.add_span_event(event).unwrap();
    
    // Set span status
    plugin_obs.set_span_status(SpanStatus::Ok).unwrap();
    
    // Return a result
    "processed"
})?;

// Shutdown observability
obs.shutdown()?;
```

## Capability-Based Security

The observability system integrates with Lion's capability-based security model. Each observability operation requires a specific capability:

- `Log(level)`: Capability to write logs at the specified level
- `Tracing`: Capability to create and record spans
- `Metrics`: Capability to record metrics
- `ViewLogs`: Capability to view logs from other plugins
- `ViewTraces`: Capability to view traces from other plugins
- `ViewMetrics`: Capability to view metrics from other plugins
- `Configure`: Capability to configure observability settings

## Configuration

The observability system is highly configurable:

```rust
// Create custom configuration
let config = ObservabilityConfig {
    logging: LoggingConfig {
        enabled: true,
        level: "info".to_string(),
        structured: true,
        file_path: Some(PathBuf::from("/var/log/lion/lion.log")),
        ..LoggingConfig::default()
    },
    tracing: TracingConfig {
        enabled: true,
        sampling_rate: 0.01, // 1% sampling
        export_enabled: true,
        collector_endpoint: Some("http://localhost:4317".to_string()),
        ..TracingConfig::default()
    },
    metrics: MetricsConfig {
        enabled: true,
        prometheus_enabled: true,
        prometheus_endpoint: "0.0.0.0:9090".to_string(),
        ..MetricsConfig::default()
    },
    enforce_capabilities: true,
    ..ObservabilityConfig::default()
};
```

## Context Propagation

The observability system provides context propagation across plugin boundaries:

```rust
// Create a context with plugin ID
let context = Context::new().with_plugin_id("plugin1");

// Run with this context
context.with_current(|| {
    // Create a span
    tracer.with_span("operation", || {
        // Log inside the span
        logger.info("Inside span").unwrap();
        
        // Context is automatically propagated
        let current = Context::current().unwrap();
        assert_eq!(current.plugin_id.as_deref(), Some("plugin1"));
    }).unwrap();
});
```

## Integration with Other Lion Crates

The observability system integrates with other Lion crates:

- **lion_capability**: For capability-based security
- **lion_core**: For core Lion types and interfaces
- **lion_isolation**: For plugin isolation and lifecycle management

## Examples

See the `examples` directory for complete examples:

- **plugin_demo.rs**: Demonstrates how to use observability in a plugin system

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.