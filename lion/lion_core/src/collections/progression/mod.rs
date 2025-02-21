mod tests;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur when working with a Progression
#[derive(Error, Debug)]
pub enum ProgressionError {
    #[error("Lock acquisition failed: {0}")]
    LockError(String),
    #[error("Step not found: {0}")]
    NotFound(Uuid),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
}

/// Result type for Progression operations
pub type ProgressionResult<T> = Result<T, ProgressionError>;

/// Metadata for a progression step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepMetadata {
    /// Agent that created this step
    pub agent_id: Uuid,
    /// Timestamp when the step was created
    pub timestamp: DateTime<Utc>,
    /// Optional parent step ID (for branching)
    pub parent_id: Option<Uuid>,
    /// Branch name if this step is part of a branch
    pub branch: Option<String>,
    /// Additional metadata as JSON
    pub metadata: serde_json::Value,
}

/// A step in the progression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressionStep {
    /// Unique identifier for the step
    pub id: Uuid,
    /// Step metadata
    pub metadata: StepMetadata,
    /// Next steps (for branching)
    pub next_steps: Vec<Uuid>,
}

/// A thread-safe ordered progression of steps that supports branching and merging
/// for tracking multi-agent task execution and collaboration.
///
/// The Progression structure is a key component in the microkernel's task tracking system:
/// - Maintains a chronological record of task execution steps
/// - Supports branching for parallel agent work streams
/// - Enables merging of parallel work back into the main progression
/// - Provides thread-safe access for concurrent agent operations
///
/// Key use cases:
/// - Task decomposition: Track subtasks assigned to different agents
/// - Parallel exploration: Allow agents to work on different solution approaches
/// - Collaborative work: Merge results from multiple agents into a final solution
/// - Progress monitoring: Track the evolution of complex multi-agent tasks
///
/// Example:
/// ```rust
/// # use lion_core::collections::Progression;
/// let progression = Progression::new();
/// // Track main task steps and create branches for subtasks
/// ```
///
#[derive(Debug, Clone)]
pub struct Progression {
    /// All steps in the progression
    steps: Arc<Mutex<HashMap<Uuid, ProgressionStep>>>,
    /// Root step IDs (steps with no parents)
    roots: Arc<Mutex<Vec<Uuid>>>,
    /// Current active branches
    branches: Arc<Mutex<HashMap<String, Vec<Uuid>>>>,
}

impl Progression {
    pub fn new() -> Self {
        Self {
            steps: Arc::new(Mutex::new(HashMap::new())),
            roots: Arc::new(Mutex::new(Vec::new())),
            branches: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a new step to the main progression
    pub fn push(&self, agent_id: Uuid, metadata: serde_json::Value) -> ProgressionResult<Uuid> {
        let step_id = Uuid::new_v4();
        let step = ProgressionStep {
            id: step_id,
            metadata: StepMetadata {
                agent_id,
                timestamp: Utc::now(),
                parent_id: None,
                branch: None,
                metadata,
            },
            next_steps: Vec::new(),
        };

        let mut steps = self
            .steps
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        let mut roots = self
            .roots
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;

        steps.insert(step_id, step);
        roots.push(step_id);

        Ok(step_id)
    }

    /// Create a new branch from a specific step
    pub fn create_branch(&self, branch_name: String, parent_id: Uuid) -> ProgressionResult<()> {
        let steps = self
            .steps
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        let mut branches = self
            .branches
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;

        if !steps.contains_key(&parent_id) {
            return Err(ProgressionError::NotFound(parent_id));
        }

        branches.insert(branch_name, vec![parent_id]);
        Ok(())
    }

    /// Add a step to a specific branch
    pub fn push_to_branch(
        &self,
        branch_name: &str,
        agent_id: Uuid,
        metadata: serde_json::Value,
    ) -> ProgressionResult<Uuid> {
        let mut branches = self
            .branches
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        let mut steps = self
            .steps
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;

        let branch = branches
            .get_mut(branch_name)
            .ok_or_else(|| ProgressionError::BranchNotFound(branch_name.to_string()))?;

        let parent_id = *branch
            .last()
            .ok_or_else(|| ProgressionError::InvalidOperation("Branch is empty".to_string()))?;

        let step_id = Uuid::new_v4();
        let step = ProgressionStep {
            id: step_id,
            metadata: StepMetadata {
                agent_id,
                timestamp: Utc::now(),
                parent_id: Some(parent_id),
                branch: Some(branch_name.to_string()),
                metadata,
            },
            next_steps: Vec::new(),
        };

        if let Some(parent_step) = steps.get_mut(&parent_id) {
            parent_step.next_steps.push(step_id);
        }

        steps.insert(step_id, step);
        branch.push(step_id);

        Ok(step_id)
    }

    /// Merge a branch back into the main progression
    pub fn merge_branch(&self, branch_name: &str) -> ProgressionResult<Uuid> {
        let mut branches = self
            .branches
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;

        let branch = branches
            .remove(branch_name)
            .ok_or_else(|| ProgressionError::BranchNotFound(branch_name.to_string()))?;

        let last_step_id = *branch
            .last()
            .ok_or_else(|| ProgressionError::InvalidOperation("Branch is empty".to_string()))?;

        Ok(last_step_id)
    }

    /// Get all steps in order (for a specific branch if specified)
    pub fn list(&self, branch_name: Option<&str>) -> ProgressionResult<Vec<ProgressionStep>> {
        let steps = self
            .steps
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        let branches = self
            .branches
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;

        match branch_name {
            Some(name) => {
                let branch = branches
                    .get(name)
                    .ok_or_else(|| ProgressionError::BranchNotFound(name.to_string()))?;
                Ok(branch
                    .iter()
                    .filter_map(|id| steps.get(id).cloned())
                    .collect())
            }
            None => {
                let roots = self
                    .roots
                    .lock()
                    .map_err(|e| ProgressionError::LockError(e.to_string()))?;
                let mut result = Vec::new();
                let mut stack = roots.clone();

                while let Some(id) = stack.pop() {
                    if let Some(step) = steps.get(&id) {
                        result.push(step.clone());
                        stack.extend(step.next_steps.iter());
                    }
                }

                Ok(result)
            }
        }
    }

    /// Get steps created by a specific agent
    pub fn get_agent_steps(&self, agent_id: Uuid) -> ProgressionResult<Vec<ProgressionStep>> {
        let steps = self
            .steps
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        Ok(steps
            .values()
            .filter(|step| step.metadata.agent_id == agent_id)
            .cloned()
            .collect())
    }

    /// Get the number of steps
    pub fn len(&self) -> ProgressionResult<usize> {
        let steps = self
            .steps
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        Ok(steps.len())
    }

    /// Check if the progression is empty
    pub fn is_empty(&self) -> ProgressionResult<bool> {
        Ok(self.len()? == 0)
    }

    /// Check if a step exists
    pub fn contains(&self, id: &Uuid) -> ProgressionResult<bool> {
        let steps = self
            .steps
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        Ok(steps.contains_key(id))
    }

    /// Clear all steps
    pub fn clear(&self) -> ProgressionResult<()> {
        let mut steps = self
            .steps
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        let mut roots = self
            .roots
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        let mut branches = self
            .branches
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;

        steps.clear();
        roots.clear();
        branches.clear();
        Ok(())
    }

    /// Get active branches
    pub fn list_branches(&self) -> ProgressionResult<Vec<String>> {
        let branches = self
            .branches
            .lock()
            .map_err(|e| ProgressionError::LockError(e.to_string()))?;
        Ok(branches.keys().cloned().collect())
    }
}

impl Default for Progression {
    fn default() -> Self {
        Self::new()
    }
}

// Helper trait for recovering from poisoned mutexes
trait PoisonRecovery<T> {
    fn recover(self) -> Result<T, ProgressionError>;
}

impl<T> PoisonRecovery<T> for Result<T, PoisonError<T>> {
    fn recover(self) -> Result<T, ProgressionError> {
        self.map_err(|e| ProgressionError::LockError(format!("Mutex poisoned: {}", e)))
    }
}
