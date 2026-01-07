//! Integration tests for thread-safe persistent data structures.
//!
//! These tests verify that all persistent data structures work correctly
//! with the `arc` feature enabled, providing thread-safe access to
//! immutable data across multiple threads.

#![cfg(feature = "arc")]
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use lambars::persistent::{
    PersistentHashMap, PersistentHashSet, PersistentList, PersistentTreeMap, PersistentVector,
};
use rstest::rstest;
use std::sync::Arc;
use std::thread;

// =============================================================================
// PersistentList Integration Tests
// =============================================================================

#[rstest]
fn test_list_cross_thread_structural_sharing() {
    let original = Arc::new(PersistentList::new().cons(3).cons(2).cons(1));

    let handles: Vec<_> = (0..4)
        .map(|index| {
            let list_clone = Arc::clone(&original);
            thread::spawn(move || {
                // Each thread creates a new version by prepending
                let extended = list_clone.cons(index * 10);
                assert_eq!(extended.head(), Some(&(index * 10)));
                assert_eq!(extended.len(), 4);
                // Original should be unchanged
                assert_eq!(list_clone.len(), 3);
                extended
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("Thread panicked"))
        .collect();

    // Verify each thread created an independent list
    for (index, list) in results.iter().enumerate() {
        assert_eq!(list.head(), Some(&((index * 10) as i32)));
    }

    // Original should still be unchanged
    assert_eq!(original.len(), 3);
    assert_eq!(original.head(), Some(&1));
}

// =============================================================================
// PersistentVector Integration Tests
// =============================================================================

#[rstest]
fn test_vector_cross_thread_structural_sharing() {
    let original: Arc<PersistentVector<i32>> = Arc::new((0..100).collect());

    let handles: Vec<_> = (0..4)
        .map(|index| {
            let vector_clone = Arc::clone(&original);
            thread::spawn(move || {
                // Each thread modifies a different element
                let modified = vector_clone.update(index * 10, 999).unwrap();
                assert_eq!(modified.get(index * 10), Some(&999));
                // Original should be unchanged
                assert_eq!(vector_clone.get(index * 10), Some(&((index * 10) as i32)));
                modified
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("Thread panicked"))
        .collect();

    // Verify each thread created an independent vector
    for (index, vector) in results.iter().enumerate() {
        assert_eq!(vector.get(index * 10), Some(&999));
        // Other elements should be unchanged
        assert_eq!(
            vector.get(0),
            if index == 0 { Some(&999) } else { Some(&0) }
        );
    }

    // Original should still be unchanged
    for index in 0..100 {
        assert_eq!(original.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// PersistentHashMap Integration Tests
// =============================================================================

#[rstest]
fn test_hashmap_cross_thread_structural_sharing() {
    let original = Arc::new(
        PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3),
    );

    let handles: Vec<_> = (0..4)
        .map(|index| {
            let map_clone = Arc::clone(&original);
            thread::spawn(move || {
                // Each thread adds a new key
                let extended = map_clone.insert(format!("key_{index}"), index * 100);
                assert_eq!(extended.get(&format!("key_{index}")), Some(&(index * 100)));
                assert_eq!(extended.len(), 4);
                // Original keys should still exist
                assert_eq!(extended.get("a"), Some(&1));
                extended
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("Thread panicked"))
        .collect();

    // Verify each thread created an independent map
    for (index, map) in results.iter().enumerate() {
        assert_eq!(
            map.get(&format!("key_{index}")),
            Some(&((index * 100) as i32))
        );
    }

    // Original should still be unchanged
    assert_eq!(original.len(), 3);
    assert_eq!(original.get("key_0"), None);
}

// =============================================================================
// PersistentHashSet Integration Tests
// =============================================================================

#[rstest]
fn test_hashset_cross_thread_structural_sharing() {
    let original = Arc::new(PersistentHashSet::new().insert(1).insert(2).insert(3));

    let handles: Vec<_> = (0..4)
        .map(|index| {
            let set_clone = Arc::clone(&original);
            thread::spawn(move || {
                // Each thread adds a new element
                let extended = set_clone.insert((index + 1) * 100);
                assert!(extended.contains(&((index + 1) * 100)));
                assert_eq!(extended.len(), 4);
                // Original elements should still exist
                assert!(extended.contains(&1));
                extended
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("Thread panicked"))
        .collect();

    // Verify each thread created an independent set
    for (index, set) in results.iter().enumerate() {
        assert!(set.contains(&(((index + 1) * 100) as i32)));
    }

    // Original should still be unchanged
    assert_eq!(original.len(), 3);
    assert!(!original.contains(&100));
}

// =============================================================================
// PersistentTreeMap Integration Tests
// =============================================================================

#[rstest]
fn test_treemap_cross_thread_structural_sharing() {
    let original = Arc::new(
        PersistentTreeMap::new()
            .insert(10, "ten")
            .insert(20, "twenty")
            .insert(30, "thirty"),
    );

    let handles: Vec<_> = (0..4)
        .map(|index| {
            let map_clone = Arc::clone(&original);
            thread::spawn(move || {
                // Each thread adds a new key (using unique keys that don't conflict with 10, 20, 30)
                let key = index + 100;
                let extended = map_clone.insert(key, "new");
                assert_eq!(extended.get(&key), Some(&"new"));
                assert_eq!(extended.len(), 4);
                // Original keys should still exist
                assert_eq!(extended.get(&10), Some(&"ten"));
                // Keys should still be in sorted order
                let keys: Vec<&i32> = extended.keys().collect();
                let mut sorted_keys = keys.clone();
                sorted_keys.sort();
                assert_eq!(keys, sorted_keys);
                extended
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("Thread panicked"))
        .collect();

    // Verify each thread created an independent map with correct ordering
    for (index, map) in results.iter().enumerate() {
        let key = (index + 100) as i32;
        assert_eq!(map.get(&key), Some(&"new"));
        // Verify ordering is maintained
        let keys: Vec<&i32> = map.keys().collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys);
    }

    // Original should still be unchanged
    assert_eq!(original.len(), 3);
    assert_eq!(original.get(&100), None);
}

// =============================================================================
// Cross-Data-Structure Integration Tests
// =============================================================================

#[rstest]
fn test_combined_data_structures_across_threads() {
    // Create a complex nested structure
    let list = Arc::new(PersistentList::new().cons(3).cons(2).cons(1));
    let vector: Arc<PersistentVector<i32>> = Arc::new((0..10).collect());
    let map = Arc::new(
        PersistentHashMap::new()
            .insert("list_sum".to_string(), 6)
            .insert("vector_sum".to_string(), 45),
    );

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let list_clone = Arc::clone(&list);
            let vector_clone = Arc::clone(&vector);
            let map_clone = Arc::clone(&map);
            thread::spawn(move || {
                // Compute sum from list
                let list_sum: i32 = list_clone.iter().sum();
                assert_eq!(list_sum, 6);

                // Compute sum from vector
                let vector_sum: i32 = vector_clone.iter().sum();
                assert_eq!(vector_sum, 45);

                // Verify map has correct values
                assert_eq!(map_clone.get("list_sum"), Some(&6));
                assert_eq!(map_clone.get("vector_sum"), Some(&45));

                (list_sum, vector_sum)
            })
        })
        .collect();

    for handle in handles {
        let (list_sum, vector_sum) = handle.join().expect("Thread panicked");
        assert_eq!(list_sum, 6);
        assert_eq!(vector_sum, 45);
    }
}

// =============================================================================
// Stress Tests
// =============================================================================

#[rstest]
fn test_high_contention_list_operations() {
    let base_list = Arc::new(PersistentList::<i32>::new());

    // Many threads concurrently create derived lists
    let handles: Vec<_> = (0..100)
        .map(|index| {
            let list_clone = Arc::clone(&base_list);
            thread::spawn(move || {
                let new_list = list_clone.cons(index);
                assert_eq!(new_list.head(), Some(&index));
                assert_eq!(new_list.len(), 1);
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Original should still be empty
    assert!(base_list.is_empty());
}

#[rstest]
fn test_high_contention_vector_operations() {
    let base_vector: Arc<PersistentVector<i32>> = Arc::new((0..100).collect());

    // Many threads concurrently update the vector
    let handles: Vec<_> = (0..50)
        .map(|index| {
            let vector_clone = Arc::clone(&base_vector);
            thread::spawn(move || {
                let new_vector = vector_clone.update(index, -1).unwrap();
                assert_eq!(new_vector.get(index), Some(&-1));
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Original should be unchanged
    for index in 0..100 {
        assert_eq!(base_vector.get(index), Some(&(index as i32)));
    }
}

#[rstest]
fn test_high_contention_hashmap_operations() {
    let base_map: Arc<PersistentHashMap<i32, i32>> = Arc::new(PersistentHashMap::new());

    // Many threads concurrently insert into the map
    let handles: Vec<_> = (0..100)
        .map(|index| {
            let map_clone = Arc::clone(&base_map);
            thread::spawn(move || {
                let new_map = map_clone.insert(index, index * 2);
                assert_eq!(new_map.get(&index), Some(&(index * 2)));
                assert_eq!(new_map.len(), 1);
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Original should still be empty
    assert!(base_map.is_empty());
}
