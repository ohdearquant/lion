//! Topic-based publish-subscribe system.

use dashmap::DashMap;
use lion_core::message::{Message, TopicId};
use lion_core::plugin::PluginId;
use std::collections::HashSet;
use std::sync::Arc;

use crate::error::MessageBusError;
use crate::message_queue::MessageQueue;

/// A topic in the publish-subscribe system
pub struct Topic {
    /// The ID of the topic
    id: TopicId,
    
    /// The subscribers for this topic
    subscribers: DashMap<PluginId, Arc<MessageQueue>>,
    
    /// The last N messages for this topic, if retention is enabled
    retained_messages: Option<Vec<Message>>,
    
    /// The maximum number of retained messages
    max_retained_messages: usize,
}

impl Topic {
    /// Create a new topic
    pub fn new(id: TopicId, retain_history: bool, max_retained_messages: usize) -> Self {
        Self {
            id,
            subscribers: DashMap::new(),
            retained_messages: if retain_history {
                Some(Vec::with_capacity(max_retained_messages))
            } else {
                None
            },
            max_retained_messages,
        }
    }
    
    /// Add a subscriber to the topic
    pub fn add_subscriber(&self, plugin_id: PluginId, queue: Arc<MessageQueue>) -> Result<(), MessageBusError> {
        // Check if the plugin is already subscribed
        if self.subscribers.contains_key(&plugin_id) {
            return Err(MessageBusError::SubscriberAlreadyExists);
        }
        
        // Add the subscriber
        self.subscribers.insert(plugin_id, queue);
        
        // Deliver retained messages
        if let Some(retained) = &self.retained_messages {
            let queue = self.subscribers.get(&plugin_id).unwrap();
            for message in retained {
                let _ = queue.push(message.clone());
            }
        }
        
        Ok(())
    }
    
    /// Remove a subscriber from the topic
    pub fn remove_subscriber(&self, plugin_id: PluginId) -> Result<(), MessageBusError> {
        // Check if the plugin is subscribed
        if !self.subscribers.contains_key(&plugin_id) {
            return Err(MessageBusError::SubscriberNotFound);
        }
        
        // Remove the subscriber
        self.subscribers.remove(&plugin_id);
        
        Ok(())
    }
    
    /// Publish a message to all subscribers
    pub fn publish(&self, message: Message) -> Result<(), MessageBusError> {
        // Publish to all subscribers
        for subscriber in self.subscribers.iter() {
            let _ = subscriber.push(message.clone());
        }
        
        // Retain the message if enabled
        if let Some(retained) = &mut self.retained_messages {
            retained.push(message);
            
            // Truncate if necessary
            if retained.len() > self.max_retained_messages {
                retained.remove(0);
            }
        }
        
        Ok(())
    }
    
    /// Get the subscribers for this topic
    pub fn subscribers(&self) -> HashSet<PluginId> {
        self.subscribers.iter().map(|entry| *entry.key()).collect()
    }
    
    /// Get the topic ID
    pub fn id(&self) -> &TopicId {
        &self.id
    }
}