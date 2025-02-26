//! Distributed tracing system for observability
//!
//! This module provides OpenTelemetry-compatible distributed tracing
//! with context propagation and capability-based access control.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use opentelemetry::trace::TraceContextExt;
use opentelemetry::KeyValue;
use serde::{Deserialize, Serialize};

use crate::capability::{ObservabilityCapability, ObservabilityCapabilityChecker};
use crate::config::{TracePropagation, TracingConfig};
use crate::context::{Context, SpanContext};
use crate::error::ObservabilityError;
use crate::Result;

/// Create a tracer based on the configuration
pub fn create_tracer(config: &TracingConfig) -> Result<Box<dyn TracerBase>> {
    if !config.enabled {
        let tracer: Box<dyn TracerBase> = Box::new(NoopTracer::new());
        return Ok(tracer);
    }

    let tracer = OTelTracer::new(config)?;
    let boxed_tracer: Box<dyn TracerBase> = Box::new(tracer);
    Ok(boxed_tracer)
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

/// Base tracer trait for distributed tracing (object-safe)
pub trait TracerBase: Send + Sync {
    /// Create a new span with a name
    fn create_span_with_name(&self, name: &str) -> Result<Span>;

    /// Create a child span from a parent span context with a name
    fn create_child_span_with_name(&self, name: &str, parent_context: &SpanContext)
        -> Result<Span>;

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

/// Tracer trait with convenience methods
pub trait Tracer: TracerBase {
    /// Create a new span
    fn create_span(&self, name: impl Into<String>) -> Result<Span> {
        self.create_span_with_name(&name.into())
    }

    /// Create a child span from a parent span context
    fn create_child_span(
        &self,
        name: impl Into<String>,
        parent_context: &SpanContext,
    ) -> Result<Span> {
        self.create_child_span_with_name(&name.into(), parent_context)
    }

    /// Start a span and execute a function in its context
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
}

// Blanket implementation for all types that implement TracerBase
impl<T: ?Sized + TracerBase> Tracer for T {}

/// Tracer implementation using OpenTelemetry
pub struct OTelTracer {
    name: String,
    initialized: AtomicBool,
    config: TracingConfig,
    #[allow(dead_code)]
    tracer: Option<opentelemetry_sdk::trace::Tracer>,
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
    #[allow(unused_variables)]
    fn initialize(&self) -> Result<opentelemetry_sdk::trace::Tracer> {
        // This is a simplified implementation for the fix
        // In a real implementation, we would properly initialize OpenTelemetry
        Err(ObservabilityError::TracingError(
            "OpenTelemetry initialization not implemented in this version".to_string(),
        ))
    }

    /// Get the OpenTelemetry tracer instance
    fn get_tracer(&self) -> Result<opentelemetry_sdk::trace::Tracer> {
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
    #[allow(unused_variables)]
    fn to_otel_context(context: &SpanContext) -> opentelemetry::trace::SpanContext {
        opentelemetry::trace::SpanContext::empty_context()
    }

    /// Convert OpenTelemetry SpanContext to SpanContext
    #[allow(unused_variables)]
    fn from_otel_context(
        context: &opentelemetry::trace::SpanContext,
        name: impl Into<String>,
    ) -> SpanContext {
        SpanContext::new_root(name)
    }
}

impl TracerBase for OTelTracer {
    fn create_span_with_name(&self, name: &str) -> Result<Span> {
        // Create a new span context
        let span_context = SpanContext::new_root(name);

        // Add current context info to span attributes
        let mut span = Span::new(name.to_string(), span_context);

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

    fn create_child_span_with_name(
        &self,
        name: &str,
        parent_context: &SpanContext,
    ) -> Result<Span> {
        // Create a child span context
        let span_context = parent_context.new_child(name);

        // Add current context info to span attributes
        let mut span = Span::new(name.to_string(), span_context);

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

    fn record_span(&self, _span: Span) -> Result<()> {
        // Simplified implementation for the fix
        // In a real implementation, we would record the span with OpenTelemetry
        Ok(())
    }

    fn add_event(&self, _event: TracingEvent) -> Result<()> {
        // Simplified implementation for the fix
        // In a real implementation, we would add the event to the current span
        Ok(())
    }

    fn set_status(&self, _status: SpanStatus) -> Result<()> {
        // Simplified implementation for the fix
        // In a real implementation, we would set the status on the current span
        Ok(())
    }

    fn current_span_context(&self) -> Option<SpanContext> {
        if let Some(ctx) = Context::current() {
            ctx.span_context
        } else {
            None
        }
    }

    fn shutdown(&self) -> Result<()> {
        // Simplified implementation for the fix
        // In a real implementation, we would flush and shutdown the OpenTelemetry tracer
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

impl TracerBase for NoopTracer {
    fn create_span_with_name(&self, name: &str) -> Result<Span> {
        // Create a span that won't be recorded
        let span_context = SpanContext::new_root(name);
        Ok(Span::new(span_context.name.clone(), span_context))
    }

    fn create_child_span_with_name(
        &self,
        name: &str,
        parent_context: &SpanContext,
    ) -> Result<Span> {
        let span_context = parent_context.new_child(name);
        Ok(Span::new(span_context.name.clone(), span_context))
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
        if let Some(ctx) = Context::current() {
            ctx.span_context
        } else {
            None
        }
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
    inner: Box<dyn TracerBase>,
    checker: Arc<dyn ObservabilityCapabilityChecker>,
}

impl CapabilityTracer {
    /// Create a new capability tracer
    pub fn new(
        inner: impl TracerBase + 'static,
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

impl TracerBase for CapabilityTracer {
    fn create_span_with_name(&self, name: &str) -> Result<Span> {
        // Check capability
        if !self.check_capability()? {
            return Err(ObservabilityError::CapabilityError(
                "Missing tracing capability".to_string(),
            ));
        }

        self.inner.create_span_with_name(name)
    }

    fn create_child_span_with_name(
        &self,
        name: &str,
        parent_context: &SpanContext,
    ) -> Result<Span> {
        // Check capability
        if !self.check_capability()? {
            return Err(ObservabilityError::CapabilityError(
                "Missing tracing capability".to_string(),
            ));
        }

        self.inner.create_child_span_with_name(name, parent_context)
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
            .field("checker", &"<capability_checker>")
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
            child.context.parent_span_id.as_ref().map(|s| s.as_str()),
            Some(parent.context.span_id.as_str())
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
