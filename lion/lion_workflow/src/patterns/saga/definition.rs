use crate::patterns::saga::step::SagaStepDefinition;
use crate::patterns::saga::types::SagaError;
use crate::patterns::saga::types::SagaStrategy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Saga definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaDefinition {
    /// Saga ID
    pub id: String,

    /// Saga name
    pub name: String,

    /// Saga steps
    pub steps: Vec<SagaStepDefinition>,

    /// Saga coordination strategy
    #[serde(default)]
    pub strategy: SagaStrategy,

    /// Maximum execution time for the entire saga
    pub timeout_ms: u64,

    /// Number of retries for failed steps
    pub max_retries: u32,

    /// Duration between retries
    pub retry_delay_ms: u64,

    /// Whether to use idempotent operations
    pub use_idempotent_operations: bool,

    /// Custom metadata
    pub metadata: serde_json::Value,
}

impl SagaDefinition {
    /// Create a new saga definition
    pub fn new(id: &str, name: &str) -> Self {
        SagaDefinition {
            id: id.to_string(),
            name: name.to_string(),
            steps: Vec::new(),
            strategy: SagaStrategy::Orchestration,
            timeout_ms: 300000, // 5 minutes
            max_retries: 3,
            retry_delay_ms: 1000, // 1 second
            use_idempotent_operations: true,
            metadata: serde_json::Value::Null,
        }
    }

    /// Add a step to the saga
    pub fn add_step(&mut self, step: SagaStepDefinition) -> Result<(), SagaError> {
        // Check if step ID is unique
        if self.steps.iter().any(|s| s.id == step.id) {
            return Err(SagaError::DefinitionError(format!(
                "Step ID already exists: {}",
                step.id
            )));
        }

        // Validate step dependencies
        for dep_id in &step.dependencies {
            if !self.steps.iter().any(|s| &s.id == dep_id)
                && !self.steps.iter().any(|s| s.id == *dep_id)
            {
                return Err(SagaError::DefinitionError(format!(
                    "Dependency not found: {}",
                    dep_id
                )));
            }
        }

        // Add the step
        self.steps.push(step);

        Ok(())
    }

    /// Set the saga strategy
    pub fn with_strategy(mut self, strategy: SagaStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the saga timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32, retry_delay_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_ms = retry_delay_ms;
        self
    }

    /// Set whether to use idempotent operations
    pub fn with_idempotence(mut self, use_idempotent: bool) -> Self {
        self.use_idempotent_operations = use_idempotent;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Validate the saga definition
    pub fn validate(&self) -> Result<(), SagaError> {
        // Check for empty steps
        if self.steps.is_empty() {
            return Err(SagaError::DefinitionError("Saga has no steps".to_string()));
        }

        // Check for cycles in dependencies
        let mut visited = HashSet::new();
        let mut path = HashSet::new();

        // Create an adjacency list representation of the dependency graph
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for step in &self.steps {
            let deps = step.dependencies.clone();
            graph.insert(step.id.clone(), deps);
        }

        // Check for cycles using DFS
        for step in &self.steps {
            if !visited.contains(&step.id)
                && Self::has_cycle_dfs(&step.id, &graph, &mut visited, &mut path)
            {
                return Err(SagaError::DefinitionError(
                    "Dependency cycle detected".to_string(),
                ));
            }
        }

        Ok(())
    }

    // Helper for cycle detection
    fn has_cycle_dfs(
        step_id: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        path: &mut HashSet<String>,
    ) -> bool {
        visited.insert(step_id.to_string());
        path.insert(step_id.to_string());

        if let Some(deps) = graph.get(step_id) {
            for dep in deps {
                if !visited.contains(dep) {
                    if Self::has_cycle_dfs(dep, graph, visited, path) {
                        return true;
                    }
                } else if path.contains(dep) {
                    return true; // Cycle detected
                }
            }
        }

        path.remove(step_id);
        false
    }

    /// Get steps in execution order
    pub fn get_execution_order(&self) -> Result<Vec<String>, SagaError> {
        // Validate first
        self.validate()?;

        // Topological sort
        let mut result = Vec::new();
        let mut in_degree = HashMap::new();
        let mut zero_degree = Vec::new();

        // Create an adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut reverse_graph: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize adjacency list and in-degree
        for step in &self.steps {
            graph.insert(step.id.clone(), Vec::new());
            reverse_graph.insert(step.id.clone(), step.dependencies.clone());
            in_degree.insert(step.id.clone(), step.dependencies.len());

            if step.dependencies.is_empty() {
                zero_degree.push(step.id.clone());
            }
        }

        // Add edges (reversed because dependencies)
        for step in &self.steps {
            for dep in &step.dependencies {
                graph.entry(dep.clone()).or_default().push(step.id.clone());
            }
        }

        // Topological sort
        while let Some(step_id) = zero_degree.pop() {
            result.push(step_id.clone());

            if let Some(dependents) = graph.get(&step_id) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            zero_degree.push(dependent.clone());
                        }
                    }
                }
            }
        }

        // Check if all steps were included (no cycles)
        if result.len() != self.steps.len() {
            return Err(SagaError::DefinitionError(
                "Dependency cycle detected".to_string(),
            ));
        }

        Ok(result)
    }
}
