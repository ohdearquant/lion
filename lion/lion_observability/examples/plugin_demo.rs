//! Example demonstrating how to use Lion Observability in a plugin system
//!
//! This example shows:
//! - Setting up the observability system
//! - Creating plugin-specific observability
//! - Using logging, tracing, and metrics
//! - Enforcing capability-based access control
//! - Context propagation across plugin boundaries

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use lion_observability::capability::{
    LogLevel, ObservabilityCapability, SelectiveCapabilityChecker,
};
use lion_observability::config::{
    LoggingConfig, MetricsConfig, ObservabilityConfig, TracingConfig,
};
use lion_observability::error::ObservabilityError;
use lion_observability::tracing_system::SpanStatus;
use lion_observability::tracing_system::Tracer;
use lion_observability::{Observability, Result};

fn main() -> Result<()> {
    println!("Starting Lion Observability Plugin Demo");

    // Create observability configuration
    let config = create_observability_config();

    // Create capability checker
    let checker = create_capability_checker();

    // Initialize observability system
    let obs = Observability::new(config)?.with_capability_checker(checker);

    // Create plugin manager
    let mut plugin_manager = ExamplePluginManager::new(obs.clone());

    // Register some plugins
    println!("Registering plugins...");
    plugin_manager.register_plugin("auth_plugin")?;
    plugin_manager.register_plugin("data_plugin")?;
    plugin_manager.register_plugin("ui_plugin")?;

    // Run a simulated workflow
    println!("Running simulated workflow...");
    plugin_manager.run_example_workflow()?;

    // Shutdown observability
    println!("Shutting down observability...");
    obs.shutdown()?;

    println!("Demo completed successfully");
    Ok(())
}

/// Create observability configuration
fn create_observability_config() -> ObservabilityConfig {
    // Customize logging configuration
    let logging = LoggingConfig {
        enabled: true,
        level: "debug".to_string(),
        structured: true,
        log_to_stdout: true,
        ..LoggingConfig::default()
    };

    // Customize tracing configuration
    let tracing = TracingConfig {
        enabled: true,
        sampling_rate: 1.0, // Sample all traces
        ..TracingConfig::default()
    };

    // Customize metrics configuration
    let metrics = MetricsConfig {
        enabled: true,
        prometheus_enabled: true,
        prometheus_endpoint: "127.0.0.1:9091".to_string(),
        ..MetricsConfig::default()
    };

    // Create master configuration
    ObservabilityConfig {
        logging,
        tracing,
        metrics,
        enforce_capabilities: true,
        ..ObservabilityConfig::default()
    }
}

/// Create capability checker
fn create_capability_checker() -> impl lion_observability::capability::ObservabilityCapabilityChecker
{
    SelectiveCapabilityChecker::new("demo_checker")
        // Auth plugin can log at all levels and use metrics
        .allow("auth_plugin", ObservabilityCapability::Log(LogLevel::Trace))
        .allow("auth_plugin", ObservabilityCapability::Metrics)
        // Data plugin can log at info and above, use metrics and tracing
        .allow("data_plugin", ObservabilityCapability::Log(LogLevel::Info))
        .allow("data_plugin", ObservabilityCapability::Metrics)
        .allow("data_plugin", ObservabilityCapability::Tracing)
        // UI plugin can only log at warn and above
        .allow("ui_plugin", ObservabilityCapability::Log(LogLevel::Warn))
}

/// Example plugin trait
#[allow(dead_code)]
trait Plugin: Send + Sync {
    /// Get the plugin ID
    fn id(&self) -> &str;

    /// Execute a plugin operation
    fn execute(&self, operation: &str, args: &[&str]) -> Result<String>;
}

/// Example plugin manager
struct ExamplePluginManager {
    observability: Observability,
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl ExamplePluginManager {
    /// Create a new plugin manager
    fn new(observability: Observability) -> Self {
        Self {
            observability,
            plugins: HashMap::new(),
        }
    }

    /// Register a plugin
    fn register_plugin(&mut self, plugin_id: &str) -> Result<()> {
        // Create plugin-specific observability
        let plugin_obs = self.observability.create_plugin_observability(plugin_id);

        // Create plugin instance
        let plugin: Box<dyn Plugin> = match plugin_id {
            "auth_plugin" => Box::new(AuthPlugin::new(plugin_obs)),
            "data_plugin" => Box::new(DataPlugin::new(plugin_obs)),
            "ui_plugin" => Box::new(UiPlugin::new(plugin_obs)),
            _ => {
                return Err(ObservabilityError::InvalidPluginId(format!(
                    "Unknown plugin: {}",
                    plugin_id
                )))
            }
        };

        // Register the plugin
        self.plugins.insert(plugin_id.to_string(), plugin);

        Ok(())
    }

    /// Execute a plugin operation
    fn execute_plugin(&self, plugin_id: &str, operation: &str, args: &[&str]) -> Result<String> {
        // Get the plugin
        let plugin = self.plugins.get(plugin_id).ok_or_else(|| {
            ObservabilityError::InvalidPluginId(format!("Plugin not found: {}", plugin_id))
        })?;

        // Execute the operation
        plugin.execute(operation, args)
    }

    /// Run an example workflow using multiple plugins
    fn run_example_workflow(&self) -> Result<()> {
        println!("Starting example workflow");

        // Create a root span for the workflow
        let observability = self.observability.clone();
        let tracer = observability.tracer();

        // Track workflow execution time
        let metrics = observability.metrics_registry();
        let workflow_histogram = metrics.histogram(
            "workflow_execution_time",
            "Time to execute the entire workflow",
            HashMap::new(),
        )?;
        let timer = workflow_histogram.start_timer();

        // Run workflow with tracing
        let result = tracer.with_span("example_workflow", || {
            // First, authenticate user
            println!("Step 1: Authenticating user");
            let auth_result = self
                .execute_plugin("auth_plugin", "authenticate", &["user123", "password"])
                .expect("Authentication failed");
            println!("  Auth result: {}", auth_result);

            // Next, fetch data
            println!("Step 2: Fetching data");
            let data_result = self
                .execute_plugin("data_plugin", "fetch_data", &["user123"])
                .expect("Data fetch failed");
            println!("  Data result: {}", data_result);

            // Finally, render UI
            println!("Step 3: Rendering UI");
            let ui_result = self
                .execute_plugin("ui_plugin", "render", &[&data_result])
                .expect("UI rendering failed");
            println!("  UI result: {}", ui_result);

            // Something might go wrong here
            if data_result.contains("error") {
                return Err(ObservabilityError::Unknown("Workflow error".to_string()));
            }

            Ok(ui_result)
        })?;

        // Record workflow duration
        let duration = timer.stop()?;
        println!(
            "Workflow completed in {:.2} ms with result: {:?}",
            duration.as_secs_f64() * 1000.0,
            result
        );

        Ok(())
    }
}

/// Example authentication plugin
struct AuthPlugin {
    obs: lion_observability::PluginObservability,
    request_counter: Arc<dyn lion_observability::metrics::Counter>,
}

impl AuthPlugin {
    fn new(obs: lion_observability::PluginObservability) -> Self {
        // Create metrics
        let request_counter = obs
            .counter(
                "auth_requests_total",
                "Total number of authentication requests",
                HashMap::new(),
            )
            .expect("Failed to create counter");

        Self {
            obs,
            request_counter,
        }
    }
}

impl Plugin for AuthPlugin {
    fn id(&self) -> &str {
        self.obs.plugin_id()
    }

    fn execute(&self, operation: &str, args: &[&str]) -> Result<String> {
        // Create context for tracing
        let ctx = self.obs.create_context();

        // Increment the request counter
        self.request_counter.increment(1)?;

        // Execute with context
        let result = ctx.with_current(|| {
            match operation {
                "authenticate" => {
                    if args.len() < 2 {
                        self.obs
                            .error("Missing username or password in authenticate call")?;
                        return Err(ObservabilityError::Unknown(
                            "Missing arguments for authentication".to_string(),
                        ));
                    }

                    let username = args[0];
                    let password = args[1];

                    // Log authentication attempt (at different levels to test capabilities)
                    self.obs
                        .trace(&format!("Auth attempt with password: {}", password))?;
                    self.obs
                        .debug(&format!("User {} attempting login", username))?;
                    self.obs
                        .info(&format!("Authentication request for user {}", username))?;

                    // Simulate authentication
                    if username == "user123" && password == "password" {
                        self.obs
                            .info(&format!("User {} authenticated successfully", username))?;
                        Ok(format!("auth_token_for_{}", username))
                    } else {
                        self.obs
                            .warn(&format!("Authentication failed for user {}", username))?;
                        Err(ObservabilityError::Unknown(
                            "Authentication failed".to_string(),
                        ))
                    }
                }
                _ => {
                    self.obs
                        .error(&format!("Unknown operation: {}", operation))?;
                    Err(ObservabilityError::Unknown(format!(
                        "Unknown operation: {}",
                        operation
                    )))
                }
            }
        });

        // Unwrap the nested Result
        match result {
            Ok(s) => Ok(s),
            Err(e) => Err(e),
        }
    }
}

/// Example data plugin
struct DataPlugin {
    obs: lion_observability::PluginObservability,
    data_gauge: Arc<dyn lion_observability::metrics::Gauge>,
}

impl DataPlugin {
    fn new(obs: lion_observability::PluginObservability) -> Self {
        // Create metrics
        let data_gauge = obs
            .gauge("data_size_bytes", "Size of data in bytes", HashMap::new())
            .expect("Failed to create gauge");

        Self { obs, data_gauge }
    }
}

impl Plugin for DataPlugin {
    fn id(&self) -> &str {
        self.obs.plugin_id()
    }

    fn execute(&self, operation: &str, args: &[&str]) -> Result<String> {
        // Create context for tracing
        let ctx = self.obs.create_context();

        // Execute with context
        let result = ctx.with_current(|| {
            match operation {
                "fetch_data" => {
                    if args.is_empty() {
                        self.obs.error("Missing user ID in fetch_data call")?;
                        return Err(ObservabilityError::Unknown(
                            "Missing user ID for data fetch".to_string(),
                        ));
                    }

                    let user_id = args[0];

                    // Create a span for the data fetching operation
                    self.obs.with_span("fetch_data", || {
                        // These logs should be visible due to capability settings
                        self.obs
                            .info(&format!("Fetching data for user {}", user_id))?;

                        // This should be invisible due to capability settings
                        let debug_result = self.obs.debug("Detailed debug info");
                        if debug_result.is_err() {
                            println!("  Expected: Debug log was blocked by capability checks");
                        }

                        // Simulate data fetching
                        std::thread::sleep(Duration::from_millis(50));

                        // Update metrics
                        let data_size = 1024 + (user_id.len() * 10);
                        self.data_gauge.set(data_size as f64)?;

                        // Add event to span
                        let event =
                            lion_observability::tracing_system::TracingEvent::new("data_loaded")
                                .with_attribute("bytes", data_size)?;
                        self.obs.add_span_event(event)?;

                        // Set success status
                        self.obs.set_span_status(SpanStatus::Ok)?;

                        Ok(format!(
                            "{{\"user\":\"{}\",\"data\":\"sample_data\"}}",
                            user_id
                        ))
                    })
                }
                _ => {
                    self.obs
                        .error(&format!("Unknown operation: {}", operation))?;
                    Err(ObservabilityError::Unknown(format!(
                        "Unknown operation: {}",
                        operation
                    )))
                }
            }
        });

        // Unwrap the nested Result
        match result {
            Ok(Ok(s)) => Ok(s),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(e),
        }
    }
}

/// Example UI plugin
struct UiPlugin {
    obs: lion_observability::PluginObservability,
}

impl UiPlugin {
    fn new(obs: lion_observability::PluginObservability) -> Self {
        Self { obs }
    }
}

impl Plugin for UiPlugin {
    fn id(&self) -> &str {
        self.obs.plugin_id()
    }

    fn execute(&self, operation: &str, args: &[&str]) -> Result<String> {
        // Create context for tracing
        let ctx = self.obs.create_context();

        // Execute with context
        let result = ctx.with_current(|| {
            match operation {
                "render" => {
                    if args.is_empty() {
                        self.obs.warn("Missing data in render call")?;
                        return Err(ObservabilityError::Unknown(
                            "Missing data for rendering".to_string(),
                        ));
                    }

                    let data = args[0];

                    // This should be invisible due to capability settings
                    let info_result = self.obs.info("Rendering UI with data");
                    if info_result.is_err() {
                        println!("  Expected: Info log was blocked by capability checks");
                    }

                    // This should be visible due to capability settings
                    self.obs.warn("UI rendering may be slow")?;

                    // Simulate rendering
                    std::thread::sleep(Duration::from_millis(30));

                    // Attempt to create a span (should fail due to capability settings)
                    let span_result = self.obs.start_span("render_span");
                    if span_result.is_err() {
                        println!("  Expected: Span creation was blocked by capability checks");
                    }

                    Ok(format!("Rendered UI with data: {}", data))
                }
                _ => {
                    self.obs
                        .error(&format!("Unknown operation: {}", operation))?;
                    Err(ObservabilityError::Unknown(format!(
                        "Unknown operation: {}",
                        operation
                    )))
                }
            }
        });

        // Unwrap the nested Result
        match result {
            Ok(s) => Ok(s),
            Err(e) => Err(e),
        }
    }
}
