//! Network capability model.
//! 
//! This module defines capabilities for network access.

use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;

use super::capability::{Capability, Constraint};

/// A network host specification.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NetworkHost {
    /// A specific IPv4 or IPv6 address.
    IpAddress(IpAddr),
    
    /// A domain name.
    Domain(String),
    
    /// Any host.
    Any,
}

impl NetworkHost {
    /// Check if this host specification matches the given host.
    ///
    /// # Arguments
    ///
    /// * `host` - The host to check.
    ///
    /// # Returns
    ///
    /// `true` if the host matches, `false` otherwise.
    pub fn matches(&self, host: &str) -> bool {
        match self {
            Self::IpAddress(ip) => {
                // Try to parse the host as an IP address
                if let Ok(host_ip) = IpAddr::from_str(host) {
                    return host_ip == *ip;
                }
                
                // Maybe it's a domain that resolves to this IP?
                // For now, we just return false
                false
            },
            Self::Domain(domain) => {
                // Exact match
                if host == domain {
                    return true;
                }
                
                // Check if host is a subdomain of domain
                host.ends_with(&format!(".{}", domain))
            },
            Self::Any => true,
        }
    }
}

/// A network port specification.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NetworkPort {
    /// A specific port.
    Port(u16),
    
    /// A range of ports.
    Range(u16, u16),
    
    /// Any port.
    Any,
}

impl NetworkPort {
    /// Check if this port specification matches the given port.
    ///
    /// # Arguments
    ///
    /// * `port` - The port to check.
    ///
    /// # Returns
    ///
    /// `true` if the port matches, `false` otherwise.
    pub fn matches(&self, port: u16) -> bool {
        match self {
            Self::Port(p) => port == *p,
            Self::Range(start, end) => port >= *start && port <= *end,
            Self::Any => true,
        }
    }
}

/// A capability that grants permission to access the network.
#[derive(Debug, Clone)]
pub struct NetworkCapability {
    /// The hosts that are allowed.
    hosts: HashSet<NetworkHost>,
    
    /// The ports that are allowed.
    ports: HashSet<NetworkPort>,
    
    /// Whether outbound connections are allowed.
    connect: bool,
    
    /// Whether listening for inbound connections is allowed.
    listen: bool,
}

impl NetworkCapability {
    /// Create a new network capability.
    ///
    /// # Arguments
    ///
    /// * `hosts` - The hosts that are allowed.
    /// * `ports` - The ports that are allowed.
    /// * `connect` - Whether outbound connections are allowed.
    /// * `listen` - Whether listening for inbound connections is allowed.
    ///
    /// # Returns
    ///
    /// A new network capability.
    pub fn new(
        hosts: impl IntoIterator<Item = NetworkHost>,
        ports: impl IntoIterator<Item = NetworkPort>,
        connect: bool,
        listen: bool,
    ) -> Self {
        Self {
            hosts: hosts.into_iter().collect(),
            ports: ports.into_iter().collect(),
            connect,
            listen,
        }
    }
    
    /// Create a new outbound-only network capability.
    ///
    /// # Arguments
    ///
    /// * `hosts` - The hosts that are allowed.
    /// * `ports` - The ports that are allowed.
    ///
    /// # Returns
    ///
    /// A new outbound-only network capability.
    pub fn outbound_only(
        hosts: impl IntoIterator<Item = NetworkHost>,
        ports: impl IntoIterator<Item = NetworkPort>,
    ) -> Self {
        Self::new(hosts, ports, true, false)
    }
    
    /// Create a new inbound-only network capability.
    ///
    /// # Arguments
    ///
    /// * `hosts` - The hosts that are allowed.
    /// * `ports` - The ports that are allowed.
    ///
    /// # Returns
    ///
    /// A new inbound-only network capability.
    pub fn inbound_only(
        hosts: impl IntoIterator<Item = NetworkHost>,
        ports: impl IntoIterator<Item = NetworkPort>,
    ) -> Self {
        Self::new(hosts, ports, false, true)
    }
    
    /// Create a new bidirectional network capability.
    ///
    /// # Arguments
    ///
    /// * `hosts` - The hosts that are allowed.
    /// * `ports` - The ports that are allowed.
    ///
    /// # Returns
    ///
    /// A new bidirectional network capability.
    pub fn bidirectional(
        hosts: impl IntoIterator<Item = NetworkHost>,
        ports: impl IntoIterator<Item = NetworkPort>,
    ) -> Self {
        Self::new(hosts, ports, true, true)
    }
    
    /// Check if a host is allowed.
    ///
    /// # Arguments
    ///
    /// * `host` - The host to check.
    ///
    /// # Returns
    ///
    /// `true` if the host is allowed, `false` otherwise.
    fn is_host_allowed(&self, host: &str) -> bool {
        // If the hosts set is empty, nothing is allowed
        if self.hosts.is_empty() {
            return false;
        }
        
        // Check if any host specification matches
        self.hosts.iter().any(|h| h.matches(host))
    }
    
    /// Check if a port is allowed.
    ///
    /// # Arguments
    ///
    /// * `port` - The port to check.
    ///
    /// # Returns
    ///
    /// `true` if the port is allowed, `false` otherwise.
    fn is_port_allowed(&self, port: u16) -> bool {
        // If the ports set is empty, nothing is allowed
        if self.ports.is_empty() {
            return false;
        }
        
        // Check if any port specification matches
        self.ports.iter().any(|p| p.matches(port))
    }
    
    /// Get the allowed hosts.
    pub fn hosts(&self) -> &HashSet<NetworkHost> {
        &self.hosts
    }
    
    /// Get the allowed ports.
    pub fn ports(&self) -> &HashSet<NetworkPort> {
        &self.ports
    }
    
    /// Check if outbound connections are allowed.
    pub fn can_connect(&self) -> bool {
        self.connect
    }
    
    /// Check if inbound connections are allowed.
    pub fn can_listen(&self) -> bool {
        self.listen
    }
}

impl Capability for NetworkCapability {
    fn capability_type(&self) -> &str {
        "network"
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::Network { host, port, connect, listen } => {
                // Check if the host is allowed
                if !self.is_host_allowed(host) {
                    return Err(CapabilityError::PermissionDenied(
                        format!("Access to host {} is not allowed", host)
                    ).into());
                }
                
                // Check if the port is allowed
                if !self.is_port_allowed(*port) {
                    return Err(CapabilityError::PermissionDenied(
                        format!("Access to port {} is not allowed", port)
                    ).into());
                }
                
                // Check if the operations are allowed
                if *connect && !self.connect {
                    return Err(CapabilityError::PermissionDenied(
                        "Outbound connections are not allowed".into()
                    ).into());
                }
                
                if *listen && !self.listen {
                    return Err(CapabilityError::PermissionDenied(
                        "Inbound connections are not allowed".into()
                    ).into());
                }
                
                Ok(())
            },
            _ => Err(CapabilityError::PermissionDenied(
                "Only network access is allowed".into()
            ).into()),
        }
    }
    
    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        let mut hosts = self.hosts.clone();
        let mut ports = self.ports.clone();
        let mut connect = self.connect;
        let mut listen = self.listen;
        
        for constraint in constraints {
            match constraint {
                Constraint::NetworkHost(host) => {
                    // Add the host to the set
                    hosts.insert(NetworkHost::Domain(host.clone()));
                },
                Constraint::NetworkPort(port) => {
                    // Add the port to the set
                    ports.insert(NetworkPort::Port(*port));
                },
                Constraint::NetworkOperation { connect: c, listen: l } => {
                    // Can only remove permissions, not add them
                    connect = connect && *c;
                    listen = listen && *l;
                    
                    // If all operations are disallowed, return an error
                    if !connect && !listen {
                        return Err(CapabilityError::ConstraintError(
                            "No operations allowed after applying constraint".into()
                        ).into());
                    }
                },
                _ => return Err(CapabilityError::ConstraintError(
                    format!("Constraint type {} not supported for network capability", constraint.constraint_type())
                ).into()),
            }
        }
        
        Ok(Box::new(Self { hosts, ports, connect, listen }))
    }
    
    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut capabilities = Vec::new();
        
        // Split by operation
        if self.connect {
            capabilities.push(Box::new(Self::new(
                self.hosts.iter().cloned(),
                self.ports.iter().cloned(),
                true,
                false,
            )) as Box<dyn Capability>);
        }
        
        if self.listen {
            capabilities.push(Box::new(Self::new(
                self.hosts.iter().cloned(),
                self.ports.iter().cloned(),
                false,
                true,
            )) as Box<dyn Capability>);
        }
        
        // If we didn't split by operation, just clone
        if capabilities.is_empty() {
            capabilities.push(Box::new(self.clone()));
        }
        
        capabilities
    }
    
    fn can_join_with(&self, other: &dyn Capability) -> bool {
        other.capability_type() == "network"
    }
    
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        if !self.can_join_with(other) {
            return Err(CapabilityError::CompositionError(
                format!("Cannot join network capability with {}", other.capability_type())
            ).into());
        }
        
        // Downcast the other capability to a NetworkCapability
        let other = match other.permits(&AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: true,
        }) {
            Ok(()) => {
                // If it permits everything, it's probably a super-capability
                return Ok(Box::new(Self {
                    hosts: self.hosts.union(&self.hosts).cloned().collect(),
                    ports: self.ports.union(&self.ports).cloned().collect(),
                    connect: true,
                    listen: true,
                }));
            },
            Err(_) => {
                // Try to get more precise information
                let mut hosts = self.hosts.clone();
                let mut ports = self.ports.clone();
                let mut connect = self.connect;
                let mut listen = self.listen;
                
                // Check if it permits connect
                if other.permits(&AccessRequest::Network {
                    host: "example.com".to_string(),
                    port: 80,
                    connect: true,
                    listen: false,
                }).is_ok() {
                    connect = true;
                }
                
                // Check if it permits listen
                if other.permits(&AccessRequest::Network {
                    host: "example.com".to_string(),
                    port: 80,
                    connect: false,
                    listen: true,
                }).is_ok() {
                    listen = true;
                }
                
                // TODO: More precise host and port information
                
                Self {
                    hosts,
                    ports,
                    connect,
                    listen,
                }
            }
        };
        
        // Join the capabilities
        let joined = Self {
            hosts: self.hosts.union(&other.hosts).cloned().collect(),
            ports: self.ports.union(&other.ports).cloned().collect(),
            connect: self.connect || other.connect,
            listen: self.listen || other.listen,
        };
        
        Ok(Box::new(joined))
    }
    
    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_host_matches() {
        // Test IP address matching
        let host = NetworkHost::IpAddress("127.0.0.1".parse().unwrap());
        assert!(host.matches("127.0.0.1"));
        assert!(!host.matches("192.168.1.1"));
        assert!(!host.matches("localhost"));
        
        // Test domain matching
        let host = NetworkHost::Domain("example.com".to_string());
        assert!(host.matches("example.com"));
        assert!(host.matches("sub.example.com"));
        assert!(!host.matches("example.org"));
        
        // Test wildcard matching
        let host = NetworkHost::Any;
        assert!(host.matches("example.com"));
        assert!(host.matches("192.168.1.1"));
    }
    
    #[test]
    fn test_network_port_matches() {
        // Test specific port matching
        let port = NetworkPort::Port(80);
        assert!(port.matches(80));
        assert!(!port.matches(443));
        
        // Test port range matching
        let port = NetworkPort::Range(8000, 9000);
        assert!(port.matches(8000));
        assert!(port.matches(8500));
        assert!(port.matches(9000));
        assert!(!port.matches(7999));
        assert!(!port.matches(9001));
        
        // Test wildcard matching
        let port = NetworkPort::Any;
        assert!(port.matches(80));
        assert!(port.matches(443));
    }
    
    #[test]
    fn test_network_capability_permits() {
        let capability = NetworkCapability::new(
            vec![NetworkHost::Domain("example.com".to_string())],
            vec![NetworkPort::Port(80), NetworkPort::Port(443)],
            true,
            false,
        );
        
        // Test outbound connection to allowed host and port
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(capability.permits(&request).is_ok());
        
        // Test outbound connection to allowed host and disallowed port
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 8080,
            connect: true,
            listen: false,
        };
        assert!(capability.permits(&request).is_err());
        
        // Test outbound connection to disallowed host and allowed port
        let request = AccessRequest::Network {
            host: "evil.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(capability.permits(&request).is_err());
        
        // Test inbound connection to allowed host and port (should fail)
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: false,
            listen: true,
        };
        assert!(capability.permits(&request).is_err());
        
        // Test non-network access
        let request = AccessRequest::File {
            path: std::path::PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
    }
    
    #[test]
    fn test_network_capability_constrain() {
        let capability = NetworkCapability::new(
            vec![NetworkHost::Any],
            vec![NetworkPort::Any],
            true,
            true,
        );
        
        // Constrain to a specific host
        let constraints = vec![Constraint::NetworkHost("example.com".to_string())];
        let constrained = capability.constrain(&constraints).unwrap();
        
        // Should allow access to example.com
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Should still allow any port
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 1234,
            connect: true,
            listen: false,
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Constrain to outbound-only
        let constraints = vec![Constraint::NetworkOperation {
            connect: true,
            listen: false,
        }];
        let constrained = capability.constrain(&constraints).unwrap();
        
        // Should allow outbound connection
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Should deny inbound connection
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: false,
            listen: true,
        };
        assert!(constrained.permits(&request).is_err());
    }
    
    #[test]
    fn test_network_capability_split() {
        let capability = NetworkCapability::new(
            vec![NetworkHost::Any],
            vec![NetworkPort::Any],
            true,
            true,
        );
        
        let split = capability.split();
        assert_eq!(split.len(), 2);
        
        // Check that the first capability allows connect but not listen
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(split[0].permits(&request).is_ok());
        
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: false,
            listen: true,
        };
        assert!(split[0].permits(&request).is_err());
        
        // Check that the second capability allows listen but not connect
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(split[1].permits(&request).is_err());
        
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: false,
            listen: true,
        };
        assert!(split[1].permits(&request).is_ok());
    }
    
    #[test]
    fn test_network_capability_join() {
        let capability1 = NetworkCapability::new(
            vec![NetworkHost::Domain("example.com".to_string())],
            vec![NetworkPort::Port(80)],
            true,
            false,
        );
        
        let capability2 = NetworkCapability::new(
            vec![NetworkHost::Domain("example.org".to_string())],
            vec![NetworkPort::Port(443)],
            false,
            true,
        );
        
        let joined = capability1.join(&capability2).unwrap();
        
        // Should allow outbound connection to example.com:80
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should allow inbound connection to example.org:443
        let request = AccessRequest::Network {
            host: "example.org".to_string(),
            port: 443,
            connect: false,
            listen: true,
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should deny outbound connection to example.org:443
        let request = AccessRequest::Network {
            host: "example.org".to_string(),
            port: 443,
            connect: true,
            listen: false,
        };
        assert!(joined.permits(&request).is_err());
        
        // Should deny inbound connection to example.com:80
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: false,
            listen: true,
        };
        assert!(joined.permits(&request).is_err());
    }
}