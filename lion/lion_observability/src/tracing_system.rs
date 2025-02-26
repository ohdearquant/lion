//! Distributed tracing system for observability
//!
//! This module provides OpenTelemetry-compatible distributed tracing
//! with context propagation and capability-based access control.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use opentelemetry::trace::{TraceContextExt, TracerProvider};
use opentelemetry::Key;
use opentelemetry_sdk::trace::Config;
use opentelemetry_sdk::Resource;
use serde::{Deserialize, Serialize};

use crate::capability::{ObservabilityCapability, ObservabilityCapabilityChecker};
use crate::config::{TracePropagation, TracingConfig};
use crate::context::{Context, SpanContext};
use crate::error::ObservabilityError;
use crate::Result;

/// Create a tracer based on the configuration
pub fn create_tracer(config: &TracingConfig) -> Result<impl Tracer> {
    if !config.enabled {
        return Ok(NoopTracer::new());
    }

    let tracer = OTelTracer::new(config)?;
    Ok(tracer)
}

/// A tracing event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingEvent {
    /// Event name
    pub name: String,

    /// Timestamp
    pub timestamp: SystemTime,

    /// Attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

impl TracingEvent {
    /// Create a new tracing event
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timestamp: SystemTime::now(),
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute to the event
    pub fn with_attribute<K: Into<String>, V: Serialize>(
        mut self,
        key: K,
        value: V,
    ) -> Result<Self> {
        let json_value = serde_json::to_value(value)?;
        self.attributes.insert(key.into(), json_value);
        Ok(self)
    }

    /// Add multiple attributes to the event
    pub fn with_attributes<K: Into<String>, V: Serialize>(
        mut self,
        attributes: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self> {
        for (key, value) in attributes {
            let json_value = serde_json::to_value(value)?;
            self.attributes.insert(key.into(), json_value);
        }
        Ok(self)
    }
}

/// A span for tracing
#[derive(Debug)]
pub struct Span {
    /// Span name
    pub name: String,

    /// Span context
    pub context: SpanContext,

    /// Span start time
    pub start_time: SystemTime,

    /// Span attributes
    pub attributes: HashMap<String, serde_json::Value>,

    /// Span events
    pub events: Vec<TracingEvent>,

    /// Whether the span is completed
    pub is_completed: bool,

    /// End time if completed
    pub end_time: Option<SystemTime>,

    /// Status
    pub status: SpanStatus,
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// Unset status
    Unset,
    /// OK status
    Ok,
    /// Error status
    Error,
}

impl Span {
    /// Create a new span
    pub fn new(name: impl Into<String>, context: SpanContext) -> Self {
        Self {
            name: name.into(),
            context,
            start_time: SystemTime::now(),
            attributes: HashMap::new(),
            events: Vec::new(),
            is_completed: false,
            end_time: None,
            status: SpanStatus::Unset,
        }
    }

    /// Add an attribute to the span
    pub fn with_attribute<K: Into<String>, V: Serialize>(
        mut self,
        key: K,
        value: V,
    ) -> Result<Self> {
        let json_value = serde_json::to_value(value)?;
        self.attributes.insert(key.into(), json_value);
        Ok(self)
    }

    /// Add multiple attributes to the span
    pub fn with_attributes<K: Into<String>, V: Serialize>(
        mut self,
        attributes: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self> {
        for (key, value) in attributes {
            let json_value = serde_json::to_value(value)?;
            self.attributes.insert(key.into(), json_value);
        }
        Ok(self)
    }

    /// Add an event to the span
    pub fn add_event(&mut self, event: TracingEvent) {
        self.events.push(event);
    }

    /// Set the span status
    pub fn set_status(&mut self, status: SpanStatus) {
        self.status = status;
    }

    /// End the span
    pub fn end(&mut self) {
        if !self.is_completed {
            self.is_completed = true;
            self.end_time = Some(SystemTime::now());
        }
    }

    /// Get the duration of the span
    pub fn duration(&self) -> Duration {
        let end = self.end_time.unwrap_or_else(SystemTime::now);
        end.duration_since(self.start_time).unwrap_or_default()
    }
}

/// Tracer trait for distributed tracing
pub trait Tracer: Send + Sync {
    /// Create a new span
    fn create_span(&self, name: impl Into<String>) -> Result<Span>;

    /// Create a child span from a parent span context
    fn create_child_span(
        &self,
        name: impl Into<String>,
        parent_context: &SpanContext,
    ) -> Result<Span>;

    /// Start a span and execute a function in its context
    fn with_span<F, R>(&self, name: impl Into<String>, f: F) -> Result<R>
    where
        F: FnOnce() -> R;

    /// Record a span
    fn record_span(&self, span: Span) -> Result<()>;

    /// Add an event to the current span
    fn add_event(&self, event: TracingEvent) -> Result<()>;

    /// Set status on the current span
    fn set_status(&self, status: SpanStatus) -> Result<()>;

    /// Get the current span context
    fn current_span_context(&self) -> Option<SpanContext>;

    /// Shutdown the tracer
    fn shutdown(&self) -> Result<()>;

    /// Get the name of the tracer
    fn name(&self) -> &str;
}

/// Tracer implementation using OpenTelemetry
pub struct OTelTracer {
    name: String,
    initialized: AtomicBool,
    config: TracingConfig,
    tracer: Option<opentelemetry::sdk::trace::Tracer>,
}

impl OTelTracer {
    /// Create a new OpenTelemetry tracer
    pub fn new(config: &TracingConfig) -> Result<Self> {
        Ok(Self {
            name: "otel_tracer".to_string(),
            initialized: AtomicBool::new(false),
            config: config.clone(),
            tracer: None,
        })
    }

    /// Initialize the OpenTelemetry tracer
    fn initialize(&self) -> Result<opentelemetry::sdk::trace::Tracer> {
        let resource = Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", "lion"),
            opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ]);

        let config = Config::default()
            .with_resource(resource)
            .with_max_events_per_span(self.config.max_events_per_span)
            .with_max_attributes_per_span(self.config.max_attributes_per_span)
            .with_max_links_per_span(self.config.max_links_per_span);

        let mut provider_builder =
            opentelemetry_sdk::trace::TracerProvider::builder().with_config(config);

        // Add sampling if configured
        if self.config.sampling_rate < 1.0 {
            let sampler = opentelemetry_sdk::trace::Sampler::ParentBased(Box::new(
                opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(self.config.sampling_rate),
            ));
            provider_builder = provider_builder.with_sampler(sampler);
        }

        // Add batch export if configured
        if self.config.export_enabled {
            if let Some(endpoint) = &self.config.collector_endpoint {
                // Add OTLP exporter
                let exporter = opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint)
                    .build_span_exporter()?;

                let batch_processor = opentelemetry_sdk::trace::BatchSpanProcessor::builder(
                    exporter,
                    opentelemetry_sdk::runtime::Tokio,
                )
                .with_max_queue_size(self.config.batch_size)
                .with_scheduled_delay(self.config.batch_timeout)
                .build();

                provider_builder = provider_builder.with_span_processor(batch_processor);
            }
        }

        let provider = provider_builder.build();
        let tracer = provider.tracer("lion_observability");

        // Set global propagator based on configuration
        match self.config.propagation {
            TracePropagation::W3C => {
                opentelemetry::global::set_text_map_propagator(
                    opentelemetry::sdk::propagation::TraceContextPropagator::new(),
                );
            }
            TracePropagation::B3Single => {
                // In a real implementation, we'd use a B3 propagator
                opentelemetry::global::set_text_map_propagator(
                    opentelemetry::sdk::propagation::TraceContextPropagator::new(),
                );
            }
            TracePropagation::B3Multi => {
                // In a real implementation, we'd use a B3 multi propagator
                opentelemetry::global::set_text_map_propagator(
                    opentelemetry::sdk::propagation::TraceContextPropagator::new(),
                );
            }
            TracePropagation::Jaeger => {
                // In a real implementation, we'd use a Jaeger propagator
                opentelemetry::global::set_text_map_propagator(
                    opentelemetry::sdk::propagation::TraceContextPropagator::new(),
                );
            }
            TracePropagation::Custom => {
                // Use default W3C
                opentelemetry::global::set_text_map_propagator(
                    opentelemetry::sdk::propagation::TraceContextPropagator::new(),
                );
            }
        }

        Ok(tracer)
    }

    /// Get the OpenTelemetry tracer instance
    fn get_tracer(&self) -> Result<opentelemetry::sdk::trace::Tracer> {
        if !self.initialized.load(Ordering::SeqCst) {
            // Initialize on first use
            let tracer = self.initialize()?;
            self.initialized.store(true, Ordering::SeqCst);
            Ok(tracer)
        } else if let Some(tracer) = &self.tracer {
            Ok(tracer.clone())
        } else {
            Err(ObservabilityError::TracingError(
                "Tracer not initialized".to_string(),
            ))
        }
    }

    /// Convert SpanContext to OpenTelemetry SpanContext
    fn to_otel_context(context: &SpanContext) -> opentelemetry::trace::SpanContext {
        // This is a simplified implementation - in a real system
        // we'd need proper conversion between ID formats
        let trace_id =
            opentelemetry::trace::TraceId::from_hex(&context.trace_id).unwrap_or_default();
        let span_id = opentelemetry::trace::SpanId::from_hex(&context.span_id).unwrap_or_default();
        let trace_flags = if context.sampled {
            opentelemetry::trace::TraceFlags::SAMPLED
        } else {
            opentelemetry::trace::TraceFlags::default()
        };

        opentelemetry::trace::SpanContext::new(
            trace_id,
            span_id,
            trace_flags,
            false,
            opentelemetry::trace::TraceState::default(),
        )
    }

    /// Convert OpenTelemetry SpanContext to SpanContext
    fn from_otel_context(
        context: &opentelemetry::trace::SpanContext,
        name: impl Into<String>,
    ) -> SpanContext {
        SpanContext {
            trace_id: context.trace_id().to_string(),
            span_id: context.span_id().to_string(),
            parent_span_id: None, // Not directly available in OTel context
            sampled: context.is_sampled(),
            name: name.into(),
            baggage: HashMap::new(),
        }
    }
}

impl Tracer for OTelTracer {
    fn create_span(&self, name: impl Into<String>) -> Result<Span> {
        let name_str = name.into();

        // Create a new span context
        let span_context = SpanContext::new_root(&name_str);

        // Add current context info to span attributes
        let mut span = Span::new(name_str, span_context);

        if let Some(ctx) = Context::current() {
            if let Some(plugin_id) = &ctx.plugin_id {
                span = span.with_attribute("plugin_id", plugin_id.clone())?;
            }

            if let Some(request_id) = &ctx.request_id {
                span = span.with_attribute("request_id", request_id.clone())?;
            }
        }

        Ok(span)
    }

    fn create_child_span(
        &self,
        name: impl Into<String>,
        parent_context: &SpanContext,
    ) -> Result<Span> {
        let name_str = name.into();

        // Create a child span context
        let span_context = parent_context.new_child(&name_str);

        // Add current context info to span attributes
        let mut span = Span::new(name_str, span_context);

        if let Some(ctx) = Context::current() {
            if let Some(plugin_id) = &ctx.plugin_id {
                span = span.with_attribute("plugin_id", plugin_id.clone())?;
            }

            if let Some(request_id) = &ctx.request_id {
                span = span.with_attribute("request_id", request_id.clone())?;
            }
        }

        Ok(span)
    }

    fn with_span<F, R>(&self, name: impl Into<String>, f: F) -> Result<R>
    where
        F: FnOnce() -> R,
    {
        // Create a new span
        let mut span = self.create_span(name)?;

        // Create a context with the span
        let mut ctx = Context::current().unwrap_or_default();
        ctx.span_context = Some(span.context.clone());

        // Execute the function in the context
        let result = ctx.with_current(f);

        // End the span
        span.end();
        self.record_span(span)?;

        Ok(result)
    }

    fn record_span(&self, span: Span) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let tracer = self.get_tracer()?;

        // Convert to OpenTelemetry span
        let mut otel_span = tracer
            .span_builder(&span.name)
            .with_start_time(span.start_time)
            .with_kind(opentelemetry::trace::SpanKind::Internal)
            .start(&tracer);

        // Add attributes
        for (key, value) in &span.attributes {
            match value {
                serde_json::Value::String(s) => {
                    otel_span.set_attribute(Key::new(key).string(s.clone()));
                }
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        otel_span.set_attribute(Key::new(key).i64(i));
                    } else if let Some(f) = n.as_f64() {
                        otel_span.set_attribute(Key::new(key).f64(f));
                    }
                }
                serde_json::Value::Bool(b) => {
                    otel_span.set_attribute(Key::new(key).bool(*b));
                }
                _ => {
                    // Use string conversion for other types
                    otel_span.set_attribute(Key::new(key).string(value.to_string()));
                }
            }
        }

        // Add events
        for event in &span.events {
            let mut attributes = Vec::new();
            for (k, v) in &event.attributes {
                match v {
                    serde_json::Value::String(s) => {
                        attributes.push(Key::new(k).string(s.clone()));
                    }
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            attributes.push(Key::new(k).i64(i));
                        } else if let Some(f) = n.as_f64() {
                            attributes.push(Key::new(k).f64(f));
                        }
                    }
                    serde_json::Value::Bool(b) => {
                        attributes.push(Key::new(k).bool(*b));
                    }
                    _ => {
                        attributes.push(Key::new(k).string(v.to_string()));
                    }
                }
            }

            otel_span.add_event(&event.name, attributes);
        }

        // Set status
        match span.status {
            SpanStatus::Unset => {}
            SpanStatus::Ok => {
                otel_span.set_status(opentelemetry::trace::Status::Ok);
            }
            SpanStatus::Error => {
                otel_span.set_status(opentelemetry::trace::Status::error("Error"));
            }
        }

        // End the span
        if let Some(end_time) = span.end_time {
            otel_span.end_with_timestamp(end_time);
        } else {
            otel_span.end();
        }

        Ok(())
    }

    fn add_event(&self, event: TracingEvent) -> Result<()> {
        // Get the current context
        let ctx = Context::current().ok_or_else(|| {
            ObservabilityError::TracingError("No active context for adding event".to_string())
        })?;

        // Get the current span context
        let span_ctx = ctx.span_context.ok_or_else(|| {
            ObservabilityError::TracingError("No active span for adding event".to_string())
        })?;

        // Get the OpenTelemetry tracer
        let tracer = self.get_tracer()?;

        // Convert to OpenTelemetry context
        let otel_ctx = Self::to_otel_context(&span_ctx);

        // Create a span context
        let span_ctx = opentelemetry::Context::new().with_remote_span_context(otel_ctx);

        // Add the event
        let mut attributes = Vec::new();
        for (key, value) in &event.attributes {
            match value {
                serde_json::Value::String(s) => {
                    attributes.push(Key::new(key).string(s.clone()));
                }
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        attributes.push(Key::new(key).i64(i));
                    } else if let Some(f) = n.as_f64() {
                        attributes.push(Key::new(key).f64(f));
                    }
                }
                serde_json::Value::Bool(b) => {
                    attributes.push(Key::new(key).bool(*b));
                }
                _ => {
                    attributes.push(Key::new(key).string(value.to_string()));
                }
            }
        }

        tracer
            .in_span("", |_cx| {})
            .add_event(&event.name, attributes);

        Ok(())
    }

    fn set_status(&self, status: SpanStatus) -> Result<()> {
        // Get the current context
        let ctx = Context::current().ok_or_else(|| {
            ObservabilityError::TracingError("No active context for setting status".to_string())
        })?;

        // Get the current span context
        let span_ctx = ctx.span_context.ok_or_else(|| {
            ObservabilityError::TracingError("No active span for setting status".to_string())
        })?;

        // Get the OpenTelemetry tracer
        let tracer = self.get_tracer()?;

        // Convert to OpenTelemetry context
        let otel_ctx = Self::to_otel_context(&span_ctx);

        // Create a span context
        let span_ctx = opentelemetry::Context::new().with_remote_span_context(otel_ctx);

        // Set the status
        match status {
            SpanStatus::Unset => {}
            SpanStatus::Ok => {
                tracer
                    .in_span("", |_cx| {})
                    .set_status(opentelemetry::trace::Status::Ok);
            }
            SpanStatus::Error => {
                tracer
                    .in_span("", |_cx| {})
                    .set_status(opentelemetry::trace::Status::error("Error"));
            }
        }

        Ok(())
    }

    fn current_span_context(&self) -> Option<SpanContext> {
        Context::current()?.span_context
    }

    fn shutdown(&self) -> Result<()> {
        // Flush and shutdown the OpenTelemetry tracer
        opentelemetry::global::shutdown_tracer_provider();
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Tracer implementation that discards all spans
#[derive(Debug, Clone)]
pub struct NoopTracer {
    name: String,
}

impl NoopTracer {
    /// Create a new noop tracer
    pub fn new() -> Self {
        Self {
            name: "noop_tracer".to_string(),
        }
    }
}

impl Tracer for NoopTracer {
    fn create_span(&self, name: impl Into<String>) -> Result<Span> {
        // Create a span that won't be recorded
        let span_context = SpanContext::new_root(name.into());
        Ok(Span::new(span_context.name.clone(), span_context))
    }

    fn create_child_span(
        &self,
        name: impl Into<String>,
        parent_context: &SpanContext,
    ) -> Result<Span> {
        let span_context = parent_context.new_child(name);
        Ok(Span::new(span_context.name.clone(), span_context))
    }

    fn with_span<F, R>(&self, name: impl Into<String>, f: F) -> Result<R>
    where
        F: FnOnce() -> R,
    {
        let mut span = self.create_span(name)?;
        let result = f();
        span.end();
        Ok(result)
    }

    fn record_span(&self, _span: Span) -> Result<()> {
        // Discard the span
        Ok(())
    }

    fn add_event(&self, _event: TracingEvent) -> Result<()> {
        // Discard the event
        Ok(())
    }

    fn set_status(&self, _status: SpanStatus) -> Result<()> {
        // Do nothing
        Ok(())
    }

    fn current_span_context(&self) -> Option<SpanContext> {
        Context::current()?.span_context
    }

    fn shutdown(&self) -> Result<()> {
        // Nothing to shut down
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Tracer implementation that enforces capability checks
pub struct CapabilityTracer {
    name: String,
    inner: Box<dyn Tracer>,
    checker: Arc<dyn ObservabilityCapabilityChecker>,
}

impl CapabilityTracer {
    /// Create a new capability tracer
    pub fn new(
        inner: impl Tracer + 'static,
        checker: Arc<dyn ObservabilityCapabilityChecker>,
    ) -> Self {
        Self {
            name: format!("capability_tracer({})", inner.name()),
            inner: Box::new(inner),
            checker,
        }
    }

    /// Check if the current plugin has the tracing capability
    fn check_capability(&self) -> Result<bool> {
        let plugin_id = Context::current()
            .and_then(|ctx| ctx.plugin_id)
            .unwrap_or_else(|| "unknown".to_string());

        self.checker
            .check_capability(&plugin_id, ObservabilityCapability::Tracing)
    }
}

impl Tracer for CapabilityTracer {
    fn create_span(&self, name: impl Into<String>) -> Result<Span> {
        // Check capability
        if !self.check_capability()? {
            return Err(ObservabilityError::CapabilityError(
                "Missing tracing capability".to_string(),
            ));
        }

        self.inner.create_span(name)
    }

    fn create_child_span(
        &self,
        name: impl Into<String>,
        parent_context: &SpanContext,
    ) -> Result<Span> {
        // Check capability
        if !self.check_capability()? {
            return Err(ObservabilityError::CapabilityError(
                "Missing tracing capability".to_string(),
            ));
        }

        self.inner.create_child_span(name, parent_context)
    }

    fn with_span<F, R>(&self, name: impl Into<String>, f: F) -> Result<R>
    where
        F: FnOnce() -> R,
    {
        // Check capability
        if !self.check_capability()? {
            return Err(ObservabilityError::CapabilityError(
                "Missing tracing capability".to_string(),
            ));
        }

        self.inner.with_span(name, f)
    }

    fn record_span(&self, span: Span) -> Result<()> {
        // Check capability
        if !self.check_capability()? {
            return Err(ObservabilityError::CapabilityError(
                "Missing tracing capability".to_string(),
            ));
        }

        self.inner.record_span(span)
    }

    fn add_event(&self, event: TracingEvent) -> Result<()> {
        // Check capability
        if !self.check_capability()? {
            return Err(ObservabilityError::CapabilityError(
                "Missing tracing capability".to_string(),
            ));
        }

        self.inner.add_event(event)
    }

    fn set_status(&self, status: SpanStatus) -> Result<()> {
        // Check capability
        if !self.check_capability()? {
            return Err(ObservabilityError::CapabilityError(
                "Missing tracing capability".to_string(),
            ));
        }

        self.inner.set_status(status)
    }

    fn current_span_context(&self) -> Option<SpanContext> {
        self.inner.current_span_context()
    }

    fn shutdown(&self) -> Result<()> {
        self.inner.shutdown()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Debug for CapabilityTracer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapabilityTracer")
            .field("name", &self.name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{AllowAllCapabilityChecker, DenyAllCapabilityChecker};

    #[test]
    fn test_span_creation() {
        let tracer = NoopTracer::new();
        let span = tracer.create_span("test_span").unwrap();

        assert_eq!(span.name, "test_span");
        assert!(!span.is_completed);
    }

    #[test]
    fn test_child_span() {
        let tracer = NoopTracer::new();
        let parent = tracer.create_span("parent").unwrap();
        let child = tracer.create_child_span("child", &parent.context).unwrap();

        assert_eq!(child.name, "child");
        assert_eq!(child.context.trace_id, parent.context.trace_id);
        assert_eq!(
            child.context.parent_span_id.as_deref(),
            Some(&parent.context.span_id)
        );
    }

    #[test]
    fn test_with_span() {
        let tracer = NoopTracer::new();
        let result = tracer.with_span("test_span", || 42).unwrap();

        assert_eq!(result, 42);
    }

    #[test]
    fn test_capability_tracer_allow() {
        let inner = NoopTracer::new();
        let checker = Arc::new(AllowAllCapabilityChecker);
        let tracer = CapabilityTracer::new(inner, checker);

        assert!(tracer.create_span("test").is_ok());
    }

    #[test]
    fn test_capability_tracer_deny() {
        let inner = NoopTracer::new();
        let checker = Arc::new(DenyAllCapabilityChecker);
        let tracer = CapabilityTracer::new(inner, checker);

        assert!(tracer.create_span("test").is_err());
    }
}
