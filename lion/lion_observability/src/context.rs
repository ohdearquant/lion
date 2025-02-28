//! Context propagation for observability
//!
//! This module provides structures and methods for maintaining
//! observability context across async boundaries and between plugins.

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use thread_local::ThreadLocal;

/// Static thread-local storage for the current context
static CURRENT_CONTEXT: Lazy<ThreadLocal<RwLock<Option<Context>>>> = Lazy::new(ThreadLocal::new);

/// Span context for tracing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanContext {
    /// Trace ID
    pub trace_id: String,

    /// Span ID
    pub span_id: String,

    /// Parent span ID
    pub parent_span_id: Option<String>,

    /// Whether the span is sampled
    pub sampled: bool,

    /// Span name
    pub name: String,

    /// Additional baggage items
    pub baggage: HashMap<String, String>,
}

impl SpanContext {
    /// Create a new root span context
    pub fn new_root(name: impl Into<String>) -> Self {
        let trace_id = generate_id();
        let span_id = generate_id();

        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            sampled: true,
            name: name.into(),
            baggage: HashMap::new(),
        }
    }

    /// Create a child span context
    pub fn new_child(&self, name: impl Into<String>) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: generate_id(),
            parent_span_id: Some(self.span_id.clone()),
            sampled: self.sampled,
            name: name.into(),
            baggage: self.baggage.clone(),
        }
    }

    /// Add baggage item
    pub fn with_baggage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.baggage.insert(key.into(), value.into());
        self
    }

    /// Convert to W3C TraceContext format
    pub fn to_w3c(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id,
            self.span_id,
            if self.sampled { 1 } else { 0 }
        )
    }

    /// Parse from W3C TraceContext format
    pub fn from_w3c(traceparent: &str, name: impl Into<String>) -> Option<Self> {
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() < 4 || parts[0] != "00" {
            return None;
        }

        let trace_id = parts[1].to_string();
        let span_id = parts[2].to_string();
        let sampled = parts[3].starts_with('1');

        Some(Self {
            trace_id,
            span_id,
            parent_span_id: None, // Not included in traceparent
            sampled,
            name: name.into(),
            baggage: HashMap::new(),
        })
    }
}

/// Generate a random ID for tracing
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    format!("{:016x}", now)
}

/// Context for observability
#[derive(Debug, Clone)]
pub struct Context {
    /// Identifier for the context
    pub id: String,

    /// Plugin ID
    pub plugin_id: Option<String>,

    /// Request ID
    pub request_id: Option<String>,

    /// Current span context
    pub span_context: Option<SpanContext>,

    /// Additional attributes
    attributes: HashMap<String, Arc<dyn ContextValue>>,
}

/// Trait for context values
pub trait ContextValue: Send + Sync + fmt::Debug {
    /// Convert to string for display
    fn to_string_value(&self) -> String;

    /// Convert to JSON value
    fn to_json_value(&self) -> serde_json::Value;
}

impl<T: Send + Sync + fmt::Debug + fmt::Display + Clone + 'static> ContextValue for T {
    fn to_string_value(&self) -> String {
        format!("{}", self)
    }

    fn to_json_value(&self) -> serde_json::Value {
        serde_json::Value::String(format!("{}", self))
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            id: generate_id(),
            plugin_id: None,
            request_id: None,
            span_context: None,
            attributes: HashMap::new(),
        }
    }
}

impl Context {
    /// Create a new context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the plugin ID
    pub fn with_plugin_id(mut self, plugin_id: impl Into<String>) -> Self {
        self.plugin_id = Some(plugin_id.into());
        self
    }

    /// Set the request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set the span context
    pub fn with_span_context(mut self, span_context: SpanContext) -> Self {
        self.span_context = Some(span_context);
        self
    }

    /// Add an attribute
    pub fn with_attribute<T: Into<String>, V: ContextValue + 'static>(
        mut self,
        key: T,
        value: V,
    ) -> Self {
        self.attributes.insert(key.into(), Arc::new(value));
        self
    }

    /// Get an attribute
    pub fn attribute<T: AsRef<str>>(&self, key: T) -> Option<&Arc<dyn ContextValue>> {
        self.attributes.get(key.as_ref())
    }

    /// Create a child context with a new span
    pub fn create_child_span(&self, name: impl Into<String>) -> Self {
        let mut child = self.clone();

        // Create child span if parent has a span context
        if let Some(parent_span_ctx) = &self.span_context {
            child.span_context = Some(parent_span_ctx.new_child(name));
        } else {
            // Create new root span if parent has no span context
            child.span_context = Some(SpanContext::new_root(name));
        }

        child
    }

    /// Get the current context from thread-local storage
    pub fn current() -> Option<Self> {
        CURRENT_CONTEXT.get_or(|| RwLock::new(None)).read().clone()
    }

    /// Set the current context in thread-local storage
    pub fn set_current(context: Option<Self>) {
        if let Some(local) = CURRENT_CONTEXT.get() {
            *local.write() = context;
        } else {
            // Initialize with a new RwLock and then get it to set the value
            let local = CURRENT_CONTEXT.get_or(|| RwLock::new(None));
            *local.write() = context;
        }
    }

    /// Execute a function with this context as the current context
    pub fn with_current<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let prev = Self::current();
        Self::set_current(Some(self.clone()));
        let result = f();
        Self::set_current(prev);
        result
    }

    /// Execute a function with no context
    pub fn with_none<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let prev = Self::current();
        Self::set_current(None);
        let result = f();
        Self::set_current(prev);
        result
    }

    /// Create a map of attributes for logging/tracing
    pub fn to_attribute_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();

        if let Some(plugin_id) = &self.plugin_id {
            map.insert("plugin_id".to_string(), plugin_id.clone());
        }

        if let Some(request_id) = &self.request_id {
            map.insert("request_id".to_string(), request_id.clone());
        }

        if let Some(span_ctx) = &self.span_context {
            map.insert("trace_id".to_string(), span_ctx.trace_id.clone());
            map.insert("span_id".to_string(), span_ctx.span_id.clone());
            if let Some(parent_id) = &span_ctx.parent_span_id {
                map.insert("parent_span_id".to_string(), parent_id.clone());
            }
        }

        for (key, value) in &self.attributes {
            map.insert(key.clone(), value.to_string_value());
        }

        map
    }

    /// Export to W3C TraceContext format
    pub fn to_trace_context(&self) -> Option<String> {
        self.span_context.as_ref().map(|ctx| ctx.to_w3c())
    }

    /// Import from W3C TraceContext format
    pub fn from_trace_context(traceparent: &str, name: impl Into<String>) -> Self {
        let mut context = Self::new();
        if let Some(span_ctx) = SpanContext::from_w3c(traceparent, name) {
            context.span_context = Some(span_ctx);
        }
        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_context() {
        let root = SpanContext::new_root("root");
        assert_eq!(root.name, "root");
        assert!(root.parent_span_id.is_none());

        let child = root.new_child("child");
        assert_eq!(child.name, "child");
        assert_eq!(child.trace_id, root.trace_id);
        assert_eq!(child.parent_span_id, Some(root.span_id.clone()));
    }

    #[test]
    fn test_w3c_conversion() {
        let root = SpanContext::new_root("root");
        let w3c = root.to_w3c();

        assert!(w3c.starts_with("00-"));

        let parsed = SpanContext::from_w3c(&w3c, "parsed").unwrap();
        assert_eq!(parsed.trace_id, root.trace_id);
        assert_eq!(parsed.span_id, root.span_id);
        assert_eq!(parsed.name, "parsed");
    }

    #[test]
    fn test_context_thread_local() {
        let ctx1 = Context::new().with_plugin_id("plugin1");

        // Set and get current
        Context::set_current(Some(ctx1.clone()));
        let current = Context::current().unwrap();
        assert_eq!(current.plugin_id, Some("plugin1".to_string()));

        // With current function
        let ctx2 = Context::new().with_plugin_id("plugin2");
        ctx2.with_current(|| {
            let inner = Context::current().unwrap();
            assert_eq!(inner.plugin_id, Some("plugin2".to_string()));
        });

        // Verify original context was restored
        let after = Context::current().unwrap();
        assert_eq!(after.plugin_id, Some("plugin1".to_string()));
    }

    #[test]
    fn test_child_context() {
        let parent = Context::new()
            .with_plugin_id("plugin1")
            .with_span_context(SpanContext::new_root("parent"));

        let child = parent.create_child_span("child");

        assert_eq!(child.plugin_id, parent.plugin_id);

        let parent_span = parent.span_context.unwrap();
        let child_span = child.span_context.unwrap();

        assert_eq!(child_span.trace_id, parent_span.trace_id);
        assert_eq!(child_span.parent_span_id, Some(parent_span.span_id));
        assert_eq!(child_span.name, "child");
    }
}
