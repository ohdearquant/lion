//! Message capability model.
//! 
//! This module defines capabilities for sending messages.

use std::collections::HashMap;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;

use super::capability::{Capability, Constraint};

/// A capability that grants permission to send messages.
#[derive(Debug, Clone)]
pub struct MessageCapability {
    /// The recipient-topic pairs that are allowed.
    allowed_messages: HashMap<String, Vec<String>>,
}

impl MessageCapability {
    /// Create a new message capability.
    ///
    /// # Arguments
    ///
    /// * `allowed_messages` - Map of recipient IDs to allowed topics.
    ///
    /// # Returns
    ///
    /// A new message capability.
    pub fn new(allowed_messages: HashMap<String, Vec<String>>) -> Self {
        Self { allowed_messages }
    }
    
    /// Create a new message capability allowing specific topics.
    ///
    /// # Arguments
    ///
    /// * `recipient` - The recipient ID.
    /// * `topics` - The topics that are allowed.
    ///
    /// # Returns
    ///
    /// A new message capability.
    pub fn for_recipient(recipient: impl Into<String>, topics: impl IntoIterator<Item = String>) -> Self {
        let mut allowed_messages = HashMap::new();
        allowed_messages.insert(recipient.into(), topics.into_iter().collect());
        Self { allowed_messages }
    }
    
    /// Check if a message is allowed.
    ///
    /// # Arguments
    ///
    /// * `recipient` - The recipient ID.
    /// * `topic` - The topic.
    ///
    /// # Returns
    ///
    /// `true` if the message is allowed, `false` otherwise.
    fn is_message_allowed(&self, recipient: &str, topic: &str) -> bool {
        match self.allowed_messages.get(recipient) {
            Some(topics) => topics.contains(&topic.to_string()),
            None => false,
        }
    }
    
    /// Get the allowed messages.
    pub fn allowed_messages(&self) -> &HashMap<String, Vec<String>> {
        &self.allowed_messages
    }
}

impl Capability for MessageCapability {
    fn capability_type(&self) -> &str {
        "message"
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::Message { recipient, topic } => {
                // Check if the message is allowed
                if !self.is_message_allowed(recipient, topic) {
                    return Err(CapabilityError::PermissionDenied(
                        format!("Message to recipient {} on topic {} is not allowed", recipient, topic)
                    ).into());
                }
                
                Ok(())
            },
            _ => Err(CapabilityError::PermissionDenied(
                "Only message sending is allowed".into()
            ).into()),
        }
    }
    
    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        let mut allowed_messages = self.allowed_messages.clone();
        
        for constraint in constraints {
            match constraint {
                Constraint::Message { recipient, topic } => {
                    // Check if the recipient is already allowed
                    if let Some(topics) = allowed_messages.get_mut(recipient) {
                        // Check if the topic is already allowed
                        if !topics.contains(topic) {
                            topics.push(topic.clone());
                        }
                    } else {
                        // Add the recipient and topic
                        allowed_messages.insert(recipient.clone(), vec![topic.clone()]);
                    }
                },
                _ => return Err(CapabilityError::ConstraintError(
                    format!("Constraint type {} not supported for message capability", constraint.constraint_type())
                ).into()),
            }
        }
        
        Ok(Box::new(Self { allowed_messages }))
    }
    
    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut capabilities = Vec::new();
        
        // Split by recipient
        for (recipient, topics) in &self.allowed_messages {
            let mut allowed_messages = HashMap::new();
            allowed_messages.insert(recipient.clone(), topics.clone());
            capabilities.push(Box::new(Self { allowed_messages }) as Box<dyn Capability>);
        }
        
        // If we didn't split, just clone
        if capabilities.is_empty() {
            capabilities.push(Box::new(self.clone()));
        }
        
        capabilities
    }
    
    fn can_join_with(&self, other: &dyn Capability) -> bool {
        other.capability_type() == "message"
    }
    
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        if !self.can_join_with(other) {
            return Err(CapabilityError::CompositionError(
                format!("Cannot join message capability with {}", other.capability_type())
            ).into());
        }
        
        // Try to get more precise information by checking specific messages
        let mut allowed_messages = self.allowed_messages.clone();
        
        // For each recipient-topic pair
        for (recipient, topics) in &self.allowed_messages {
            for topic in topics {
                // Check if the other capability permits this message
                if other.permits(&AccessRequest::Message {
                    recipient: recipient.clone(),
                    topic: topic.clone(),
                }).is_ok() {
                    // The other capability also allows this message
                    // It would be in the joined capability anyway
                }
            }
        }
        
        // TODO: More precise information from the other capability
        
        // Just merge the allowed messages
        let other_messages = match other.permits(&AccessRequest::Message {
            recipient: "test".to_string(),
            topic: "test".to_string(),
        }) {
            Ok(()) => {
                // If it permits everything, it's probably a super-capability
                HashMap::new()
            },
            Err(_) => {
                // Just add the other's allowed messages
                HashMap::new() // Placeholder
            }
        };
        
        for (recipient, topics) in other_messages {
            if let Some(existing_topics) = allowed_messages.get_mut(&recipient) {
                // Merge the topics
                for topic in topics {
                    if !existing_topics.contains(&topic) {
                        existing_topics.push(topic);
                    }
                }
            } else {
                // Add the recipient and topics
                allowed_messages.insert(recipient, topics);
            }
        }
        
        Ok(Box::new(Self { allowed_messages }))
    }
    
    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_capability_permits() {
        let mut allowed_messages = HashMap::new();
        allowed_messages.insert("plugin1".to_string(), vec!["topic1".to_string(), "topic2".to_string()]);
        let capability = MessageCapability::new(allowed_messages);
        
        // Test allowed message
        let request = AccessRequest::Message {
            recipient: "plugin1".to_string(),
            topic: "topic1".to_string(),
        };
        assert!(capability.permits(&request).is_ok());
        
        // Test disallowed message
        let request = AccessRequest::Message {
            recipient: "plugin1".to_string(),
            topic: "topic3".to_string(),
        };
        assert!(capability.permits(&request).is_err());
        
        // Test message to disallowed recipient
        let request = AccessRequest::Message {
            recipient: "plugin2".to_string(),
            topic: "topic1".to_string(),
        };
        assert!(capability.permits(&request).is_err());
        
        // Test non-message access
        let request = AccessRequest::File {
            path: std::path::PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
    }
    
    #[test]
    fn test_message_capability_constrain() {
        let mut allowed_messages = HashMap::new();
        allowed_messages.insert("plugin1".to_string(), vec!["topic1".to_string()]);
        let capability = MessageCapability::new(allowed_messages);
        
        // Constrain to add a new topic
        let constraints = vec![Constraint::Message {
            recipient: "plugin1".to_string(),
            topic: "topic2".to_string(),
        }];
        let constrained = capability.constrain(&constraints).unwrap();
        
        // Should allow message to the original topic
        let request = AccessRequest::Message {
            recipient: "plugin1".to_string(),
            topic: "topic1".to_string(),
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Should allow message to the new topic
        let request = AccessRequest::Message {
            recipient: "plugin1".to_string(),
            topic: "topic2".to_string(),
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Should still deny message to other topics
        let request = AccessRequest::Message {
            recipient: "plugin1".to_string(),
            topic: "topic3".to_string(),
        };
        assert!(constrained.permits(&request).is_err());
    }
    
    #[test]
    fn test_message_capability_split() {
        let mut allowed_messages = HashMap::new();
        allowed_messages.insert("plugin1".to_string(), vec!["topic1".to_string()]);
        allowed_messages.insert("plugin2".to_string(), vec!["topic2".to_string()]);
        let capability = MessageCapability::new(allowed_messages);
        
        let split = capability.split();
        assert_eq!(split.len(), 2);
        
        // Check that the first capability allows plugin1 but not plugin2
        let request = AccessRequest::Message {
            recipient: "plugin1".to_string(),
            topic: "topic1".to_string(),
        };
        let allows_plugin1 = split[0].permits(&request).is_ok() || split[1].permits(&request).is_ok();
        assert!(allows_plugin1);
        
        // Check that the second capability allows plugin2 but not plugin1
        let request = AccessRequest::Message {
            recipient: "plugin2".to_string(),
            topic: "topic2".to_string(),
        };
        let allows_plugin2 = split[0].permits(&request).is_ok() || split[1].permits(&request).is_ok();
        assert!(allows_plugin2);
    }
    
    #[test]
    fn test_message_capability_join() {
        let mut allowed_messages1 = HashMap::new();
        allowed_messages1.insert("plugin1".to_string(), vec!["topic1".to_string()]);
        let capability1 = MessageCapability::new(allowed_messages1);
        
        let mut allowed_messages2 = HashMap::new();
        allowed_messages2.insert("plugin2".to_string(), vec!["topic2".to_string()]);
        let capability2 = MessageCapability::new(allowed_messages2);
        
        let joined = capability1.join(&capability2).unwrap();
        
        // Should allow message to plugin1.topic1
        let request = AccessRequest::Message {
            recipient: "plugin1".to_string(),
            topic: "topic1".to_string(),
        };
        assert!(joined.permits(&request).is_ok());

        // Should allow message to plugin2.topic2
        let request = AccessRequest::Message {
            recipient: "plugin2".to_string(),
            topic: "topic2".to_string(),
        };
        assert!(joined.permits(&request).is_ok());

        // Should deny message to plugin1.topic2
        let request = AccessRequest::Message {
            recipient: "plugin1".to_string(),
            topic: "topic2".to_string(),
        };
        assert!(joined.permits(&request).is_err());

        // Should deny message to plugin2.topic1
        let request = AccessRequest::Message {
            recipient: "plugin2".to_string(),
            topic: "topic1".to_string(),
        };
        assert!(joined.permits(&request).is_err());
    }
}