//! In-memory message bus implementation.

use crate::error::MessageBusError;
use crossbeam_channel::{bounded, Receiver, Sender};
use dashmap::DashMap;
use lion_core::capability::{CapabilityManager, CoreCapability};
use lion_core::message::{Message, MessageBus, TopicId};
use lion_core::plugin::PluginId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

/// Configuration for the in-memory message bus
#[derive(Clone)]
pub struct InMemoryMessageBusConfig {
    /// Maximum queue size for each plugin
    pub max_queue_size: usize,
    
    /// Maximum message size in bytes
    pub max_message_size: usize,
    
    /// Whether to check capabilities
    pub check_capabilities: bool,
}

impl Default for InMemoryMessageBusConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 100,
            max_message_size: 1024 * 1024, // 1 MB
            check_capabilities: true,
        }
    }
}

/// In-memory message bus implementation
pub struct InMemoryMessageBus {
    /// Configuration
    config: InMemoryMessageBusConfig,
    
    /// Capability manager for checking capabilities
    capability_manager: Option<Arc<dyn CapabilityManager>>,
    
    /// Topic subscriptions (topic -> list of plugin IDs)
    subscriptions: DashMap<TopicId, Vec<PluginId>>,
    
    /// Message queues for each plugin (plugin ID -> sender)
    message_queues: DashMap<PluginId, Sender<Message>>,
    
    /// Message receivers for each plugin (plugin ID -> receiver)
    message_receivers: DashMap<PluginId, Receiver<Message>>,
}

impl InMemoryMessageBus {
    /// Create a new in-memory message bus
    pub fn new(
        config: InMemoryMessageBusConfig,
        capability_manager: Option<Arc<dyn CapabilityManager>>,
    ) -> Self {
        Self {
            config,
            capability_manager,
            subscriptions: DashMap::new(),
            message_queues: DashMap::new(),
            message_receivers: DashMap::new(),
        }
    }
    
    /// Create a default in-memory message bus
    pub fn default_bus() -> Arc<dyn MessageBus> {
        Arc::new(Self::new(InMemoryMessageBusConfig::default(), None))
    }
    
    /// Create a new in-memory message bus with a capability manager
    pub fn with_capability_manager(
        config: InMemoryMessageBusConfig,
        capability_manager: Arc<dyn CapabilityManager>,
    ) -> Self {
        Self::new(config, Some(capability_manager))
    }
    
    /// Get or create a message queue for a plugin
    fn get_or_create_queue(&self, plugin_id: PluginId) -> (Sender<Message>, Receiver<Message>) {
        if let Some(sender) = self.message_queues.get(&plugin_id) {
            // Return existing queue
            let receiver = self.message_receivers
                .get(&plugin_id)
                .expect("Receiver should exist if sender exists");
            (sender.clone(), receiver.clone())
        } else {
            // Create a new queue
            let (sender, receiver) = bounded(self.config.max_queue_size);
            self.message_queues.insert(plugin_id, sender.clone());
            self.message_receivers.insert(plugin_id, receiver.clone());
            (sender, receiver)
        }
    }
    
    /// Check if a plugin has the necessary capability for messaging
    fn check_messaging_capability(&self, plugin_id: PluginId) -> Result<(), MessageBusError> {
        if !self.config.check_capabilities {
            return Ok(());
        }
        
        if let Some(capability_manager) = &self.capability_manager {
            if !capability_manager.has_capability(plugin_id, &CoreCapability::InterPluginComm) {
                return Err(MessageBusError::PermissionDenied(
                    "Plugin does not have InterPluginComm capability".to_string(),
                ));
            }
        }
        
        Ok(())
    }
}

impl MessageBus for InMemoryMessageBus {
    fn publish(
        &self,
        sender: PluginId,
        topic: TopicId,
        content: serde_json::Value,
    ) -> Result<(), lion_core::error::MessageError> {
        // Check if the sender has the necessary capability
        self.check_messaging_capability(sender)
            .map_err(|e| e.into())?;
        
        // Check if the content is too large
        let content_size = serde_json::to_string(&content)
            .map_err(|e| lion_core::error::MessageError::FormatError(e.to_string()))?
            .len();
        
        if content_size > self.config.max_message_size {
            return Err(lion_core::error::MessageError::FormatError(
                "Message too large".to_string(),
            ));
        }
        
        // Create a message
        let message = Message::new_topic(sender, topic.clone(), content);
        
        // Get subscribers for this topic
        let subscribers = self.subscriptions
            .get(&topic)
            .map(|s| s.clone())
            .unwrap_or_default();
        
        // Deliver the message to each subscriber
        for subscriber in subscribers {
            if let Some(sender) = self.message_queues.get(&subscriber) {
                if sender.try_send(message.clone()).is_err() {
                    log::warn!("Message queue full for plugin {}", subscriber.0);
                    // Continue with other subscribers
                }
            }
        }
        
        Ok(())
    }
    
    fn subscribe(
        &self,
        plugin_id: PluginId,
        topic: TopicId,
    ) -> Result<(), lion_core::error::MessageError> {
        // Check if the plugin has the necessary capability
        self.check_messaging_capability(plugin_id)
            .map_err(|e| e.into())?;
        
        // Create a message queue for the plugin if it doesn't exist
        self.get_or_create_queue(plugin_id);
        
        // Add the plugin to the topic's subscribers
        self.subscriptions
            .entry(topic)
            .or_insert_with(Vec::new)
            .push(plugin_id);
        
        Ok(())
    }
    
    fn unsubscribe(
        &self,
        plugin_id: PluginId,
        topic: TopicId,
    ) -> Result<(), lion_core::error::MessageError> {
        // Get subscribers for this topic
        let mut subscribers = self.subscriptions
            .get_mut(&topic)
            .ok_or_else(|| lion_core::error::MessageError::NoSuchTopic)?;
        
        // Remove the plugin from the subscribers
        subscribers.retain(|&id| id != plugin_id);
        
        Ok(())
    }
    
    fn send_direct(
        &self,
        sender: PluginId,
        target: PluginId,
        content: serde_json::Value,
    ) -> Result<(), lion_core::error::MessageError> {
        // Check if the sender has the necessary capability
        self.check_messaging_capability(sender)
            .map_err(|e| e.into())?;
        
        // Check if the content is too large
        let content_size = serde_json::to_string(&content)
            .map_err(|e| lion_core::error::MessageError::FormatError(e.to_string()))?
            .len();
        
        if content_size > self.config.max_message_size {
            return Err(lion_core::error::MessageError::FormatError(
                "Message too large".to_string(),
            ));
        }
        
        // Create a message
        let message = Message::new(sender, content);
        
        // Get the target's message queue
        let sender = self.message_queues
            .get(&target)
            .ok_or_else(|| lion_core::error::MessageError::NoSuchPlugin)?;
        
        // Send the message
        sender
            .try_send(message)
            .map_err(|_| lion_core::error::MessageError::BusFull)?;
        
        Ok(())
    }
    
    fn has_messages(&self, plugin_id: PluginId) -> bool {
        if let Some(receiver) = self.message_receivers.get(&plugin_id) {
            !receiver.is_empty()
        } else {
            false
        }
    }
    
    fn next_message(&self, plugin_id: PluginId) -> Option<Message> {
        if let Some(receiver) = self.message_receivers.get(&plugin_id) {
            // Try to receive a message without blocking
            receiver.try_recv().ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lion_core::capability::CapabilityId;
    use std::sync::Arc;
    
    // A simple mock capability manager for testing
    struct MockCapabilityManager {
        // Map of plugin IDs to their capabilities
        capabilities: DashMap<PluginId, Vec<CoreCapability>>,
    }
    
    impl MockCapabilityManager {
        fn new() -> Self {
            Self {
                capabilities: DashMap::new(),
            }
        }
        
        fn grant(&self, plugin_id: PluginId, capability: CoreCapability) {
            self.capabilities
                .entry(plugin_id)
                .or_insert_with(Vec::new)
                .push(capability);
        }
    }
    
    impl CapabilityManager for MockCapabilityManager {
        fn has_capability(&self, plugin_id: PluginId, capability: &CoreCapability) -> bool {
            if let Some(capabilities) = self.capabilities.get(&plugin_id) {
                capabilities.iter().any(|cap| {
                    match (cap, capability) {
                        (
                            CoreCapability::FileSystemRead { path: cap_path },
                            CoreCapability::FileSystemRead { path: req_path },
                        ) => {
                            // Simplified check
                            cap_path.is_none() || req_path.is_none() ||
                            cap_path == req_path
                        }
                        (
                            CoreCapability::FileSystemWrite { path: cap_path },
                            CoreCapability::FileSystemWrite { path: req_path },
                        ) => {
                            // Simplified check
                            cap_path.is_none() || req_path.is_none() ||
                            cap_path == req_path
                        }
                        (
                            CoreCapability::NetworkClient { hosts: cap_hosts },
                            CoreCapability::NetworkClient { hosts: req_hosts },
                        ) => {
                            // Simplified check
                            cap_hosts.is_none() || req_hosts.is_none() ||
                            cap_hosts == req_hosts
                        }
                        (CoreCapability::InterPluginComm, CoreCapability::InterPluginComm) => {
                            true
                        }
                        _ => false,
                    }
                })
            } else {
                false
            }
        }
        
        fn grant_capability(
            &self,
            plugin_id: PluginId,
            capability: CoreCapability,
        ) -> Result<CapabilityId, lion_core::error::CapabilityError> {
            self.grant(plugin_id, capability);
            Ok(CapabilityId::new())
        }
        
        fn revoke_capability(
            &self,
            _plugin_id: PluginId,
            _capability_id: CapabilityId,
        ) -> Result<(), lion_core::error::CapabilityError> {
            // Not implemented for the mock
            Ok(())
        }
        
        fn list_capabilities(&self, _plugin_id: PluginId) -> Vec<lion_core::capability::Capability> {
            // Not implemented for the mock
            Vec::new()
        }
    }
    
    #[test]
    fn test_publish_and_subscribe() {
        // Create a message bus without capability checking
        let config = InMemoryMessageBusConfig {
            check_capabilities: false,
            ..Default::default()
        };
        let bus = InMemoryMessageBus::new(config, None);
        
        // Create plugin IDs
        let publisher = PluginId::new();
        let subscriber1 = PluginId::new();
        let subscriber2 = PluginId::new();
        
        // Subscribe to a topic
        let topic = TopicId("test-topic".to_string());
        bus.subscribe(subscriber1, topic.clone()).unwrap();
        bus.subscribe(subscriber2, topic.clone()).unwrap();
        
        // Publish a message
        let content = serde_json::json!({ "test": "message" });
        bus.publish(publisher, topic, content.clone()).unwrap();
        
        // Check that both subscribers received the message
        assert!(bus.has_messages(subscriber1));
        assert!(bus.has_messages(subscriber2));
        
        // Get the messages
        let message1 = bus.next_message(subscriber1).unwrap();
        let message2 = bus.next_message(subscriber2).unwrap();
        
        // Check that the messages match
        assert_eq!(message1.sender, publisher);
        assert_eq!(message1.content, content);
        assert_eq!(message2.sender, publisher);
        assert_eq!(message2.content, content);
    }
    
    #[test]
    fn test_send_direct() {
        // Create a message bus without capability checking
        let config = InMemoryMessageBusConfig {
            check_capabilities: false,
            ..Default::default()
        };
        let bus = InMemoryMessageBus::new(config, None);
        
        // Create plugin IDs
        let sender = PluginId::new();
        let receiver = PluginId::new();
        
        // Create a message queue for the receiver
        bus.get_or_create_queue(receiver);
        
        // Send a direct message
        let content = serde_json::json!({ "test": "message" });
        bus.send_direct(sender, receiver, content.clone()).unwrap();
        
        // Check that the receiver got the message
        assert!(bus.has_messages(receiver));
        
        // Get the message
        let message = bus.next_message(receiver).unwrap();
        
        // Check that the message matches
        assert_eq!(message.sender, sender);
        assert_eq!(message.content, content);
    }
    
    #[test]
    fn test_capability_check() {
        // Create a mock capability manager
        let capability_manager = Arc::new(MockCapabilityManager::new());
        
        // Create a message bus with capability checking
        let config = InMemoryMessageBusConfig {
            check_capabilities: true,
            ..Default::default()
        };
        let bus = InMemoryMessageBus::with_capability_manager(
            config,
            capability_manager.clone(),
        );
        
        // Create plugin IDs
        let publisher = PluginId::new();
        let subscriber = PluginId::new();
        
        // Grant InterPluginComm capability to the subscriber
        capability_manager.grant(subscriber, CoreCapability::InterPluginComm);
        
        // Try to publish without capability
        let topic = TopicId("test-topic".to_string());
        let content = serde_json::json!({ "test": "message" });
        let result = bus.publish(publisher, topic.clone(), content.clone());
        
        // Should fail because the publisher doesn't have the capability
        assert!(result.is_err());
        
        // Grant InterPluginComm capability to the publisher
        capability_manager.grant(publisher, CoreCapability::InterPluginComm);
        
        // Try to publish again
        let result = bus.publish(publisher, topic.clone(), content.clone());
        
        // Should succeed now
        assert!(result.is_ok());
        
        // Subscribe to the topic
        let result = bus.subscribe(subscriber, topic.clone());
        
        // Should succeed because the subscriber has the capability
        assert!(result.is_ok());
    }
}
