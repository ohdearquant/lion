use lion_core::id::PluginId;
use lion_core::types::AccessRequest;
use lion_policy::integration::ConstraintResolver;
use lion_policy::model::rule::FileObject;
use lion_policy::model::{PolicyAction, PolicyObject, PolicyRule, PolicySubject};
use lion_policy::store::InMemoryPolicyStore;
use lion_policy::store::PolicyStore;
use std::path::PathBuf;

pub fn main() {
    // Create a policy store
    let policy_store = InMemoryPolicyStore::new();
    let mut resolver = ConstraintResolver::new(&policy_store);

    // Create a plugin
    let plugin_id = PluginId::new();
    println!("Plugin ID: {:?}", plugin_id);

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
    println!("Rule 1: {:?}", rule1);

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
    println!("Rule 2: {:?}", rule2);

    let rule3 = PolicyRule::new(
        "rule3",
        "Constraint Rule",
        "A constraint rule",
        PolicySubject::Plugin(plugin_id.clone()),
        PolicyObject::File(FileObject {
            path: "/var".to_string(),
            is_directory: true,
        }),
        PolicyAction::AllowWithConstraints(vec![
            "file_operation:read=true,write=false,execute=false".to_string(),
        ]),
        None,
        0,
    );
    println!("Rule 3: {:?}", rule3);

    // Add the rules to the store
    policy_store.add_rule(rule1).unwrap();
    policy_store.add_rule(rule2).unwrap();
    policy_store.add_rule(rule3).unwrap();

    // Evaluate a request that should be allowed
    let request = AccessRequest::File {
        path: PathBuf::from("/tmp/file"),
        read: false,
        write: false,
        execute: false,
    };
    println!("Evaluating request for /tmp/file: {:?}", request);

    // Get matching policies
    let policies = policy_store
        .list_rules_matching(|rule| {
            // Check if the rule applies to this plugin
            let plugin_match = match &rule.subject {
                lion_policy::model::PolicySubject::Any => true,
                lion_policy::model::PolicySubject::Plugin(id) => *id == plugin_id,
                _ => false,
            };

            // Check if the rule applies to this request type
            let request_match = match &request {
                AccessRequest::File { path, .. } => {
                    matches!(rule.object, PolicyObject::Any)
                        || match &rule.object {
                            PolicyObject::File(file_obj) => {
                                let path_str = path.to_string_lossy();
                                println!(
                                    "Checking if path {} starts with {}",
                                    path_str, file_obj.path
                                );
                                path_str.starts_with(&file_obj.path)
                            }
                            _ => false,
                        }
                }
                _ => false,
            };

            println!(
                "Rule: {:?}, plugin_match: {}, request_match: {}",
                rule.id, plugin_match, request_match
            );
            plugin_match && request_match
        })
        .unwrap();

    println!("Matching policies for /tmp/file: {:?}", policies);

    let result = resolver.evaluate(&plugin_id, &request).unwrap();
    println!("Result for /tmp/file: {:?}", result);

    // Evaluate a request that should be denied
    let request = AccessRequest::File {
        path: PathBuf::from("/etc/passwd"),
        read: true,
        write: false,
        execute: false,
    };
    println!("Evaluating request for /etc/passwd: {:?}", request);

    // Get matching policies
    let policies = policy_store
        .list_rules_matching(|rule| {
            // Check if the rule applies to this plugin
            let plugin_match = match &rule.subject {
                lion_policy::model::PolicySubject::Any => true,
                lion_policy::model::PolicySubject::Plugin(id) => *id == plugin_id,
                _ => false,
            };

            // Check if the rule applies to this request type
            let request_match = match &request {
                AccessRequest::File { path, .. } => {
                    matches!(rule.object, PolicyObject::Any)
                        || match &rule.object {
                            PolicyObject::File(file_obj) => {
                                let path_str = path.to_string_lossy();
                                println!(
                                    "Checking if path {} starts with {}",
                                    path_str, file_obj.path
                                );
                                path_str.starts_with(&file_obj.path)
                            }
                            _ => false,
                        }
                }
                _ => false,
            };

            println!(
                "Rule: {:?}, plugin_match: {}, request_match: {}",
                rule.id, plugin_match, request_match
            );
            plugin_match && request_match
        })
        .unwrap();

    println!("Matching policies for /etc/passwd: {:?}", policies);

    let result = resolver.evaluate(&plugin_id, &request).unwrap();
    println!("Result for /etc/passwd: {:?}", result);
}
