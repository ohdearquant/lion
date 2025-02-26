//! Policy constraint model.
//! 
//! This module defines policy constraints.

use std::fmt;
use serde::{Serialize, Deserialize};

/// A policy constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint {
    /// Constrain file paths.
    FilePath(String),
    
    /// Constrain file operations.
    FileOperation {
        /// Whether read operations are allowed.
        read: bool,
        
        /// Whether write operations are allowed.
        write: bool,
        
        /// Whether execute operations are allowed.
        execute: bool,
    },
    
    /// Constrain network hosts.
    NetworkHost(String),
    
    /// Constrain network ports.
    NetworkPort(u16),
    
    /// Constrain network operations.
    NetworkOperation {
        /// Whether outbound connections are allowed.
        connect: bool,
        
        /// Whether listening for inbound connections is allowed.
        listen: bool,
    },
    
    /// Constrain plugin calls.
    PluginCall {
        /// Plugin ID.
        plugin_id: String,
        
        /// Function name.
        function: String,
    },
    
    /// Constrain memory regions.
    MemoryRegion {
        /// Region ID.
        region_id: String,
        
        /// Whether read operations are allowed.
        read: bool,
        
        /// Whether write operations are allowed.
        write: bool,
    },
    
    /// Constrain message sending.
    Message {
        /// Recipient plugin ID.
        recipient: String,
        
        /// Topic.
        topic: String,
    },
    
    /// Constrain resource usage.
    ResourceUsage {
        /// Maximum CPU usage.
        max_cpu: Option<f64>,
        
        /// Maximum memory usage.
        max_memory: Option<usize>,
        
        /// Maximum network usage.
        max_network: Option<usize>,
        
        /// Maximum disk usage.
        max_disk: Option<usize>,
    },
    
    /// Custom constraint.
    Custom {
        /// The constraint type.
        constraint_type: String,
        
        /// The constraint value.
        value: String,
    },
}

impl Constraint {
    /// Get the type of this constraint.
    pub fn constraint_type(&self) -> &str {
        match self {
            Self::FilePath(_) => "file_path",
            Self::FileOperation { .. } => "file_operation",
            Self::NetworkHost(_) => "network_host",
            Self::NetworkPort(_) => "network_port",
            Self::NetworkOperation { .. } => "network_operation",
            Self::PluginCall { .. } => "plugin_call",
            Self::MemoryRegion { .. } => "memory_region",
            Self::Message { .. } => "message",
            Self::ResourceUsage { .. } => "resource_usage",
            Self::Custom { constraint_type, .. } => constraint_type,
        }
    }
    
    /// Convert this policy constraint to a capability constraint.
    pub fn to_capability_constraint(&self) -> lion_capability::model::capability::Constraint {
        match self {
            Self::FilePath(path) => {
                lion_capability::model::capability::Constraint::FilePath(path.clone())
            },
            Self::FileOperation { read, write, execute } => {
                lion_capability::model::capability::Constraint::FileOperation {
                    read: *read,
                    write: *write,
                    execute: *execute,
                }
            },
            Self::NetworkHost(host) => {
                lion_capability::model::capability::Constraint::NetworkHost(host.clone())
            },
            Self::NetworkPort(port) => {
                lion_capability::model::capability::Constraint::NetworkPort(*port)
            },
            Self::NetworkOperation { connect, listen } => {
                lion_capability::model::capability::Constraint::NetworkOperation {
                    connect: *connect,
                    listen: *listen,
                }
            },
            Self::PluginCall { plugin_id, function } => {
                lion_capability::model::capability::Constraint::PluginCall {
                    plugin_id: plugin_id.clone(),
                    function: function.clone(),
                }
            },
            Self::MemoryRegion { region_id, read, write } => {
                lion_capability::model::capability::Constraint::MemoryRegion {
                    region_id: region_id.clone(),
                    read: *read,
                    write: *write,
                }
            },
            Self::Message { recipient, topic } => {
                lion_capability::model::capability::Constraint::Message {
                    recipient: recipient.clone(),
                    topic: topic.clone(),
                }
            },
            Self::ResourceUsage { .. } => {
                // Resource usage constraints don't map directly to capability constraints
                lion_capability::model::capability::Constraint::Custom {
                    constraint_type: "resource_usage".to_string(),
                    value: "".to_string(),
                }
            },
            Self::Custom { constraint_type, value } => {
                lion_capability::model::capability::Constraint::Custom {
                    constraint_type: constraint_type.clone(),
                    value: value.clone(),
                }
            },
        }
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FilePath(path) => write!(f, "File path '{}'", path),
            Self::FileOperation { read, write, execute } => {
                write!(f, "File operations: read={}, write={}, execute={}", read, write, execute)
            },
            Self::NetworkHost(host) => write!(f, "Network host '{}'", host),
            Self::NetworkPort(port) => write!(f, "Network port {}", port),
            Self::NetworkOperation { connect, listen } => {
                write!(f, "Network operations: connect={}, listen={}", connect, listen)
            },
            Self::PluginCall { plugin_id, function } => {
                write!(f, "Plugin call to '{}' function '{}'", plugin_id, function)
            },
            Self::MemoryRegion { region_id, read, write } => {
                write!(f, "Memory region '{}': read={}, write={}", region_id, read, write)
            },
            Self::Message { recipient, topic } => {
                write!(f, "Message to '{}' on topic '{}'", recipient, topic)
            },
            Self::ResourceUsage { max_cpu, max_memory, max_network, max_disk } => {
                write!(f, "Resource usage: ")?;
                
                if let Some(max_cpu) = max_cpu {
                    write!(f, "max_cpu={}, ", max_cpu)?;
                }
                
                if let Some(max_memory) = max_memory {
                    write!(f, "max_memory={}, ", max_memory)?;
                }
                
                if let Some(max_network) = max_network {
                    write!(f, "max_network={}, ", max_network)?;
                }
                
                if let Some(max_disk) = max_disk {
                    write!(f, "max_disk={}", max_disk)?;
                }
                
                Ok(())
            },
            Self::Custom { constraint_type, value } => {
                write!(f, "Custom constraint of type '{}' with value '{}'", constraint_type, value)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_constraint_type() {
        let constraint = Constraint::FilePath("/tmp".to_string());
        assert_eq!(constraint.constraint_type(), "file_path");
        
        let constraint = Constraint::FileOperation {
            read: true,
            write: false,
            execute: false,
        };
        assert_eq!(constraint.constraint_type(), "file_operation");
        
        let constraint = Constraint::Custom {
            constraint_type: "test".to_string(),
            value: "value".to_string(),
        };
        assert_eq!(constraint.constraint_type(), "test");
    }
    
    #[test]
    fn test_to_capability_constraint() {
        let constraint = Constraint::FilePath("/tmp".to_string());
        let cap_constraint = constraint.to_capability_constraint();
        
        match cap_constraint {
            lion_capability::model::capability::Constraint::FilePath(path) => {
                assert_eq!(path, "/tmp");
            },
            _ => panic!("Unexpected constraint type"),
        }
        
        let constraint = Constraint::FileOperation {
            read: true,
            write: false,
            execute: false,
        };
        let cap_constraint = constraint.to_capability_constraint();
        
        match cap_constraint {
            lion_capability::model::capability::Constraint::FileOperation { read, write, execute } => {
                assert!(read);
                assert!(!write);
                assert!(!execute);
            },
            _ => panic!("Unexpected constraint type"),
        }
    }
}