//! Capability types for the Lion security system.
//!
//! This module provides concrete capability implementations
//! that define access control boundaries.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use lion_core::error::CapabilityError;
use lion_core::types::PluginId;

/// Type of capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityType {
    /// File system access.
    File,
    
    /// Network access.
    Network,
    
    /// Plugin interaction.
    PluginCall,
    
    /// Message sending/receiving.
    Message,
    
    /// Shared memory access.
    Memory,
    
    /// System information access.
    System,
}

impl std::fmt::Display for CapabilityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Network => write!(f, "network"),
            Self::PluginCall => write!(f, "plugin_call"),
            Self::Message => write!(f, "message"),
            Self::Memory => write!(f, "memory"),
            Self::System => write!(f, "system"),
        }
    }
}

/// Core capability trait.
pub trait Capability: Send + Sync {
    /// Get the capability type.
    fn get_type(&self) -> &str;
    
    /// Check if this capability includes another.
    /// 
    /// This implements the subsumption relation for capabilities:
    /// if A includes B, then a plugin with capability A can
    /// perform any operation that requires capability B.
    fn includes(&self, other: &dyn Capability) -> bool;
    
    /// Convert capability to a string for debugging.
    fn to_string(&self) -> String;
    
    /// Downcast to concrete type.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// A set of capabilities.
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    /// The capabilities in this set.
    capabilities: Vec<Arc<dyn Capability>>,
}

impl CapabilitySet {
    /// Create a new empty capability set.
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
        }
    }
    
    /// Add a capability to the set.
    pub fn add(&mut self, capability: Arc<dyn Capability>) {
        self.capabilities.push(capability);
    }
    
    /// Check if the set includes a capability.
    pub fn includes(&self, capability: &dyn Capability) -> bool {
        self.capabilities.iter().any(|c| c.includes(capability))
    }
    
    /// Get all capabilities.
    pub fn get_all(&self) -> &[Arc<dyn Capability>] {
        &self.capabilities
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::new()
    }
}

/// File system capability with path restrictions.
#[derive(Debug, Clone)]
pub struct FileCapability {
    /// Whether the capability allows reading.
    pub read: bool,
    
    /// Whether the capability allows writing.
    pub write: bool,
    
    /// Specific paths that are allowed.
    pub allowed_paths: HashSet<PathBuf>,
}

impl FileCapability {
    /// Create a new file capability.
    pub fn new(read: bool, write: bool) -> Self {
        Self {
            read,
            write,
            allowed_paths: HashSet::new(),
        }
    }
    
    /// Add an allowed path.
    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.allowed_paths.insert(path.into());
    }
    
    /// Check if a path is allowed.
    pub fn is_path_allowed(&self, path: &PathBuf) -> bool {
        // If no paths are specified, deny all
        if self.allowed_paths.is_empty() {
            return false;
        }
        
        // Check if the path or any parent is explicitly allowed
        for allowed in &self.allowed_paths {
            if path.starts_with(allowed) {
                return true;
            }
        }
        
        false
    }
    
    /// Create a read-only capability.
    pub fn read_only() -> Self {
        Self::new(true, false)
    }
    
    /// Create a read-write capability.
    pub fn read_write() -> Self {
        Self::new(true, true)
    }
}

impl Capability for FileCapability {
    fn get_type(&self) -> &str {
        "file"
    }
    
    fn includes(&self, other: &dyn Capability) -> bool {
        // Only include other file capabilities
        if other.get_type() != "file" {
            return false;
        }
        
        // Check if we're compatible with the other capability
        if let Some(other_file) = other.as_any().downcast_ref::<FileCapability>() {
            // Check read permission
            if other_file.read && !self.read {
                return false;
            }
            
            // Check write permission
            if other_file.write && !self.write {
                return false;
            }
            
            // Check all other's paths are included in our paths
            for path in &other_file.allowed_paths {
                if !self.is_path_allowed(path) {
                    return false;
                }
            }
            
            true
        } else {
            false
        }
    }
    
    fn to_string(&self) -> String {
        let perm = match (self.read, self.write) {
            (true, true) => "read-write",
            (true, false) => "read-only",
            (false, true) => "write-only",
            (false, false) => "no-access",
        };
        
        let paths: Vec<String> = self.allowed_paths.iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        
        format!("file({}, paths=[{}])", perm, paths.join(", "))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Network capability with domain/address restrictions.
#[derive(Debug, Clone)]
pub struct NetworkCapability {
    /// Whether outbound connections are allowed.
    pub connect: bool,
    
    /// Whether listening for connections is allowed.
    pub listen: bool,
    
    /// Allowed hosts for connection.
    pub allowed_hosts: HashSet<String>,
    
    /// Allowed ports for listening.
    pub allowed_ports: HashSet<u16>,
}

impl NetworkCapability {
    /// Create a new network capability.
    pub fn new(connect: bool, listen: bool) -> Self {
        Self {
            connect,
            listen,
            allowed_hosts: HashSet::new(),
            allowed_ports: HashSet::new(),
        }
    }
    
    /// Add an allowed host.
    pub fn add_host(&mut self, host: impl Into<String>) {
        self.allowed_hosts.insert(host.into());
    }
    
    /// Add an allowed port.
    pub fn add_port(&mut self, port: u16) {
        self.allowed_ports.insert(port);
    }
    
    /// Check if a host is allowed.
    pub fn is_host_allowed(&self, host: &str) -> bool {
        // If no hosts are specified, deny all
        if self.allowed_hosts.is_empty() {
            return false;
        }
        
        // Check exact match
        if self.allowed_hosts.contains(host) {
            return true;
        }
        
        // Check wildcard domains (*.example.com)
        for allowed in &self.allowed_hosts {
            if allowed.starts_with("*.") {
                let suffix = &allowed[1..]; // Include the dot
                if host.ends_with(suffix) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Check if a port is allowed.
    pub fn is_port_allowed(&self, port: u16) -> bool {
        // If no ports are specified, deny all
        if self.allowed_ports.is_empty() {
            return false;
        }
        
        self.allowed_ports.contains(&port)
    }
    
    /// Create a connect-only capability.
    pub fn connect_only() -> Self {
        Self::new(true, false)
    }
    
    /// Create a listen-only capability.
    pub fn listen_only() -> Self {
        Self::new(false, true)
    }
}

impl Capability for NetworkCapability {
    fn get_type(&self) -> &str {
        "network"
    }
    
    fn includes(&self, other: &dyn Capability) -> bool {
        // Only include other network capabilities
        if other.get_type() != "network" {
            return false;
        }
        
        // Check if we're compatible with the other capability
        if let Some(other_net) = other.as_any().downcast_ref::<NetworkCapability>() {
            // Check connect permission
            if other_net.connect && !self.connect {
                return false;
            }
            
            // Check listen permission
            if other_net.listen && !self.listen {
                return false;
            }
            
            // Check all other's hosts are included in our hosts
            if !other_net.allowed_hosts.is_empty() {
                for host in &other_net.allowed_hosts {
                    if !self.is_host_allowed(host) {
                        return false;
                    }
                }
            }
            
            // Check all other's ports are included in our ports
            if !other_net.allowed_ports.is_empty() {
                for port in &other_net.allowed_ports {
                    if !self.is_port_allowed(*port) {
                        return false;
                    }
                }
            }
            
            true
        } else {
            false
        }
    }
    
    fn to_string(&self) -> String {
        let perm = match (self.connect, self.listen) {
            (true, true) => "connect-listen",
            (true, false) => "connect-only",
            (false, true) => "listen-only",
            (false, false) => "no-network",
        };
        
        let hosts: Vec<String> = self.allowed_hosts.iter().cloned().collect();
        let ports: Vec<String> = self.allowed_ports.iter().map(|p| p.to_string()).collect();
        
        format!("network({}, hosts=[{}], ports=[{}])", 
            perm, hosts.join(", "), ports.join(", "))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Plugin call capability with restrictions.
#[derive(Debug, Clone)]
pub struct PluginCallCapability {
    /// Allowed plugins to call.
    pub allowed_plugins: HashSet<PluginId>,
    
    /// Allowed functions to call.
    pub allowed_functions: HashSet<String>,
}

impl PluginCallCapability {
    /// Create a new plugin call capability.
    pub fn new() -> Self {
        Self {
            allowed_plugins: HashSet::new(),
            allowed_functions: HashSet::new(),
        }
    }
    
    /// Add an allowed plugin.
    pub fn add_plugin(&mut self, plugin_id: PluginId) {
        self.allowed_plugins.insert(plugin_id);
    }
    
    /// Add an allowed function.
    pub fn add_function(&mut self, function: impl Into<String>) {
        self.allowed_functions.insert(function.into());
    }
    
    /// Check if a plugin is allowed.
    pub fn is_plugin_allowed(&self, plugin_id: &PluginId) -> bool {
        // If no plugins are specified, allow all
        if self.allowed_plugins.is_empty() {
            return true;
        }
        
        self.allowed_plugins.contains(plugin_id)
    }
    
    /// Check if a function is allowed.
    pub fn is_function_allowed(&self, function: &str) -> bool {
        // If no functions are specified, allow all
        if self.allowed_functions.is_empty() {
            return true;
        }
        
        self.allowed_functions.contains(function)
    }
}

impl Capability for PluginCallCapability {
    fn get_type(&self) -> &str {
        "plugin_call"
    }
    
    fn includes(&self, other: &dyn Capability) -> bool {
        // Only include other plugin call capabilities
        if other.get_type() != "plugin_call" {
            return false;
        }
        
        // Check if we're compatible with the other capability
        if let Some(other_call) = other.as_any().downcast_ref::<PluginCallCapability>() {
            // Check all other's plugins are included in our plugins
            if !other_call.allowed_plugins.is_empty() {
                for plugin_id in &other_call.allowed_plugins {
                    if !self.is_plugin_allowed(plugin_id) {
                        return false;
                    }
                }
            }
            
            // Check all other's functions are included in our functions
            if !other_call.allowed_functions.is_empty() {
                for function in &other_call.allowed_functions {
                    if !self.is_function_allowed(function) {
                        return false;
                    }
                }
            }
            
            true
        } else {
            false
        }
    }
    
    fn to_string(&self) -> String {
        let plugins: Vec<String> = self.allowed_plugins.iter()
            .map(|p| p.to_string())
            .collect();
        
        let functions: Vec<String> = self.allowed_functions.iter().cloned().collect();
        
        format!("plugin_call(plugins=[{}], functions=[{}])",
            plugins.join(", "), functions.join(", "))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Message capability for inter-plugin communication.
#[derive(Debug, Clone)]
pub struct MessageCapability {
    /// Whether the plugin can send messages.
    pub send: bool,
    
    /// Whether the plugin can receive messages.
    pub receive: bool,
    
    /// Allowed recipients for sending.
    pub allowed_recipients: HashSet<PluginId>,
    
    /// Allowed topics for publishing/subscribing.
    pub allowed_topics: HashSet<String>,
}

impl MessageCapability {
    /// Create a new message capability.
    pub fn new(send: bool, receive: bool) -> Self {
        Self {
            send,
            receive,
            allowed_recipients: HashSet::new(),
            allowed_topics: HashSet::new(),
        }
    }
    
    /// Add an allowed recipient.
    pub fn add_recipient(&mut self, plugin_id: PluginId) {
        self.allowed_recipients.insert(plugin_id);
    }
    
    /// Add an allowed topic.
    pub fn add_topic(&mut self, topic: impl Into<String>) {
        self.allowed_topics.insert(topic.into());
    }
    
    /// Check if a recipient is allowed.
    pub fn is_recipient_allowed(&self, plugin_id: &PluginId) -> bool {
        // If no recipients are specified, allow all
        if self.allowed_recipients.is_empty() {
            return true;
        }
        
        self.allowed_recipients.contains(plugin_id)
    }
    
    /// Check if a topic is allowed.
    pub fn is_topic_allowed(&self, topic: &str) -> bool {
        // If no topics are specified, allow all
        if self.allowed_topics.is_empty() {
            return true;
        }
        
        self.allowed_topics.contains(topic)
    }
    
    /// Create a send-only capability.
    pub fn send_only() -> Self {
        Self::new(true, false)
    }
    
    /// Create a receive-only capability.
    pub fn receive_only() -> Self {
        Self::new(false, true)
    }
}

impl Capability for MessageCapability {
    fn get_type(&self) -> &str {
        "message"
    }
    
    fn includes(&self, other: &dyn Capability) -> bool {
        // Only include other message capabilities
        if other.get_type() != "message" {
            return false;
        }
        
        // Check if we're compatible with the other capability
        if let Some(other_msg) = other.as_any().downcast_ref::<MessageCapability>() {
            // Check send permission
            if other_msg.send && !self.send {
                return false;
            }
            
            // Check receive permission
            if other_msg.receive && !self.receive {
                return false;
            }
            
            // Check all other's recipients are included in our recipients
            if !other_msg.allowed_recipients.is_empty() {
                for plugin_id in &other_msg.allowed_recipients {
                    if !self.is_recipient_allowed(plugin_id) {
                        return false;
                    }
                }
            }
            
            // Check all other's topics are included in our topics
            if !other_msg.allowed_topics.is_empty() {
                for topic in &other_msg.allowed_topics {
                    if !self.is_topic_allowed(topic) {
                        return false;
                    }
                }
            }
            
            true
        } else {
            false
        }
    }
    
    fn to_string(&self) -> String {
        let perm = match (self.send, self.receive) {
            (true, true) => "send-receive",
            (true, false) => "send-only",
            (false, true) => "receive-only",
            (false, false) => "no-messaging",
        };
        
        let recipients: Vec<String> = self.allowed_recipients.iter()
            .map(|p| p.to_string())
            .collect();
        
        let topics: Vec<String> = self.allowed_topics.iter().cloned().collect();
        
        format!("message({}, recipients=[{}], topics=[{}])",
            perm, recipients.join(", "), topics.join(", "))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}