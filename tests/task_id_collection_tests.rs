//! Unit tests for TaskIdCollection.
//!
//! These tests follow the TDD approach, testing all API methods
//! and state transitions for the TaskIdCollection implementation.

#![cfg(feature = "persistent")]

use lambars::persistent::TaskIdCollection;
use rstest::rstest;

// =============================================================================
// TDD Cycle 1: Empty collection creation
// =============================================================================

#[rstest]
fn test_new_creates_empty_collection() {
    let collection: TaskIdCollection<i32> = TaskIdCollection::new();
    assert!(collection.is_empty());
    assert_eq!(collection.len(), 0);
}

// =============================================================================
// TDD Cycle 2: Empty to Small transition (insert)
// =============================================================================

#[rstest]
fn test_insert_single_element_transitions_to_small() {
    let collection: TaskIdCollection<i32> = TaskIdCollection::new();
    let collection = collection.insert(42);

    assert!(!collection.is_empty());
    assert_eq!(collection.len(), 1);
    assert!(collection.contains(&42));
}

#[rstest]
fn test_insert_multiple_elements_stays_in_small() {
    let collection: TaskIdCollection<i32> = TaskIdCollection::new();
    let collection = collection
        .insert(1)
        .insert(2)
        .insert(3)
        .insert(4)
        .insert(5)
        .insert(6)
        .insert(7)
        .insert(8);

    assert_eq!(collection.len(), 8);
    for i in 1..=8 {
        assert!(collection.contains(&i));
    }
}

#[rstest]
fn test_insert_duplicate_returns_same_length() {
    let collection = TaskIdCollection::new().insert(42);
    let collection_with_duplicate = collection.insert(42);

    assert_eq!(collection.len(), 1);
    assert_eq!(collection_with_duplicate.len(), 1);
}

#[rstest]
fn test_insert_preserves_immutability() {
    let collection1 = TaskIdCollection::new().insert(1);
    let collection2 = collection1.insert(2);

    assert_eq!(collection1.len(), 1);
    assert!(collection1.contains(&1));
    assert!(!collection1.contains(&2));

    assert_eq!(collection2.len(), 2);
    assert!(collection2.contains(&1));
    assert!(collection2.contains(&2));
}

// =============================================================================
// TDD Cycle 3: Small to Large promotion (9th element)
// =============================================================================

#[rstest]
fn test_insert_ninth_element_promotes_to_large() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=8 {
        collection = collection.insert(i);
    }
    assert_eq!(collection.len(), 8);

    // Insert 9th element - should promote to Large
    let collection = collection.insert(9);

    assert_eq!(collection.len(), 9);
    for i in 1..=9 {
        assert!(
            collection.contains(&i),
            "Should contain {} after promotion",
            i
        );
    }
}

#[rstest]
fn test_insert_many_elements_in_large_state() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=100 {
        collection = collection.insert(i);
    }

    assert_eq!(collection.len(), 100);
    for i in 1..=100 {
        assert!(collection.contains(&i));
    }
}

// =============================================================================
// TDD Cycle 4: Remove operation
// =============================================================================

#[rstest]
fn test_remove_from_small_collection() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);
    let collection = collection.remove(&2);

    assert_eq!(collection.len(), 2);
    assert!(collection.contains(&1));
    assert!(!collection.contains(&2));
    assert!(collection.contains(&3));
}

#[rstest]
fn test_remove_nonexistent_element_returns_same_length() {
    let collection = TaskIdCollection::new().insert(1).insert(2);
    let collection_after_remove = collection.remove(&999);

    assert_eq!(collection_after_remove.len(), 2);
}

#[rstest]
fn test_remove_last_element_transitions_to_empty() {
    let collection = TaskIdCollection::new().insert(42);
    let collection = collection.remove(&42);

    assert!(collection.is_empty());
    assert_eq!(collection.len(), 0);
}

#[rstest]
fn test_remove_preserves_immutability() {
    let collection1 = TaskIdCollection::new().insert(1).insert(2);
    let collection2 = collection1.remove(&1);

    assert_eq!(collection1.len(), 2);
    assert!(collection1.contains(&1));

    assert_eq!(collection2.len(), 1);
    assert!(!collection2.contains(&1));
    assert!(collection2.contains(&2));
}

// =============================================================================
// TDD Cycle 5: Large to Small demotion
// =============================================================================

#[rstest]
fn test_remove_from_large_demotes_to_small_when_8_or_less() {
    // Create a Large collection with 9 elements
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=9 {
        collection = collection.insert(i);
    }
    assert_eq!(collection.len(), 9);

    // Remove one element - should demote to Small (8 elements)
    let collection = collection.remove(&9);

    assert_eq!(collection.len(), 8);
    for i in 1..=8 {
        assert!(collection.contains(&i));
    }
}

#[rstest]
fn test_multiple_removes_from_large_eventually_to_empty() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=10 {
        collection = collection.insert(i);
    }

    // Remove all elements
    for i in 1..=10 {
        collection = collection.remove(&i);
    }

    assert!(collection.is_empty());
    assert_eq!(collection.len(), 0);
}

// =============================================================================
// TDD Cycle 6: contains operation
// =============================================================================

#[rstest]
fn test_contains_on_empty_collection() {
    let collection: TaskIdCollection<i32> = TaskIdCollection::new();
    assert!(!collection.contains(&42));
}

#[rstest]
fn test_contains_on_small_collection() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);

    assert!(collection.contains(&1));
    assert!(collection.contains(&2));
    assert!(collection.contains(&3));
    assert!(!collection.contains(&4));
}

#[rstest]
fn test_contains_on_large_collection() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=20 {
        collection = collection.insert(i);
    }

    for i in 1..=20 {
        assert!(collection.contains(&i));
    }
    assert!(!collection.contains(&21));
    assert!(!collection.contains(&0));
}

// =============================================================================
// TDD Cycle 7: iter_sorted operation
// =============================================================================

#[rstest]
fn test_iter_sorted_on_empty_collection() {
    let collection: TaskIdCollection<i32> = TaskIdCollection::new();
    let sorted: Vec<&i32> = collection.iter_sorted().collect();
    assert!(sorted.is_empty());
}

#[rstest]
fn test_iter_sorted_on_small_collection_returns_sorted_order() {
    let collection = TaskIdCollection::new()
        .insert(5)
        .insert(1)
        .insert(3)
        .insert(2)
        .insert(4);

    let sorted: Vec<&i32> = collection.iter_sorted().collect();
    assert_eq!(sorted, vec![&1, &2, &3, &4, &5]);
}

#[rstest]
fn test_iter_sorted_on_large_collection_returns_sorted_order() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    // Insert in reverse order
    for i in (1..=20).rev() {
        collection = collection.insert(i);
    }

    let sorted: Vec<i32> = collection.iter_sorted().copied().collect();
    let expected: Vec<i32> = (1..=20).collect();
    assert_eq!(sorted, expected);
}

// =============================================================================
// TDD Cycle 8: iter operation (unordered)
// =============================================================================

#[rstest]
fn test_iter_on_empty_collection() {
    let collection: TaskIdCollection<i32> = TaskIdCollection::new();
    let items: Vec<&i32> = collection.iter().collect();
    assert!(items.is_empty());
}

#[rstest]
fn test_iter_on_small_collection_contains_all_elements() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);

    let mut items: Vec<i32> = collection.iter().copied().collect();
    items.sort();
    assert_eq!(items, vec![1, 2, 3]);
}

#[rstest]
fn test_iter_on_large_collection_contains_all_elements() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=20 {
        collection = collection.insert(i);
    }

    let mut items: Vec<i32> = collection.iter().copied().collect();
    items.sort();
    let expected: Vec<i32> = (1..=20).collect();
    assert_eq!(items, expected);
}

// =============================================================================
// TDD Cycle 9: Idempotency of duplicate insertion
// =============================================================================

#[rstest]
fn test_duplicate_insertion_is_idempotent_in_small() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);

    // Insert duplicates
    let collection = collection.insert(1).insert(2).insert(3);

    assert_eq!(collection.len(), 3);
}

#[rstest]
fn test_duplicate_insertion_is_idempotent_in_large() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=20 {
        collection = collection.insert(i);
    }
    let original_len = collection.len();

    // Insert duplicates
    for i in 1..=20 {
        collection = collection.insert(i);
    }

    assert_eq!(collection.len(), original_len);
}

// =============================================================================
// TDD Cycle 10: Edge cases and string type support with Borrow trait
// =============================================================================

#[rstest]
fn test_with_string_elements() {
    let collection = TaskIdCollection::new()
        .insert("apple".to_string())
        .insert("banana".to_string())
        .insert("cherry".to_string());

    assert_eq!(collection.len(), 3);
    // Use borrowed &str for contains (no allocation)
    assert!(collection.contains("apple"));
    assert!(collection.contains("banana"));
    assert!(collection.contains("cherry"));
}

#[rstest]
fn test_iter_sorted_with_strings() {
    let collection = TaskIdCollection::new()
        .insert("cherry".to_string())
        .insert("apple".to_string())
        .insert("banana".to_string());

    let sorted: Vec<&String> = collection.iter_sorted().collect();
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0], "apple");
    assert_eq!(sorted[1], "banana");
    assert_eq!(sorted[2], "cherry");
}

#[rstest]
fn test_borrow_contains_in_small_collection() {
    let collection = TaskIdCollection::new()
        .insert("one".to_string())
        .insert("two".to_string())
        .insert("three".to_string());

    // Borrow search - no String allocation needed
    assert!(collection.contains("one"));
    assert!(collection.contains("two"));
    assert!(collection.contains("three"));
    assert!(!collection.contains("four"));
}

#[rstest]
fn test_borrow_contains_in_large_collection() {
    let mut collection: TaskIdCollection<String> = TaskIdCollection::new();
    for i in 1..=20 {
        collection = collection.insert(format!("item_{}", i));
    }

    // Borrow search - no String allocation needed
    assert!(collection.contains("item_1"));
    assert!(collection.contains("item_10"));
    assert!(collection.contains("item_20"));
    assert!(!collection.contains("item_21"));
}

#[rstest]
fn test_borrow_remove_in_small_collection() {
    let collection = TaskIdCollection::new()
        .insert("apple".to_string())
        .insert("banana".to_string())
        .insert("cherry".to_string());

    // Borrow remove - no String allocation needed
    let collection = collection.remove("banana");

    assert_eq!(collection.len(), 2);
    assert!(collection.contains("apple"));
    assert!(!collection.contains("banana"));
    assert!(collection.contains("cherry"));
}

#[rstest]
fn test_borrow_remove_in_large_collection() {
    let mut collection: TaskIdCollection<String> = TaskIdCollection::new();
    for i in 1..=20 {
        collection = collection.insert(format!("item_{}", i));
    }

    // Borrow remove - no String allocation needed
    let collection = collection.remove("item_10");

    assert_eq!(collection.len(), 19);
    assert!(!collection.contains("item_10"));
    assert!(collection.contains("item_1"));
    assert!(collection.contains("item_20"));
}

// =============================================================================
// TDD Cycle 11: Boundary conditions for state transitions
// =============================================================================

#[rstest]
fn test_exactly_8_elements_stays_small() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=8 {
        collection = collection.insert(i);
    }

    assert_eq!(collection.len(), 8);
    // Verify all elements are accessible
    for i in 1..=8 {
        assert!(collection.contains(&i));
    }
}

#[rstest]
fn test_transition_boundary_9_elements() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=9 {
        collection = collection.insert(i);
    }

    assert_eq!(collection.len(), 9);
    // Verify all elements are accessible after promotion
    for i in 1..=9 {
        assert!(collection.contains(&i));
    }
}

#[rstest]
fn test_demotion_boundary_from_9_to_8() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=9 {
        collection = collection.insert(i);
    }
    assert_eq!(collection.len(), 9);

    let collection = collection.remove(&1);
    assert_eq!(collection.len(), 8);

    // Verify all remaining elements
    for i in 2..=9 {
        assert!(collection.contains(&i));
    }
    assert!(!collection.contains(&1));
}

// =============================================================================
// TDD Cycle 12: Clone and Default traits
// =============================================================================

#[rstest]
fn test_clone_empty_collection() {
    let collection: TaskIdCollection<i32> = TaskIdCollection::new();
    let cloned = collection.clone();

    assert!(cloned.is_empty());
}

#[rstest]
fn test_clone_small_collection() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);
    let cloned = collection.clone();

    assert_eq!(cloned.len(), 3);
    assert!(cloned.contains(&1));
    assert!(cloned.contains(&2));
    assert!(cloned.contains(&3));
}

#[rstest]
fn test_clone_large_collection() {
    let mut collection: TaskIdCollection<i32> = TaskIdCollection::new();
    for i in 1..=20 {
        collection = collection.insert(i);
    }
    let cloned = collection.clone();

    assert_eq!(cloned.len(), 20);
    for i in 1..=20 {
        assert!(cloned.contains(&i));
    }
}

#[rstest]
fn test_default_creates_empty() {
    let collection: TaskIdCollection<i32> = TaskIdCollection::default();
    assert!(collection.is_empty());
}

// =============================================================================
// TDD Cycle 13: Iterator must_use and ExactSizeIterator
// =============================================================================

#[rstest]
fn test_iter_exact_size() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);
    let iter = collection.iter();

    assert_eq!(iter.len(), 3);
}

#[rstest]
fn test_iter_sorted_exact_size() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);
    let iter = collection.iter_sorted();

    assert_eq!(iter.len(), 3);
}

#[rstest]
fn test_iter_size_hint() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);
    let iter = collection.iter();

    assert_eq!(iter.size_hint(), (3, Some(3)));
}

#[rstest]
fn test_iter_sorted_size_hint() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);
    let iter = collection.iter_sorted();

    assert_eq!(iter.size_hint(), (3, Some(3)));
}

// =============================================================================
// TDD Cycle 14: Debug and IntoIterator traits
// =============================================================================

#[rstest]
fn test_debug_empty() {
    let collection: TaskIdCollection<i32> = TaskIdCollection::new();
    let debug_str = format!("{:?}", collection);
    assert_eq!(debug_str, "{}");
}

#[rstest]
fn test_debug_small() {
    let collection = TaskIdCollection::new().insert(1);
    let debug_str = format!("{:?}", collection);
    assert!(debug_str.contains("1"));
}

#[rstest]
fn test_into_iterator() {
    let collection = TaskIdCollection::new().insert(1).insert(2).insert(3);

    let mut items: Vec<i32> = Vec::new();
    for &item in &collection {
        items.push(item);
    }
    items.sort();

    assert_eq!(items, vec![1, 2, 3]);
}

// =============================================================================
// TDD Cycle 15: Encapsulation - enum variants are not directly accessible
// =============================================================================

// Note: This test verifies that the internal state is properly encapsulated.
// Users cannot directly construct TaskIdCollection variants, only use the
// public API (new, insert, remove, etc.).
#[rstest]
fn test_encapsulation_only_public_api_available() {
    // The only way to create a TaskIdCollection is through the public API
    let empty: TaskIdCollection<i32> = TaskIdCollection::new();
    let with_element = empty.insert(42);
    let removed = with_element.remove(&42);

    // All state transitions happen internally through the API
    assert!(removed.is_empty());

    // We cannot do: TaskIdCollection::Empty or TaskIdCollection::Small(...)
    // because the internal representation is not exposed
}
