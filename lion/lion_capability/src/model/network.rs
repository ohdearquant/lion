use bitflags::bitflags;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::capability::{AccessRequest, Capability, CapabilityError, Constraint};

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    /// Represents network operation permissions as a bit field
    pub struct NetworkOperations: u8 {
        const CONNECT = 0b00000001;
        const LISTEN = 0b00000010;
        const BIND = 0b00000100;
    }
}

/// Rule for matching network hosts
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostRule {
    /// Any host
    Any,
    /// Exact domain name
    Domain(String),
    /// Domain with wildcard prefix (e.g., *.example.com)
    WildcardDomain(String),
    /// Exact IP address
    ExactIp(IpAddr),
    /// IP subnet (IPv4)
    IpV4Subnet(Ipv4Addr, u8),
    /// IP subnet (IPv6)
    IpV6Subnet(Ipv6Addr, u8),
}

impl HostRule {
    /// Checks if a host matches this rule
    pub fn matches(&self, host: &str) -> bool {
        match self {
            HostRule::Any => true,
            HostRule::Domain(domain) => host == domain,
            HostRule::WildcardDomain(suffix) => {
                host.ends_with(suffix)
                    && host.len() > suffix.len()
                    && host.as_bytes()[host.len() - suffix.len() - 1] == b'.'
            }
            HostRule::ExactIp(ip) => {
                if let Ok(host_ip) = host.parse::<IpAddr>() {
                    &host_ip == ip
                } else {
                    false
                }
            }
            HostRule::IpV4Subnet(subnet_ip, prefix_len) => {
                if let Ok(host_ip) = host.parse::<IpAddr>() {
                    if let IpAddr::V4(ipv4) = host_ip {
                        // Compare masked IPs
                        let mask = !0u32 << (32 - prefix_len);
                        let subnet_bits = u32::from(*subnet_ip) & mask;
                        let ip_bits = u32::from(ipv4) & mask;
                        subnet_bits == ip_bits
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            HostRule::IpV6Subnet(subnet_ip, prefix_len) => {
                if let Ok(host_ip) = host.parse::<IpAddr>() {
                    if let IpAddr::V6(ipv6) = host_ip {
                        // Compare the relevant segments of the address based on prefix length
                        let segments = ipv6.segments();
                        let subnet_segments = subnet_ip.segments();

                        let full_segments = (*prefix_len / 16) as usize;

                        // Compare full segments
                        for i in 0..full_segments {
                            if segments[i] != subnet_segments[i] {
                                return false;
                            }
                        }

                        // Compare partial segment if needed
                        let remaining_bits = *prefix_len % 16;
                        if remaining_bits > 0 && full_segments < 8 {
                            let mask = !0u16 << (16 - remaining_bits);
                            let subnet_bits = subnet_segments[full_segments] & mask;
                            let ip_bits = segments[full_segments] & mask;
                            if subnet_bits != ip_bits {
                                return false;
                            }
                        }

                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    /// Returns true if this rule is more specific than or equal to another
    pub fn is_subset_of(&self, other: &HostRule) -> bool {
        match (self, other) {
            // Any host is a superset of all other rules
            (_, HostRule::Any) => true,
            (HostRule::Any, _) => false,

            // Exact domain is a subset of the same domain or a wildcard for that domain
            (HostRule::Domain(d1), HostRule::Domain(d2)) => d1 == d2,
            (HostRule::Domain(d), HostRule::WildcardDomain(suffix)) => {
                d.ends_with(suffix) && d.len() > suffix.len()
            }

            // Wildcard domain is a subset only if the other is the same wildcard or Any
            (HostRule::WildcardDomain(s1), HostRule::WildcardDomain(s2)) => {
                s1 == s2 || s1.ends_with(s2)
            }

            // Exact IP is a subset of itself or a subnet containing it
            (HostRule::ExactIp(ip1), HostRule::ExactIp(ip2)) => ip1 == ip2,
            (HostRule::ExactIp(IpAddr::V4(ip)), HostRule::IpV4Subnet(subnet, prefix_len)) => {
                let mask = !0u32 << (32 - prefix_len);
                let subnet_bits = u32::from(*subnet) & mask;
                let ip_bits = u32::from(*ip) & mask;
                subnet_bits == ip_bits
            }
            (HostRule::ExactIp(IpAddr::V6(ip)), HostRule::IpV6Subnet(subnet, prefix_len)) => {
                // Compare masked IPs for IPv6
                let segments = ip.segments();
                let subnet_segments = subnet.segments();

                let full_segments = (*prefix_len / 16) as usize;

                // Compare full segments
                for i in 0..full_segments {
                    if segments[i] != subnet_segments[i] {
                        return false;
                    }
                }

                // Compare partial segment if needed
                let remaining_bits = *prefix_len % 16;
                if remaining_bits > 0 && full_segments < 8 {
                    let mask = !0u16 << (16 - remaining_bits);
                    let subnet_bits = subnet_segments[full_segments] & mask;
                    let ip_bits = segments[full_segments] & mask;
                    if subnet_bits != ip_bits {
                        return false;
                    }
                }

                true
            }

            // IPv4 subnet is a subset of another IPv4 subnet if this subnet is contained in the other
            (
                HostRule::IpV4Subnet(subnet1, prefix_len1),
                HostRule::IpV4Subnet(subnet2, prefix_len2),
            ) => {
                // A subnet is a subset if its prefix is longer (more specific) and the network addresses match
                // when masked with the less specific subnet's mask
                if *prefix_len1 >= *prefix_len2 {
                    let mask = !0u32 << (32 - prefix_len2);
                    let subnet1_bits = u32::from(*subnet1) & mask;
                    let subnet2_bits = u32::from(*subnet2) & mask;
                    subnet1_bits == subnet2_bits
                } else {
                    false
                }
            }

            // IPv6 subnet is a subset of another IPv6 subnet
            (
                HostRule::IpV6Subnet(subnet1, prefix_len1),
                HostRule::IpV6Subnet(subnet2, prefix_len2),
            ) => {
                // Similar logic to IPv4 but with IPv6 addresses
                if *prefix_len1 >= *prefix_len2 {
                    let segments1 = subnet1.segments();
                    let segments2 = subnet2.segments();

                    let full_segments = (*prefix_len2 / 16) as usize;

                    // Compare full segments
                    for i in 0..full_segments {
                        if segments1[i] != segments2[i] {
                            return false;
                        }
                    }

                    // Compare partial segment if needed
                    let remaining_bits = *prefix_len2 % 16;
                    if remaining_bits > 0 && full_segments < 8 {
                        let mask = !0u16 << (16 - remaining_bits);
                        let subnet1_bits = segments1[full_segments] & mask;
                        let subnet2_bits = segments2[full_segments] & mask;
                        if subnet1_bits != subnet2_bits {
                            return false;
                        }
                    }

                    true
                } else {
                    false
                }
            }

            // All other combinations are not subset relationships
            _ => false,
        }
    }
}

/// Represents a capability to access network resources
#[derive(Debug, Clone)]
pub struct NetworkCapability {
    /// Set of host rules that this capability grants access to
    host_rules: HashSet<HostRule>,

    /// Map of port numbers to allowed operations
    port_operations: HashMap<Option<u16>, NetworkOperations>,

    /// Default operations allowed for any port not in the port_operations map
    default_operations: NetworkOperations,
}

impl NetworkCapability {
    /// Creates a new network capability
    pub fn new(
        host_rules: HashSet<HostRule>,
        port_operations: HashMap<Option<u16>, NetworkOperations>,
        default_operations: NetworkOperations,
    ) -> Self {
        Self {
            host_rules,
            port_operations,
            default_operations,
        }
    }

    /// Creates a new network capability that allows connections to specific hosts on any port
    pub fn outbound(host_rules: HashSet<HostRule>) -> Self {
        Self {
            host_rules,
            port_operations: HashMap::new(),
            default_operations: NetworkOperations::CONNECT,
        }
    }

    /// Creates a new network capability that allows listening on specific ports
    pub fn inbound(ports: Vec<u16>) -> Self {
        let mut port_operations = HashMap::new();

        for port in ports {
            port_operations.insert(
                Some(port),
                NetworkOperations::LISTEN | NetworkOperations::BIND,
            );
        }

        Self {
            host_rules: [HostRule::Any].into_iter().collect(),
            port_operations,
            default_operations: NetworkOperations::empty(),
        }
    }

    /// Returns host rules this capability grants access to
    pub fn host_rules(&self) -> &HashSet<HostRule> {
        &self.host_rules
    }

    /// Returns port operations this capability permits
    pub fn port_operations(&self) -> &HashMap<Option<u16>, NetworkOperations> {
        &self.port_operations
    }

    /// Checks if the capability allows access to the given host
    fn host_allowed(&self, host: &Option<String>) -> bool {
        match host {
            None => self.host_rules.contains(&HostRule::Any),
            Some(host_str) => self.host_rules.iter().any(|rule| rule.matches(host_str)),
        }
    }

    /// Gets the operations allowed for a specific port
    fn operations_for_port(&self, port: &Option<u16>) -> NetworkOperations {
        self.port_operations
            .get(port)
            .copied()
            .unwrap_or(self.default_operations)
    }

    /// Applies a host constraint to this capability
    fn apply_host_constraint(&self, host: &str) -> Result<Self, CapabilityError> {
        // Check if the host is allowed by any of our rules
        if !self.host_allowed(&Some(host.to_string())) {
            return Err(CapabilityError::InvalidConstraint(format!(
                "Host '{}' is not covered by this capability",
                host
            )));
        }

        // Create a new rule for this specific host
        let host_rule = if host.contains('*') {
            HostRule::WildcardDomain(host.to_string())
        } else if let Ok(ip) = host.parse::<IpAddr>() {
            HostRule::ExactIp(ip)
        } else {
            HostRule::Domain(host.to_string())
        };

        // Create a new capability with just this host rule
        let mut host_rules = HashSet::new();
        host_rules.insert(host_rule);

        Ok(Self {
            host_rules,
            port_operations: self.port_operations.clone(),
            default_operations: self.default_operations,
        })
    }

    /// Applies a port constraint to this capability
    fn apply_port_constraint(&self, port: u16) -> Result<Self, CapabilityError> {
        // Check if we have specific operations for this port, or use default
        let operations = self.operations_for_port(&Some(port));

        if operations.is_empty() {
            return Err(CapabilityError::InvalidConstraint(format!(
                "Port {} is not allowed by this capability",
                port
            )));
        }

        // Create a new capability with just this port
        let mut port_operations = HashMap::new();
        port_operations.insert(Some(port), operations);

        Ok(Self {
            host_rules: self.host_rules.clone(),
            port_operations,
            default_operations: NetworkOperations::empty(),
        })
    }

    /// Applies an operation constraint to this capability
    fn apply_operation_constraint(
        &self,
        connect: bool,
        listen: bool,
        bind: bool,
    ) -> Result<Self, CapabilityError> {
        let mut new_ops = NetworkOperations::empty();

        // Only allow operations that the constraint enables
        if connect {
            new_ops |= NetworkOperations::CONNECT;
        }
        if listen {
            new_ops |= NetworkOperations::LISTEN;
        }
        if bind {
            new_ops |= NetworkOperations::BIND;
        }

        // Apply the constraint to all port operations
        let mut port_operations = HashMap::new();
        for (port, ops) in &self.port_operations {
            let constrained_ops = ops.clone() & new_ops;
            if !constrained_ops.is_empty() {
                port_operations.insert(*port, constrained_ops);
            }
        }

        // Apply to default operations
        let default_operations = self.default_operations.clone() & new_ops;

        // Ensure we still have some operations allowed
        if port_operations.is_empty() && default_operations.is_empty() {
            return Err(CapabilityError::InvalidConstraint(
                "No operations would be allowed after applying this constraint".to_string(),
            ));
        }

        Ok(Self {
            host_rules: self.host_rules.clone(),
            port_operations,
            default_operations,
        })
    }
}

impl Capability for NetworkCapability {
    fn capability_type(&self) -> &str {
        "network"
    }

    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::Network {
                host,
                port,
                connect,
                listen,
                bind,
            } => {
                // Check if the host is allowed
                if !self.host_allowed(host) {
                    return Err(CapabilityError::AccessDenied(format!(
                        "Host '{:?}' is not allowed by this capability",
                        host
                    )));
                }

                // Get the operations allowed for this port
                let allowed_ops = self.operations_for_port(port);

                // Check if the requested operations are permitted
                let mut requested_ops = NetworkOperations::empty();
                if *connect {
                    requested_ops |= NetworkOperations::CONNECT;
                }
                if *listen {
                    requested_ops |= NetworkOperations::LISTEN;
                }
                if *bind {
                    requested_ops |= NetworkOperations::BIND;
                }

                if !allowed_ops.contains(requested_ops) {
                    return Err(CapabilityError::AccessDenied(format!(
                        "Operation not permitted on host '{:?}' port '{:?}'",
                        host, port
                    )));
                }

                Ok(())
            }
            _ => Err(CapabilityError::IncompatibleTypes(format!(
                "Expected Network request, got {:?}",
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
                Constraint::NetworkHost(host) => {
                    result = result.apply_host_constraint(host)?;
                }
                Constraint::NetworkPort(port) => {
                    result = result.apply_port_constraint(*port)?;
                }
                Constraint::NetworkOperation {
                    connect,
                    listen,
                    bind,
                } => {
                    result = result.apply_operation_constraint(*connect, *listen, *bind)?;
                }
                _ => {
                    return Err(CapabilityError::InvalidConstraint(format!(
                        "Constraint {:?} not applicable to NetworkCapability",
                        constraint
                    )))
                }
            }
        }

        Ok(Box::new(result))
    }

    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut result = Vec::new();

        // Split by host rule
        for host_rule in &self.host_rules {
            let mut host_rules = HashSet::new();
            host_rules.insert(host_rule.clone());

            result.push(Box::new(NetworkCapability {
                host_rules,
                port_operations: self.port_operations.clone(),
                default_operations: self.default_operations,
            }) as Box<dyn Capability>);
        }

        result
    }

    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to NetworkCapability
        if let Some(other_net) = other.as_any().downcast_ref::<NetworkCapability>() {
            // Create a union of the host rules
            let mut host_rules = self.host_rules.clone();
            host_rules.extend(other_net.host_rules.clone());

            // Merge port operations
            let mut port_operations = self.port_operations.clone();

            for (port, ops) in &other_net.port_operations {
                if let Some(existing_ops) = port_operations.get_mut(port) {
                    *existing_ops |= *ops;
                } else {
                    port_operations.insert(*port, *ops);
                }
            }

            // Union of default operations
            let default_operations =
                self.default_operations.clone() | other_net.default_operations.clone();

            Ok(Box::new(NetworkCapability {
                host_rules,
                port_operations,
                default_operations,
            }))
        } else {
            Err(CapabilityError::IncompatibleTypes(
                "Cannot join NetworkCapability with a different capability type".to_string(),
            ))
        }
    }

    fn leq(&self, other: &dyn Capability) -> bool {
        // Try to downcast the other capability to NetworkCapability
        if let Some(other_net) = other.as_any().downcast_ref::<NetworkCapability>() {
            // This capability is <= other if:
            // 1. All host rules in self have a superset in other
            // 2. All port operations in self are a subset of those in other

            // Check host rules
            for self_rule in &self.host_rules {
                if !other_net
                    .host_rules
                    .iter()
                    .any(|other_rule| self_rule.is_subset_of(other_rule))
                {
                    return false;
                }
            }

            // Check default operations
            let ops_intersection =
                self.default_operations.clone() & other_net.default_operations.clone();
            let self_bits = self.default_operations.bits();

            if ops_intersection.bits() != self_bits {
                return false;
            }

            // Check port operations
            for (port, ops) in &self.port_operations {
                let other_ops = other_net.operations_for_port(port);
                let ops_intersection = ops.clone() & other_ops;
                if ops_intersection.bits() != ops.bits() {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    fn meet(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to NetworkCapability
        if let Some(other_net) = other.as_any().downcast_ref::<NetworkCapability>() {
            // This is a simplification for the meet operation that creates host rules only for exact matches
            // A more complete implementation would find the intersection of host rules

            // Find host rules that are in both capabilities
            let mut host_rules = HashSet::new();
            for self_rule in &self.host_rules {
                for other_rule in &other_net.host_rules {
                    if self_rule == other_rule {
                        host_rules.insert(self_rule.clone());
                    }
                }
            }

            if host_rules.is_empty() {
                return Err(CapabilityError::InvalidState(
                    "No hosts in common between the capabilities".to_string(),
                ));
            }

            // Intersect port operations
            let mut port_operations = HashMap::new();

            // Collect all ports from both capabilities
            let mut all_ports = self.port_operations.keys().cloned().collect::<Vec<_>>();
            all_ports.extend(other_net.port_operations.keys().cloned());
            all_ports.sort();
            all_ports.dedup();

            for port in all_ports {
                let self_ops = self.operations_for_port(&port);
                let other_ops = other_net.operations_for_port(&port);

                let combined_ops = self_ops & other_ops;
                if !combined_ops.is_empty() {
                    port_operations.insert(port, combined_ops);
                }
            }

            // Intersect default operations
            let default_operations =
                self.default_operations.clone() & other_net.default_operations.clone();

            // Ensure there are some operations allowed
            if port_operations.is_empty() && default_operations.is_empty() {
                return Err(CapabilityError::InvalidState(
                    "No operations in common between the capabilities".to_string(),
                ));
            }

            Ok(Box::new(NetworkCapability {
                host_rules,
                port_operations,
                default_operations,
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
    fn test_host_rules_matching() {
        let any = HostRule::Any;
        let domain = HostRule::Domain("example.com".to_string());
        let wildcard = HostRule::WildcardDomain(".example.com".to_string());
        let exact_ip = HostRule::ExactIp("192.168.1.1".parse().unwrap());
        let subnet = HostRule::IpV4Subnet("192.168.1.0".parse().unwrap(), 24);

        // Test Any rule
        assert!(any.matches("example.com"));
        assert!(any.matches("192.168.1.1"));

        // Test Domain rule
        assert!(domain.matches("example.com"));
        assert!(!domain.matches("subdomain.example.com"));
        assert!(!domain.matches("example.org"));

        // Test Wildcard rule
        assert!(wildcard.matches("subdomain.example.com"));
        assert!(wildcard.matches("sub.sub.example.com"));
        assert!(!wildcard.matches("example.com"));
        assert!(!wildcard.matches("exampleXcom"));

        // Test ExactIp rule
        assert!(exact_ip.matches("192.168.1.1"));
        assert!(!exact_ip.matches("192.168.1.2"));
        assert!(!exact_ip.matches("example.com"));

        // Test Subnet rule
        assert!(subnet.matches("192.168.1.1"));
        assert!(subnet.matches("192.168.1.254"));
        assert!(!subnet.matches("192.168.2.1"));
        assert!(!subnet.matches("example.com"));
    }

    #[test]
    fn test_host_rule_subset() {
        let any = HostRule::Any;
        let domain = HostRule::Domain("example.com".to_string());
        let subdomain = HostRule::Domain("sub.example.com".to_string());
        let wildcard = HostRule::WildcardDomain(".example.com".to_string());
        let exact_ip = HostRule::ExactIp("192.168.1.1".parse().unwrap());
        let subnet = HostRule::IpV4Subnet("192.168.1.0".parse().unwrap(), 24);
        let smaller_subnet = HostRule::IpV4Subnet("192.168.1.0".parse().unwrap(), 28);

        // Test relationships with Any
        assert!(domain.is_subset_of(&any));
        assert!(wildcard.is_subset_of(&any));
        assert!(exact_ip.is_subset_of(&any));
        assert!(subnet.is_subset_of(&any));
        assert!(!any.is_subset_of(&domain));

        // Test domain relationships
        assert!(subdomain.is_subset_of(&wildcard));
        assert!(!domain.is_subset_of(&wildcard)); // example.com is not a subdomain of .example.com
        assert!(!wildcard.is_subset_of(&domain));

        // Test IP relationships
        assert!(exact_ip.is_subset_of(&subnet));
        assert!(!subnet.is_subset_of(&exact_ip));
        assert!(smaller_subnet.is_subset_of(&subnet));
        assert!(!subnet.is_subset_of(&smaller_subnet));
    }

    #[test]
    fn test_network_capability_permits() {
        // Create a capability that allows connecting to example.com and listening on port 8080
        let mut host_rules = HashSet::new();
        host_rules.insert(HostRule::Domain("example.com".to_string()));

        let mut port_operations = HashMap::new();
        port_operations.insert(
            Some(8080),
            NetworkOperations::LISTEN | NetworkOperations::BIND,
        );

        let cap = NetworkCapability::new(host_rules, port_operations, NetworkOperations::CONNECT);

        // Test valid connect request
        assert!(cap
            .permits(&AccessRequest::Network {
                host: Some("example.com".to_string()),
                port: None,
                connect: true,
                listen: false,
                bind: false,
            })
            .is_ok());

        // Test valid listen request
        assert!(cap
            .permits(&AccessRequest::Network {
                host: None,
                port: Some(8080),
                connect: false,
                listen: true,
                bind: false,
            })
            .is_ok());

        // Test invalid host
        assert!(cap
            .permits(&AccessRequest::Network {
                host: Some("example.org".to_string()),
                port: None,
                connect: true,
                listen: false,
                bind: false,
            })
            .is_err());

        // Test invalid operation on valid port
        assert!(cap
            .permits(&AccessRequest::Network {
                host: None,
                port: Some(9090),
                connect: false,
                listen: true,
                bind: false,
            })
            .is_err());
    }

    #[test]
    fn test_network_capability_constrain() {
        // Create a capability that allows connecting to any host and listening on any port
        let mut host_rules = HashSet::new();
        host_rules.insert(HostRule::Any);

        let cap = NetworkCapability::new(host_rules, HashMap::new(), NetworkOperations::all());

        // Constrain to specific host
        let constrained = cap
            .constrain(&[Constraint::NetworkHost("example.com".to_string())])
            .unwrap();

        assert!(constrained
            .permits(&AccessRequest::Network {
                host: Some("example.com".to_string()),
                port: None,
                connect: true,
                listen: false,
                bind: false,
            })
            .is_ok());

        assert!(constrained
            .permits(&AccessRequest::Network {
                host: Some("example.org".to_string()),
                port: None,
                connect: true,
                listen: false,
                bind: false,
            })
            .is_err());

        // Constrain to specific port
        let constrained = cap.constrain(&[Constraint::NetworkPort(8080)]).unwrap();

        assert!(constrained
            .permits(&AccessRequest::Network {
                host: None,
                port: Some(8080),
                connect: true,
                listen: false,
                bind: false,
            })
            .is_ok());

        assert!(constrained
            .permits(&AccessRequest::Network {
                host: None,
                port: Some(9090),
                connect: true,
                listen: false,
                bind: false,
            })
            .is_err());

        // Constrain to specific operation
        let constrained = cap
            .constrain(&[Constraint::NetworkOperation {
                connect: true,
                listen: false,
                bind: false,
            }])
            .unwrap();

        assert!(constrained
            .permits(&AccessRequest::Network {
                host: None,
                port: None,
                connect: true,
                listen: false,
                bind: false,
            })
            .is_ok());

        assert!(constrained
            .permits(&AccessRequest::Network {
                host: None,
                port: None,
                connect: false,
                listen: true,
                bind: false,
            })
            .is_err());
    }

    #[test]
    fn test_network_capability_join() {
        // Create two capabilities with different hosts and operations
        let mut host_rules1 = HashSet::new();
        host_rules1.insert(HostRule::Domain("example.com".to_string()));

        let mut port_operations1 = HashMap::new();
        port_operations1.insert(Some(8080), NetworkOperations::LISTEN);

        let cap1 =
            NetworkCapability::new(host_rules1, port_operations1, NetworkOperations::empty());

        let mut host_rules2 = HashSet::new();
        host_rules2.insert(HostRule::Domain("example.org".to_string()));

        let mut port_operations2 = HashMap::new();
        port_operations2.insert(Some(9090), NetworkOperations::CONNECT);

        let cap2 =
            NetworkCapability::new(host_rules2, port_operations2, NetworkOperations::empty());

        // Join the capabilities
        let joined = cap1.join(&cap2).unwrap();

        // The joined capability should allow both hosts and operations
        assert!(joined
            .permits(&AccessRequest::Network {
                host: Some("example.com".to_string()),
                port: Some(8080),
                connect: false,
                listen: true,
                bind: false,
            })
            .is_ok());

        assert!(joined
            .permits(&AccessRequest::Network {
                host: Some("example.org".to_string()),
                port: Some(9090),
                connect: true,
                listen: false,
                bind: false,
            })
            .is_ok());
    }
}
