use lion_capability::model::file::{FileCapability, FileOperations};
use lion_capability::store::InMemoryCapabilityStore;
use lion_capability::CapabilityStore;
use lion_core::id::PluginId;
use lion_core::types::AccessRequest as CoreAccessRequest;
use lion_policy::integration::{CapabilityMapper, ConstraintResolver};
use lion_policy::model::rule::FileObject;
use lion_policy::model::{EvaluationResult, PolicyAction, PolicyObject, PolicyRule, PolicySubject};
use lion_policy::store::InMemoryPolicyStore;
use lion_policy::store::PolicyStore;
use std::collections::HashSet;
use std::path::PathBuf;

#[test]
fn debug_apply_policy_constraints() {
    // Create stores
    let policy_store = InMemoryPolicyStore::new();
    let capability_store = InMemoryCapabilityStore::new();
    let mapper = CapabilityMapper::new(&policy_store, &capability_store);

    // Create a plugin
    let plugin_id = PluginId::new();
    println!("Plugin ID: {:?}", plugin_id);

    // Create a file capability
    let paths = ["/tmp/*".to_string(), "/var/*".to_string()]
        .into_iter()
        .collect::<HashSet<String>>();
    let operations = FileOperations::READ | FileOperations::WRITE;
    println!("Original capability operations: {:?}", operations);

    let capability = Box::new(FileCapability::new(paths, operations));

    // Add the capability to the store
    let capability_id = capability_store
        .add_capability(plugin_id.clone(), capability)
        .unwrap();
    println!("Capability ID: {:?}", capability_id);

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
    println!("Policy rule: {:?}", rule);

    // Add the rule to the store
    policy_store.add_rule(rule).unwrap();

    // Apply the policy constraints
    println!("Applying policy constraints...");
    mapper
        .apply_policy_constraints(&plugin_id, &capability_id)
        .unwrap();

    // Get the constrained capability
    let constrained = capability_store
        .get_capability(&plugin_id, &capability_id)
        .unwrap();

    if let Some(file_cap) = constrained.as_any().downcast_ref::<FileCapability>() {
        println!(
            "Constrained capability operations: {:?}",
            file_cap.operations()
        );
    }

    // Check that it permits read access to /tmp
    let request = lion_capability::AccessRequest::File {
        path: "/tmp/file".to_string(),
        read: true,
        write: false,
        execute: false,
    };
    println!("Testing read access to /tmp/file");
    match constrained.permits(&request) {
        Ok(_) => println!("Read access permitted"),
        Err(e) => {
            println!("Read access denied: {:?}", e);
            panic!("Read access should be permitted");
        }
    }

    // Check that it denies write access to /tmp
    let request = lion_capability::AccessRequest::File {
        path: "/tmp/file".to_string(),
        read: false,
        write: true,
        execute: false,
    };
    println!("Testing write access to /tmp/file");
    match constrained.permits(&request) {
        Ok(_) => {
            println!("Write access permitted");
            panic!("Write access should be denied");
        }
        Err(e) => println!("Write access denied: {:?}", e),
    }
}

#[test]
fn debug_evaluate() {
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
    let request = CoreAccessRequest::File {
        path: PathBuf::from("/tmp/file"),
        read: true,
        write: false,
        execute: false,
    };
    println!("Evaluating request for /tmp/file: {:?}", request);
    let result = resolver.evaluate(&plugin_id, &request).unwrap();
    println!("Result for /tmp/file: {:?}", result);
    assert!(
        matches!(
            result,
            EvaluationResult::Allow | EvaluationResult::AllowWithConstraints(_)
        ),
        "Expected Allow or AllowWithConstraints, got {:?}",
        result
    );

    // Evaluate a request that should be denied
    let request = CoreAccessRequest::File {
        path: PathBuf::from("/etc/passwd"),
        read: true,
        write: false,
        execute: false,
    };
    println!("Evaluating request for /etc/passwd: {:?}", request);
    let result = resolver.evaluate(&plugin_id, &request).unwrap();
    println!("Result for /etc/passwd: {:?}", result);
    assert!(
        matches!(result, EvaluationResult::Deny),
        "Expected Deny, got {:?}",
        result
    );

    // Evaluate a request that should be allowed with constraints
    let request = CoreAccessRequest::File {
        path: PathBuf::from("/var/file"),
        read: true,
        write: false,
        execute: false,
    };
    println!("Evaluating request for /var/file (read): {:?}", request);
    let result = resolver.evaluate(&plugin_id, &request).unwrap();
    println!("Result for /var/file (read): {:?}", result);
    assert!(
        matches!(result, EvaluationResult::AllowWithConstraints(_)),
        "Expected AllowWithConstraints, got {:?}",
        result
    );

    // Evaluate a request that should be denied (write to /var)
    let request = CoreAccessRequest::File {
        path: PathBuf::from("/var/file"),
        read: false,
        write: true,
        execute: false,
    };
    println!("Evaluating request for /var/file (write): {:?}", request);
    let result = resolver.evaluate(&plugin_id, &request).unwrap();
    println!("Result for /var/file (write): {:?}", result);
    assert!(
        matches!(result, EvaluationResult::Deny),
        "Expected Deny, got {:?}",
        result
    );
}
