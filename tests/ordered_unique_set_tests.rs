//! Unit tests for OrderedUniqueSet.
//!
//! These tests follow the TDD approach, testing all API methods
//! and state transitions for the OrderedUniqueSet implementation.

#![cfg(feature = "persistent")]

use lambars::persistent::OrderedUniqueSet;
use rstest::rstest;

#[rstest]
fn test_new_creates_empty_collection() {
    let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    assert!(collection.is_empty());
    assert_eq!(collection.len(), 0);
}

#[rstest]
fn test_insert_single_element_transitions_to_small() {
    let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    let collection = collection.insert(42);

    assert!(!collection.is_empty());
    assert_eq!(collection.len(), 1);
    assert!(collection.contains(&42));
}

#[rstest]
fn test_insert_multiple_elements_stays_in_small() {
    let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
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
fn test_insert_duplicate_returns_same_length_and_content() {
    let collection = OrderedUniqueSet::new().insert(42);
    let collection_with_duplicate = collection.insert(42);

    assert_eq!(collection.len(), 1);
    assert_eq!(collection_with_duplicate.len(), 1);
    // Verify content equality (not just length)
    assert_eq!(collection, collection_with_duplicate);
    assert!(collection_with_duplicate.contains(&42));
}

#[rstest]
fn test_insert_preserves_immutability() {
    let collection1 = OrderedUniqueSet::new().insert(1);
    let collection2 = collection1.insert(2);

    assert_eq!(collection1.len(), 1);
    assert!(collection1.contains(&1));
    assert!(!collection1.contains(&2));

    assert_eq!(collection2.len(), 2);
    assert!(collection2.contains(&1));
    assert!(collection2.contains(&2));
}

#[rstest]
fn test_insert_ninth_element_promotes_to_large() {
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
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
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    for i in 1..=100 {
        collection = collection.insert(i);
    }

    assert_eq!(collection.len(), 100);
    for i in 1..=100 {
        assert!(collection.contains(&i));
    }
}

#[rstest]
fn test_remove_from_small_collection() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
    let collection = collection.remove(&2);

    assert_eq!(collection.len(), 2);
    assert!(collection.contains(&1));
    assert!(!collection.contains(&2));
    assert!(collection.contains(&3));
}

#[rstest]
fn test_remove_nonexistent_element_returns_same_length_and_content() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2);
    let collection_after_remove = collection.remove(&999);

    assert_eq!(collection_after_remove.len(), 2);
    // Verify content equality (not just length)
    assert_eq!(collection, collection_after_remove);
    assert!(collection_after_remove.contains(&1));
    assert!(collection_after_remove.contains(&2));
    assert!(!collection_after_remove.contains(&999));
}

#[rstest]
fn test_remove_last_element_transitions_to_empty() {
    let collection = OrderedUniqueSet::new().insert(42);
    let collection = collection.remove(&42);

    assert!(collection.is_empty());
    assert_eq!(collection.len(), 0);
}

#[rstest]
fn test_remove_preserves_immutability() {
    let collection1 = OrderedUniqueSet::new().insert(1).insert(2);
    let collection2 = collection1.remove(&1);

    assert_eq!(collection1.len(), 2);
    assert!(collection1.contains(&1));

    assert_eq!(collection2.len(), 1);
    assert!(!collection2.contains(&1));
    assert!(collection2.contains(&2));
}

#[rstest]
fn test_remove_from_large_demotes_to_small_when_8_or_less() {
    // Create a Large collection with 9 elements
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
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
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
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

#[rstest]
fn test_contains_on_empty_collection() {
    let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    assert!(!collection.contains(&42));
}

#[rstest]
fn test_contains_on_small_collection() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);

    assert!(collection.contains(&1));
    assert!(collection.contains(&2));
    assert!(collection.contains(&3));
    assert!(!collection.contains(&4));
}

#[rstest]
fn test_contains_on_large_collection() {
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    for i in 1..=20 {
        collection = collection.insert(i);
    }

    for i in 1..=20 {
        assert!(collection.contains(&i));
    }
    assert!(!collection.contains(&21));
    assert!(!collection.contains(&0));
}

#[rstest]
fn test_iter_sorted_on_empty_collection() {
    let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    let sorted: Vec<&i32> = collection.iter_sorted().collect();
    assert!(sorted.is_empty());
}

#[rstest]
fn test_iter_sorted_on_small_collection_returns_sorted_order() {
    let collection = OrderedUniqueSet::new()
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
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    // Insert in reverse order
    for i in (1..=20).rev() {
        collection = collection.insert(i);
    }

    let sorted: Vec<i32> = collection.iter_sorted().copied().collect();
    let expected: Vec<i32> = (1..=20).collect();
    assert_eq!(sorted, expected);
}

#[rstest]
fn test_iter_on_empty_collection() {
    let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    let items: Vec<&i32> = collection.iter().collect();
    assert!(items.is_empty());
}

#[rstest]
fn test_iter_on_small_collection_contains_all_elements() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);

    let mut items: Vec<i32> = collection.iter().copied().collect();
    items.sort();
    assert_eq!(items, vec![1, 2, 3]);
}

#[rstest]
fn test_iter_on_large_collection_contains_all_elements() {
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    for i in 1..=20 {
        collection = collection.insert(i);
    }

    let mut items: Vec<i32> = collection.iter().copied().collect();
    items.sort();
    let expected: Vec<i32> = (1..=20).collect();
    assert_eq!(items, expected);
}

#[rstest]
fn test_duplicate_insertion_is_idempotent_in_small() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);

    // Insert duplicates
    let collection_after = collection.insert(1).insert(2).insert(3);

    assert_eq!(collection_after.len(), 3);
    // Verify content equality
    assert_eq!(collection, collection_after);
    assert!(collection_after.contains(&1));
    assert!(collection_after.contains(&2));
    assert!(collection_after.contains(&3));
}

#[rstest]
fn test_duplicate_insertion_is_idempotent_in_large() {
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    for i in 1..=20 {
        collection = collection.insert(i);
    }
    let original = collection.clone();
    let original_len = collection.len();

    // Insert duplicates
    for i in 1..=20 {
        collection = collection.insert(i);
    }

    assert_eq!(collection.len(), original_len);
    // Verify content equality
    assert_eq!(collection, original);
    // Verify all elements are still present
    for i in 1..=20 {
        assert!(collection.contains(&i));
    }
}

#[rstest]
fn test_with_string_elements() {
    let collection = OrderedUniqueSet::new()
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
    let collection = OrderedUniqueSet::new()
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
    let collection = OrderedUniqueSet::new()
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
    let mut collection: OrderedUniqueSet<String> = OrderedUniqueSet::new();
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
    let collection = OrderedUniqueSet::new()
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
    let mut collection: OrderedUniqueSet<String> = OrderedUniqueSet::new();
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

#[rstest]
fn test_exactly_8_elements_stays_small() {
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
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
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
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
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
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

#[rstest]
fn test_clone_empty_collection() {
    let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    let cloned = collection.clone();

    assert!(cloned.is_empty());
}

#[rstest]
fn test_clone_small_collection() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
    let cloned = collection.clone();

    assert_eq!(cloned.len(), 3);
    assert!(cloned.contains(&1));
    assert!(cloned.contains(&2));
    assert!(cloned.contains(&3));
}

#[rstest]
fn test_clone_large_collection() {
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
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
    let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::default();
    assert!(collection.is_empty());
}

#[rstest]
fn test_iter_exact_size() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
    let iter = collection.iter();

    assert_eq!(iter.len(), 3);
}

#[rstest]
fn test_iter_sorted_exact_size() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
    let iter = collection.iter_sorted();

    assert_eq!(iter.len(), 3);
}

#[rstest]
fn test_iter_size_hint() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
    let iter = collection.iter();

    assert_eq!(iter.size_hint(), (3, Some(3)));
}

#[rstest]
fn test_iter_sorted_size_hint() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);
    let iter = collection.iter_sorted();

    assert_eq!(iter.size_hint(), (3, Some(3)));
}

#[rstest]
fn test_debug_empty() {
    let collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    let debug_str = format!("{:?}", collection);
    assert_eq!(debug_str, "{}");
}

#[rstest]
fn test_debug_small() {
    let collection = OrderedUniqueSet::new().insert(1);
    let debug_str = format!("{:?}", collection);
    assert!(debug_str.contains("1"));
}

#[rstest]
fn test_into_iterator() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);

    let mut items: Vec<i32> = Vec::new();
    for &item in &collection {
        items.push(item);
    }
    items.sort();

    assert_eq!(items, vec![1, 2, 3]);
}

#[rstest]
fn test_encapsulation_only_public_api_available() {
    // The only way to create an OrderedUniqueSet is through the public API
    let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    let with_element = empty.insert(42);
    let removed = with_element.remove(&42);

    // All state transitions happen internally through the API
    assert!(removed.is_empty());
}

// =============================================================================
// Law Tests
// =============================================================================

/// Law: remove_idempotent
/// Removing a non-existent element twice should be idempotent.
/// Equation: remove(remove(s, x), x) = remove(s, x)
#[rstest]
fn test_law_remove_idempotent_small() {
    let collection = OrderedUniqueSet::new().insert(1).insert(2).insert(3);

    let after_first_remove = collection.remove(&99); // 99 doesn't exist
    let after_second_remove = after_first_remove.remove(&99);

    assert_eq!(after_first_remove, after_second_remove);
    // Also verify content is preserved
    assert_eq!(after_first_remove.len(), 3);
    assert!(after_first_remove.contains(&1));
    assert!(after_first_remove.contains(&2));
    assert!(after_first_remove.contains(&3));
}

#[rstest]
fn test_law_remove_idempotent_large() {
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    for i in 1..=20 {
        collection = collection.insert(i);
    }

    let after_first_remove = collection.remove(&99); // 99 doesn't exist
    let after_second_remove = after_first_remove.remove(&99);

    assert_eq!(after_first_remove, after_second_remove);
    assert_eq!(after_first_remove.len(), 20);
    // Verify content is preserved (Large state content verification)
    assert_eq!(after_first_remove, collection);
    for i in 1..=20 {
        assert!(after_first_remove.contains(&i));
    }
}

/// Law: insert_remove_inverse
/// Inserting an element that doesn't exist and then removing it should return the original.
/// Equation: remove(insert(s, x), x) = s (when x is not in s)
#[rstest]
fn test_law_insert_remove_inverse_small() {
    let original = OrderedUniqueSet::new().insert(1).insert(2).insert(3);

    // Insert element that doesn't exist
    let after_insert = original.insert(99);
    assert_eq!(after_insert.len(), 4);
    assert!(after_insert.contains(&99));

    // Remove the inserted element
    let after_remove = after_insert.remove(&99);

    // Should be equal to original
    assert_eq!(after_remove, original);
    assert_eq!(after_remove.len(), 3);
    assert!(after_remove.contains(&1));
    assert!(after_remove.contains(&2));
    assert!(after_remove.contains(&3));
    assert!(!after_remove.contains(&99));
}

#[rstest]
fn test_law_insert_remove_inverse_large() {
    let mut original: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    for i in 1..=20 {
        original = original.insert(i);
    }
    let original_len = original.len();

    // Insert element that doesn't exist
    let after_insert = original.insert(99);
    assert_eq!(after_insert.len(), original_len + 1);

    // Remove the inserted element
    let after_remove = after_insert.remove(&99);

    // Should be equal to original
    assert_eq!(after_remove, original);
    assert_eq!(after_remove.len(), original_len);
}

/// Law: contains_after_insert
/// An inserted element should always be found by contains.
/// Equation: contains(insert(s, x), x) = true
#[rstest]
fn test_law_contains_after_insert() {
    let empty: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    let collection = empty.insert(42);
    assert!(collection.contains(&42));

    // Also test with existing collection
    let collection2 = OrderedUniqueSet::new().insert(1).insert(2);
    let collection2 = collection2.insert(3);
    assert!(collection2.contains(&3));
}

/// Law: sorted_iteration
/// iter_sorted should always return elements in ascending order.
/// Equation: is_sorted(iter_sorted(s)) = true
#[rstest]
fn test_law_sorted_iteration() {
    // Test with various insertion orders
    let collection = OrderedUniqueSet::new()
        .insert(5)
        .insert(1)
        .insert(9)
        .insert(3)
        .insert(7)
        .insert(2)
        .insert(8)
        .insert(4);

    let sorted: Vec<i32> = collection.iter_sorted().copied().collect();
    let mut expected = sorted.clone();
    expected.sort();
    assert_eq!(sorted, expected);
}

#[rstest]
fn test_law_sorted_iteration_large() {
    let mut collection: OrderedUniqueSet<i32> = OrderedUniqueSet::new();
    // Insert in reverse order
    for i in (1..=50).rev() {
        collection = collection.insert(i);
    }

    let sorted: Vec<i32> = collection.iter_sorted().copied().collect();
    let expected: Vec<i32> = (1..=50).collect();
    assert_eq!(sorted, expected);
}
