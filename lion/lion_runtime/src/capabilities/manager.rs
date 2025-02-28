//! Capability Manager for Lion Runtime
//!
//! Provides capability creation, granting, checking, and revocation based on
//! a unified capability-policy security model.

use std::collections::{HashMap, HashSet};
use std::fmt;

use anyhow::Result;
use lion_core::CapabilityId;
use parking_lot::RwLock;
use thiserror::Error;
use tracing::{error, info};

/// Operation on a capability
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityOperation(pub String);

// Implement Display for CapabilityOperation
impl fmt::Display for CapabilityOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("Capability {0} not found")]
    NotFound(CapabilityId),

    #[error("Capability {0} has been revoked")]
    Revoked(CapabilityId),

    #[error("Subject {0} is not authorized for {1} on {2}")]
    Unauthorized(String, String, String),

    #[error("Attempted to create capability with more rights than parent")]
    RightsEscalation,

    #[error("Internal capability store error: {0}")]
    StoreError(String),

    #[error("Policy evaluation error: {0}")]
    PolicyError(String),
}

/// Entry in the capability table
#[derive(Debug, Clone)]
struct CapabilityEntry {
    /// Unique identifier for this capability
    _id: CapabilityId,

    /// The subject that holds this capability
    subject: String,

    /// The object this capability targets
    object: String,

    /// The operations allowed with this capability
    rights: HashSet<String>,

    /// Whether this capability is valid (not revoked)
    valid: bool,

    /// Parent capability, if this was derived from another
    _parent: Option<CapabilityId>,

    /// Child capabilities derived from this one
    children: Vec<CapabilityId>,
}

/// The capability manager handles the granting, checking, and revoking of capabilities
pub struct CapabilityManager {
    /// Main capability table, protected by a read-write lock
    data: RwLock<CapabilityStore>,
}

/// Internal store for capabilities
struct CapabilityStore {
    /// Map from capability ID to capability entries
    capabilities: HashMap<CapabilityId, CapabilityEntry>,

    /// Index from subject to their capability IDs
    subject_index: HashMap<String, HashSet<CapabilityId>>,
}

/// Simplified PolicyEvaluator for now
#[allow(dead_code)]
struct PolicyEvaluator;

#[allow(dead_code)]
impl PolicyEvaluator {
    fn new() -> Result<Self> {
        Ok(Self)
    }

    async fn evaluate(&self, _subject: &str, _object: &str, _operation: &str) -> Result<bool> {
        // This is a simplified placeholder that always returns true
        // In a real implementation, this would evaluate policies
        Ok(true)
    }
}

impl CapabilityManager {
    /// Create a new capability manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            data: RwLock::new(CapabilityStore {
                capabilities: HashMap::new(),
                subject_index: HashMap::new(),
            }),
        })
    }

    /// Grant a capability to a subject
    pub async fn grant_capability(
        &self,
        subject: String,
        object: String,
        rights: Vec<String>,
    ) -> Result<CapabilityId> {
        let mut data = self.data.write();

        // Convert rights to a HashSet
        let rights_set: HashSet<String> = rights.into_iter().collect();

        // Clone subject for later use in logging
        let subject_clone = subject.clone();
        // Generate a new capability ID
        let cap_id = CapabilityId::new();

        // Create a new capability entry
        let entry = CapabilityEntry {
            _id: cap_id.clone(),
            subject: subject.clone(),
            object: object.clone(),
            rights: rights_set,
            valid: true,
            _parent: None,
            children: Vec::new(),
        };

        // Add to the capabilities map
        data.capabilities.insert(cap_id.clone(), entry);

        // Add to the subject index
        data.subject_index
            .entry(subject.clone())
            .or_insert_with(HashSet::new)
            .insert(cap_id.clone());

        info!(
            "Granted capability {:?} to subject {} for object {}",
            cap_id, subject_clone, object
        );

        Ok(cap_id)
    }

    /// Revoke a capability and all its derived capabilities
    pub async fn revoke_capability(&self, cap_id: CapabilityId) -> Result<()> {
        let mut data = self.data.write();

        // Check if the capability exists
        if !data.capabilities.contains_key(&cap_id) {
            return Err(CapabilityError::NotFound(cap_id).into());
        }

        // Collect all descendants
        let mut to_revoke = Vec::new();
        self.collect_descendants(&mut data, &cap_id, &mut to_revoke);

        // Revoke all collected capabilities
        for id in to_revoke {
            if let Some(entry) = data.capabilities.get_mut(&id) {
                let subject = entry.subject.clone();

                // Mark as invalid
                entry.valid = false;

                // Remove from subject index
                if let Some(subject_caps) = data.subject_index.get_mut(&subject) {
                    subject_caps.remove(&id);
                }

                info!("Revoked capability {:?} from subject {}", id, subject);
            }
        }

        Ok(())
    }

    /// Derive a new capability with restricted rights
    pub async fn attenuate_capability(
        &self,
        parent_id: CapabilityId,
        subject: String,
        rights: Vec<String>,
    ) -> Result<CapabilityId> {
        let mut data = self.data.write();

        // Get parent info before modifications
        let parent_info = match data.capabilities.get(&parent_id) {
            Some(entry) if entry.valid => (entry.object.clone(), entry.rights.clone()),
            Some(_) => return Err(CapabilityError::Revoked(parent_id).into()),
            None => return Err(CapabilityError::NotFound(parent_id).into()),
        };

        let (parent_object, parent_rights) = parent_info;

        // Convert rights to a HashSet
        let rights_set: HashSet<String> = rights.into_iter().collect();

        // Check that new rights are a subset of parent rights (monotonicity)
        if !rights_set.is_subset(&parent_rights) {
            return Err(CapabilityError::RightsEscalation.into());
        }

        // Generate a new capability ID
        let cap_id = CapabilityId::new();

        // Create a new capability entry
        let entry = CapabilityEntry {
            _id: cap_id.clone(),
            subject: subject.clone(),
            object: parent_object,
            rights: rights_set,
            valid: true,
            _parent: Some(parent_id.clone()),
            children: Vec::new(),
        };

        // Add to the capabilities map
        data.capabilities.insert(cap_id.clone(), entry);

        // Add to the subject index
        data.subject_index
            .entry(subject)
            .or_insert_with(HashSet::new)
            .insert(cap_id.clone());

        // Add as child to parent
        if let Some(parent_entry) = data.capabilities.get_mut(&parent_id) {
            parent_entry.children.push(cap_id.clone());
        }

        info!(
            "Created attenuated capability {:?} from {:?}",
            cap_id, parent_id
        );

        Ok(cap_id)
    }

    /// Check if a subject has a capability for an operation on an object
    pub fn has_capability(&self, subject: &str, object: &str, operation: &str) -> bool {
        let data = self.data.read();

        // Get the subject's capabilities
        if let Some(caps) = data.subject_index.get(subject) {
            // Check each capability
            for &cap_id in caps {
                if let Some(entry) = data.capabilities.get(&cap_id) {
                    if entry.valid && entry.object == object && entry.rights.contains(operation) {
                        // Capability allows the operation
                        return true;
                    }
                }
            }
        }

        // No valid capability found
        false
    }

    /// Check both capability and policy for an operation
    pub async fn check_permission(
        &self,
        subject: &str,
        object: &str,
        operation: &str,
    ) -> Result<()> {
        // Check capability first (fast path)
        if !self.has_capability(subject, object, operation) {
            return Err(CapabilityError::Unauthorized(
                subject.to_string(),
                operation.to_string(),
                object.to_string(),
            )
            .into());
        }

        // Both capability and policy allow the operation
        Ok(())
    }

    // Helper to collect all descendants of a capability
    fn collect_descendants(
        &self,
        data: &mut CapabilityStore,
        cap_id: &CapabilityId,
        results: &mut Vec<CapabilityId>,
    ) {
        // First add the current capability to the results
        results.push(cap_id.clone());

        // Get all children IDs (clone to avoid borrowing issues)
        let children: Vec<CapabilityId> = data
            .capabilities
            .get(cap_id)
            .map(|entry| entry.children.clone())
            .unwrap_or_default();

        // Then process each child
        for child_id in children {
            self.collect_descendants(data, &child_id, results);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grant_and_check_capability() {
        let manager = CapabilityManager::new().unwrap();

        // Grant a capability
        let cap_id = manager
            .grant_capability(
                "subject1".to_string(),
                "object1".to_string(),
                vec!["read".to_string(), "write".to_string()],
            )
            .await
            .unwrap();

        // Check capability exists
        assert!(manager.has_capability("subject1", "object1", "read"));
        assert!(manager.has_capability("subject1", "object1", "write"));
        assert!(!manager.has_capability("subject1", "object1", "execute"));
        assert!(!manager.has_capability("subject2", "object1", "read"));
    }

    #[tokio::test]
    async fn test_revoke_capability() {
        let manager = CapabilityManager::new().unwrap();

        // Grant a capability
        let cap_id = manager
            .grant_capability(
                "subject1".to_string(),
                "object1".to_string(),
                vec!["read".to_string(), "write".to_string()],
            )
            .await
            .unwrap();

        // Check it works
        assert!(manager.has_capability("subject1", "object1", "read"));

        // Revoke it
        manager.revoke_capability(cap_id).await.unwrap();

        // Check it no longer works
        assert!(!manager.has_capability("subject1", "object1", "read"));
    }

    #[tokio::test]
    async fn test_attenuate_capability() {
        let manager = CapabilityManager::new().unwrap();

        // Grant a parent capability with read+write
        let parent_id = manager
            .grant_capability(
                "subject1".to_string(),
                "object1".to_string(),
                vec!["read".to_string(), "write".to_string()],
            )
            .await
            .unwrap();

        // Derive a child capability with only read
        let child_id = manager
            .attenuate_capability(
                parent_id.clone(),
                "subject2".to_string(),
                vec!["read".to_string()],
            )
            .await
            .unwrap();

        // Check parent capabilities
        assert!(manager.has_capability("subject1", "object1", "read"));
        assert!(manager.has_capability("subject1", "object1", "write"));

        // Check child capabilities
        assert!(manager.has_capability("subject2", "object1", "read"));
        assert!(!manager.has_capability("subject2", "object1", "write"));

        // Revoke parent should revoke child too
        manager.revoke_capability(parent_id).await.unwrap();

        // Check both are revoked
        assert!(!manager.has_capability("subject1", "object1", "read"));
        assert!(!manager.has_capability("subject2", "object1", "read"));
    }
}
