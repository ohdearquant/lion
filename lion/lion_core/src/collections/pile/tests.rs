use super::*;
use std::thread;

#[test]
fn test_pile_basic_operations() {
    let pile = Pile::<String>::new();
    let id = Uuid::new_v4();
    
    // Test insert
    pile.insert(id, "test".to_string()).unwrap();
    assert_eq!(pile.get(&id).unwrap(), "test");
    
    // Test remove
    pile.remove(&id).unwrap();
    assert!(pile.get(&id).is_err());
}

#[test]
fn test_pile_concurrent_access() {
    let pile = Arc::new(Pile::<i32>::new());
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let pile = pile.clone();
        handles.push(thread::spawn(move || {
            let id = Uuid::new_v4();
            pile.insert(id, i).unwrap();
            assert_eq!(pile.get(&id).unwrap(), i);
        }));
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_pile_ordering() {
    let pile = Pile::<i32>::new();
    let ids: Vec<_> = (0..5).map(|_| Uuid::new_v4()).collect();
    
    // Insert in order
    for (i, id) in ids.iter().enumerate() {
        pile.insert(*id, i as i32).unwrap();
    }
    
    // Check ordered retrieval
    let ordered = pile.get_ordered().unwrap();
    assert_eq!(ordered.len(), 5);
    for (i, value) in ordered.iter().enumerate() {
        assert_eq!(*value, i as i32);
    }
}
