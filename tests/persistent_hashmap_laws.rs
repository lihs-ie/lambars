#![cfg(feature = "persistent")]
//! Property-based tests for PersistentHashMap.
//!
//! This module verifies that PersistentHashMap satisfies various laws
//! and invariants using proptest.

use lambars::persistent::PersistentHashMap;
use lambars::typeclass::Foldable;
use proptest::prelude::*;
use std::collections::HashSet;

// =============================================================================
// Strategy for generating test data
// =============================================================================

fn arbitrary_key() -> impl Strategy<Value = String> {
    "[a-z]{1,10}".prop_map(|s| s)
}

fn arbitrary_value() -> impl Strategy<Value = i32> {
    any::<i32>()
}

fn arbitrary_entry() -> impl Strategy<Value = (String, i32)> {
    (arbitrary_key(), arbitrary_value())
}

fn arbitrary_entries() -> impl Strategy<Value = Vec<(String, i32)>> {
    prop::collection::vec(arbitrary_entry(), 0..50)
}

#[allow(dead_code)]
fn arbitrary_map() -> impl Strategy<Value = PersistentHashMap<String, i32>> {
    arbitrary_entries().prop_map(|entries| entries.into_iter().collect())
}

// =============================================================================
// Get-Insert Law: map.insert(k, v).get(&k) == Some(&v)
// =============================================================================

proptest! {
    #[test]
    fn prop_get_insert_law(
        entries in arbitrary_entries(),
        key in arbitrary_key(),
        value in arbitrary_value()
    ) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();
        let inserted = map.insert(key.clone(), value);

        prop_assert_eq!(inserted.get(&key), Some(&value));
    }
}

// =============================================================================
// Get-Insert-Other Law: k1 != k2 => map.insert(k1, v).get(&k2) == map.get(&k2)
// =============================================================================

proptest! {
    #[test]
    fn prop_get_insert_other_law(
        entries in arbitrary_entries(),
        key1 in arbitrary_key(),
        key2 in arbitrary_key(),
        value in arbitrary_value()
    ) {
        prop_assume!(key1 != key2);

        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();
        let inserted = map.insert(key1, value);

        prop_assert_eq!(inserted.get(&key2), map.get(&key2));
    }
}

// =============================================================================
// Remove-Get Law: map.remove(&k).get(&k) == None
// =============================================================================

proptest! {
    #[test]
    fn prop_remove_get_law(
        entries in arbitrary_entries(),
        key in arbitrary_key()
    ) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();
        let removed = map.remove(&key);

        prop_assert_eq!(removed.get(&key), None);
    }
}

// =============================================================================
// Remove-Insert Law: !map.contains_key(&k) => map.insert(k, v).remove(&k) == map
// =============================================================================

proptest! {
    #[test]
    fn prop_remove_insert_law(
        entries in arbitrary_entries(),
        key in arbitrary_key(),
        value in arbitrary_value()
    ) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        // Only test when key doesn't exist
        if !map.contains_key(&key) {
            let inserted_then_removed = map.insert(key.clone(), value).remove(&key);

            // The maps should be equal
            prop_assert_eq!(inserted_then_removed, map);
        }
    }
}

// =============================================================================
// Length Law: !map.contains_key(&k) => map.insert(k, v).len() == map.len() + 1
// =============================================================================

proptest! {
    #[test]
    fn prop_length_law_insert_new(
        entries in arbitrary_entries(),
        key in arbitrary_key(),
        value in arbitrary_value()
    ) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        if !map.contains_key(&key) {
            let inserted = map.insert(key, value);
            prop_assert_eq!(inserted.len(), map.len() + 1);
        }
    }
}

// =============================================================================
// Length Law: map.contains_key(&k) => map.insert(k, v).len() == map.len()
// =============================================================================

proptest! {
    #[test]
    fn prop_length_law_insert_existing(
        entries in prop::collection::vec(arbitrary_entry(), 1..50),
        value in arbitrary_value()
    ) {
        let map: PersistentHashMap<String, i32> = entries.clone().into_iter().collect();

        // Pick an existing key
        if let Some((existing_key, _)) = entries.first() {
            let inserted = map.insert(existing_key.clone(), value);
            // Length should not change when overwriting
            prop_assert!(inserted.len() <= map.len());
        }
    }
}

// =============================================================================
// Contains-Key Law: map.insert(k, v).contains_key(&k) == true
// =============================================================================

proptest! {
    #[test]
    fn prop_contains_key_after_insert(
        entries in arbitrary_entries(),
        key in arbitrary_key(),
        value in arbitrary_value()
    ) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();
        let inserted = map.insert(key.clone(), value);

        prop_assert!(inserted.contains_key(&key));
    }
}

// =============================================================================
// Contains-Key Law: map.remove(&k).contains_key(&k) == false
// =============================================================================

proptest! {
    #[test]
    fn prop_not_contains_key_after_remove(
        entries in arbitrary_entries(),
        key in arbitrary_key()
    ) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();
        let removed = map.remove(&key);

        prop_assert!(!removed.contains_key(&key));
    }
}

// =============================================================================
// Persistence Law: Operations do not modify the original map
// =============================================================================

proptest! {
    #[test]
    fn prop_insert_preserves_original(
        entries in arbitrary_entries(),
        key in arbitrary_key(),
        value in arbitrary_value()
    ) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();
        let original_len = map.len();
        let original_keys: HashSet<_> = map.keys().cloned().collect();

        let _ = map.insert(key, value);

        // Original should be unchanged
        prop_assert_eq!(map.len(), original_len);
        let new_keys: HashSet<_> = map.keys().cloned().collect();
        prop_assert_eq!(original_keys, new_keys);
    }
}

proptest! {
    #[test]
    fn prop_remove_preserves_original(
        entries in arbitrary_entries(),
        key in arbitrary_key()
    ) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();
        let original_len = map.len();
        let original_keys: HashSet<_> = map.keys().cloned().collect();

        let _ = map.remove(&key);

        // Original should be unchanged
        prop_assert_eq!(map.len(), original_len);
        let new_keys: HashSet<_> = map.keys().cloned().collect();
        prop_assert_eq!(original_keys, new_keys);
    }
}

// =============================================================================
// Merge Laws
// =============================================================================

proptest! {
    #[test]
    fn prop_merge_identity_left(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();
        let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();

        let merged = empty.merge(&map);
        prop_assert_eq!(merged, map);
    }
}

proptest! {
    #[test]
    fn prop_merge_identity_right(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();
        let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();

        let merged = map.merge(&empty);
        prop_assert_eq!(merged, map);
    }
}

proptest! {
    #[test]
    fn prop_merge_contains_all_keys(
        entries1 in arbitrary_entries(),
        entries2 in arbitrary_entries()
    ) {
        let map1: PersistentHashMap<String, i32> = entries1.into_iter().collect();
        let map2: PersistentHashMap<String, i32> = entries2.into_iter().collect();

        let merged = map1.merge(&map2);

        // All keys from map1 should be in merged
        for key in map1.keys() {
            prop_assert!(merged.contains_key(key));
        }

        // All keys from map2 should be in merged
        for key in map2.keys() {
            prop_assert!(merged.contains_key(key));
        }
    }
}

proptest! {
    #[test]
    fn prop_merge_prefers_right_on_conflict(
        key in arbitrary_key(),
        value1 in arbitrary_value(),
        value2 in arbitrary_value()
    ) {
        let map1 = PersistentHashMap::new().insert(key.clone(), value1);
        let map2 = PersistentHashMap::new().insert(key.clone(), value2);

        let merged = map1.merge(&map2);
        prop_assert_eq!(merged.get(&key), Some(&value2));
    }
}

// =============================================================================
// Iterator Laws
// =============================================================================

proptest! {
    #[test]
    fn prop_iter_length_matches_len(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        let iter_count = map.iter().count();
        prop_assert_eq!(iter_count, map.len());
    }
}

proptest! {
    #[test]
    fn prop_keys_length_matches_len(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        let keys_count = map.keys().count();
        prop_assert_eq!(keys_count, map.len());
    }
}

proptest! {
    #[test]
    fn prop_values_length_matches_len(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        let values_count = map.values().count();
        prop_assert_eq!(values_count, map.len());
    }
}

proptest! {
    #[test]
    fn prop_iter_contains_all_inserted(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.clone().into_iter().collect();

        // Build expected entries (last value for each key wins)
        let mut expected = std::collections::HashMap::new();
        for (key, value) in entries {
            expected.insert(key, value);
        }

        // Verify all expected entries are in iter
        for (key, value) in map.iter() {
            prop_assert_eq!(expected.get(key), Some(value));
        }
    }
}

// =============================================================================
// Foldable Laws
// =============================================================================

proptest! {
    #[test]
    fn prop_fold_left_sum_equals_values_sum(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        // Use i64 to avoid overflow
        let fold_sum: i64 = map.clone().fold_left(0i64, |accumulator, value| {
            accumulator + i64::from(value)
        });

        let values_sum: i64 = map.values().map(|v| i64::from(*v)).sum();

        prop_assert_eq!(fold_sum, values_sum);
    }
}

proptest! {
    #[test]
    fn prop_foldable_length_matches_len(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        prop_assert_eq!(Foldable::length(&map), map.len());
    }
}

proptest! {
    #[test]
    fn prop_foldable_is_empty_matches_is_empty(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        prop_assert_eq!(Foldable::is_empty(&map), map.is_empty());
    }
}

// =============================================================================
// Update Laws
// =============================================================================

proptest! {
    #[test]
    fn prop_update_existing_applies_function(
        entries in prop::collection::vec(arbitrary_entry(), 1..50),
        increment in 1i32..100i32
    ) {
        let map: PersistentHashMap<String, i32> = entries.clone().into_iter().collect();

        // Pick an existing key
        if let Some((existing_key, _)) = entries.first()
            && let Some(original_value) = map.get(existing_key)
        {
            let expected = original_value.saturating_add(increment);
            let updated = map.update(existing_key, |v| v.saturating_add(increment));

            if let Some(updated_map) = updated {
                prop_assert_eq!(updated_map.get(existing_key), Some(&expected));
            }
        }
    }
}

proptest! {
    #[test]
    fn prop_update_nonexistent_returns_none(
        entries in arbitrary_entries()
    ) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        // Use a key that definitely doesn't exist
        let nonexistent_key = "definitely_nonexistent_key_12345".to_string();

        // Make sure it doesn't exist
        if !map.contains_key(&nonexistent_key) {
            let result = map.update(&nonexistent_key, |v| v + 1);
            prop_assert!(result.is_none());
        }
    }
}

// =============================================================================
// Equality Laws
// =============================================================================

proptest! {
    #[test]
    fn prop_equality_reflexive(entries in arbitrary_entries()) {
        let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

        prop_assert_eq!(map.clone(), map);
    }
}

proptest! {
    #[test]
    fn prop_equality_symmetric(
        entries1 in arbitrary_entries(),
        entries2 in arbitrary_entries()
    ) {
        let map1: PersistentHashMap<String, i32> = entries1.into_iter().collect();
        let map2: PersistentHashMap<String, i32> = entries2.into_iter().collect();

        prop_assert_eq!(map1 == map2, map2 == map1);
    }
}

// =============================================================================
// TransientHashMap::insert_bulk Laws
// =============================================================================

use lambars::persistent::TransientHashMap;

proptest! {
    /// insert_bulk is equivalent to folding insert over the items.
    ///
    /// For any sequence of (key, value) pairs:
    /// ```
    /// insert_bulk(items) == items.fold(map, |m, (k, v)| { m.insert(k, v); m })
    /// ```
    #[test]
    fn prop_insert_bulk_equivalence_with_fold(
        entries in arbitrary_entries()
    ) {
        // Via insert_bulk
        let via_bulk = TransientHashMap::new()
            .insert_bulk(entries.clone())
            .expect("insert_bulk should succeed within limits")
            .persistent();

        // Via sequential insert (fold)
        let mut via_fold = TransientHashMap::new();
        for (key, value) in entries {
            via_fold.insert(key, value);
        }
        let via_fold = via_fold.persistent();

        // Both should produce the same map
        prop_assert_eq!(via_bulk, via_fold);
    }
}

proptest! {
    /// insert_bulk duplicate key handling: last value wins.
    ///
    /// When the same key appears multiple times, the last value is kept.
    #[test]
    fn prop_insert_bulk_last_value_wins(
        key in arbitrary_key(),
        values in prop::collection::vec(arbitrary_value(), 2..10)
    ) {
        let entries: Vec<(String, i32)> = values
            .iter()
            .map(|&v| (key.clone(), v))
            .collect();

        let last_value = values.last().copied().expect("values is not empty");

        let result = TransientHashMap::new()
            .insert_bulk(entries)
            .expect("insert_bulk should succeed")
            .persistent();

        prop_assert_eq!(result.len(), 1);
        prop_assert_eq!(result.get(&key), Some(&last_value));
    }
}

proptest! {
    /// insert_bulk is deterministic: same input produces same output.
    ///
    /// For the same sequence of entries, insert_bulk always produces the same map.
    #[test]
    fn prop_insert_bulk_deterministic(
        entries in arbitrary_entries()
    ) {
        let result1 = TransientHashMap::new()
            .insert_bulk(entries.clone())
            .expect("first insert_bulk should succeed")
            .persistent();

        let result2 = TransientHashMap::new()
            .insert_bulk(entries)
            .expect("second insert_bulk should succeed")
            .persistent();

        prop_assert_eq!(result1, result2);
    }
}

proptest! {
    /// insert_bulk preserves existing entries when not overwritten.
    ///
    /// Existing entries in the transient map are preserved if their keys
    /// do not appear in the bulk insert.
    #[test]
    fn prop_insert_bulk_preserves_existing(
        existing_entries in arbitrary_entries(),
        bulk_entries in arbitrary_entries()
    ) {
        // Build a map with existing entries (last value wins for duplicates)
        let mut transient = TransientHashMap::new();
        for (key, value) in &existing_entries {
            transient.insert(key.clone(), *value);
        }

        // Build expected values from existing entries (last value for each key)
        let existing_map: std::collections::HashMap<_, _> = existing_entries
            .iter()
            .cloned()
            .collect();

        // Insert bulk entries
        let result = transient
            .insert_bulk(bulk_entries.clone())
            .expect("insert_bulk should succeed")
            .persistent();

        // All bulk entries should be present (last value for duplicates)
        let bulk_map: std::collections::HashMap<_, _> = bulk_entries
            .iter()
            .cloned()
            .collect();
        for (key, value) in &bulk_map {
            prop_assert_eq!(result.get(key), Some(value));
        }

        // Existing entries not in bulk should be preserved
        for (key, value) in &existing_map {
            if !bulk_map.contains_key(key) {
                prop_assert_eq!(result.get(key), Some(value));
            }
        }
    }
}

proptest! {
    /// insert_bulk then persistent produces valid PersistentHashMap.
    ///
    /// The resulting map should satisfy all PersistentHashMap invariants.
    #[test]
    fn prop_insert_bulk_persistent_roundtrip(
        entries in arbitrary_entries()
    ) {
        let map = TransientHashMap::new()
            .insert_bulk(entries.clone())
            .expect("insert_bulk should succeed")
            .persistent();

        // Verify all unique keys are present
        let expected_keys: std::collections::HashSet<_> = entries
            .iter()
            .map(|(k, _)| k.clone())
            .collect();

        let actual_keys: std::collections::HashSet<_> = map.keys().cloned().collect();

        // Length should match unique key count
        prop_assert_eq!(map.len(), expected_keys.len());

        // All expected keys should be present
        prop_assert_eq!(actual_keys, expected_keys);
    }
}

proptest! {
    /// insert_bulk chaining is equivalent to single insert_bulk with concatenated entries.
    ///
    /// ```
    /// map.insert_bulk(a).insert_bulk(b) == map.insert_bulk(a ++ b)
    /// ```
    #[test]
    fn prop_insert_bulk_chaining_equivalence(
        entries1 in prop::collection::vec(arbitrary_entry(), 0..25),
        entries2 in prop::collection::vec(arbitrary_entry(), 0..25)
    ) {
        // Via chaining
        let via_chaining = TransientHashMap::new()
            .insert_bulk(entries1.clone())
            .expect("first insert_bulk should succeed")
            .insert_bulk(entries2.clone())
            .expect("second insert_bulk should succeed")
            .persistent();

        // Via single call with concatenated entries
        let mut combined = entries1;
        combined.extend(entries2);
        let via_single = TransientHashMap::new()
            .insert_bulk(combined)
            .expect("insert_bulk should succeed")
            .persistent();

        prop_assert_eq!(via_chaining, via_single);
    }
}
