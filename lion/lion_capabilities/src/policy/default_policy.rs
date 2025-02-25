//! Default implementation of the capability policy.

use super::CapabilityPolicy;
use lion_core::capability::CoreCapability;
use lion_core::plugin::PluginId;
use std::collections::HashMap;
use std::path::PathBuf;

/// A default capability policy implementation
pub struct DefaultCapabilityPolicy {
    /// Whether to allow all capabilities by default
    allow_all: bool,
    
    /// Plugin-specific policies (plugin_id -> allowed capabilities)
    plugin_policies: HashMap<PluginId, Vec<CoreCapability>>,
    
    /// Default allowed file system paths
    allowed_fs_paths: Vec<PathBuf>,
    
    /// Default allowed network hosts
    allowed_network_hosts: Vec<String>,
    
    /// Whether to allow inter-plugin communication by default
    allow_interplugin_comm: bool,
}

impl DefaultCapabilityPolicy {
    /// Create a new default capability policy
    pub fn new(allow_all: bool) -> Self {
        Self {
            allow_all,
            plugin_policies: HashMap::new(),
            allowed_fs_paths: Vec::new(),
            allowed_network_hosts: Vec::new(),
            allow_interplugin_comm: true,
        }
    }
    
    /// Set plugin-specific policies
    pub fn with_plugin_policies(
        mut self,
        plugin_policies: HashMap<PluginId, Vec<CoreCapability>>,
    ) -> Self {
        self.plugin_policies = plugin_policies;
        self
    }
    
    /// Set allowed file system paths
    pub fn with_allowed_fs_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_fs_paths = paths;
        self
    }
    
    /// Set allowed network hosts
    pub fn with_allowed_network_hosts(mut self, hosts: Vec<String>) -> Self {
        self.allowed_network_hosts = hosts;
        self
    }
    
    /// Set whether to allow inter-plugin communication
    pub fn with_allow_interplugin_comm(mut self, allow: bool) -> Self {
        self.allow_interplugin_comm = allow;
        self
    }
    
    /// Add a plugin-specific policy
    pub fn add_plugin_policy(
        &mut self,
        plugin_id: PluginId,
        capabilities: Vec<CoreCapability>,
    ) {
        self.plugin_policies.insert(plugin_id, capabilities);
    }
    
    /// Add an allowed file system path
    pub fn add_allowed_fs_path(&mut self, path: PathBuf) {
        self.allowed_fs_paths.push(path);
    }
    
    /// Add an allowed network host
    pub fn add_allowed_network_host(&mut self, host: String) {
        self.allowed_network_hosts.push(host);
    }
    
    /// Check if a path is allowed
    fn is_path_allowed(&self, path: Option<&String>) -> bool {
        // If no path is specified, check if any paths are allowed
        let path = match path {
            Some(path) => path,
            None => return !self.allowed_fs_paths.is_empty(),
        };
        
        // If no allowed paths are specified, deny all
        if self.allowed_fs_paths.is_empty() {
            return false;
        }
        
        // Check if the path is under any allowed path
        let path = PathBuf::from(path);
        self.allowed_fs_paths.iter().any(|allowed_path| {
            path.starts_with(allowed_path)
        })
    }
    
    /// Check if a host is allowed
    fn is_host_allowed(&self, hosts: Option<&Vec<String>>) -> bool {
        // If no hosts are specified, check if any hosts are allowed
        let hosts = match hosts {
            Some(hosts) => hosts,
            None => return !self.allowed_network_hosts.is_empty(),
        };
        
        // If no allowed hosts are specified, deny all
        if self.allowed_network_hosts.is_empty() {
            return false;
        }
        
        // Check if all hosts are allowed
        hosts.iter().all(|host| {
            self.allowed_network_hosts.iter().any(|allowed_host| {
                // Exact match or wildcard
                host == allowed_host || 
                (allowed_host.starts_with("*.") && host.ends_with(&allowed_host[1..]))
            })
        })
    }
}

impl CapabilityPolicy for DefaultCapabilityPolicy {
    fn can_grant_capability(
        &self,
        plugin_id: PluginId,
        capability: &CoreCapability,
    ) -> bool {
        // Check plugin-specific policy first
        if let Some(allowed_capabilities) = self.plugin_policies.get(&plugin_id) {
            return allowed_capabilities.iter().any(|allowed| {
                match (allowed, capability) {
                    (
                        CoreCapability::FileSystemRead { path: allowed_path },
                        CoreCapability::FileSystemRead { path: requested_path },
                    ) => {
                        // Allow if the allowed path is None or the requested path is under the allowed path
                        match (allowed_path, requested_path) {
                            (None, _) => true,
                            (Some(_), None) => true,
                            (Some(allowed), Some(requested)) => {
                                let allowed = PathBuf::from(allowed);
                                let requested = PathBuf::from(requested);
                                requested.starts_with(allowed)
                            }
                        }
                    }
                    (
                        CoreCapability::FileSystemWrite { path: allowed_path },
                        CoreCapability::FileSystemWrite { path: requested_path },
                    ) => {
                        // Allow if the allowed path is None or the requested path is under the allowed path
                        match (allowed_path, requested_path) {
                            (None, _) => true,
                            (Some(_), None) => true,
                            (Some(allowed), Some(requested)) => {
                                let allowed = PathBuf::from(allowed);
                                let requested = PathBuf::from(requested);
                                requested.starts_with(allowed)
                            }
                        }
                    }
                    (
                        CoreCapability::NetworkClient { hosts: allowed_hosts },
                        CoreCapability::NetworkClient { hosts: requested_hosts },
                    ) => {
                        // Allow if the allowed hosts is None or all requested hosts are in the allowed hosts
                        match (allowed_hosts, requested_hosts) {
                            (None, _) => true,
                            (Some(_), None) => true,
                            (Some(allowed), Some(requested)) => {
                                requested.iter().all(|req_host| {
                                    allowed.iter().any(|allowed_host| {
                                        // Exact match or wildcard
                                        req_host == allowed_host || 
                                        (allowed_host.starts_with("*.") && req_host.ends_with(&allowed_host[1..]))
                                    })
                                })
                            }
                        }
                    }
                    (CoreCapability::InterPluginComm, CoreCapability::InterPluginComm) => true,
                    _ => false,
                }
            });
        }
        
        // Fall back to global policy
        if self.allow_all {
            return true;
        }
        
        match capability {
            CoreCapability::FileSystemRead { path } => self.is_path_allowed(path.as_ref()),
            CoreCapability::FileSystemWrite { path } => self.is_path_allowed(path.as_ref()),
            CoreCapability::NetworkClient { hosts } => self.is_host_allowed(hosts.as_ref()),
            CoreCapability::InterPluginComm => self.allow_interplugin_comm,
        }
    }
    
    fn get_denial_reason(
        &self,
        plugin_id: PluginId,
        capability: &CoreCapability,
    ) -> Option<String> {
        if self.can_grant_capability(plugin_id, capability) {
            return None;
        }
        
        Some(match capability {
            CoreCapability::FileSystemRead { path } => {
                format!(
                    "File system read access to {} is not allowed",
                    path.as_ref().map(|p| p.as_str()).unwrap_or("any path")
                )
            }
            CoreCapability::FileSystemWrite { path } => {
                format!(
                    "File system write access to {} is not allowed",
                    path.as_ref().map(|p| p.as_str()).unwrap_or("any path")
                )
            }
            CoreCapability::NetworkClient { hosts } => {
                if let Some(hosts) = hosts {
                    format!("Network access to hosts {:?} is not allowed", hosts)
                } else {
                    "Network access is not allowed".to_string()
                }
            }
            CoreCapability::InterPluginComm => {
                "Inter-plugin communication is not allowed".to_string()
            }
        })
    }
}

impl Default for DefaultCapabilityPolicy {
    fn default() -> Self {
        // By default, create a restrictive policy
        Self::new(false)
            .with_allowed_fs_paths(vec![PathBuf::from("/tmp")])
            .with_allowed_network_hosts(vec![])
            .with_allow_interplugin_comm(true)
    }
}