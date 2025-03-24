# Lion Policy

`lion_policy` provides a flexible policy engine for the Lion microkernel system,
enabling fine-grained security policy enforcement that complements the
capability-based security model.

## Features

- **Policy Evaluation**: Evaluate access requests against defined policies
- **Policy Aggregation**: Combine multiple policies with different precedence
- **Audit Logging**: Record policy decisions for security analysis
- **Policy Storage**: In-memory and persistent policy storage
- **Integration**: Seamless integration with Lion's capability system
- **Extensible Model**: Support for custom policy types and constraints

## Architecture

The policy system is built around several key components:

1. **Engine**: Core policy evaluation and decision making
   - Aggregator: Combines multiple policy sources
   - Evaluator: Evaluates access requests against policies
   - Audit: Records policy decisions for later analysis

2. **Integration**: Connects policies to the rest of the system
   - Mapper: Maps system objects to policy subjects/objects
   - Resolver: Resolves policy references and dependencies

3. **Model**: Defines the structure of policies
   - Constraints: Restrictions on resource access
   - Rules: Policy rules with conditions and effects
   - Evaluation Results: Structured policy decisions

4. **Store**: Manages policy persistence
   - In-Memory: Fast, ephemeral policy storage
   - Registry: Persistent policy storage

## Usage

### Defining Policies

```rust
use lion_policy::model::{Policy, Rule, Effect, Subject, Object, Action};

// Create a policy
let policy = Policy::new("file-access-policy")
    .add_rule(
        Rule::new("rule1")
            .with_subject(Subject::Plugin("calculator".into()))
            .with_object(Object::File("/tmp/results.txt".into()))
            .with_action(Action::Read)
            .with_effect(Effect::Allow)
    )
    .add_rule(
        Rule::new("rule2")
            .with_subject(Subject::Plugin("calculator".into()))
            .with_object(Object::File("/etc".into()))
            .with_action(Action::Any)
            .with_effect(Effect::Deny)
    );
```

### Evaluating Access Requests

```rust
use lion_policy::engine::{PolicyEngine, AccessRequest};

// Create a policy engine
let engine = PolicyEngine::new();

// Register policies
engine.add_policy(policy);

// Evaluate an access request
let request = AccessRequest::new()
    .with_subject(Subject::Plugin("calculator".into()))
    .with_object(Object::File("/tmp/results.txt".into()))
    .with_action(Action::Read);

let result = engine.evaluate(&request)?;

// Check the result
match result.effect {
    Effect::Allow => println!("Access allowed"),
    Effect::Deny => println!("Access denied: {}", result.reason.unwrap_or_default()),
    Effect::AllowWithConstraints(constraints) => {
        println!("Access allowed with constraints: {:?}", constraints);
    }
}
```

### Policy Aggregation

```rust
use lion_policy::engine::{PolicyAggregator, AggregationStrategy};

// Create a policy aggregator
let aggregator = PolicyAggregator::new(AggregationStrategy::DenyOverrides);

// Add policies with different precedence
aggregator.add_policy(system_policy, 100); // Higher precedence
aggregator.add_policy(user_policy, 50);    // Lower precedence

// Create an engine with the aggregator
let engine = PolicyEngine::with_aggregator(aggregator);
```

### Audit Logging

```rust
use lion_policy::engine::{PolicyAuditor, AuditConfig};

// Create an auditor
let auditor = PolicyAuditor::new(AuditConfig {
    log_all_decisions: true,
    log_path: Some("/var/log/lion/policy.log".into()),
    ..Default::default()
});

// Create an engine with auditing
let engine = PolicyEngine::new().with_auditor(auditor);
```

## Integration with Other Lion Crates

The policy system integrates with other Lion crates:

- **lion_core**: For core types and traits
- **lion_capability**: For capability-based access control
- **lion_runtime**: For runtime policy enforcement
- **lion_observability**: For policy decision logging

## Debug Tools

The policy crate includes several debugging tools:

- **Debug Mapper**: Visualize subject/object mappings
- **Debug Resolver**: Trace policy resolution
- **Policy Analyzer**: Detect conflicts and redundancies

## License

Licensed under the Apache License, Version 2.0 - see the
[LICENSE](../../LICENSE) file for details.
