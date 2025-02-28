use std::any::Any;
use std::collections::{HashMap, HashSet};

use super::capability::{AccessRequest, Capability, CapabilityError, Constraint};

/// Represents the type of permission for a messaging topic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessagePermission {
    /// Permission to publish messages to a topic
    Publish,
    /// Permission to subscribe to messages from a topic
    Subscribe,
    /// Permission to do both publish and subscribe
    Both,
}

impl MessagePermission {
    /// Returns true if this permission includes publish permission
    pub fn can_publish(&self) -> bool {
        matches!(self, MessagePermission::Publish | MessagePermission::Both)
    }

    /// Returns true if this permission includes subscribe permission
    pub fn can_subscribe(&self) -> bool {
        matches!(self, MessagePermission::Subscribe | MessagePermission::Both)
    }

    /// Returns the intersection of two permissions
    pub fn intersect(&self, other: &MessagePermission) -> Option<MessagePermission> {
        match (self, other) {
            (MessagePermission::Both, _) => Some(*other),
            (_, MessagePermission::Both) => Some(*self),
            (a, b) if a == b => Some(*a),
            _ => None, // Incompatible permissions
        }
    }

    /// Returns the union of two permissions
    pub fn union(&self, other: &MessagePermission) -> MessagePermission {
        if self.can_publish() || other.can_publish() {
            if self.can_subscribe() || other.can_subscribe() {
                MessagePermission::Both
            } else {
                MessagePermission::Publish
            }
        } else {
            MessagePermission::Subscribe
        }
    }
}

/// Represents a capability to send and receive messages on specific topics
#[derive(Debug, Clone)]
pub struct MessageCapability {
    /// Map of topic patterns to permissions
    topic_permissions: HashMap<String, MessagePermission>,

    /// Default permission for topics not explicitly listed
    default_permission: Option<MessagePermission>,
}

impl Default for MessageCapability {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageCapability {
    /// Creates a new message capability with no default permissions
    pub fn new() -> Self {
        Self {
            topic_permissions: HashMap::new(),
            default_permission: None,
        }
    }

    /// Creates a new message capability with the specified topics and permissions
    pub fn with_topics(topic_permissions: HashMap<String, MessagePermission>) -> Self {
        Self {
            topic_permissions,
            default_permission: None,
        }
    }

    /// Creates a new message capability with a single topic and permission
    pub fn with_topic(topic: String, permission: MessagePermission) -> Self {
        let mut topic_permissions = HashMap::new();
        topic_permissions.insert(topic, permission);

        Self {
            topic_permissions,
            default_permission: None,
        }
    }

    /// Creates a message capability that allows publishing to specific topics
    pub fn publisher(topics: Vec<String>) -> Self {
        let mut topic_permissions = HashMap::new();

        for topic in topics {
            topic_permissions.insert(topic, MessagePermission::Publish);
        }

        Self {
            topic_permissions,
            default_permission: None,
        }
    }

    /// Creates a message capability that allows subscribing to specific topics
    pub fn subscriber(topics: Vec<String>) -> Self {
        let mut topic_permissions = HashMap::new();

        for topic in topics {
            topic_permissions.insert(topic, MessagePermission::Subscribe);
        }

        Self {
            topic_permissions,
            default_permission: None,
        }
    }

    /// Add a topic permission
    pub fn add_topic_permission(&mut self, topic: String, permission: MessagePermission) {
        self.topic_permissions.insert(topic, permission);
    }

    /// Set the default permission for topics not explicitly listed
    pub fn set_default_permission(&mut self, permission: Option<MessagePermission>) {
        self.default_permission = permission;
    }

    /// Get the topic permissions
    pub fn topic_permissions(&self) -> &HashMap<String, MessagePermission> {
        &self.topic_permissions
    }

    /// Get the default permission
    pub fn default_permission(&self) -> Option<MessagePermission> {
        self.default_permission
    }

    /// Helper function to check if a topic is allowed for a specific operation
    fn is_topic_allowed(&self, topic: &str, publish: bool) -> bool {
        // First check for explicit topic matches
        for (pattern, permission) in &self.topic_permissions {
            if topic_matches(pattern, topic) {
                if publish {
                    return permission.can_publish();
                } else {
                    return permission.can_subscribe();
                }
            }
        }

        // Fall back to default permission if any
        if let Some(permission) = self.default_permission {
            if publish {
                return permission.can_publish();
            } else {
                return permission.can_subscribe();
            }
        }

        false
    }

    /// Apply a topic constraint to this capability
    fn apply_topic_constraint(&self, topic: &str) -> Result<Self, CapabilityError> {
        // Check if the topic is matched by any of our patterns
        let mut has_matching_pattern = false;
        let mut allowed_permission = None;

        for (pattern, permission) in &self.topic_permissions {
            if topic_matches(pattern, topic) {
                has_matching_pattern = true;
                allowed_permission = Some(*permission);
                break;
            }
        }

        if !has_matching_pattern && self.default_permission.is_none() {
            return Err(CapabilityError::InvalidConstraint(format!(
                "Topic '{}' is not covered by this capability",
                topic
            )));
        }

        // Create a new capability with just this topic
        let mut topic_permissions = HashMap::new();
        topic_permissions.insert(
            topic.to_string(),
            allowed_permission.unwrap_or_else(|| self.default_permission.unwrap()),
        );

        Ok(Self {
            topic_permissions,
            default_permission: None,
        })
    }
}

/// Helper function to check if a topic matches a pattern
/// Implements a simple wildcard matching for topic patterns
fn topic_matches(pattern: &str, topic: &str) -> bool {
    // If pattern ends with #, it's a prefix match
    if let Some(prefix) = pattern.strip_suffix("#") {
        return topic.starts_with(prefix);
    }

    // Simple exact match
    pattern == topic
}

impl Capability for MessageCapability {
    fn capability_type(&self) -> &str {
        "message"
    }

    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::Message {
                topic,
                recipient: _,
            } => {
                // For now, we only check the topic, not the recipient
                // Publishing is assumed here (we'll expand this later if needed)

                if self.is_topic_allowed(topic, true) {
                    Ok(())
                } else {
                    Err(CapabilityError::AccessDenied(format!(
                        "Topic '{}' is not allowed for publication by this capability",
                        topic
                    )))
                }
            }
            _ => Err(CapabilityError::IncompatibleTypes(format!(
                "Expected Message request, got {:?}",
                request
            ))),
        }
    }

    fn constrain(
        &self,
        constraints: &[Constraint],
    ) -> Result<Box<dyn Capability>, CapabilityError> {
        let mut result = self.clone();

        for constraint in constraints {
            match constraint {
                Constraint::MessageTopic(topic) => {
                    result = result.apply_topic_constraint(topic)?;
                }
                _ => {
                    return Err(CapabilityError::InvalidConstraint(format!(
                        "Constraint {:?} not applicable to MessageCapability",
                        constraint
                    )))
                }
            }
        }

        Ok(Box::new(result))
    }

    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut result = Vec::new();

        // Split by topic
        for (topic, permission) in &self.topic_permissions {
            result.push(
                Box::new(MessageCapability::with_topic(topic.clone(), *permission))
                    as Box<dyn Capability>,
            );
        }

        result
    }

    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to MessageCapability
        if let Some(other_msg) = other.as_any().downcast_ref::<MessageCapability>() {
            let mut topic_permissions = self.topic_permissions.clone();

            // Merge topic permissions
            for (topic, permission) in &other_msg.topic_permissions {
                if let Some(existing_permission) = topic_permissions.get_mut(topic) {
                    *existing_permission = existing_permission.union(permission);
                } else {
                    topic_permissions.insert(topic.clone(), *permission);
                }
            }

            // Merge default permissions
            let default_permission = match (self.default_permission, other_msg.default_permission) {
                (Some(p1), Some(p2)) => Some(p1.union(&p2)),
                (Some(p), None) | (None, Some(p)) => Some(p),
                (None, None) => None,
            };

            Ok(Box::new(MessageCapability {
                topic_permissions,
                default_permission,
            }))
        } else {
            Err(CapabilityError::IncompatibleTypes(
                "Cannot join MessageCapability with a different capability type".to_string(),
            ))
        }
    }

    fn leq(&self, other: &dyn Capability) -> bool {
        // Try to downcast the other capability to MessageCapability
        if let Some(other_msg) = other.as_any().downcast_ref::<MessageCapability>() {
            // This capability is <= other if:
            // 1. For each topic in self, there's a corresponding topic in other with equal or greater permissions
            // 2. If self has a default permission, other must have a default permission that includes it

            // Check default permissions
            if let Some(self_default) = self.default_permission {
                if let Some(other_default) = other_msg.default_permission {
                    // If other's default permission doesn't include self's, this is not a subset
                    if (!self_default.can_publish()
                        || (self_default.can_publish() && !other_default.can_publish()))
                        && (!self_default.can_subscribe()
                            || (self_default.can_subscribe() && !other_default.can_subscribe()))
                    {
                        return false;
                    }
                } else {
                    // Other has no default permission, so self can't have one either
                    return false;
                }
            }

            // Check topic permissions
            for (topic, permission) in &self.topic_permissions {
                // Find a matching topic pattern in other
                let mut is_covered = false;

                for (other_topic, other_permission) in &other_msg.topic_permissions {
                    if topic_matches(other_topic, topic) {
                        // Check if the permission is included
                        if (permission.can_publish() && !other_permission.can_publish())
                            || (permission.can_subscribe() && !other_permission.can_subscribe())
                        {
                            continue;
                        }

                        is_covered = true;
                        break;
                    }
                }

                // If not covered by a specific topic, check the default
                if !is_covered && other_msg.default_permission.is_some() {
                    let other_default = other_msg.default_permission.unwrap();

                    if (permission.can_publish() && other_default.can_publish())
                        || (permission.can_subscribe() && other_default.can_subscribe())
                    {
                        is_covered = true;
                    }
                }

                if !is_covered {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    fn meet(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to MessageCapability
        if let Some(other_msg) = other.as_any().downcast_ref::<MessageCapability>() {
            let mut topic_permissions = HashMap::new();

            // Find all topics in either capability
            let mut all_topics = HashSet::new();
            all_topics.extend(self.topic_permissions.keys().cloned());
            all_topics.extend(other_msg.topic_permissions.keys().cloned());

            // For each topic, find the intersection of permissions
            for topic in all_topics {
                let self_permission = self
                    .topic_permissions
                    .get(&topic)
                    .copied()
                    .or(self.default_permission);

                let other_permission = other_msg
                    .topic_permissions
                    .get(&topic)
                    .copied()
                    .or(other_msg.default_permission);

                // If both capabilities have a permission for this topic, find the intersection
                if let (Some(p1), Some(p2)) = (self_permission, other_permission) {
                    if let Some(intersection) = p1.intersect(&p2) {
                        topic_permissions.insert(topic, intersection);
                    }
                }
            }

            // Find intersection of default permissions
            let default_permission = match (self.default_permission, other_msg.default_permission) {
                (Some(p1), Some(p2)) => p1.intersect(&p2),
                _ => None,
            };

            // If we have no topic permissions and no default permission, return an empty capability
            // instead of throwing an error
            if topic_permissions.is_empty() && default_permission.is_none() {
                // Return an empty message capability
                return Ok(Box::new(MessageCapability {
                    topic_permissions: HashMap::new(),
                    default_permission: None,
                }));
            }

            Ok(Box::new(MessageCapability {
                topic_permissions,
                default_permission,
            }))
        } else {
            Err(CapabilityError::IncompatibleTypes(
                "Cannot compute meet with different capability types".to_string(),
            ))
        }
    }

    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_permission() {
        assert!(MessagePermission::Publish.can_publish());
        assert!(!MessagePermission::Publish.can_subscribe());

        assert!(!MessagePermission::Subscribe.can_publish());
        assert!(MessagePermission::Subscribe.can_subscribe());

        assert!(MessagePermission::Both.can_publish());
        assert!(MessagePermission::Both.can_subscribe());

        // Test intersection
        assert_eq!(
            MessagePermission::Publish.intersect(&MessagePermission::Publish),
            Some(MessagePermission::Publish)
        );

        assert_eq!(
            MessagePermission::Publish.intersect(&MessagePermission::Subscribe),
            None
        );

        assert_eq!(
            MessagePermission::Publish.intersect(&MessagePermission::Both),
            Some(MessagePermission::Publish)
        );

        // Test union
        assert_eq!(
            MessagePermission::Publish.union(&MessagePermission::Subscribe),
            MessagePermission::Both
        );

        assert_eq!(
            MessagePermission::Publish.union(&MessagePermission::Publish),
            MessagePermission::Publish
        );
    }

    #[test]
    fn test_topic_matches() {
        assert!(topic_matches("topic", "topic"));
        assert!(!topic_matches("topic", "other"));

        // Test prefix matching
        assert!(topic_matches("topic#", "topic"));
        assert!(topic_matches("topic#", "topic.subtopic"));
        assert!(!topic_matches("topic#", "other"));
    }

    #[test]
    fn test_message_capability_permits() {
        let mut cap = MessageCapability::new();
        cap.add_topic_permission("topic.a".to_string(), MessagePermission::Publish);
        cap.add_topic_permission("topic.b#".to_string(), MessagePermission::Subscribe);
        cap.add_topic_permission("topic.c".to_string(), MessagePermission::Both);

        // Test allowed publish
        assert!(cap
            .permits(&AccessRequest::Message {
                topic: "topic.a".to_string(),
                recipient: None,
            })
            .is_ok());

        // Test denied publish (subscribe-only topic)
        assert!(cap
            .permits(&AccessRequest::Message {
                topic: "topic.b".to_string(),
                recipient: None,
            })
            .is_err());

        // Test allowed publish on "both" permission
        assert!(cap
            .permits(&AccessRequest::Message {
                topic: "topic.c".to_string(),
                recipient: None,
            })
            .is_ok());

        // Test prefix matching
        assert!(cap
            .permits(&AccessRequest::Message {
                topic: "topic.b.subtopic".to_string(),
                recipient: None,
            })
            .is_err());

        // Test missing topic
        assert!(cap
            .permits(&AccessRequest::Message {
                topic: "topic.d".to_string(),
                recipient: None,
            })
            .is_err());

        // Test default permission
        let mut cap2 = MessageCapability::new();
        cap2.set_default_permission(Some(MessagePermission::Publish));

        assert!(cap2
            .permits(&AccessRequest::Message {
                topic: "any.topic".to_string(),
                recipient: None,
            })
            .is_ok());
    }

    #[test]
    fn test_message_capability_join_and_meet() {
        let mut cap1 = MessageCapability::new();
        cap1.add_topic_permission("topic.a".to_string(), MessagePermission::Publish);
        cap1.add_topic_permission("topic.common".to_string(), MessagePermission::Publish);

        let mut cap2 = MessageCapability::new();
        cap2.add_topic_permission("topic.b".to_string(), MessagePermission::Subscribe);
        cap2.add_topic_permission("topic.common".to_string(), MessagePermission::Subscribe);

        // Join
        let join = cap1.join(&cap2).unwrap();

        assert!(join
            .permits(&AccessRequest::Message {
                topic: "topic.a".to_string(),
                recipient: None,
            })
            .is_ok());

        // topic.b is subscribe-only, so publishing is not allowed
        assert!(join
            .permits(&AccessRequest::Message {
                topic: "topic.b".to_string(),
                recipient: None,
            })
            .is_err());

        // topic.common has Both permission after join, so publishing is allowed
        assert!(join
            .permits(&AccessRequest::Message {
                topic: "topic.common".to_string(),
                recipient: None,
            })
            .is_ok());

        // Meet
        let meet = cap1.meet(&cap2).unwrap();

        // topic.a and topic.b are not in the meet since they have incompatible permissions
        assert!(meet
            .permits(&AccessRequest::Message {
                topic: "topic.a".to_string(),
                recipient: None,
            })
            .is_err());

        assert!(meet
            .permits(&AccessRequest::Message {
                topic: "topic.b".to_string(),
                recipient: None,
            })
            .is_err());

        // topic.common is in the meet, but has no permission intersection
        assert!(meet
            .permits(&AccessRequest::Message {
                topic: "topic.common".to_string(),
                recipient: None,
            })
            .is_err());
    }
}
