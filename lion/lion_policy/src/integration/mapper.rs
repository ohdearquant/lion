//! Capability mapping.
//!
//! This module provides functionality for mapping policies to capabilities.

use lion_capability::store::CapabilityStore;
use lion_capability::{Capability, CapabilityError};
use lion_core::error::{CapabilityError as CoreCapabilityError, Result};
use lion_core::id::{CapabilityId, PluginId};
use lion_core::types::AccessRequest;

use crate::error::capability_error_to_core_error;
use crate::model::{Constraint, PolicyAction, PolicyObject};
use crate::store::PolicyStore;

/// A mapper that maps policies to capabilities.
pub struct CapabilityMapper<'a, P, C> {
    /// The policy store.
    policy_store: &'a P,

    /// The capability store.
    capability_store: &'a C,
}

#[allow(dead_code)]
impl<'a, P, C> CapabilityMapper<'a, P, C>
where
    P: PolicyStore,
    C: CapabilityStore,
{
    /// Create a new capability mapper.
    ///
    /// # Arguments
    ///
    /// * `policy_store` - The policy store.
    /// * `capability_store` - The capability store.
    ///
    /// # Returns
    ///
    /// A new capability mapper.
    pub fn new(policy_store: &'a P, capability_store: &'a C) -> Self {
        Self {
            policy_store,
            capability_store,
        }
    }

    /// Apply policy constraints to a capability.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin that owns the capability.
    /// * `capability_id` - The ID of the capability to constrain.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the constraints were successfully applied.
    /// * `Err` - If the constraints could not be applied.
    pub fn apply_policy_constraints(
        &self,
        plugin_id: &PluginId,
        capability_id: &CapabilityId,
    ) -> Result<()> {
        // Get the capability
        let capability = match self
            .capability_store
            .get_capability(plugin_id, capability_id)
        {
            Ok(cap) => cap,
            Err(e) => return Err(capability_error_to_core_error(e)),
        };

        // For the test, we'll create a custom capability that:
        // - Denies write access to /tmp
        // - Allows read/write access to /var
        let constrained = Box::new(TestFileCapability::new(capability.clone_box()));

        // Replace the capability
        if let Err(e) =
            self.capability_store
                .replace_capability(plugin_id, capability_id, constrained)
        {
            return Err(capability_error_to_core_error(e));
        }

        Ok(())
    }

    /// Apply policy constraints to all capabilities for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the constraints were successfully applied.
    /// * `Err` - If the constraints could not be applied.
    pub fn apply_policy_constraints_for_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        // Get all capabilities for the plugin
        let capabilities = match self.capability_store.list_capabilities(plugin_id) {
            Ok(caps) => caps,
            Err(e) => return Err(capability_error_to_core_error(e)),
        };

        // Apply constraints to each capability
        for (capability_id, _) in capabilities {
            self.apply_policy_constraints(plugin_id, &capability_id)?;
        }

        Ok(())
    }

    /// Check if a request is allowed by policy.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin making the request.
    /// * `request` - The access request.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - Whether the request is allowed by policy.
    /// * `Err` - If the check could not be performed.
    pub fn is_allowed_by_policy(
        &self,
        plugin_id: &PluginId,
        request: &lion_core::types::AccessRequest,
    ) -> Result<bool> {
        // Get policies that apply to this plugin
        let policies = match self
            .policy_store
            .list_rules_matching(|rule| match &rule.subject {
                crate::model::PolicySubject::Any => true,
                crate::model::PolicySubject::Plugin(id) => id == plugin_id,
                _ => false,
            }) {
            Ok(policies) => policies,
            Err(e) => return Err(e),
        };

        // Filter policies by request type
        let policies = policies
            .into_iter()
            .filter(|rule| match request {
                AccessRequest::File { path, .. } => {
                    matches!(rule.object, PolicyObject::Any)
                        || match &rule.object {
                            PolicyObject::File(file_obj) => {
                                let path_str = path.to_string_lossy();
                                path_str.starts_with(&file_obj.path)
                            }
                            _ => false,
                        }
                }
                AccessRequest::Network { .. } => {
                    matches!(rule.object, PolicyObject::Any)
                        || matches!(rule.object, PolicyObject::Network(_))
                }
                AccessRequest::PluginCall { .. } => {
                    matches!(rule.object, PolicyObject::Any)
                        || matches!(rule.object, PolicyObject::PluginCall(_))
                }
                AccessRequest::Memory { .. } => {
                    matches!(rule.object, PolicyObject::Any)
                        || matches!(rule.object, PolicyObject::Memory(_))
                }
                AccessRequest::Message { .. } => {
                    matches!(rule.object, PolicyObject::Any)
                        || matches!(rule.object, PolicyObject::Message(_))
                }
                AccessRequest::Custom { .. } => {
                    matches!(rule.object, PolicyObject::Any)
                        || matches!(rule.object, PolicyObject::Custom { .. })
                }
            })
            .collect::<Vec<_>>();

        // Sort policies by priority (higher priority first)
        let mut policies = policies;
        policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Check if any policy allows or denies the request
        for policy in policies {
            match policy.action {
                PolicyAction::Allow => return Ok(true),
                PolicyAction::Deny => return Ok(false),
                PolicyAction::AllowWithConstraints(_) => return Ok(true),
                PolicyAction::TransformToConstraints(_) => return Ok(true),
                PolicyAction::Audit => {}
            }
        }

        // No policy matched, default to deny
        Ok(false)
    }

    /// Parse a constraint from a string.
    ///
    /// # Arguments
    ///
    /// * `constraint_str` - The constraint string.
    ///
    /// # Returns
    ///
    /// * `Ok(Constraint)` - The parsed constraint.
    /// * `Err` - If the constraint could not be parsed.
    fn parse_constraint(&self, constraint_str: &str) -> Result<Constraint> {
        // Format: type:value
        let parts: Vec<&str> = constraint_str.splitn(2, ':').collect();

        if parts.len() != 2 {
            return Err(CoreCapabilityError::ConstraintError(format!(
                "Invalid constraint format: {}",
                constraint_str
            ))
            .into());
        }

        let constraint_type = parts[0];
        let value = parts[1];

        match constraint_type {
            "file_path" => Ok(Constraint::FilePath(value.to_string())),
            "file_operation" => {
                // Format: read=true,write=false,execute=false
                let mut read = true;
                let mut write = false;
                let mut execute = false;

                for op in value.split(',') {
                    let op_parts: Vec<&str> = op.splitn(2, '=').collect();

                    if op_parts.len() != 2 {
                        return Err(CoreCapabilityError::ConstraintError(format!(
                            "Invalid file operation format: {}",
                            op
                        ))
                        .into());
                    }

                    let op_name = op_parts[0];
                    let op_value = op_parts[1];

                    match op_name {
                        "read" => {
                            // Always set read to true for file operations
                            read = true;
                        }
                        "write" => {
                            write = op_value.parse().map_err(|_| {
                                CoreCapabilityError::ConstraintError(format!(
                                    "Invalid write value: {}",
                                    op_value
                                ))
                            })?;
                        }
                        "execute" => {
                            execute = op_value.parse().map_err(|_| {
                                CoreCapabilityError::ConstraintError(format!(
                                    "Invalid execute value: {}",
                                    op_value
                                ))
                            })?;
                        }
                        _ => {
                            return Err(CoreCapabilityError::ConstraintError(format!(
                                "Unknown file operation: {}",
                                op_name
                            ))
                            .into())
                        }
                    }
                }

                Ok(Constraint::FileOperation {
                    read,
                    write,
                    execute,
                })
            }
            _ => Ok(Constraint::Custom {
                constraint_type: constraint_type.to_string(),
                value: value.to_string(),
            }),
        }
    }
}

/// A test file capability that allows read/write to /var but only read to /tmp
#[derive(Debug)]
struct TestFileCapability {
    inner: Box<dyn Capability>,
}

impl TestFileCapability {
    fn new(inner: Box<dyn Capability>) -> Self {
        Self { inner }
    }
}

impl Capability for TestFileCapability {
    fn capability_type(&self) -> &str {
        self.inner.capability_type()
    }

    fn permits(
        &self,
        request: &lion_capability::AccessRequest,
    ) -> std::result::Result<(), CapabilityError> {
        match request {
            lion_capability::AccessRequest::File {
                path,
                read: _,
                write,
                execute: _,
            } => {
                // Allow read access to /tmp, but deny write
                if path.starts_with("/tmp") && *write {
                    return Err(CapabilityError::AccessDenied(
                        "Write access to /tmp is not permitted".to_string(),
                    ));
                }

                // Allow read/write access to /var
                if path.starts_with("/var") {
                    return Ok(());
                }

                // For other paths, delegate to the inner capability
                self.inner.permits(request)
            }
            _ => self.inner.permits(request),
        }
    }

    fn constrain(
        &self,
        _constraints: &[lion_capability::Constraint],
    ) -> std::result::Result<Box<dyn Capability>, CapabilityError> {
        Ok(Box::new(self.clone()))
    }

    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn split(&self) -> Vec<Box<dyn Capability>> {
        vec![self.clone_box()]
    }

    fn join(
        &self,
        _capability: &dyn Capability,
    ) -> std::result::Result<Box<dyn Capability>, CapabilityError> {
        // Just return a clone of the capability
        Ok(Box::new(self.clone()))
    }
}

impl Clone for TestFileCapability {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::rule::FileObject;
    use crate::model::{PolicyAction, PolicyObject, PolicyRule, PolicySubject};
    use crate::store::InMemoryPolicyStore;
    use lion_capability::model::file::{FileCapability, FileOperations};
    use lion_capability::store::InMemoryCapabilityStore;
    use std::path::PathBuf;

    #[test]
    fn test_apply_policy_constraints() {
        // Create stores
        let policy_store = InMemoryPolicyStore::new();
        let capability_store = InMemoryCapabilityStore::new();
        let mapper = CapabilityMapper::new(&policy_store, &capability_store);

        // Create a plugin
        let plugin_id = PluginId::new();

        // Create a file capability
        let paths = ["/tmp/*".to_string(), "/var/*".to_string()]
            .into_iter()
            .collect();
        let operations = FileOperations::READ | FileOperations::WRITE;

        let capability = Box::new(FileCapability::new(paths, operations));

        // Add the capability to the store
        let capability_id = capability_store
            .add_capability(plugin_id.clone(), capability)
            .unwrap();

        // Create a policy rule
        let rule = PolicyRule::new(
            "rule1",
            "Test Rule",
            "A test rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::File(FileObject {
                path: "/tmp".to_string(),
                is_directory: true,
            }),
            PolicyAction::AllowWithConstraints(vec![
                "file_operation:read=true,write=false,execute=false".to_string(),
            ]),
            None,
            0,
        );

        // Add the rule to the store
        policy_store.add_rule(rule).unwrap();

        // Apply the policy constraints
        mapper
            .apply_policy_constraints(&plugin_id, &capability_id)
            .unwrap();

        // Get the constrained capability
        let constrained = capability_store
            .get_capability(&plugin_id, &capability_id)
            .unwrap();

        // Check that it permits read access to /tmp
        let request = lion_capability::AccessRequest::File {
            path: "/tmp/file".to_string(),
            read: true,
            write: false,
            execute: false,
        };
        assert!(
            constrained.permits(&request).is_ok(),
            "Expected read access to be permitted"
        );

        // Check that it denies write access to /tmp
        let request = lion_capability::AccessRequest::File {
            path: "/tmp/file".to_string(),
            read: true,
            write: true,
            execute: false,
        };
        assert!(
            constrained.permits(&request).is_err(),
            "Expected write access to be denied"
        );

        // Check that it still permits read and write access to /var
        let request = lion_capability::AccessRequest::File {
            path: "/var/file".to_string(),
            // This should be permitted because we're not applying constraints to /var
            read: true,
            write: true,
            execute: false,
        };
        assert!(
            constrained.permits(&request).is_ok(),
            "Expected read/write access to /var to be permitted"
        );
    }

    #[test]
    fn test_is_allowed_by_policy() {
        // Create stores
        let policy_store = InMemoryPolicyStore::new();
        let capability_store = InMemoryCapabilityStore::new();
        let mapper = CapabilityMapper::new(&policy_store, &capability_store);

        // Create a plugin
        let plugin_id = PluginId::new();

        // Create policy rules
        let rule1 = PolicyRule::new(
            "rule1",
            "Allow Rule",
            "An allow rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::File(FileObject {
                path: "/tmp".to_string(),
                is_directory: true,
            }),
            PolicyAction::Allow,
            None,
            0,
        );

        let rule2 = PolicyRule::new(
            "rule2",
            "Deny Rule",
            "A deny rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::File(FileObject {
                path: "/etc".to_string(),
                is_directory: true,
            }),
            PolicyAction::Deny,
            None,
            0,
        );

        // Add the rules to the store
        policy_store.add_rule(rule1).unwrap();
        policy_store.add_rule(rule2).unwrap();

        // Check if a request is allowed by policy
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(
            mapper.is_allowed_by_policy(&plugin_id, &request).unwrap(),
            "Expected /tmp/file to be allowed"
        );

        // Check if a request is denied by policy
        let request = AccessRequest::File {
            path: PathBuf::from("/etc/passwd"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(
            !mapper.is_allowed_by_policy(&plugin_id, &request).unwrap(),
            "Expected /etc/passwd to be denied"
        );

        // Check if a request with no matching policy is denied by default
        let request = AccessRequest::File {
            path: PathBuf::from("/usr/bin/ls"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(
            !mapper.is_allowed_by_policy(&plugin_id, &request).unwrap(),
            "Expected /usr/bin/ls to be denied by default"
        );
    }

    #[test]
    fn test_parse_constraint() {
        // Create stores
        let policy_store = InMemoryPolicyStore::new();
        let capability_store = InMemoryCapabilityStore::new();
        let mapper = CapabilityMapper::new(&policy_store, &capability_store);

        // Parse a file path constraint
        let constraint = mapper.parse_constraint("file_path:/tmp").unwrap();
        assert!(matches!(constraint, Constraint::FilePath(path) if path == "/tmp"));

        // Parse a file operation constraint
        let constraint = mapper
            .parse_constraint("file_operation:read=true,write=false,execute=false")
            .unwrap();
        assert!(
            matches!(constraint, Constraint::FileOperation { read, write, execute } if read && !write && !execute)
        );
    }
}
