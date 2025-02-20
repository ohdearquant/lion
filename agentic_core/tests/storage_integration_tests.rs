use agentic_core::storage::{ElementId, FileStorage};
use serde_json::json;
use std::{fs, sync::Arc};
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn test_storage_basic_operations() {
    let temp_dir = tempdir().unwrap();
    let storage = FileStorage::new(temp_dir.path());

    // Create test data
    let id = ElementId(Uuid::new_v4());
    let data = json!({
        "name": "test-element",
        "value": 42,
        "tags": ["test", "example"]
    });

    // Test set operation
    storage.set(id, data.clone()).unwrap();

    // Test get operation
    let element = storage.get(id).unwrap();
    assert_eq!(element.data.content, data);

    // Test list operation
    let elements = storage.list();
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].id, id);

    // Test remove operation
    assert!(storage.remove(id));
    assert!(storage.get(id).is_none());
    assert_eq!(storage.list().len(), 0);
}

#[test]
fn test_storage_persistence() {
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path();

    // Store data using first instance
    let storage1 = FileStorage::new(storage_path);
    let id = ElementId(Uuid::new_v4());
    let data = json!({
        "name": "persistent-data",
        "value": "test"
    });
    storage1.set(id, data.clone()).unwrap();

    // Create new storage instance and verify data
    let storage2 = FileStorage::new(storage_path);
    let element = storage2.get(id).unwrap();
    assert_eq!(element.data.content, data);
}

#[test]
fn test_storage_concurrent_access() {
    let temp_dir = tempdir().unwrap();
    let storage = FileStorage::new(temp_dir.path());
    let storage = Arc::new(storage);

    // Create multiple elements concurrently
    let mut handles = Vec::new();
    for i in 0..10 {
        let storage = storage.clone();
        handles.push(std::thread::spawn(move || {
            let id = ElementId(Uuid::new_v4());
            let data = json!({
                "thread": i,
                "value": format!("test-{}", i)
            });
            storage.set(id, data).unwrap();
        }));
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all elements were stored
    assert_eq!(storage.list().len(), 10);
}

#[test]
fn test_storage_error_handling() {
    let temp_dir = tempdir().unwrap();
    let storage = FileStorage::new(temp_dir.path());

    // Test invalid JSON data
    let id = ElementId(Uuid::new_v4());
    let file_path = temp_dir.path().join(format!("{}", id));
    fs::write(&file_path, "invalid json").unwrap();
    assert!(storage.get(id).is_none());

    // Test non-existent element
    let non_existent_id = ElementId(Uuid::new_v4());
    assert!(storage.get(non_existent_id).is_none());

    // Test removing non-existent element
    assert!(!storage.remove(non_existent_id));
}

#[test]
fn test_storage_large_data() {
    let temp_dir = tempdir().unwrap();
    let storage = FileStorage::new(temp_dir.path());

    // Create large test data
    let id = ElementId(Uuid::new_v4());
    let mut large_array = Vec::new();
    for i in 0..1000 {
        large_array.push(json!({
            "index": i,
            "data": "some relatively long string to increase the data size"
        }));
    }
    let large_data = json!({
        "name": "large-element",
        "array": large_array
    });

    // Store and retrieve large data
    storage.set(id, large_data.clone()).unwrap();
    let element = storage.get(id).unwrap();
    assert_eq!(element.data.content, large_data);
}

#[test]
fn test_storage_file_organization() {
    let temp_dir = tempdir().unwrap();
    let storage = FileStorage::new(temp_dir.path());

    // Create multiple elements
    let mut ids = Vec::new();
    for i in 0..5 {
        let id = ElementId(Uuid::new_v4());
        ids.push(id);
        let data = json!({
            "index": i,
            "name": format!("element-{}", i)
        });
        storage.set(id, data).unwrap();
    }

    // Verify file structure
    for id in ids {
        let file_path = temp_dir.path().join(format!("{}", id));
        assert!(file_path.exists());
        assert!(file_path.is_file());
    }
}

#[test]
fn test_storage_data_integrity() {
    let temp_dir = tempdir().unwrap();
    let storage = FileStorage::new(temp_dir.path());

    // Store element with complex data
    let id = ElementId(Uuid::new_v4());
    let data = json!({
        "string": "test string",
        "number": 42,
        "float": 3.14,
        "boolean": true,
        "null": null,
        "array": [1, 2, 3],
        "object": {
            "nested": "value",
            "array": ["a", "b", "c"]
        }
    });

    storage.set(id, data.clone()).unwrap();

    // Retrieve and verify all data types are preserved
    let element = storage.get(id).unwrap();
    assert_eq!(element.data.content, data);
    assert_eq!(element.data.content["string"], "test string");
    assert_eq!(element.data.content["number"], 42);
    assert_eq!(element.data.content["float"], 3.14);
    assert_eq!(element.data.content["boolean"], true);
    assert!(element.data.content["null"].is_null());
    assert_eq!(element.data.content["array"], json!([1, 2, 3]));
    assert_eq!(element.data.content["object"]["nested"], "value");
    assert_eq!(
        element.data.content["object"]["array"],
        json!(["a", "b", "c"])
    );
}
