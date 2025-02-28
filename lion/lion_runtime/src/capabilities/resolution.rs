//! Capability Resolution for Lion Runtime
//!
//! Provides mechanisms to resolve and check capabilities against policies.

use std::sync::Arc;

use super::manager::CapabilityOperation;
use anyhow::{Context, Result};
use lion_core::CapabilityId;
use thiserror::Error;
use tracing::{debug, error, info, warn};

use super::manager::CapabilityManager;

/// Errors that can occur during capability resolution
#[derive(Debug, Error)]
pub enum ResolutionError {
    #[error("Capability resolution failed: {0}")]
    Failed(String),

    #[error("Operation {0} not allowed by capability")]
    NotAllowed(String),

    #[error("Policy denied operation {0}")]
    PolicyDenied(String),
}

/// Capability resolver for checking permissions
pub struct CapabilityResolver {
    /// Reference to the capability manager
    manager: Arc<CapabilityManager>,
}

impl CapabilityResolver {
    /// Create a new capability resolver
    pub fn new(manager: Arc<CapabilityManager>) -> Self {
        Self { manager }
    }

    /// Check if a subject can perform an operation on an object
    pub async fn can_access(&self, subject: &str, object: &str, operation: &str) -> Result<bool> {
        debug!(
            "Checking if subject '{}' can perform '{}' on '{}'",
            subject, operation, object
        );

        // Use the capability manager to check permission
        match self
            .manager
            .check_permission(subject, object, operation)
            .await
        {
            Ok(_) => {
                debug!(
                    "Access granted for '{}' to perform '{}' on '{}'",
                    subject, operation, object
                );
                Ok(true)
            }
            Err(e) => {
                debug!(
                    "Access denied for '{}' to perform '{}' on '{}': {}",
                    subject, operation, object, e
                );
                Ok(false)
            }
        }
    }

    /// Authorize an operation or return an error
    pub async fn authorize(&self, subject: &str, object: &str, operation: &str) -> Result<()> {
        debug!(
            "Authorizing subject '{}' to perform '{}' on '{}'",
            subject, operation, object
        );

        // Use the capability manager to check permission
        self.manager
            .check_permission(subject, object, operation)
            .await
            .context(format!(
                "Authorization failed for '{}' to perform '{}' on '{}'",
                subject, operation, object
            ))
    }

    /// Check multiple operations at once
    pub async fn batch_authorize(
        &self,
        subject: &str,
        object: &str,
        operations: &[&str],
    ) -> Result<()> {
        debug!(
            "Batch authorizing subject '{}' for {} operations on '{}'",
            subject,
            operations.len(),
            object
        );

        // Check each operation
        for &operation in operations {
            self.authorize(subject, object, operation).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_can_access() {
        // Create a capability manager
        let manager = Arc::new(CapabilityManager::new().unwrap());

        // Grant some capabilities
        let cap_id = manager
            .grant_capability(
                "subject1".to_string(),
                "object1".to_string(),
                vec!["read".to_string(), "write".to_string()],
            )
            .await
            .unwrap();

        // Create the resolver
        let resolver = CapabilityResolver::new(manager.clone());

        // Check access
        assert!(resolver
            .can_access("subject1", "object1", "read")
            .await
            .unwrap());
        assert!(resolver
            .can_access("subject1", "object1", "write")
            .await
            .unwrap());
        assert!(!resolver
            .can_access("subject1", "object1", "execute")
            .await
            .unwrap());
        assert!(!resolver
            .can_access("subject2", "object1", "read")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_authorize() {
        // Create a capability manager
        let manager = Arc::new(CapabilityManager::new().unwrap());

        // Grant some capabilities
        let cap_id = manager
            .grant_capability(
                "subject1".to_string(),
                "object1".to_string(),
                vec!["read".to_string(), "write".to_string()],
            )
            .await
            .unwrap();

        // Create the resolver
        let resolver = CapabilityResolver::new(manager.clone());

        // Test authorize
        resolver
            .authorize("subject1", "object1", "read")
            .await
            .unwrap();
        resolver
            .authorize("subject1", "object1", "write")
            .await
            .unwrap();

        // These should fail
        assert!(resolver
            .authorize("subject1", "object1", "execute")
            .await
            .is_err());
        assert!(resolver
            .authorize("subject2", "object1", "read")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_batch_authorize() {
        // Create a capability manager
        let manager = Arc::new(CapabilityManager::new().unwrap());

        // Grant some capabilities
        let cap_id = manager
            .grant_capability(
                "subject1".to_string(),
                "object1".to_string(),
                vec!["read".to_string(), "write".to_string()],
            )
            .await
            .unwrap();

        // Create the resolver
        let resolver = CapabilityResolver::new(manager.clone());

        // Test batch authorize
        resolver
            .batch_authorize("subject1", "object1", &["read", "write"])
            .await
            .unwrap();

        // This should fail because execute is not granted
        assert!(resolver
            .batch_authorize("subject1", "object1", &["read", "execute"])
            .await
            .is_err());
    }
}
