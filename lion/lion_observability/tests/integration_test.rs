use std::collections::HashMap;
use std::time::Duration;

use lion_observability::capability::{
    LogLevel, ObservabilityCapability, SelectiveCapabilityChecker,
};
use lion_observability::config::ObservabilityConfig;
use lion_observability::context::Context;
use lion_observability::logging::Logger;
use lion_observability::tracing_system::SpanStatus;
use lion_observability::tracing_system::{Tracer, TracerBase};
use lion_observability::{Observability, Result};

#[test]
fn test_observability_lifecycle() -> Result<()> {
    // Create minimal config for testing
    let config = ObservabilityConfig::minimal_for_testing();

    // Create observability
    let obs = Observability::new(config)?;

    // Test logger
    let logger = obs.logger();
    logger.info("Test info message")?;
    logger.error("Test error message")?;

    // Test tracer
    let tracer = obs.tracer();
    let mut span = tracer.create_span("test_span")?;
    span.end(); // End the span before recording it
    TracerBase::record_span(&*tracer, span)?; // Use the TracerBase trait method

    // Test metrics
    let metrics = obs.metrics_registry();
    let counter = metrics.counter("test_counter", "Test counter", HashMap::new())?;
    counter.increment(1)?;

    // Shutdown
    obs.shutdown()?;

    Ok(())
}

#[test]
fn test_plugin_observability() -> Result<()> {
    // Create minimal config for testing
    let config = ObservabilityConfig::minimal_for_testing();

    // Create observability
    let obs = Observability::new(config)?;

    // Create plugin observability
    let plugin_obs = obs.create_plugin_observability("test_plugin");

    // Test logging
    plugin_obs.info("Plugin info message")?;
    plugin_obs.error("Plugin error message")?;

    // Test metrics
    let counter = plugin_obs.counter("plugin_counter", "Plugin counter", HashMap::new())?;
    counter.increment(1)?;

    // Test tracing
    let ctx = plugin_obs.create_context();
    ctx.with_current(|| {
        // Test with_span pattern
        plugin_obs
            .with_span("plugin_operation", || {
                // This would contain plugin code
                plugin_obs.debug("Inside operation span").unwrap();

                // Set span status
                plugin_obs.set_span_status(SpanStatus::Ok).unwrap();

                // Return a value
                42
            })
            .unwrap()
    });

    // Shutdown
    obs.shutdown()?;

    Ok(())
}

#[test]
fn test_capability_checking() -> Result<()> {
    // Create minimal config for testing
    let mut config = ObservabilityConfig::minimal_for_testing();
    config.enforce_capabilities = true;

    // Create capability checker
    let checker = SelectiveCapabilityChecker::new("test_checker")
        .allow(
            "allowed_plugin",
            ObservabilityCapability::Log(LogLevel::Info),
        )
        .allow("allowed_plugin", ObservabilityCapability::Metrics)
        .allow("tracer_plugin", ObservabilityCapability::Tracing);

    // Create observability with capability checker
    let obs = Observability::new(config)?.with_capability_checker(checker);

    // Create plugin observability for allowed plugin
    let allowed_plugin = obs.create_plugin_observability("allowed_plugin");

    // Create plugin observability for denied plugin
    let denied_plugin = obs.create_plugin_observability("denied_plugin");

    // Create plugin observability for tracer plugin
    let tracer_plugin = obs.create_plugin_observability("tracer_plugin");

    // Test allowed plugin
    let ctx = allowed_plugin.create_context();
    ctx.with_current(|| {
        // Logging should work
        assert!(allowed_plugin.info("Allowed info").is_ok());

        // Debug should fail (only Info level is allowed)
        assert!(allowed_plugin.debug("Allowed debug").is_err());

        // Metrics should work
        let counter = allowed_plugin.counter("allowed_counter", "Allowed counter", HashMap::new());
        assert!(counter.is_ok());

        // Tracing should fail
        let span_result = allowed_plugin.start_span("allowed_span");
        assert!(span_result.is_err());
    });

    // Test denied plugin
    let ctx = denied_plugin.create_context();
    ctx.with_current(|| {
        // Logging should fail
        assert!(denied_plugin.info("Denied info").is_err());

        // Metrics should fail
        let counter = denied_plugin.counter("denied_counter", "Denied counter", HashMap::new());
        assert!(counter.is_err());

        // Tracing should fail
        let span_result = denied_plugin.start_span("denied_span");
        assert!(span_result.is_err());
    });

    // Test tracer plugin
    let ctx = tracer_plugin.create_context();
    ctx.with_current(|| {
        // Logging should fail
        assert!(tracer_plugin.info("Tracer info").is_err());

        // Metrics should fail
        let counter = tracer_plugin.counter("tracer_counter", "Tracer counter", HashMap::new());
        assert!(counter.is_err());

        // Tracing should work
        let span_result = tracer_plugin.start_span("tracer_span");
        assert!(span_result.is_ok());
    });

    // Shutdown
    obs.shutdown()?;

    Ok(())
}

#[test]
fn test_context_propagation() -> Result<()> {
    // Create minimal config for testing
    let config = ObservabilityConfig::minimal_for_testing();

    // Create observability
    let obs = Observability::new(config)?;

    // Create plugin observability
    let plugin_obs = obs.create_plugin_observability("context_plugin");

    // Create a context with plugin ID and span
    let context = plugin_obs.create_context();

    // Run with the context
    context.with_current(|| {
        // Create a span
        plugin_obs
            .with_span("outer_operation", || {
                // Log inside the span
                plugin_obs.info("Inside outer span").unwrap();

                // Create a nested span
                plugin_obs
                    .with_span("inner_operation", || {
                        // Log inside the nested span
                        plugin_obs.info("Inside inner span").unwrap();

                        // Check that the context is properly nested
                        let current_ctx = Context::current().unwrap();
                        assert_eq!(current_ctx.plugin_id.as_deref(), Some("context_plugin"));

                        // The span context should be present
                        assert!(current_ctx.span_context.is_some());

                        // The trace ID should remain the same
                        let outer_span_ctx = context.span_context.as_ref().unwrap();
                        let inner_span_ctx = current_ctx.span_context.as_ref().unwrap();
                        assert_eq!(outer_span_ctx.trace_id, inner_span_ctx.trace_id);

                        // But the span IDs should differ
                        assert_ne!(outer_span_ctx.span_id, inner_span_ctx.span_id);

                        // The parent span ID should be set
                        assert_eq!(
                            inner_span_ctx.parent_span_id.as_deref(),
                            Some(&outer_span_ctx.span_id).map(|s| s.as_str())
                        );
                    })
                    .unwrap();
            })
            .unwrap();
    });

    // Shutdown
    obs.shutdown()?;

    Ok(())
}

#[tokio::test]
async fn test_async_context_propagation() -> Result<()> {
    // Create minimal config for testing
    let config = ObservabilityConfig::minimal_for_testing();

    // Create observability
    let obs = Observability::new(config)?;

    // Create plugin observability
    let plugin_obs = obs.create_plugin_observability("async_plugin");

    // Create a context with plugin ID and clone for async use
    let context = plugin_obs.create_context();
    let cloned_context = context.clone();
    let plugin_obs_clone = plugin_obs.clone();

    // Start with context
    context.with_current(|| {
        // Create a span
        plugin_obs
            .with_span("outer_operation", || {
                // Capture span context
                let span_ctx = Context::current().unwrap().span_context.unwrap();

                // Spawn async task
                tokio::spawn(async move {
                    // Restore context in async task
                    let ctx = cloned_context.with_span_context(span_ctx);

                    ctx.with_current(|| {
                        // Log in async context
                        plugin_obs_clone.info("Inside async task").unwrap();

                        // Create a new span in the async context
                        plugin_obs_clone
                            .with_span("async_operation", || {
                                // Log inside the async span
                                plugin_obs_clone.info("Inside async span").unwrap();

                                // Check that the context is properly propagated
                                let current_ctx = Context::current().unwrap();
                                assert_eq!(current_ctx.plugin_id.as_deref(), Some("async_plugin"));

                                // The span context should be present
                                assert!(current_ctx.span_context.is_some());
                            })
                            .unwrap();
                    });
                });
            })
            .unwrap();
    });

    // Allow async task to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Shutdown
    obs.shutdown()?;

    Ok(())
}
