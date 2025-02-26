//! Access request data types.
//! 
//! This module defines data structures for access requests used in
//! capability checks. Access requests represent operations on resources
//! that are governed by the capability-based security system.

use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// A request to access a resource.
///
/// This enum represents different types of resource access requests
/// that can be checked against capabilities to determine if they
/// are permitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessRequest {
    /// File access.
    ///
    /// This represents a request to access a file in the filesystem.
    File {
        /// Path to the file.
        path: PathBuf,
        
        /// Whether this is a read operation.
        read: bool,
        
        /// Whether this is a write operation.
        write: bool,
        
        /// Whether this is an execute operation.
        execute: bool,
    },
    
    /// Network access.
    ///
    /// This represents a request to access the network.
    Network {
        /// Host to connect to or listen on.
        host: String,
        
        /// Port to connect to or listen on.
        port: u16,
        
        /// Whether this is an outbound connection.
        connect: bool,
        
        /// Whether this is an inbound connection.
        listen: bool,
    },
    
    /// Plugin call.
    ///
    /// This represents a request to call a function in another plugin.
    PluginCall {
        /// Plugin ID to call.
        plugin_id: String,
        
        /// Function to call.
        function: String,
    },
    
    /// Memory access.
    ///
    /// This represents a request to access a memory region.
    Memory {
        /// Region ID.
        region_id: String,
        
        /// Whether this is a read operation.
        read: bool,
        
        /// Whether this is a write operation.
        write: bool,
    },
    
    /// Message sending.
    ///
    /// This represents a request to send a message to another plugin.
    Message {
        /// Recipient plugin ID.
        recipient: String,
        
        /// Topic.
        topic: String,
    },
    
    /// Custom access type.
    ///
    /// This represents a custom access request type that doesn't
    /// fit into the predefined categories.
    Custom {
        /// Resource type.
        resource_type: String,
        
        /// Operation.
        operation: String,
        
        /// Additional parameters.
        params: serde_json::Value,
    },
}

impl AccessRequest {
    /// Get the type of this access request.
    ///
    /// # Returns
    ///
    /// The type of this access request.
    pub fn access_type(&self) -> AccessRequestType {
        match self {
            Self::File { .. } => AccessRequestType::File,
            Self::Network { .. } => AccessRequestType::Network,
            Self::PluginCall { .. } => AccessRequestType::PluginCall,
            Self::Memory { .. } => AccessRequestType::Memory,
            Self::Message { .. } => AccessRequestType::Message,
            Self::Custom { resource_type, .. } => AccessRequestType::Custom(resource_type.clone()),
        }
    }
    
    /// Create a file read access request.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to read.
    ///
    /// # Returns
    ///
    /// An access request for reading the specified file.
    pub fn file_read(path: impl Into<PathBuf>) -> Self {
        Self::File {
            path: path.into(),
            read: true,
            write: false,
            execute: false,
        }
    }
    
    /// Create a file write access request.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to write.
    ///
    /// # Returns
    ///
    /// An access request for writing to the specified file.
    pub fn file_write(path: impl Into<PathBuf>) -> Self {
        Self::File {
            path: path.into(),
            read: false,
            write: true,
            execute: false,
        }
    }
    
    /// Create a file execute access request.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to execute.
    ///
    /// # Returns
    ///
    /// An access request for executing the specified file.
    pub fn file_execute(path: impl Into<PathBuf>) -> Self {
        Self::File {
            path: path.into(),
            read: false,
            write: false,
            execute: true,
        }
    }
    
    /// Create a file read/write access request.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to read and write.
    ///
    /// # Returns
    ///
    /// An access request for reading and writing the specified file.
    pub fn file_read_write(path: impl Into<PathBuf>) -> Self {
        Self::File {
            path: path.into(),
            read: true,
            write: true,
            execute: false,
        }
    }
    
    /// Create a network connect access request.
    ///
    /// # Arguments
    ///
    /// * `host` - The host to connect to.
    /// * `port` - The port to connect to.
    ///
    /// # Returns
    ///
    /// An access request for connecting to the specified host and port.
    pub fn network_connect(host: impl Into<String>, port: u16) -> Self {
        Self::Network {
            host: host.into(),
            port,
            connect: true,
            listen: false,
        }
    }
    
    /// Create a network listen access request.
    ///
    /// # Arguments
    ///
    /// * `host` - The host to listen on.
    /// * `port` - The port to listen on.
    ///
    /// # Returns
    ///
    /// An access request for listening on the specified host and port.
    pub fn network_listen(host: impl Into<String>, port: u16) -> Self {
        Self::Network {
            host: host.into(),
            port,
            connect: false,
            listen: true,
        }
    }
    
    /// Create a plugin call access request.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to call.
    /// * `function` - The function to call.
    ///
    /// # Returns
    ///
    /// An access request for calling the specified function in the specified plugin.
    pub fn plugin_call(plugin_id: impl Into<String>, function: impl Into<String>) -> Self {
        Self::PluginCall {
            plugin_id: plugin_id.into(),
            function: function.into(),
        }
    }
    
    /// Create a memory read access request.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The ID of the memory region to read.
    ///
    /// # Returns
    ///
    /// An access request for reading the specified memory region.
    pub fn memory_read(region_id: impl Into<String>) -> Self {
        Self::Memory {
            region_id: region_id.into(),
            read: true,
            write: false,
        }
    }
    
    /// Create a memory write access request.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The ID of the memory region to write.
    ///
    /// # Returns
    ///
    /// An access request for writing to the specified memory region.
    pub fn memory_write(region_id: impl Into<String>) -> Self {
        Self::Memory {
            region_id: region_id.into(),
            read: false,
            write: true,
        }
    }
    
    /// Create a message sending access request.
    ///
    /// # Arguments
    ///
    /// * `recipient` - The ID of the plugin to send the message to.
    /// * `topic` - The topic of the message.
    ///
    /// # Returns
    ///
    /// An access request for sending a message to the specified plugin on the specified topic.
    pub fn message_send(recipient: impl Into<String>, topic: impl Into<String>) -> Self {
        Self::Message {
            recipient: recipient.into(),
            topic: topic.into(),
        }
    }
}

/// Type of resource access request.
///
/// This enum represents the general category of an access request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessRequestType {
    /// File access.
    File,
    
    /// Network access.
    Network,
    
    /// Plugin call.
    PluginCall,
    
    /// Memory access.
    Memory,
    
    /// Message sending.
    Message,
    
    /// Custom access type.
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_access_type() {
        // Test file access request
        let request = AccessRequest::file_read("/tmp/test.txt");
        assert_eq!(request.access_type(), AccessRequestType::File);
        
        // Test network access request
        let request = AccessRequest::network_connect("example.com", 80);
        assert_eq!(request.access_type(), AccessRequestType::Network);
        
        // Test plugin call access request
        let request = AccessRequest::plugin_call("plugin1", "function1");
        assert_eq!(request.access_type(), AccessRequestType::PluginCall);
        
        // Test memory access request
        let request = AccessRequest::memory_read("region1");
        assert_eq!(request.access_type(), AccessRequestType::Memory);
        
        // Test message access request
        let request = AccessRequest::message_send("plugin1", "topic1");
        assert_eq!(request.access_type(), AccessRequestType::Message);
        
        // Test custom access request
        let request = AccessRequest::Custom {
            resource_type: "custom".to_string(),
            operation: "operation".to_string(),
            params: serde_json::json!({}),
        };
        match request.access_type() {
            AccessRequestType::Custom(type_name) => assert_eq!(type_name, "custom"),
            _ => panic!("Expected AccessRequestType::Custom"),
        }
    }
    
    #[test]
    fn test_file_access_constructors() {
        // Test file_read
        let request = AccessRequest::file_read("/tmp/test.txt");
        match request {
            AccessRequest::File { path, read, write, execute } => {
                assert_eq!(path, PathBuf::from("/tmp/test.txt"));
                assert!(read);
                assert!(!write);
                assert!(!execute);
            },
            _ => panic!("Expected AccessRequest::File"),
        }
        
        // Test file_write
        let request = AccessRequest::file_write("/tmp/test.txt");
        match request {
            AccessRequest::File { path, read, write, execute } => {
                assert_eq!(path, PathBuf::from("/tmp/test.txt"));
                assert!(!read);
                assert!(write);
                assert!(!execute);
            },
            _ => panic!("Expected AccessRequest::File"),
        }
        
        // Test file_execute
        let request = AccessRequest::file_execute("/tmp/test.txt");
        match request {
            AccessRequest::File { path, read, write, execute } => {
                assert_eq!(path, PathBuf::from("/tmp/test.txt"));
                assert!(!read);
                assert!(!write);
                assert!(execute);
            },
            _ => panic!("Expected AccessRequest::File"),
        }
        
        // Test file_read_write
        let request = AccessRequest::file_read_write("/tmp/test.txt");
        match request {
            AccessRequest::File { path, read, write, execute } => {
                assert_eq!(path, PathBuf::from("/tmp/test.txt"));
                assert!(read);
                assert!(write);
                assert!(!execute);
            },
            _ => panic!("Expected AccessRequest::File"),
        }
    }
    
    #[test]
    fn test_network_access_constructors() {
        // Test network_connect
        let request = AccessRequest::network_connect("example.com", 80);
        match request {
            AccessRequest::Network { host, port, connect, listen } => {
                assert_eq!(host, "example.com");
                assert_eq!(port, 80);
                assert!(connect);
                assert!(!listen);
            },
            _ => panic!("Expected AccessRequest::Network"),
        }
        
        // Test network_listen
        let request = AccessRequest::network_listen("0.0.0.0", 8080);
        match request {
            AccessRequest::Network { host, port, connect, listen } => {
                assert_eq!(host, "0.0.0.0");
                assert_eq!(port, 8080);
                assert!(!connect);
                assert!(listen);
            },
            _ => panic!("Expected AccessRequest::Network"),
        }
    }
    
    #[test]
    fn test_plugin_call_constructor() {
        let request = AccessRequest::plugin_call("plugin1", "function1");
        match request {
            AccessRequest::PluginCall { plugin_id, function } => {
                assert_eq!(plugin_id, "plugin1");
                assert_eq!(function, "function1");
            },
            _ => panic!("Expected AccessRequest::PluginCall"),
        }
    }
    
    #[test]
    fn test_memory_access_constructors() {
        // Test memory_read
        let request = AccessRequest::memory_read("region1");
        match request {
            AccessRequest::Memory { region_id, read, write } => {
                assert_eq!(region_id, "region1");
                assert!(read);
                assert!(!write);
            },
            _ => panic!("Expected AccessRequest::Memory"),
        }
        
        // Test memory_write
        let request = AccessRequest::memory_write("region1");
        match request {
            AccessRequest::Memory { region_id, read, write } => {
                assert_eq!(region_id, "region1");
                assert!(!read);
                assert!(write);
            },
            _ => panic!("Expected AccessRequest::Memory"),
        }
    }
    
    #[test]
    fn test_message_send_constructor() {
        let request = AccessRequest::message_send("plugin1", "topic1");
        match request {
            AccessRequest::Message { recipient, topic } => {
                assert_eq!(recipient, "plugin1");
                assert_eq!(topic, "topic1");
            },
            _ => panic!("Expected AccessRequest::Message"),
        }
    }
    
    #[test]
    fn test_serialization() {
        // Test file access serialization
        let request = AccessRequest::file_read("/tmp/test.txt");
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: AccessRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(request, deserialized);
        
        // Test network access serialization
        let request = AccessRequest::network_connect("example.com", 80);
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: AccessRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(request, deserialized);
    }
}