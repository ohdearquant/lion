//! Network policy implementation.

/// A policy for network access
pub struct NetworkPolicy {
    /// Allowed hosts for network access
    allowed_hosts: Vec<String>,
    
    /// Allowed ports for network access
    allowed_ports: Vec<u16>,
    
    /// Whether to disallow access to any host/port outside the allowed list
    strict: bool,
}

impl NetworkPolicy {
    /// Create a new network policy
    pub fn new() -> Self {
        Self {
            allowed_hosts: Vec::new(),
            allowed_ports: Vec::new(),
            strict: true,
        }
    }
    
    /// Set whether to enforce strict host/port checking
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }
    
    /// Add an allowed host
    pub fn add_host(&mut self, host: String) {
        self.allowed_hosts.push(host);
    }
    
    /// Add an allowed port
    pub fn add_port(&mut self, port: u16) {
        self.allowed_ports.push(port);
    }
    
    /// Set allowed hosts
    pub fn with_hosts(mut self, hosts: Vec<String>) -> Self {
        self.allowed_hosts = hosts;
        self
    }
    
    /// Set allowed ports
    pub fn with_ports(mut self, ports: Vec<u16>) -> Self {
        self.allowed_ports = ports;
        self
    }
    
    /// Check if a host is allowed
    pub fn can_access_host(&self, host: &str) -> bool {
        if !self.strict {
            return true;
        }
        
        if self.allowed_hosts.is_empty() {
            return false;
        }
        
        self.allowed_hosts.iter().any(|allowed| {
            if allowed.starts_with("*.") {
                // Wildcard domain match
                host.ends_with(&allowed[1..])
            } else {
                // Exact match
                host == allowed
            }
        })
    }
    
    /// Check if a port is allowed
    pub fn can_access_port(&self, port: u16) -> bool {
        if !self.strict {
            return true;
        }
        
        if self.allowed_ports.is_empty() {
            return true; // Default to allowing any port if none are specified
        }
        
        self.allowed_ports.contains(&port)
    }
    
    /// Validate a host
    pub fn validate_host(&self, host: &str) -> Result<String, String> {
        // Basic validation
        if host.is_empty() {
            return Err("Host cannot be empty".to_string());
        }
        
        // Check against allowed hosts
        if self.strict && !self.can_access_host(host) {
            return Err(format!("Access to host '{}' is not allowed", host));
        }
        
        Ok(host.to_string())
    }
    
    /// Validate a URL
    pub fn validate_url(&self, url: &str) -> Result<String, String> {
        // Parse URL to extract host and port
        let url_obj = url::Url::parse(url)
            .map_err(|e| format!("Invalid URL: {}", e))?;
        
        // Get host
        let host = url_obj.host_str()
            .ok_or_else(|| "URL has no host".to_string())?;
        
        // Validate host
        self.validate_host(host)?;
        
        // Get port
        if let Some(port) = url_obj.port() {
            if self.strict && !self.can_access_port(port) {
                return Err(format!("Access to port {} is not allowed", port));
            }
        }
        
        Ok(url.to_string())
    }
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            // By default, allow localhost and example.com for testing
            allowed_hosts: vec![
                "localhost".to_string(),
                "127.0.0.1".to_string(),
                "example.com".to_string(),
            ],
            // By default, allow common HTTP/HTTPS ports
            allowed_ports: vec![80, 443, 8000, 8080],
            strict: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_policy() {
        let mut policy = NetworkPolicy::new();
        policy.add_host("api.example.com".to_string());
        policy.add_host("*.mycompany.com".to_string());
        policy.add_port(443);
        
        // Test host permissions
        assert!(policy.can_access_host("api.example.com"));
        assert!(policy.can_access_host("dev.mycompany.com"));
        assert!(policy.can_access_host("api.mycompany.com"));
        assert!(!policy.can_access_host("malicious.com"));
        
        // Test port permissions
        assert!(policy.can_access_port(443));
        assert!(!policy.can_access_port(22));
    }
    
    #[test]
    fn test_url_validation() {
        let policy = NetworkPolicy::new()
            .with_hosts(vec!["api.example.com".to_string(), "*.mycompany.com".to_string()])
            .with_ports(vec![443]);
        
        // Valid URLs
        assert!(policy.validate_url("https://api.example.com/data").is_ok());
        assert!(policy.validate_url("https://dev.mycompany.com/api").is_ok());
        
        // Invalid host
        let result = policy.validate_url("https://malicious.com/hack");
        assert!(result.is_err());
        
        // Invalid port
        let result = policy.validate_url("https://api.example.com:8080/data");
        assert!(result.is_err());
    }
}