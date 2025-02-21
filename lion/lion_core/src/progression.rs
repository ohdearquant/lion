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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_progression_push_list() {
        let prog = Progression::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        prog.push(id1);
        prog.push(id2);

        let all = prog.list();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], id1);
        assert_eq!(all[1], id2);
    }

    #[test]
    fn test_progression_concurrency() {
        let prog = Progression::new();
        let handles: Vec<_> = (0..5)
            .map(|_| {
                let p = prog.clone();
                thread::spawn(move || {
                    let id = Uuid::new_v4();
                    p.push(id);
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(prog.len(), 5);
    }

    #[test]
    fn test_progression_operations() {
        let prog = Progression::new();
        assert!(prog.is_empty());

        let id = Uuid::new_v4();
        prog.push(id);

        assert!(!prog.is_empty());
        assert_eq!(prog.len(), 1);
        assert!(prog.contains(&id));

        prog.clear();
        assert!(prog.is_empty());
        assert!(!prog.contains(&id));
    }
}
