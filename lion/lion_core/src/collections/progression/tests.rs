use super::*;
use serde_json::json;
use std::thread;

#[test]
fn test_progression_basic_operations() {
    let progression = Progression::new();
    let agent_id = Uuid::new_v4();

    // Test push
    let step_id = progression.push(agent_id, json!({"test": "data"})).unwrap();
    assert!(progression.contains(&step_id).unwrap());

    // Test list
    let steps = progression.list(None).unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].metadata.agent_id, agent_id);
}

#[test]
fn test_progression_concurrent_access() {
    let progression = Arc::new(Progression::new());
    let mut handles = Vec::new();

    for _ in 0..10 {
        let progression = progression.clone();
        handles.push(thread::spawn(move || {
            let agent_id = Uuid::new_v4();
            let step_id = progression.push(agent_id, json!({"test": "data"})).unwrap();
            assert!(progression.contains(&step_id).unwrap());
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let steps = progression.list(None).unwrap();
    assert_eq!(steps.len(), 10);
}

#[test]
fn test_progression_branching() {
    let progression = Progression::new();
    let agent_id = Uuid::new_v4();

    // Create main steps
    let step_id = progression.push(agent_id, json!({"step": "main"})).unwrap();

    // Create branch
    progression
        .create_branch("test-branch".to_string(), step_id)
        .unwrap();

    // Add steps to branch
    let branch_step = progression
        .push_to_branch("test-branch", agent_id, json!({"step": "branch"}))
        .unwrap();

    // Verify branch
    let branch_steps = progression.list(Some("test-branch")).unwrap();
    assert_eq!(branch_steps.len(), 2); // Parent step + branch step
    assert_eq!(branch_steps[1].id, branch_step);

    // Merge branch
    let merged_step = progression.merge_branch("test-branch").unwrap();
    assert_eq!(merged_step, branch_step);
}

#[test]
fn test_progression_agent_steps() {
    let progression = Progression::new();
    let agent_id = Uuid::new_v4();

    // Add multiple steps for the agent
    for i in 0..3 {
        progression.push(agent_id, json!({"step": i})).unwrap();
    }

    // Get agent steps
    let agent_steps = progression.get_agent_steps(agent_id).unwrap();
    assert_eq!(agent_steps.len(), 3);

    // Verify steps belong to agent
    for step in agent_steps {
        assert_eq!(step.metadata.agent_id, agent_id);
    }
}
