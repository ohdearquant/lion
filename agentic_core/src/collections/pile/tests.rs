use super::*;
use std::thread;

#[test]
fn test_pile_insert_retrieve() {
    let pile = Pile::new();
    let id = Uuid::new_v4();
    pile.insert(id, "test_data".to_string());
    let retrieved = pile.get(&id);
    assert_eq!(retrieved, Some("test_data".to_string()));
}

#[test]
fn test_pile_concurrency() {
    let pile = Pile::new();
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let p = pile.clone();
            thread::spawn(move || {
                let id = Uuid::new_v4();
                p.insert(id, format!("val-{}", id));
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(pile.len(), 10);
}

#[test]
fn test_pile_ordered_access() {
    let pile = Pile::new();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();

    pile.insert(id1, "first".to_string());
    pile.insert(id2, "second".to_string());
    pile.insert(id3, "third".to_string());

    let ordered = pile.get_ordered();
    assert_eq!(ordered, vec!["first", "second", "third"]);

    let recent = pile.get_recent(2);
    assert_eq!(recent, vec!["third", "second"]);
}

#[test]
fn test_pile_max_size() {
    let pile = Pile::with_max_size(2);
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();

    pile.insert(id1, "first".to_string());
    pile.insert(id2, "second".to_string());
    pile.insert(id3, "third".to_string());

    assert_eq!(pile.len(), 2);
    assert!(!pile.contains(&id1)); // First item should be removed
    assert!(pile.contains(&id2));
    assert!(pile.contains(&id3));

    let ordered = pile.get_ordered();
    assert_eq!(ordered, vec!["second", "third"]);
}

#[test]
fn test_pile_filter() {
    let pile = Pile::new();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();

    pile.insert(id1, "apple".to_string());
    pile.insert(id2, "banana".to_string());
    pile.insert(id3, "apple pie".to_string());

    let apple_items = pile.filter(|item| item.contains("apple"));
    assert_eq!(apple_items.len(), 2);
    assert!(apple_items.contains(&"apple".to_string()));
    assert!(apple_items.contains(&"apple pie".to_string()));
}

#[test]
fn test_pile_remove() {
    let pile = Pile::new();
    let id = Uuid::new_v4();
    pile.insert(id, "test".to_string());

    assert_eq!(pile.len(), 1);
    let removed = pile.remove(&id);
    assert_eq!(removed, Some("test".to_string()));
    assert_eq!(pile.len(), 0);
    assert!(pile.get_ordered().is_empty());
}
