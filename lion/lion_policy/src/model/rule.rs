//! Policy rule model.
//! 
//! This module defines the core policy rule types.

use std::fmt;
use serde::{Serialize, Deserialize};
use lion_core::id::PluginId;
use chrono::{DateTime, Utc};

/// A policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// The unique ID of this rule.
    pub id: String,
    
    /// The name of this rule.
    pub name: String,
    
    /// The description of this rule.
    pub description: String,
    
    /// The subject of this rule.
    pub subject: PolicySubject,
    
    /// The object of this rule.
    pub object: PolicyObject,
    
    /// The action of this rule.
    pub action: PolicyAction,
    
    /// The condition of this rule.
    pub condition: Option<PolicyCondition>,
    
    /// The priority of this rule.
    pub priority: i32,
    
    /// When this rule was created.
    pub created_at: DateTime<Utc>,
    
    /// When this rule was last updated.
    pub updated_at: DateTime<Utc>,
    
    /// When this rule expires.
    pub expires_at: Option<DateTime<Utc>>,
}

impl PolicyRule {
    /// Create a new policy rule.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique ID of this rule.
    /// * `name` - The name of this rule.
    /// * `description` - The description of this rule.
    /// * `subject` - The subject of this rule.
    /// * `object` - The object of this rule.
    /// * `action` - The action of this rule.
    /// * `condition` - The condition of this rule.
    /// * `priority` - The priority of this rule.
    ///
    /// # Returns
    ///
    /// A new policy rule.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        subject: PolicySubject,
        object: PolicyObject,
        action: PolicyAction,
        condition: Option<PolicyCondition>,
        priority: i32,
    ) -> Self {
        let now = Utc::now();
        
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            subject,
            object,
            action,
            condition,
            priority,
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }
    
    /// Check if this rule is expired.
    ///
    /// # Returns
    ///
    /// `true` if the rule is expired, `false` otherwise.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
    
    /// Set the expiration time for this rule.
    ///
    /// # Arguments
    ///
    /// * `expires_at` - When the rule should expire.
    ///
    /// # Returns
    ///
    /// The updated rule.
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self.updated_at = Utc::now();
        self
    }
}

/// The subject of a policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicySubject {
    /// Any plugin.
    Any,
    
    /// A specific plugin.
    Plugin(PluginId),
    
    /// Multiple specific plugins.
    Plugins(Vec<PluginId>),
    
    /// Plugins with a specific name or matching a pattern.
    PluginName(String),
    
    /// Plugins with a specific tag.
    PluginTag(String),
    
    /// Plugins with a specific role.
    PluginRole(String),
}

impl fmt::Display for PolicySubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => write!(f, "Any plugin"),
            Self::Plugin(id) => write!(f, "Plugin {}", id),
            Self::Plugins(ids) => {
                write!(f, "Plugins [")?;
                for (i, id) in ids.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", id)?;
                }
                write!(f, "]")
            },
            Self::PluginName(name) => write!(f, "Plugin name '{}'", name),
            Self::PluginTag(tag) => write!(f, "Plugin tag '{}'", tag),
            Self::PluginRole(role) => write!(f, "Plugin role '{}'", role),
        }
    }
}

/// The object of a policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyObject {
    /// Any object.
    Any,
    
    /// A file or directory.
    File(FileObject),
    
    /// A network resource.
    Network(NetworkObject),
    
    /// A plugin to call.
    PluginCall(PluginCallObject),
    
    /// A memory region.
    Memory(MemoryObject),
    
    /// A message to send.
    Message(MessageObject),
    
    /// A custom object.
    Custom {
        /// The type of the custom object.
        object_type: String,
        
        /// The value of the custom object.
        value: String,
    },
}

impl fmt::Display for PolicyObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => write!(f, "Any object"),
            Self::File(file) => write!(f, "{}", file),
            Self::Network(network) => write!(f, "{}", network),
            Self::PluginCall(plugin_call) => write!(f, "{}", plugin_call),
            Self::Memory(memory) => write!(f, "{}", memory),
            Self::Message(message) => write!(f, "{}", message),
            Self::Custom { object_type, value } => {
                write!(f, "Custom object of type '{}' with value '{}'", object_type, value)
            },
        }
    }
}

/// A file or directory object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileObject {
    /// The path to the file or directory.
    pub path: String,
    
    /// Whether this is a directory.
    pub is_directory: bool,
}

impl fmt::Display for FileObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_directory {
            write!(f, "Directory '{}'", self.path)
        } else {
            write!(f, "File '{}'", self.path)
        }
    }
}

/// A network resource object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkObject {
    /// The host name or IP address.
    pub host: String,
    
    /// The port.
    pub port: Option<u16>,
    
    /// The protocol.
    pub protocol: Option<String>,
}

impl fmt::Display for NetworkObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Network host '{}'", self.host)?;
        
        if let Some(port) = self.port {
            write!(f, " on port {}", port)?;
        }
        
        if let Some(protocol) = &self.protocol {
            write!(f, " using {}", protocol)?;
        }
        
        Ok(())
    }
}

/// A plugin call object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCallObject {
    /// The plugin ID.
    pub plugin_id: String,
    
    /// The function to call.
    pub function: Option<String>,
}

impl fmt::Display for PluginCallObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Plugin call to '{}'", self.plugin_id)?;
        
        if let Some(function) = &self.function {
            write!(f, " function '{}'", function)?;
        }
        
        Ok(())
    }
}

/// A memory region object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryObject {
    /// The region ID.
    pub region_id: String,
}

impl fmt::Display for MemoryObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Memory region '{}'", self.region_id)
    }
}

/// A message object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageObject {
    /// The recipient.
    pub recipient: String,
    
    /// The topic.
    pub topic: Option<String>,
}

impl fmt::Display for MessageObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message to '{}'", self.recipient)?;
        
        if let Some(topic) = &self.topic {
            write!(f, " on topic '{}'", topic)?;
        }
        
        Ok(())
    }
}

/// The action of a policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow the action.
    Allow,
    
    /// Deny the action.
    Deny,
    
    /// Allow the action with constraints.
    AllowWithConstraints(Vec<String>),
    
    /// Transform to constraints.
    TransformToConstraints(Vec<String>),
    
    /// Audit the action.
    Audit,
}

impl fmt::Display for PolicyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow => write!(f, "Allow"),
            Self::Deny => write!(f, "Deny"),
            Self::AllowWithConstraints(constraints) => {
                write!(f, "Allow with constraints [")?;
                for (i, constraint) in constraints.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", constraint)?;
                }
                write!(f, "]")
            },
            Self::TransformToConstraints(constraints) => {
                write!(f, "Transform to constraints [")?;
                for (i, constraint) in constraints.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", constraint)?;
                }
                write!(f, "]")
            },
            Self::Audit => write!(f, "Audit"),
        }
    }
}

/// The condition of a policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyCondition {
    /// A time-based condition.
    Time(TimeCondition),
    
    /// A resource-based condition.
    Resource(ResourceCondition),
    
    /// A context-based condition.
    Context(ContextCondition),
    
    /// An all condition (logical AND).
    All(Vec<PolicyCondition>),
    
    /// An any condition (logical OR).
    Any(Vec<PolicyCondition>),
    
    /// A not condition (logical NOT).
    Not(Box<PolicyCondition>),
}

/// A time-based condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCondition {
    /// The start time.
    pub start: Option<DateTime<Utc>>,
    
    /// The end time.
    pub end: Option<DateTime<Utc>>,
    
    /// The days of the week (0 = Sunday, 6 = Saturday).
    pub days_of_week: Option<Vec<u8>>,
    
    /// The time of day (hours, 0-23).
    pub hours: Option<Vec<u8>>,
}

/// A resource-based condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCondition {
    /// The maximum CPU usage.
    pub max_cpu: Option<f64>,
    
    /// The maximum memory usage.
    pub max_memory: Option<usize>,
    
    /// The maximum disk usage.
    pub max_disk: Option<usize>,
    
    /// The maximum network usage.
    pub max_network: Option<usize>,
}

/// A context-based condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCondition {
    /// The required context values.
    pub context: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_policy_rule_new() {
        let rule = PolicyRule::new(
            "rule1",
            "Test Rule",
            "A test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        
        assert_eq!(rule.id, "rule1");
        assert_eq!(rule.name, "Test Rule");
        assert_eq!(rule.description, "A test rule");
        assert_eq!(rule.priority, 0);
        assert!(!rule.is_expired());
    }
    
    #[test]
    fn test_policy_rule_expiration() {
        let now = Utc::now();
        let past = now - chrono::Duration::days(1);
        let future = now + chrono::Duration::days(1);
        
        let expired_rule = PolicyRule::new(
            "rule1",
            "Expired Rule",
            "An expired rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        ).with_expiration(past);
        
        let active_rule = PolicyRule::new(
            "rule2",
            "Active Rule",
            "An active rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        ).with_expiration(future);
        
        assert!(expired_rule.is_expired());
        assert!(!active_rule.is_expired());
    }
}