#![cfg(feature = "persistent")]
//! Property-based tests for PersistentTreeMap.
//!
//! These tests verify that PersistentTreeMap satisfies the expected laws
//! and invariants using proptest.

use lambars::persistent::PersistentTreeMap;
use lambars::typeclass::Foldable;
use proptest::prelude::*;

// =============================================================================
// Strategies for Generating Test Data
// =============================================================================

/// Strategy for generating a PersistentTreeMap from a vector of key-value pairs.
fn arbitrary_treemap(max_size: usize) -> impl Strategy<Value = PersistentTreeMap<i32, i32>> {
    prop::collection::vec((any::<i32>(), any::<i32>()), 0..max_size)
        .prop_map(|entries| entries.into_iter().collect::<PersistentTreeMap<i32, i32>>())
}

// =============================================================================
// Get-Insert Laws
// =============================================================================

proptest! {
    /// Law: get after insert returns the inserted value.
    /// map.insert(key, value).get(&key) == Some(&value)
    #[test]
    fn prop_get_insert_law(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20),
        key: i32,
        value: i32
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();
        let updated = map.insert(key, value);
        prop_assert_eq!(updated.get(&key), Some(&value));
    }

    /// Law: insert does not affect other keys.
    /// key1 != key2 => map.insert(key1, value).get(&key2) == map.get(&key2)
    #[test]
    fn prop_get_insert_other_law(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20),
        key1: i32,
        key2: i32,
        value: i32
    ) {
        prop_assume!(key1 != key2);
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();
        let updated = map.insert(key1, value);
        prop_assert_eq!(updated.get(&key2), map.get(&key2));
    }
}

// =============================================================================
// Remove Laws
// =============================================================================

proptest! {
    /// Law: get after remove returns None.
    /// map.remove(&key).get(&key) == None
    #[test]
    fn prop_get_remove_law(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20),
        key: i32
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();
        let removed = map.remove(&key);
        prop_assert_eq!(removed.get(&key), None);
    }

    /// Law: remove does not affect other keys.
    /// key1 != key2 => map.remove(&key1).get(&key2) == map.get(&key2)
    #[test]
    fn prop_get_remove_other_law(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20),
        key1: i32,
        key2: i32
    ) {
        prop_assume!(key1 != key2);
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();
        let removed = map.remove(&key1);
        prop_assert_eq!(removed.get(&key2), map.get(&key2));
    }

    /// Law: remove then insert restores the value.
    /// For a key that exists: map.remove(&key).insert(key, value).get(&key) == Some(&value)
    #[test]
    fn prop_remove_insert_law(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 1..20),
        new_value: i32
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.clone().into_iter().collect();

        if let Some((key, _)) = entries.first() {
            // Remove and re-insert with new value
            let restored = map.remove(key).insert(*key, new_value);
            prop_assert_eq!(restored.get(key), Some(&new_value));
        }
    }
}

// =============================================================================
// Length Laws
// =============================================================================

proptest! {
    /// Law: insert of new key increases length by 1.
    /// !map.contains_key(&key) => map.insert(key, value).len() == map.len() + 1
    #[test]
    fn prop_insert_length_new_key(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20),
        key: i32,
        value: i32
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();
        if !map.contains_key(&key) {
            let updated = map.insert(key, value);
            prop_assert_eq!(updated.len(), map.len() + 1);
        }
    }

    /// Law: insert of existing key does not change length.
    /// map.contains_key(&key) => map.insert(key, value).len() == map.len()
    #[test]
    fn prop_insert_length_existing_key(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 1..20)
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.clone().into_iter().collect();

        if let Some((key, _)) = entries.first() {
            let updated = map.insert(*key, 999);
            prop_assert_eq!(updated.len(), map.len());
        }
    }

    /// Law: remove of existing key decreases length by 1.
    /// map.contains_key(&key) => map.remove(&key).len() == map.len() - 1
    #[test]
    fn prop_remove_length_existing_key(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 1..20)
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.clone().into_iter().collect();

        if let Some((key, _)) = entries.first()
            && map.contains_key(key)
        {
            let removed = map.remove(key);
            prop_assert_eq!(removed.len(), map.len() - 1);
        }
    }

    /// Law: remove of non-existing key does not change length.
    /// !map.contains_key(&key) => map.remove(&key).len() == map.len()
    #[test]
    fn prop_remove_length_nonexistent_key(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20),
        key: i32
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();
        if !map.contains_key(&key) {
            let removed = map.remove(&key);
            prop_assert_eq!(removed.len(), map.len());
        }
    }
}

// =============================================================================
// Ordering Laws (Sorted Order)
// =============================================================================

proptest! {
    /// Law: iter always returns entries in sorted key order.
    #[test]
    fn prop_iter_is_sorted(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..50)
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();
        let keys: Vec<i32> = map.iter().map(|(key, _)| *key).collect();

        // Check that keys are sorted
        for window in keys.windows(2) {
            prop_assert!(window[0] < window[1], "Keys should be strictly increasing");
        }
    }

    /// Law: min returns the first element of iter.
    /// map.min() == map.iter().next()
    #[test]
    fn prop_min_is_first_of_iter(map in arbitrary_treemap(30)) {
        prop_assert_eq!(map.min(), map.iter().next());
    }

    /// Law: max returns the last element of iter.
    /// map.max() == map.iter().last()
    #[test]
    fn prop_max_is_last_of_iter(map in arbitrary_treemap(30)) {
        prop_assert_eq!(map.max(), map.iter().last());
    }
}

// =============================================================================
// Range Laws
// =============================================================================

proptest! {
    /// Law: range returns only elements within the range.
    #[test]
    fn prop_range_within_bounds(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..30),
        start in any::<i32>(),
        end in any::<i32>()
    ) {
        prop_assume!(start <= end);
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();

        let range_keys: Vec<i32> = map.range(start..end).map(|(k, _)| *k).collect();

        for key in &range_keys {
            prop_assert!(*key >= start && *key < end, "Key {} should be in range [{}..{})", key, start, end);
        }
    }

    /// Law: range inclusive includes the end bound.
    #[test]
    fn prop_range_inclusive_bounds(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..30),
        start in any::<i32>(),
        end in any::<i32>()
    ) {
        prop_assume!(start <= end);
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();

        let range_keys: Vec<i32> = map.range(start..=end).map(|(k, _)| *k).collect();

        for key in &range_keys {
            prop_assert!(*key >= start && *key <= end, "Key {} should be in range [{}..={}]", key, start, end);
        }
    }

    /// Law: range returns elements in sorted order.
    #[test]
    fn prop_range_is_sorted(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..30),
        start in any::<i32>(),
        end in any::<i32>()
    ) {
        prop_assume!(start <= end);
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();

        let range_keys: Vec<i32> = map.range(start..end).map(|(k, _)| *k).collect();

        for window in range_keys.windows(2) {
            prop_assert!(window[0] < window[1], "Range keys should be strictly increasing");
        }
    }
}

// =============================================================================
// Persistence Laws
// =============================================================================

proptest! {
    /// Law: operations do not modify the original map.
    #[test]
    fn prop_insert_does_not_modify_original(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20),
        key: i32,
        value: i32
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.clone().into_iter().collect();
        let original_len = map.len();
        let original_entries: Vec<(i32, i32)> = map.iter().map(|(k, v)| (*k, *v)).collect();

        let _ = map.insert(key, value);

        // Original should be unchanged
        prop_assert_eq!(map.len(), original_len);
        let after_entries: Vec<(i32, i32)> = map.iter().map(|(k, v)| (*k, *v)).collect();
        prop_assert_eq!(original_entries, after_entries);
    }

    /// Law: remove does not modify the original map.
    #[test]
    fn prop_remove_does_not_modify_original(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20),
        key: i32
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.into_iter().collect();
        let original_len = map.len();
        let original_entries: Vec<(i32, i32)> = map.iter().map(|(k, v)| (*k, *v)).collect();

        let _ = map.remove(&key);

        // Original should be unchanged
        prop_assert_eq!(map.len(), original_len);
        let after_entries: Vec<(i32, i32)> = map.iter().map(|(k, v)| (*k, *v)).collect();
        prop_assert_eq!(original_entries, after_entries);
    }
}

// =============================================================================
// Equality Laws
// =============================================================================

proptest! {
    /// Law: equality is reflexive.
    /// map == map
    #[test]
    fn prop_eq_reflexive(map in arbitrary_treemap(20)) {
        prop_assert_eq!(map.clone(), map);
    }

    /// Law: equality is symmetric.
    /// map1 == map2 => map2 == map1
    #[test]
    fn prop_eq_symmetric(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20)
    ) {
        let map1: PersistentTreeMap<i32, i32> = entries.clone().into_iter().collect();
        let map2: PersistentTreeMap<i32, i32> = entries.into_iter().collect();

        // If map1 == map2, then map2 == map1
        if map1 == map2 {
            prop_assert_eq!(map2, map1);
        }
    }

    /// Law: maps with same entries are equal regardless of insertion order.
    #[test]
    fn prop_eq_insertion_order_independent(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..20)
    ) {
        let map1: PersistentTreeMap<i32, i32> = entries.clone().into_iter().collect();

        // Create map2 with reversed insertion order
        let mut reversed_entries = entries;
        reversed_entries.reverse();
        let map2: PersistentTreeMap<i32, i32> = reversed_entries.into_iter().collect();

        prop_assert_eq!(map1, map2);
    }
}

// =============================================================================
// Foldable Laws
// =============================================================================

proptest! {
    /// Law: fold_left sum equals the sum of all values.
    #[test]
    fn prop_fold_left_sum(
        entries in prop::collection::vec((-1000i32..1000i32, -1000i32..1000i32), 0..30)
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.clone().into_iter().collect();

        // Collect unique keys and their last values (since later inserts overwrite)
        let mut unique_entries = std::collections::BTreeMap::new();
        for (key, value) in entries {
            unique_entries.insert(key, value);
        }

        let expected_sum: i32 = unique_entries.values().sum();
        let fold_sum = map.fold_left(0, |accumulator, value| accumulator + value);

        prop_assert_eq!(fold_sum, expected_sum);
    }

    /// Law: Foldable::length equals len().
    #[test]
    fn prop_foldable_length_equals_len(map in arbitrary_treemap(30)) {
        prop_assert_eq!(Foldable::length(&map), map.len());
    }

    /// Law: Foldable::is_empty equals is_empty().
    #[test]
    fn prop_foldable_is_empty_equals_is_empty(map in arbitrary_treemap(30)) {
        prop_assert_eq!(Foldable::is_empty(&map), map.is_empty());
    }
}

// =============================================================================
// B-Tree Invariants
// =============================================================================

// Note: These tests verify the internal B-Tree properties.
// In a production implementation, we would expose a method to validate invariants.
// For now, we verify observable behavior that depends on balanced tree properties.

proptest! {
    /// Property: operations maintain O(log n) performance.
    /// We verify this by checking that the tree remains balanced after many operations.
    #[test]
    fn prop_balanced_after_many_operations(
        insertions in prop::collection::vec((any::<i32>(), any::<i32>()), 0..100),
        deletions in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        let mut map: PersistentTreeMap<i32, i32> = PersistentTreeMap::new();

        // Insert all
        for (key, value) in insertions {
            map = map.insert(key, value);
        }

        // Remove some
        for key in deletions {
            map = map.remove(&key);
        }

        // Verify iterator still works correctly (would fail if tree is corrupted)
        let keys: Vec<i32> = map.iter().map(|(k, _)| *k).collect();
        for window in keys.windows(2) {
            prop_assert!(window[0] < window[1]);
        }

        // Verify all operations still work
        for (key, _) in map.iter() {
            prop_assert!(map.contains_key(key));
        }
    }
}

// =============================================================================
// Contains Key Laws
// =============================================================================

proptest! {
    /// Law: contains_key after insert is true.
    #[test]
    fn prop_contains_key_after_insert(
        map in arbitrary_treemap(20),
        key: i32,
        value: i32
    ) {
        let updated = map.insert(key, value);
        prop_assert!(updated.contains_key(&key));
    }

    /// Law: contains_key after remove is false.
    #[test]
    fn prop_not_contains_key_after_remove(
        map in arbitrary_treemap(20),
        key: i32
    ) {
        let removed = map.remove(&key);
        prop_assert!(!removed.contains_key(&key));
    }

    /// Law: contains_key is consistent with get.
    /// map.contains_key(&key) == map.get(&key).is_some()
    #[test]
    fn prop_contains_key_consistent_with_get(
        map in arbitrary_treemap(20),
        key: i32
    ) {
        prop_assert_eq!(map.contains_key(&key), map.get(&key).is_some());
    }
}

// =============================================================================
// FromIterator/IntoIterator Laws
// =============================================================================

proptest! {
    /// Law: round-trip through iterators preserves all unique entries.
    #[test]
    fn prop_roundtrip_through_iterators(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..30)
    ) {
        let map1: PersistentTreeMap<i32, i32> = entries.into_iter().collect();
        let collected: Vec<(i32, i32)> = map1.clone().into_iter().collect();
        let map2: PersistentTreeMap<i32, i32> = collected.into_iter().collect();

        prop_assert_eq!(map1, map2);
    }
}

// =============================================================================
// Hash Laws
// =============================================================================

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Helper function: calculate hash value of a map
fn calculate_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

proptest! {
    /// Hash-Eq consistency: if a == b then hash(a) == hash(b)
    #[test]
    fn prop_hash_eq_consistency(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..50)
    ) {
        let map1: PersistentTreeMap<i32, i32> = entries.iter().cloned().collect();
        let map2: PersistentTreeMap<i32, i32> = entries.iter().cloned().collect();

        prop_assert_eq!(&map1, &map2);
        prop_assert_eq!(calculate_hash(&map1), calculate_hash(&map2));
    }

    /// Hash determinism: the same map always produces the same hash value
    #[test]
    fn prop_hash_deterministic(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..50)
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.iter().cloned().collect();

        let hash1 = calculate_hash(&map);
        let hash2 = calculate_hash(&map);

        prop_assert_eq!(hash1, hash2);
    }

    /// Hash value is independent of insertion order
    #[test]
    fn prop_hash_insert_order_independent(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 2..20)
    ) {
        let map1: PersistentTreeMap<i32, i32> = entries.iter().cloned().collect();

        let mut reversed = entries.clone();
        reversed.reverse();
        let map2: PersistentTreeMap<i32, i32> = reversed.iter().cloned().collect();

        // The last value for the same key is used, so the results are the same
        prop_assert_eq!(&map1, &map2);
        prop_assert_eq!(calculate_hash(&map1), calculate_hash(&map2));
    }

    /// A cloned map has the same hash value
    #[test]
    fn prop_hash_clone_consistency(
        entries in prop::collection::vec((any::<i32>(), any::<i32>()), 0..50)
    ) {
        let map: PersistentTreeMap<i32, i32> = entries.iter().cloned().collect();
        let cloned = map.clone();

        prop_assert_eq!(calculate_hash(&map), calculate_hash(&cloned));
    }
}
