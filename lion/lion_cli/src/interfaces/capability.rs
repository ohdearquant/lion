//! Interface to the Lion capability component
//!
//! This module provides functions to interact with the Lion capability system,
//! which is responsible for managing and checking capabilities that control
//! what operations plugins are allowed to perform.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Grant a capability to a plugin
pub fn grant_capability(plugin_id: &str, capability_type: &str, params: &str) -> Result<()> {
    // In a real implementation, this would call into lion_capability::store
    #[cfg(feature = "capability-integration")]
    {
        use lion_capability::model::capability::Capability;
        use lion_capability::store::in_memory::InMemoryCapabilityStore;
        use lion_core::id::PluginId;

        let store = InMemoryCapabilityStore::global();
        let id = PluginId::from_str(plugin_id)
            .context(format!("Invalid plugin ID format: {}", plugin_id))?;

        let capability = match capability_type {
            "file" => {
                let file_params: FileCapabilityParams = serde_json::from_str(params)?;
                Capability::File {
                    path: file_params.path,
                    read: file_params.read,
                    write: file_params.write,
                    execute: file_params.execute,
                }
            }
            "network" => {
                let network_params: NetworkCapabilityParams = serde_json::from_str(params)?;
                Capability::Network {
                    host: network_params.host,
                    port: network_params.port,
                    protocol: network_params.protocol,
                }
            }
            "memory" => {
                let memory_params: MemoryCapabilityParams = serde_json::from_str(params)?;
                Capability::Memory {
                    limit_mb: memory_params.limit_mb,
                }
            }
            "plugin_call" => {
                let plugin_call_params: PluginCallCapabilityParams = serde_json::from_str(params)?;
                Capability::PluginCall {
                    target_plugin_id: PluginId::from_str(&plugin_call_params.target_plugin_id)?,
                    allowed_functions: plugin_call_params.allowed_functions,
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported capability type: {}",
                    capability_type
                ))
            }
        };

        store.add_capability(&id, capability)?;
        println!(
            "Capability {} granted to plugin {}",
            capability_type.bright_green(),
            plugin_id.bright_blue()
        );
    }

    #[cfg(not(feature = "capability-integration"))]
    {
        // Placeholder implementation
        println!("Granting capability to plugin: {}", plugin_id.bright_blue());
        println!("Capability type: {}", capability_type.bright_green());
        println!("Capability parameters: {}", params);
        println!("{}", "Capability granted successfully".bright_green());
        println!(
            "To check plugin capabilities, run: {}",
            "lion-cli plugin list".italic()
        );
    }

    Ok(())
}

/// Revoke a capability from a plugin
pub fn revoke_capability(plugin_id: &str, capability_id: &str) -> Result<()> {
    // In a real implementation, this would call into lion_capability::store
    #[cfg(feature = "capability-integration")]
    {
        use lion_capability::store::in_memory::InMemoryCapabilityStore;
        use lion_core::id::{CapabilityId, PluginId};

        let store = InMemoryCapabilityStore::global();
        let plugin_id = PluginId::from_str(plugin_id)
            .context(format!("Invalid plugin ID format: {}", plugin_id))?;
        let cap_id = CapabilityId::from_str(capability_id)
            .context(format!("Invalid capability ID format: {}", capability_id))?;

        store
            .remove_capability(&plugin_id, &cap_id)
            .context("Failed to revoke capability")?;
    }

    #[cfg(not(feature = "capability-integration"))]
    {
        // Placeholder implementation
        println!(
            "Revoking capability {} from plugin: {}",
            capability_id.bright_yellow(),
            plugin_id.bright_blue()
        );
        println!("{}", "Capability revoked successfully".bright_green());
    }

    Ok(())
}

/// List capabilities for a plugin
pub fn list_capabilities(plugin_id: &str) -> Result<Vec<CapabilityInfo>> {
    // In a real implementation, this would call into lion_capability::store
    #[cfg(feature = "capability-integration")]
    {
        use lion_capability::store::in_memory::InMemoryCapabilityStore;
        use lion_core::id::{CapabilityId, PluginId};

        let store = InMemoryCapabilityStore::global();
        let id = PluginId::from_str(plugin_id)
            .context(format!("Invalid plugin ID format: {}", plugin_id))?;

        let capabilities = store.get_capabilities(&id)?;

        let mut result = Vec::new();
        for (cap_id, cap) in capabilities {
            let info = match cap {
                Capability::File {
                    path,
                    read,
                    write,
                    execute,
                } => CapabilityInfo {
                    id: cap_id.to_string(),
                    capability_type: "file".to_string(),
                    description: format!(
                        "File access: {} (r:{} w:{} x:{})",
                        path, read, write, execute
                    ),
                    parameters: serde_json::to_string(&FileCapabilityParams {
                        path,
                        read,
                        write,
                        execute,
                    })?,
                },
                Capability::Network {
                    host,
                    port,
                    protocol,
                } => CapabilityInfo {
                    id: cap_id.to_string(),
                    capability_type: "network".to_string(),
                    description: format!("Network access: {}:{} ({})", host, port, protocol),
                    parameters: serde_json::to_string(&NetworkCapabilityParams {
                        host,
                        port,
                        protocol,
                    })?,
                },
                Capability::Memory { limit_mb } => CapabilityInfo {
                    id: cap_id.to_string(),
                    capability_type: "memory".to_string(),
                    description: format!("Memory limit: {} MB", limit_mb),
                    parameters: serde_json::to_string(&MemoryCapabilityParams { limit_mb })?,
                },
                Capability::PluginCall {
                    target_plugin_id,
                    allowed_functions,
                } => CapabilityInfo {
                    id: cap_id.to_string(),
                    capability_type: "plugin_call".to_string(),
                    description: format!(
                        "Plugin call: {} (functions: {:?})",
                        target_plugin_id, allowed_functions
                    ),
                    parameters: serde_json::to_string(&PluginCallCapabilityParams {
                        target_plugin_id: target_plugin_id.to_string(),
                        allowed_functions,
                    })?,
                },
                _ => CapabilityInfo {
                    id: cap_id.to_string(),
                    capability_type: "unknown".to_string(),
                    description: "Unknown capability type".to_string(),
                    parameters: "{}".to_string(),
                },
            };

            result.push(info);
        }

        Ok(result)
    }

    #[cfg(not(feature = "capability-integration"))]
    {
        // Placeholder implementation
        println!(
            "Listing capabilities for plugin: {}",
            plugin_id.bright_blue()
        );

        // Generate different mock capabilities based on plugin ID to match list_plugins output
        let mock_capabilities = if plugin_id.ends_with("174000") {
            // Calculator plugin capabilities
            vec![CapabilityInfo {
                id: "cap-1234-calculator".to_string(),
                capability_type: "memory".to_string(),
                description: "Memory limit: 32 MB".to_string(),
                parameters: r#"{"limit_mb":32.0}"#.to_string(),
            }]
        } else {
            // Text processor plugin capabilities
            vec![CapabilityInfo {
                id: "cap-5678-textproc".to_string(),
                capability_type: "file".to_string(),
                description: "File access: /tmp/* (r:true w:true x:false)".to_string(),
                parameters: r#"{"path":"/tmp/*","read":true,"write":true,"execute":false}"#
                    .to_string(),
            }]
        };
        Ok(mock_capabilities)
    }
}

/// Check if a plugin has a specific capability
pub fn check_capability(plugin_id: &str, access_type: &str, resource: &str) -> Result<bool> {
    // In a real implementation, this would call into lion_capability::check
    #[cfg(feature = "capability-integration")]
    {
        use lion_capability::check::engine::CapabilityEngine;
        use lion_core::id::PluginId;
        use lion_core::types::access::AccessRequest;

        let engine = CapabilityEngine::global();
        let id = PluginId::from_str(plugin_id).context("Invalid plugin ID format")?;

        let request = match access_type {
            "file_read" => AccessRequest::File {
                path: resource.to_string(),
                read: true,
                write: false,
                execute: false,
            },
            "file_write" => AccessRequest::File {
                path: resource.to_string(),
                read: false,
                write: true,
                execute: false,
            },
            "network_connect" => {
                let parts: Vec<&str> = resource.split(':').collect();
                if parts.len() != 2 {
                    return Err(anyhow::anyhow!(
                        "Invalid network resource format. Expected host:port"
                    ));
                }

                let host = parts[0].to_string();
                let port = parts[1].parse::<u16>().context("Invalid port number")?;

                AccessRequest::Network {
                    host,
                    port,
                    protocol: "tcp".to_string(),
                }
            }
            _ => return Err(anyhow::anyhow!("Unsupported access type: {}", access_type)),
        };

        Ok(engine.check_access(&id, &request).is_ok())
    }

    #[cfg(not(feature = "capability-integration"))]
    {
        // Placeholder implementation
        println!(
            "Checking if plugin {} has capability for {} access to {}",
            plugin_id, access_type, resource
        );

        // Mock check result
        let has_capability = match (access_type, resource) {
            ("file_read", "/tmp/test.txt") => true,
            ("file_write", "/etc/passwd") => false,
            ("network_connect", "example.com:80") => true,
            _ => false,
        };

        println!(
            "Capability check result: {}",
            if has_capability { "ALLOWED" } else { "DENIED" }
        );

        Ok(has_capability)
    }
}

/// Information about a capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInfo {
    pub id: String,
    pub capability_type: String,
    pub description: String,
    pub parameters: String,
}

/// Parameters for a file capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCapabilityParams {
    pub path: String,
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

/// Parameters for a network capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCapabilityParams {
    pub host: String,
    pub port: u16,
    pub protocol: String,
}

/// Parameters for a memory capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCapabilityParams {
    pub limit_mb: f64,
}

/// Parameters for a plugin call capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCallCapabilityParams {
    pub target_plugin_id: String,
    pub allowed_functions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grant_capability() {
        let plugin_id = uuid::Uuid::new_v4().to_string();
        let capability_type = "file";
        let params = r#"{"path":"/tmp/*","read":true,"write":false,"execute":false}"#;

        let result = grant_capability(&plugin_id, capability_type, params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_revoke_capability() {
        let plugin_id = uuid::Uuid::new_v4().to_string();
        let capability_id = uuid::Uuid::new_v4().to_string();

        let result = revoke_capability(&plugin_id, &capability_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_capabilities() {
        let plugin_id = uuid::Uuid::new_v4().to_string();

        let result = list_capabilities(&plugin_id);
        assert!(result.is_ok());

        let capabilities = result.unwrap();
        assert!(!capabilities.is_empty());
    }

    #[test]
    fn test_check_capability() {
        let plugin_id = uuid::Uuid::new_v4().to_string();

        let result = check_capability(&plugin_id, "file_read", "/tmp/test.txt");
        assert!(result.is_ok());
    }
}
