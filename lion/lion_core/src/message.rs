//! Message passing system for inter-plugin communication.
//!
//! This module defines the message bus that enables plugins to communicate
//! with each other using a publish-subscribe pattern or direct messaging.

use crate::error::MessageError;
use crate::plugin::PluginId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Identifier for a topic in the pub-sub system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TopicId(pub String);

impl From<&str> for TopicId {
    fn from(s: &str) -> Self {
        TopicId(s.to_string())
    }
}

/// A message that can be sent between plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The plugin that sent the message
    pub sender: PluginId,
    
    /// Optional topic if this is a pub-sub message
    pub topic: Option<TopicId>,
    
    /// The content of the message
    pub content: serde_json::Value,
    
    /// Optional metadata
    pub metadata: serde_json::Map<String, serde_json::Value>,
    
    /// Timestamp when the message was created
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Message {
    /// Create a new message with the current timestamp
    pub fn new(
        sender: PluginId, 
        content: serde_json::Value
    ) -> Self {
        Self {
            sender,
            topic: None,
            content,
            metadata: serde_json::Map::new(),
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Create a new message for a specific topic
    pub fn new_topic(
        sender: PluginId, 
        topic: TopicId, 
        content: serde_json::Value
    ) -> Self {
        Self {
            sender,
            topic: Some(topic),
            content,
            metadata: serde_json::Map::new(),
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Add metadata to this message
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

/// The message bus interface for publishing and subscribing to messages
pub trait MessageBus: Send + Sync {
    /// Publish a message to a topic
    fn publish(
        &self, 
        sender: PluginId, 
        topic: TopicId, 
        content: serde_json::Value
    ) -> Result<(), MessageError>;
    
    /// Subscribe a plugin to a topic
    fn subscribe(
        &self, 
        plugin_id: PluginId, 
        topic: TopicId
    ) -> Result<(), MessageError>;
    
    /// Unsubscribe a plugin from a topic
    fn unsubscribe(
        &self, 
        plugin_id: PluginId, 
        topic: TopicId
    ) -> Result<(), MessageError>;
    
    /// Send a direct message to a specific plugin
    fn send_direct(
        &self, 
        sender: PluginId, 
        target: PluginId, 
        content: serde_json::Value
    ) -> Result<(), MessageError>;
    
    /// Check if there are pending messages for a plugin
    fn has_messages(&self, plugin_id: PluginId) -> bool;
    
    /// Get the next message for a plugin, if any
    fn next_message(&self, plugin_id: PluginId) -> Option<Message>;
    
    /// Returns self as Any for downcasting in advanced scenarios
    fn as_any(&self) -> &dyn std::any::Any;
}