//! Access request data types.
//! 
//! This module defines data structures for access requests used in
//! capability checks.

use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// A request to access a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessRequest {
    /// File access.
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
    PluginCall {
        /// Plugin ID to call.
        plugin_id: String,
        
        /// Function to call.
        function: String,
    },
    
    /// Memory access.
    Memory {
        /// Region ID.
        region_id: String,
        
        /// Whether this is a read operation.
        read: bool,
        
        /// Whether this is a write operation.
        write: bool,
    },
    
    /// Message sending.
    Message {
        /// Recipient plugin ID.
        recipient: String,
        
        /// Topic.
        topic: String,
    },
    
    /// Custom access type.
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
    pub fn file_read(path: impl Into<PathBuf>) -> Self {
        Self::File {
            path: path.into(),
            read: true,
            write: false,
            execute: false,
        }
    }
    
    /// Create a file write access request.
    pub fn file_write(path: impl Into<PathBuf>) -> Self {
        Self::File {
            path: path.into(),
            read: false,
            write: true,
            execute: false,
        }
    }
    
    /// Create a file execute access request.
    pub fn file_execute(path: impl Into<PathBuf>) -> Self {
        Self::File {
            path: path.into(),
            read: false,
            write: false,
            execute: true,
        }
    }
    
    /// Create a network connect access request.
    pub fn network_connect(host: impl Into<String>, port: u16) -> Self {
        Self::Network {
            host: host.into(),
            port,
            connect: true,
            listen: false,
        }
    }
    
    /// Create a network listen access request.
    pub fn network_listen(host: impl Into<String>, port: u16) -> Self {
        Self::Network {
            host: host.into(),
            port,
            connect: false,
            listen: true,
        }
    }
}

/// Type of resource access request.
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