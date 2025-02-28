use lion_capability::model::file::{FileCapability, FileOperations};
use lion_capability::store::InMemoryCapabilityStore;
use lion_capability::CapabilityStore;
use lion_core::id::PluginId;
use lion_policy::integration::CapabilityMapper;
use lion_policy::model::rule::FileObject;
use lion_policy::model::{PolicyAction, PolicyObject, PolicyRule, PolicySubject};
use lion_policy::store::InMemoryPolicyStore;
use lion_policy::store::PolicyStore;

pub fn main() {
    // Create stores
    let policy_store = InMemoryPolicyStore::new();
    let capability_store = InMemoryCapabilityStore::new();
    let mapper = CapabilityMapper::new(&policy_store, &capability_store);

    // Create a plugin
    let plugin_id = PluginId::new();
    println!("Plugin ID: {:?}", plugin_id);

    // Create a file capability
    let paths = ["/tmp".to_string(), "/var".to_string()]
        .into_iter()
        .collect();
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
        Err(e) => println!("Read access denied: {:?}", e),
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
        Ok(_) => println!("Write access permitted"),
        Err(e) => println!("Write access denied: {:?}", e),
    }
}
