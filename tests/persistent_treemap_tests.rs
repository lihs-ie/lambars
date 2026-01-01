//! Unit tests for PersistentTreeMap.
//!
//! This test file follows TDD methodology - tests are written first,
//! then implementation is added to make them pass.

use lambars::persistent::PersistentTreeMap;
use lambars::typeclass::Foldable;
use rstest::rstest;
use std::ops::Bound;

// =============================================================================
// Basic Construction Tests
// =============================================================================

#[rstest]
fn test_new_creates_empty_map() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[rstest]
fn test_default_creates_empty_map() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::default();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[rstest]
fn test_singleton_creates_map_with_one_entry() {
    let map = PersistentTreeMap::singleton(42, "answer".to_string());
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&42), Some(&"answer".to_string()));
}

// =============================================================================
// Insert and Get Tests
// =============================================================================

#[rstest]
fn test_insert_single_entry() {
    let map = PersistentTreeMap::new().insert(1, "one".to_string());
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&1), Some(&"one".to_string()));
}

#[rstest]
fn test_insert_multiple_entries() {
    let map = PersistentTreeMap::new()
        .insert(2, "two".to_string())
        .insert(1, "one".to_string())
        .insert(3, "three".to_string());

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&1), Some(&"one".to_string()));
    assert_eq!(map.get(&2), Some(&"two".to_string()));
    assert_eq!(map.get(&3), Some(&"three".to_string()));
}

#[rstest]
fn test_insert_overwrites_existing_key() {
    let map1 = PersistentTreeMap::new().insert(1, "one".to_string());
    let map2 = map1.insert(1, "ONE".to_string());

    // Original map is unchanged
    assert_eq!(map1.get(&1), Some(&"one".to_string()));
    // New map has updated value
    assert_eq!(map2.get(&1), Some(&"ONE".to_string()));
    // Length should not change
    assert_eq!(map1.len(), 1);
    assert_eq!(map2.len(), 1);
}

#[rstest]
fn test_insert_preserves_original_map() {
    let map1 = PersistentTreeMap::new().insert(1, "one".to_string());
    let map2 = map1.insert(2, "two".to_string());

    assert_eq!(map1.len(), 1);
    assert_eq!(map2.len(), 2);
    assert_eq!(map1.get(&2), None);
    assert_eq!(map2.get(&2), Some(&"two".to_string()));
}

#[rstest]
fn test_get_nonexistent_key_returns_none() {
    let map = PersistentTreeMap::new().insert(1, "one".to_string());
    assert_eq!(map.get(&2), None);
}

#[rstest]
fn test_get_on_empty_map_returns_none() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    assert_eq!(map.get(&1), None);
}

// =============================================================================
// Contains Key Tests
// =============================================================================

#[rstest]
fn test_contains_key_existing() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    assert!(map.contains_key(&1));
    assert!(map.contains_key(&2));
}

#[rstest]
fn test_contains_key_nonexistent() {
    let map = PersistentTreeMap::new().insert(1, "one".to_string());
    assert!(!map.contains_key(&2));
}

#[rstest]
fn test_contains_key_empty_map() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    assert!(!map.contains_key(&1));
}

// =============================================================================
// Remove Tests
// =============================================================================

#[rstest]
fn test_remove_existing_key() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string());
    let removed = map.remove(&2);

    assert_eq!(removed.len(), 2);
    assert_eq!(removed.get(&2), None);
    assert_eq!(removed.get(&1), Some(&"one".to_string()));
    assert_eq!(removed.get(&3), Some(&"three".to_string()));
}

#[rstest]
fn test_remove_nonexistent_key() {
    let map = PersistentTreeMap::new().insert(1, "one".to_string());
    let removed = map.remove(&99);

    assert_eq!(removed.len(), 1);
    assert_eq!(removed.get(&1), Some(&"one".to_string()));
}

#[rstest]
fn test_remove_preserves_original_map() {
    let map1 = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());
    let map2 = map1.remove(&1);

    // Original unchanged
    assert_eq!(map1.len(), 2);
    assert_eq!(map1.get(&1), Some(&"one".to_string()));
    // New map has key removed
    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&1), None);
}

#[rstest]
fn test_remove_from_empty_map() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    let removed = map.remove(&1);
    assert!(removed.is_empty());
}

#[rstest]
fn test_remove_last_entry() {
    let map = PersistentTreeMap::new().insert(1, "one".to_string());
    let removed = map.remove(&1);

    assert!(removed.is_empty());
    assert_eq!(removed.len(), 0);
}

// =============================================================================
// Min and Max Tests
// =============================================================================

#[rstest]
fn test_min_on_non_empty_map() {
    let map = PersistentTreeMap::new()
        .insert(5, "five".to_string())
        .insert(3, "three".to_string())
        .insert(7, "seven".to_string())
        .insert(1, "one".to_string())
        .insert(9, "nine".to_string());

    let min = map.min();
    assert_eq!(min, Some((&1, &"one".to_string())));
}

#[rstest]
fn test_min_on_empty_map() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    assert_eq!(map.min(), None);
}

#[rstest]
fn test_min_on_singleton() {
    let map = PersistentTreeMap::singleton(42, "answer".to_string());
    assert_eq!(map.min(), Some((&42, &"answer".to_string())));
}

#[rstest]
fn test_max_on_non_empty_map() {
    let map = PersistentTreeMap::new()
        .insert(5, "five".to_string())
        .insert(3, "three".to_string())
        .insert(7, "seven".to_string())
        .insert(1, "one".to_string())
        .insert(9, "nine".to_string());

    let max = map.max();
    assert_eq!(max, Some((&9, &"nine".to_string())));
}

#[rstest]
fn test_max_on_empty_map() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    assert_eq!(map.max(), None);
}

#[rstest]
fn test_max_on_singleton() {
    let map = PersistentTreeMap::singleton(42, "answer".to_string());
    assert_eq!(map.max(), Some((&42, &"answer".to_string())));
}

// =============================================================================
// Iterator Tests
// =============================================================================

#[rstest]
fn test_iter_returns_entries_in_sorted_order() {
    let map = PersistentTreeMap::new()
        .insert(3, "three".to_string())
        .insert(1, "one".to_string())
        .insert(4, "four".to_string())
        .insert(1, "one_updated".to_string()) // Update existing
        .insert(5, "five".to_string())
        .insert(9, "nine".to_string())
        .insert(2, "two".to_string())
        .insert(6, "six".to_string());

    let entries: Vec<(&i32, &String)> = map.iter().collect();
    let keys: Vec<&i32> = entries.iter().map(|(k, _)| *k).collect();

    // Should be sorted by key
    assert_eq!(keys, vec![&1, &2, &3, &4, &5, &6, &9]);
}

#[rstest]
fn test_iter_empty_map() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    let entries: Vec<(&i32, &String)> = map.iter().collect();
    assert!(entries.is_empty());
}

#[rstest]
fn test_keys_iterator() {
    let map = PersistentTreeMap::new()
        .insert(3, "three".to_string())
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    let keys: Vec<&i32> = map.keys().collect();
    assert_eq!(keys, vec![&1, &2, &3]);
}

#[rstest]
fn test_values_iterator() {
    let map = PersistentTreeMap::new()
        .insert(3, "three".to_string())
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    let values: Vec<&String> = map.values().collect();
    // Values should be in key order
    assert_eq!(
        values,
        vec![&"one".to_string(), &"two".to_string(), &"three".to_string()]
    );
}

#[rstest]
fn test_into_iter() {
    let map = PersistentTreeMap::new()
        .insert(2, "two".to_string())
        .insert(1, "one".to_string())
        .insert(3, "three".to_string());

    let entries: Vec<(i32, String)> = map.into_iter().collect();

    assert_eq!(
        entries,
        vec![
            (1, "one".to_string()),
            (2, "two".to_string()),
            (3, "three".to_string())
        ]
    );
}

// =============================================================================
// Range Query Tests
// =============================================================================

#[rstest]
fn test_range_inclusive() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string())
        .insert(4, "four".to_string())
        .insert(5, "five".to_string());

    let range_entries: Vec<(&i32, &String)> = map.range(2..=4).collect();
    let keys: Vec<&i32> = range_entries.iter().map(|(k, _)| *k).collect();

    assert_eq!(keys, vec![&2, &3, &4]);
}

#[rstest]
fn test_range_exclusive() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string())
        .insert(4, "four".to_string())
        .insert(5, "five".to_string());

    let range_entries: Vec<(&i32, &String)> = map.range(2..4).collect();
    let keys: Vec<&i32> = range_entries.iter().map(|(k, _)| *k).collect();

    assert_eq!(keys, vec![&2, &3]);
}

#[rstest]
fn test_range_unbounded_start() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string())
        .insert(4, "four".to_string())
        .insert(5, "five".to_string());

    let range_entries: Vec<(&i32, &String)> = map.range(..3).collect();
    let keys: Vec<&i32> = range_entries.iter().map(|(k, _)| *k).collect();

    assert_eq!(keys, vec![&1, &2]);
}

#[rstest]
fn test_range_unbounded_end() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string())
        .insert(4, "four".to_string())
        .insert(5, "five".to_string());

    let range_entries: Vec<(&i32, &String)> = map.range(3..).collect();
    let keys: Vec<&i32> = range_entries.iter().map(|(k, _)| *k).collect();

    assert_eq!(keys, vec![&3, &4, &5]);
}

#[rstest]
fn test_range_full() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string());

    let range_entries: Vec<(&i32, &String)> = map.range(..).collect();
    let keys: Vec<&i32> = range_entries.iter().map(|(k, _)| *k).collect();

    assert_eq!(keys, vec![&1, &2, &3]);
}

#[rstest]
fn test_range_empty_result() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(5, "five".to_string())
        .insert(10, "ten".to_string());

    let range_entries: Vec<(&i32, &String)> = map.range(2..4).collect();
    assert!(range_entries.is_empty());
}

#[rstest]
fn test_range_on_empty_map() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    let range_entries: Vec<(&i32, &String)> = map.range(1..10).collect();
    assert!(range_entries.is_empty());
}

#[rstest]
fn test_range_with_bound_api() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string())
        .insert(4, "four".to_string())
        .insert(5, "five".to_string());

    // Using explicit Bound types
    let range_entries: Vec<(&i32, &String)> = map
        .range((Bound::Excluded(&1), Bound::Excluded(&4)))
        .collect();
    let keys: Vec<&i32> = range_entries.iter().map(|(k, _)| *k).collect();

    assert_eq!(keys, vec![&2, &3]);
}

// =============================================================================
// FromIterator Tests
// =============================================================================

#[rstest]
fn test_from_iter() {
    let entries = vec![
        (3, "three".to_string()),
        (1, "one".to_string()),
        (2, "two".to_string()),
    ];
    let map: PersistentTreeMap<i32, String> = entries.into_iter().collect();

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&1), Some(&"one".to_string()));
    assert_eq!(map.get(&2), Some(&"two".to_string()));
    assert_eq!(map.get(&3), Some(&"three".to_string()));
}

#[rstest]
fn test_from_iter_empty() {
    let entries: Vec<(i32, String)> = vec![];
    let map: PersistentTreeMap<i32, String> = entries.into_iter().collect();

    assert!(map.is_empty());
}

#[rstest]
fn test_from_iter_with_duplicates() {
    let entries = vec![
        (1, "one".to_string()),
        (1, "ONE".to_string()), // Duplicate key - should keep last
        (2, "two".to_string()),
    ];
    let map: PersistentTreeMap<i32, String> = entries.into_iter().collect();

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&1), Some(&"ONE".to_string())); // Last value wins
}

// =============================================================================
// PartialEq and Eq Tests
// =============================================================================

#[rstest]
fn test_eq_same_entries() {
    let map1 = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());
    let map2 = PersistentTreeMap::new()
        .insert(2, "two".to_string())
        .insert(1, "one".to_string());

    assert_eq!(map1, map2);
}

#[rstest]
fn test_eq_different_values() {
    let map1 = PersistentTreeMap::new().insert(1, "one".to_string());
    let map2 = PersistentTreeMap::new().insert(1, "ONE".to_string());

    assert_ne!(map1, map2);
}

#[rstest]
fn test_eq_different_keys() {
    let map1 = PersistentTreeMap::new().insert(1, "one".to_string());
    let map2 = PersistentTreeMap::new().insert(2, "one".to_string());

    assert_ne!(map1, map2);
}

#[rstest]
fn test_eq_different_sizes() {
    let map1 = PersistentTreeMap::new().insert(1, "one".to_string());
    let map2 = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    assert_ne!(map1, map2);
}

#[rstest]
fn test_eq_empty_maps() {
    let map1: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    let map2: PersistentTreeMap<i32, String> = PersistentTreeMap::new();

    assert_eq!(map1, map2);
}

// =============================================================================
// Debug Tests
// =============================================================================

#[rstest]
fn test_debug_format() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    let debug_string = format!("{:?}", map);
    assert!(debug_string.contains("1"));
    assert!(debug_string.contains("one"));
    assert!(debug_string.contains("2"));
    assert!(debug_string.contains("two"));
}

#[rstest]
fn test_debug_empty_map() {
    let map: PersistentTreeMap<i32, String> = PersistentTreeMap::new();
    let debug_string = format!("{:?}", map);
    assert!(debug_string.contains("{"));
    assert!(debug_string.contains("}"));
}

// =============================================================================
// Clone Tests
// =============================================================================

#[rstest]
fn test_clone() {
    let map1 = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());
    let map2 = map1.clone();

    assert_eq!(map1, map2);

    // Modifying clone doesn't affect original
    let map3 = map2.insert(3, "three".to_string());
    assert_eq!(map1.len(), 2);
    assert_eq!(map3.len(), 3);
}

// =============================================================================
// Foldable Type Class Tests
// =============================================================================

#[rstest]
fn test_foldable_fold_left() {
    let map = PersistentTreeMap::new()
        .insert(1, 10)
        .insert(2, 20)
        .insert(3, 30);

    let sum = map.fold_left(0, |accumulator, value| accumulator + value);
    assert_eq!(sum, 60);
}

#[rstest]
fn test_foldable_fold_right() {
    let map = PersistentTreeMap::new()
        .insert(1, "a".to_string())
        .insert(2, "b".to_string())
        .insert(3, "c".to_string());

    // fold_right processes in key order (since it uses the sorted iterator)
    let result = map.fold_right(String::new(), |value, accumulator| value + &accumulator);
    // Keys are 1, 2, 3, so values are "a", "b", "c"
    // fold_right: f("a", f("b", f("c", ""))) = f("a", f("b", "c")) = f("a", "bc") = "abc"
    assert_eq!(result, "abc");
}

#[rstest]
fn test_foldable_is_empty() {
    let empty: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
    let non_empty = PersistentTreeMap::singleton(1, 10);

    assert!(empty.is_empty());
    assert!(!non_empty.is_empty());
}

#[rstest]
fn test_foldable_length() {
    let empty: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
    let map = PersistentTreeMap::new().insert(1, 10).insert(2, 20);

    assert_eq!(Foldable::length(&empty), 0);
    assert_eq!(Foldable::length(&map), 2);
}

// =============================================================================
// Large Scale Tests
// =============================================================================

#[rstest]
fn test_large_number_of_entries() {
    let mut map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();

    for i in 0..1000 {
        map = map.insert(i, i * 10);
    }

    assert_eq!(map.len(), 1000);

    for i in 0..1000 {
        assert_eq!(map.get(&i), Some(&(i * 10)));
    }
}

#[rstest]
fn test_large_number_of_entries_in_reverse_order() {
    let mut map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();

    for i in (0..1000).rev() {
        map = map.insert(i, i * 10);
    }

    assert_eq!(map.len(), 1000);

    // Verify iteration is still in sorted order
    let keys: Vec<&i32> = map.keys().collect();
    for (index, key) in keys.iter().enumerate() {
        assert_eq!(**key, index as i32);
    }
}

#[rstest]
fn test_many_insertions_and_deletions() {
    let mut map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();

    // Insert 500 entries
    for i in 0..500 {
        map = map.insert(i, i);
    }

    // Remove even entries
    for i in (0..500).step_by(2) {
        map = map.remove(&i);
    }

    assert_eq!(map.len(), 250);

    // Verify only odd entries remain
    for i in 0..500 {
        if i % 2 == 0 {
            assert_eq!(map.get(&i), None);
        } else {
            assert_eq!(map.get(&i), Some(&i));
        }
    }
}

// =============================================================================
// Borrow Pattern Tests
// =============================================================================

#[rstest]
fn test_get_with_borrow() {
    let map = PersistentTreeMap::new().insert("hello".to_string(), 42);

    // Can use &str to look up String key
    assert_eq!(map.get("hello"), Some(&42));
}

#[rstest]
fn test_contains_key_with_borrow() {
    let map = PersistentTreeMap::new().insert("hello".to_string(), 42);

    assert!(map.contains_key("hello"));
    assert!(!map.contains_key("world"));
}

#[rstest]
fn test_remove_with_borrow() {
    let map = PersistentTreeMap::new()
        .insert("hello".to_string(), 42)
        .insert("world".to_string(), 100);

    let removed = map.remove("hello");

    assert_eq!(removed.len(), 1);
    assert_eq!(removed.get("hello"), None);
    assert_eq!(removed.get("world"), Some(&100));
}

// =============================================================================
// Structural Sharing Tests
// =============================================================================

#[rstest]
fn test_structural_sharing_on_insert() {
    // This test verifies that insert creates a new map that shares structure
    // with the original
    let map1 = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string());

    let map2 = map1.insert(4, "four".to_string());

    // Both maps should be valid and independent
    assert_eq!(map1.len(), 3);
    assert_eq!(map2.len(), 4);

    // Original unmodified
    assert_eq!(map1.get(&4), None);
    // New map has the insertion
    assert_eq!(map2.get(&4), Some(&"four".to_string()));

    // Both share the common entries
    assert_eq!(map1.get(&1), map2.get(&1));
    assert_eq!(map1.get(&2), map2.get(&2));
    assert_eq!(map1.get(&3), map2.get(&3));
}

#[rstest]
fn test_many_versions_from_same_base() {
    let base = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    // Create many versions from the same base
    let versions: Vec<PersistentTreeMap<i32, String>> = (3..103)
        .map(|i| base.insert(i, format!("value_{}", i)))
        .collect();

    // All versions should be valid
    for (index, version) in versions.iter().enumerate() {
        let key = (index + 3) as i32;
        assert_eq!(version.len(), 3);
        assert_eq!(version.get(&key), Some(&format!("value_{}", key)));
        // Should also have base entries
        assert_eq!(version.get(&1), Some(&"one".to_string()));
        assert_eq!(version.get(&2), Some(&"two".to_string()));
    }

    // Base should be unchanged
    assert_eq!(base.len(), 2);
}

// =============================================================================
// Coverage Tests: Iterator
// =============================================================================

#[rstest]
fn test_iter_size_hint() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string());

    let iter = map.iter();
    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 3);
    assert_eq!(upper, Some(3));
}

#[rstest]
fn test_iter_exact_size() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    let iter = map.iter();
    assert_eq!(iter.len(), 2);
}

#[rstest]
fn test_iter_after_partial_consumption() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string());

    let mut iter = map.iter();
    iter.next(); // Consume one element

    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 2);
    assert_eq!(upper, Some(2));
    assert_eq!(iter.len(), 2);
}

#[rstest]
fn test_into_iter_size_hint() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    let iter = map.into_iter();
    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 2);
    assert_eq!(upper, Some(2));
}

#[rstest]
fn test_into_iter_exact_size() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    let iter = map.into_iter();
    assert_eq!(iter.len(), 2);
}

#[rstest]
fn test_into_iter_after_partial_consumption() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string());

    let mut iter = map.into_iter();
    iter.next(); // Consume one element

    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 2);
    assert_eq!(upper, Some(2));
    assert_eq!(iter.len(), 2);
}

#[rstest]
fn test_ref_into_iterator() {
    let map = PersistentTreeMap::new()
        .insert(1, 10)
        .insert(2, 20)
        .insert(3, 30);

    let mut sum = 0;
    for (_, value) in &map {
        sum += value;
    }
    assert_eq!(sum, 60);
}

// =============================================================================
// Coverage Tests: Foldable additional methods
// =============================================================================

#[rstest]
fn test_foldable_find() {
    let map = PersistentTreeMap::new()
        .insert(1, 10)
        .insert(2, 25)
        .insert(3, 30);

    let found = map.find(|value| *value > 20);
    assert!(found.is_some());
    assert!(found.unwrap() > 20);
}

#[rstest]
fn test_foldable_find_not_found() {
    let map = PersistentTreeMap::new()
        .insert(1, 10)
        .insert(2, 20)
        .insert(3, 30);

    let found = map.find(|value| *value > 100);
    assert!(found.is_none());
}

#[rstest]
fn test_foldable_exists() {
    let map = PersistentTreeMap::new()
        .insert(1, 10)
        .insert(2, 20)
        .insert(3, 30);

    assert!(map.exists(|value| *value == 20));
    assert!(!map.exists(|value| *value == 100));
}

#[rstest]
fn test_foldable_for_all() {
    let map = PersistentTreeMap::new()
        .insert(1, 10)
        .insert(2, 20)
        .insert(3, 30);

    assert!(map.for_all(|value| *value > 0));
    assert!(!map.for_all(|value| *value > 15));
}

// =============================================================================
// Coverage Tests: Range query edge cases
// =============================================================================

#[rstest]
fn test_range_single_element() {
    let map = PersistentTreeMap::singleton(5, "five".to_string());
    let range_entries: Vec<_> = map.range(5..=5).collect();
    assert_eq!(range_entries.len(), 1);
    assert_eq!(range_entries[0], (&5, &"five".to_string()));
}

#[rstest]
fn test_range_no_elements_in_range() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(10, "ten".to_string());

    let range_entries: Vec<_> = map.range(5..8).collect();
    assert!(range_entries.is_empty());
}

#[rstest]
fn test_range_all_elements_in_range() {
    let map = PersistentTreeMap::new()
        .insert(2, "two".to_string())
        .insert(3, "three".to_string())
        .insert(4, "four".to_string());

    let range_entries: Vec<_> = map.range(1..10).collect();
    assert_eq!(range_entries.len(), 3);
}

// =============================================================================
// Coverage Tests: Remove edge cases
// =============================================================================

#[rstest]
fn test_remove_root_with_two_children() {
    // Create a tree where removing the root requires rebalancing
    let map = PersistentTreeMap::new()
        .insert(5, "five".to_string())
        .insert(3, "three".to_string())
        .insert(7, "seven".to_string())
        .insert(2, "two".to_string())
        .insert(4, "four".to_string())
        .insert(6, "six".to_string())
        .insert(8, "eight".to_string());

    let removed = map.remove(&5);
    assert_eq!(removed.len(), 6);
    assert!(removed.get(&5).is_none());

    // Verify tree integrity
    let keys: Vec<_> = removed.keys().copied().collect();
    assert_eq!(keys, vec![2, 3, 4, 6, 7, 8]);
}

#[rstest]
fn test_remove_leaf_node() {
    let map = PersistentTreeMap::new()
        .insert(5, "five".to_string())
        .insert(3, "three".to_string())
        .insert(7, "seven".to_string());

    let removed = map.remove(&3);
    assert_eq!(removed.len(), 2);
    assert!(removed.get(&3).is_none());
    assert_eq!(removed.get(&5), Some(&"five".to_string()));
    assert_eq!(removed.get(&7), Some(&"seven".to_string()));
}

#[rstest]
fn test_remove_node_with_one_child() {
    let map = PersistentTreeMap::new()
        .insert(5, "five".to_string())
        .insert(3, "three".to_string())
        .insert(7, "seven".to_string())
        .insert(2, "two".to_string());

    let removed = map.remove(&3);
    assert_eq!(removed.len(), 3);
    assert!(removed.get(&3).is_none());
    assert_eq!(removed.get(&2), Some(&"two".to_string()));
}

// =============================================================================
// Coverage Tests: Red-Black tree balancing
// =============================================================================

#[rstest]
fn test_insert_triggers_rebalance() {
    // Insert in ascending order to trigger rotations
    let mut map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
    for index in 1..=10 {
        map = map.insert(index, index * 10);
    }

    assert_eq!(map.len(), 10);
    for index in 1..=10 {
        assert_eq!(map.get(&index), Some(&(index * 10)));
    }
}

#[rstest]
fn test_insert_descending_triggers_rebalance() {
    // Insert in descending order to trigger rotations
    let mut map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
    for index in (1..=10).rev() {
        map = map.insert(index, index * 10);
    }

    assert_eq!(map.len(), 10);

    // Verify iteration order is still correct
    let keys: Vec<_> = map.keys().copied().collect();
    assert_eq!(keys, (1..=10).collect::<Vec<_>>());
}

#[rstest]
fn test_alternating_insert() {
    // Insert in an alternating pattern
    let map = PersistentTreeMap::new()
        .insert(5, "five".to_string())
        .insert(1, "one".to_string())
        .insert(9, "nine".to_string())
        .insert(3, "three".to_string())
        .insert(7, "seven".to_string())
        .insert(2, "two".to_string())
        .insert(8, "eight".to_string())
        .insert(4, "four".to_string())
        .insert(6, "six".to_string());

    assert_eq!(map.len(), 9);
    let keys: Vec<_> = map.keys().copied().collect();
    assert_eq!(keys, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

// =============================================================================
// Coverage Tests: Min/Max after modifications
// =============================================================================

#[rstest]
fn test_min_after_remove_min() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string());

    let removed = map.remove(&1);
    assert_eq!(removed.min(), Some((&2, &"two".to_string())));
}

#[rstest]
fn test_max_after_remove_max() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string());

    let removed = map.remove(&3);
    assert_eq!(removed.max(), Some((&2, &"two".to_string())));
}

// =============================================================================
// Coverage Tests: Large tree operations
// =============================================================================

#[rstest]
fn test_large_tree_random_access() {
    let mut map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
    for index in 0..1000 {
        map = map.insert(index, index * 2);
    }

    // Random access checks
    assert_eq!(map.get(&0), Some(&0));
    assert_eq!(map.get(&500), Some(&1000));
    assert_eq!(map.get(&999), Some(&1998));
    assert_eq!(map.get(&1000), None);
}

#[rstest]
fn test_large_tree_range_query() {
    let mut map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
    for index in 0..1000 {
        map = map.insert(index, index);
    }

    let range: Vec<_> = map.range(100..200).collect();
    assert_eq!(range.len(), 100);

    for (index, (key, value)) in range.iter().enumerate() {
        assert_eq!(**key, (index + 100) as i32);
        assert_eq!(**value, (index + 100) as i32);
    }
}

// =============================================================================
// Coverage Tests: Empty tree operations
// =============================================================================

#[rstest]
fn test_empty_tree_min_max() {
    let empty: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
    assert!(empty.min().is_none());
    assert!(empty.max().is_none());
}

#[rstest]
fn test_empty_tree_range() {
    let empty: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();
    let range: Vec<_> = empty.range(0..100).collect();
    assert!(range.is_empty());
}

// =============================================================================
// Coverage Tests: Update via insert
// =============================================================================

#[rstest]
fn test_update_same_key_multiple_times() {
    let map = PersistentTreeMap::new()
        .insert(1, "first".to_string())
        .insert(1, "second".to_string())
        .insert(1, "third".to_string());

    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&1), Some(&"third".to_string()));
}

// =============================================================================
// Coverage Tests: Clone behavior
// =============================================================================

#[rstest]
fn test_clone_independence() {
    let map1 = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string());

    let map2 = map1.clone();
    let map3 = map2.insert(3, "three".to_string());

    assert_eq!(map1.len(), 2);
    assert_eq!(map2.len(), 2);
    assert_eq!(map3.len(), 3);
}

// =============================================================================
// Coverage Tests: Debug format edge cases
// =============================================================================

#[rstest]
fn test_debug_single_entry() {
    let map = PersistentTreeMap::singleton(42, "answer".to_string());
    let debug_string = format!("{:?}", map);
    assert!(debug_string.contains("42"));
    assert!(debug_string.contains("answer"));
}

// =============================================================================
// Coverage Tests: Range with Bound API
// =============================================================================

#[rstest]
fn test_range_excluded_start() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string())
        .insert(4, "four".to_string())
        .insert(5, "five".to_string());

    let range: Vec<_> = map
        .range((Bound::Excluded(&1), Bound::Included(&4)))
        .collect();
    let keys: Vec<_> = range.iter().map(|(k, _)| **k).collect();
    assert_eq!(keys, vec![2, 3, 4]);
}

#[rstest]
fn test_range_excluded_end() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string())
        .insert(4, "four".to_string())
        .insert(5, "five".to_string());

    let range: Vec<_> = map
        .range((Bound::Included(&2), Bound::Excluded(&5)))
        .collect();
    let keys: Vec<_> = range.iter().map(|(k, _)| **k).collect();
    assert_eq!(keys, vec![2, 3, 4]);
}

#[rstest]
fn test_range_both_excluded() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string())
        .insert(4, "four".to_string())
        .insert(5, "five".to_string());

    let range: Vec<_> = map
        .range((Bound::Excluded(&1), Bound::Excluded(&5)))
        .collect();
    let keys: Vec<_> = range.iter().map(|(k, _)| **k).collect();
    assert_eq!(keys, vec![2, 3, 4]);
}

#[rstest]
fn test_range_both_unbounded() {
    let map = PersistentTreeMap::new()
        .insert(1, "one".to_string())
        .insert(2, "two".to_string())
        .insert(3, "three".to_string());

    let range: Vec<_> = map
        .range::<(Bound<&i32>, Bound<&i32>), i32>((Bound::Unbounded, Bound::Unbounded))
        .collect();
    assert_eq!(range.len(), 3);
}
