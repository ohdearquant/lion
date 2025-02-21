mod tests;

use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Progression {
    steps: Arc<Mutex<Vec<Uuid>>>,
}

impl Progression {
    pub fn new() -> Self {
        Self {
            steps: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push(&self, id: Uuid) {
        let mut steps = self.steps.lock().unwrap();
        steps.push(id);
    }

    pub fn list(&self) -> Vec<Uuid> {
        let steps = self.steps.lock().unwrap();
        steps.clone()
    }

    pub fn len(&self) -> usize {
        let steps = self.steps.lock().unwrap();
        steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains(&self, id: &Uuid) -> bool {
        let steps = self.steps.lock().unwrap();
        steps.contains(id)
    }

    pub fn clear(&self) {
        let mut steps = self.steps.lock().unwrap();
        steps.clear();
    }
}

impl Default for Progression {
    fn default() -> Self {
        Self::new()
    }
}
