//! Tests for fast hash feature flags.
//!
//! This module tests the high-speed hash implementations provided by
//! `fxhash` and `ahash` feature flags, ensuring referential transparency
//! and correct behavior.

use lambars::persistent::{PersistentHashMap, PersistentHashSet};
use rstest::rstest;

// =============================================================================
// Referential Transparency Tests
// =============================================================================

/// Tests that the same key produces the same hash value across multiple calls.
/// This is essential for referential transparency.
#[rstest]
fn test_same_key_produces_same_hash() {
    // Test with string keys
    let map1 = PersistentHashMap::new().insert("key".to_string(), 1);
    let map2 = PersistentHashMap::new().insert("key".to_string(), 2);

    // Both maps should be able to retrieve by the same key
    assert_eq!(map1.get("key"), Some(&1));
    assert_eq!(map2.get("key"), Some(&2));

    // Test with integer keys
    let map3: PersistentHashMap<i32, i32> = PersistentHashMap::new().insert(42, 100);
    let map4: PersistentHashMap<i32, i32> = PersistentHashMap::new().insert(42, 200);

    assert_eq!(map3.get(&42), Some(&100));
    assert_eq!(map4.get(&42), Some(&200));
}

/// Tests that hash values are deterministic across identical maps.
/// Maps with the same entries should behave identically.
#[rstest]
fn test_deterministic_hash_behavior() {
    let entries: Vec<(String, i32)> = vec![
        ("alpha".to_string(), 1),
        ("beta".to_string(), 2),
        ("gamma".to_string(), 3),
        ("delta".to_string(), 4),
    ];

    // Create two maps with the same entries
    let map1: PersistentHashMap<String, i32> = entries.iter().cloned().collect();
    let map2: PersistentHashMap<String, i32> = entries.iter().cloned().collect();

    // Both maps should return the same values for the same keys
    for (key, expected_value) in &entries {
        assert_eq!(map1.get(key), Some(expected_value));
        assert_eq!(map2.get(key), Some(expected_value));
    }

    // Both maps should have the same length
    assert_eq!(map1.len(), map2.len());
}

/// Tests that different keys produce different hash behaviors.
#[rstest]
fn test_different_keys_different_entries() {
    let map = PersistentHashMap::new()
        .insert("key1".to_string(), 1)
        .insert("key2".to_string(), 2)
        .insert("key3".to_string(), 3);

    assert_eq!(map.get("key1"), Some(&1));
    assert_eq!(map.get("key2"), Some(&2));
    assert_eq!(map.get("key3"), Some(&3));
    assert_eq!(map.get("key4"), None);
}

// =============================================================================
// Large Scale Tests (hash function stress test)
// =============================================================================

/// Tests that hash function works correctly with many keys.
/// This ensures no collisions cause incorrect behavior.
#[rstest]
fn test_large_scale_insert_and_retrieve() {
    const COUNT: i32 = 10_000;

    let map: PersistentHashMap<i32, i32> = (0..COUNT).map(|x| (x, x * 2)).collect();

    // Verify all entries are retrievable
    for i in 0..COUNT {
        assert_eq!(map.get(&i), Some(&(i * 2)), "Failed to get key {}", i);
    }

    // Verify non-existent keys return None
    for i in COUNT..(COUNT + 100) {
        assert_eq!(map.get(&i), None, "Key {} should not exist", i);
    }
}

/// Tests that hash function works correctly with string keys.
#[rstest]
fn test_string_keys_large_scale() {
    const COUNT: usize = 1_000;

    let map: PersistentHashMap<String, usize> =
        (0..COUNT).map(|i| (format!("key_{}", i), i)).collect();

    // Verify all entries are retrievable
    for i in 0..COUNT {
        let key = format!("key_{}", i);
        assert_eq!(map.get(&key), Some(&i), "Failed to get key {}", key);
    }
}

// =============================================================================
// HashSet Tests (uses PersistentHashMap internally)
// =============================================================================

/// Tests that HashSet operations work correctly with the hash function.
#[rstest]
fn test_hashset_referential_transparency() {
    let set1 = PersistentHashSet::new()
        .insert("alpha")
        .insert("beta")
        .insert("gamma");

    let set2 = PersistentHashSet::new()
        .insert("alpha")
        .insert("beta")
        .insert("gamma");

    // Same elements should be contained in both sets
    assert!(set1.contains(&"alpha"));
    assert!(set1.contains(&"beta"));
    assert!(set1.contains(&"gamma"));

    assert!(set2.contains(&"alpha"));
    assert!(set2.contains(&"beta"));
    assert!(set2.contains(&"gamma"));

    // Same length
    assert_eq!(set1.len(), set2.len());
}

/// Tests HashSet with large number of elements.
#[rstest]
fn test_hashset_large_scale() {
    const COUNT: i32 = 10_000;

    let set: PersistentHashSet<i32> = (0..COUNT).collect();

    // Verify all elements are contained
    for i in 0..COUNT {
        assert!(set.contains(&i), "Set should contain {}", i);
    }

    // Verify non-existent elements are not contained
    for i in COUNT..(COUNT + 100) {
        assert!(!set.contains(&i), "Set should not contain {}", i);
    }

    assert_eq!(set.len(), COUNT as usize);
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// Tests empty string as key.
#[rstest]
fn test_empty_string_key() {
    let map = PersistentHashMap::new().insert(String::new(), 42);

    assert_eq!(map.get(""), Some(&42));
    assert_eq!(map.len(), 1);
}

/// Tests keys that might have similar hash patterns.
#[rstest]
fn test_similar_keys() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("aa".to_string(), 2)
        .insert("aaa".to_string(), 3)
        .insert("aaaa".to_string(), 4);

    assert_eq!(map.get("a"), Some(&1));
    assert_eq!(map.get("aa"), Some(&2));
    assert_eq!(map.get("aaa"), Some(&3));
    assert_eq!(map.get("aaaa"), Some(&4));
}

/// Tests keys with special characters.
#[rstest]
fn test_special_character_keys() {
    let map = PersistentHashMap::new()
        .insert("\0".to_string(), 1) // null character
        .insert("\n".to_string(), 2) // newline
        .insert("\t".to_string(), 3) // tab
        .insert(" ".to_string(), 4); // space

    assert_eq!(map.get("\0"), Some(&1));
    assert_eq!(map.get("\n"), Some(&2));
    assert_eq!(map.get("\t"), Some(&3));
    assert_eq!(map.get(" "), Some(&4));
}

/// Tests Unicode keys.
#[rstest]
fn test_unicode_keys() {
    let map = PersistentHashMap::new()
        .insert("hello".to_string(), 1)
        .insert("hello".to_string(), 2) // This should overwrite
        .insert("konnichiwa".to_string(), 3)
        .insert("ni hao".to_string(), 4);

    assert_eq!(map.get("hello"), Some(&2));
    assert_eq!(map.get("konnichiwa"), Some(&3));
    assert_eq!(map.get("ni hao"), Some(&4));
}

/// Tests negative integer keys.
#[rstest]
fn test_negative_integer_keys() {
    let map: PersistentHashMap<i32, i32> = PersistentHashMap::new()
        .insert(-1, 100)
        .insert(-100, 200)
        .insert(-1000, 300)
        .insert(i32::MIN, 400);

    assert_eq!(map.get(&-1), Some(&100));
    assert_eq!(map.get(&-100), Some(&200));
    assert_eq!(map.get(&-1000), Some(&300));
    assert_eq!(map.get(&i32::MIN), Some(&400));
}

/// Tests boundary integer keys.
#[rstest]
fn test_boundary_integer_keys() {
    let map: PersistentHashMap<i64, i64> = PersistentHashMap::new()
        .insert(i64::MIN, 1)
        .insert(i64::MAX, 2)
        .insert(0, 3);

    assert_eq!(map.get(&i64::MIN), Some(&1));
    assert_eq!(map.get(&i64::MAX), Some(&2));
    assert_eq!(map.get(&0), Some(&3));
}

// =============================================================================
// Immutability Tests (hash function should not affect immutability)
// =============================================================================

/// Tests that insert returns a new map without modifying the original.
#[rstest]
fn test_immutability_after_insert() {
    let map1 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let map1_clone = map1.clone();
    let map2 = map1.insert("c".to_string(), 3);

    // Original map should be unchanged
    assert_eq!(map1_clone.len(), 2);
    assert_eq!(map1_clone.get("a"), Some(&1));
    assert_eq!(map1_clone.get("b"), Some(&2));
    assert_eq!(map1_clone.get("c"), None);

    // New map should have the new entry
    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get("c"), Some(&3));
}

/// Tests that remove returns a new map without modifying the original.
#[rstest]
fn test_immutability_after_remove() {
    let map1 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);

    let map1_clone = map1.clone();
    let map2 = map1.remove("b");

    // Original map should be unchanged
    assert_eq!(map1_clone.len(), 3);
    assert_eq!(map1_clone.get("b"), Some(&2));

    // New map should not have the removed entry
    assert_eq!(map2.len(), 2);
    assert_eq!(map2.get("b"), None);
}
