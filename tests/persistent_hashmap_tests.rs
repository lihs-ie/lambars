#![cfg(feature = "persistent")]
//! Unit tests for PersistentHashMap.
//!
//! This module contains comprehensive unit tests for the PersistentHashMap
//! implementation, following a TDD approach.

use lambars::persistent::PersistentHashMap;
use lambars::typeclass::Foldable;
use rstest::rstest;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// =============================================================================
// TDD Cycle 1: Empty map creation (new, is_empty, len)
// =============================================================================

#[rstest]
fn test_new_creates_empty_map() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[rstest]
fn test_get_on_empty_map_returns_none() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    assert_eq!(map.get("key"), None);
}

// =============================================================================
// TDD Cycle 2: Basic insert and get operations
// =============================================================================

#[rstest]
fn test_singleton_creates_single_entry_map() {
    let map = PersistentHashMap::singleton("key".to_string(), 42);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("key"), Some(&42));
}

#[rstest]
fn test_insert_and_get_single_entry() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let map = map.insert("key".to_string(), 42);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("key"), Some(&42));
}

#[rstest]
fn test_insert_multiple_entries() {
    let map = PersistentHashMap::new()
        .insert("one".to_string(), 1)
        .insert("two".to_string(), 2)
        .insert("three".to_string(), 3);

    assert_eq!(map.len(), 3);
    assert_eq!(map.get("one"), Some(&1));
    assert_eq!(map.get("two"), Some(&2));
    assert_eq!(map.get("three"), Some(&3));
    assert_eq!(map.get("four"), None);
}

#[rstest]
fn test_insert_does_not_modify_original() {
    let map1 = PersistentHashMap::new().insert("key".to_string(), 1);
    let map2 = map1.insert("key2".to_string(), 2);

    assert_eq!(map1.len(), 1);
    assert_eq!(map1.get("key2"), None);
    assert_eq!(map2.len(), 2);
    assert_eq!(map2.get("key2"), Some(&2));
}

// =============================================================================
// TDD Cycle 3: Insert overwrite existing key
// =============================================================================

#[rstest]
fn test_insert_overwrites_existing_key() {
    let map1 = PersistentHashMap::new().insert("key".to_string(), 1);
    let map2 = map1.insert("key".to_string(), 2);

    // Original map unchanged
    assert_eq!(map1.get("key"), Some(&1));
    assert_eq!(map1.len(), 1);

    // New map has updated value but same length
    assert_eq!(map2.get("key"), Some(&2));
    assert_eq!(map2.len(), 1);
}

#[rstest]
fn test_insert_multiple_overwrites() {
    let map = PersistentHashMap::new()
        .insert("key".to_string(), 1)
        .insert("key".to_string(), 2)
        .insert("key".to_string(), 3);

    assert_eq!(map.len(), 1);
    assert_eq!(map.get("key"), Some(&3));
}

// =============================================================================
// TDD Cycle 4: Remove operation
// =============================================================================

#[rstest]
fn test_remove_existing_key() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);
    let removed = map.remove("a");

    assert_eq!(removed.len(), 1);
    assert_eq!(removed.get("a"), None);
    assert_eq!(removed.get("b"), Some(&2));
}

#[rstest]
fn test_remove_nonexistent_key() {
    let map = PersistentHashMap::new().insert("a".to_string(), 1);
    let removed = map.remove("nonexistent");

    assert_eq!(removed.len(), 1);
    assert_eq!(removed.get("a"), Some(&1));
}

#[rstest]
fn test_remove_does_not_modify_original() {
    let map1 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);
    let map2 = map1.remove("a");

    // Original unchanged
    assert_eq!(map1.len(), 2);
    assert_eq!(map1.get("a"), Some(&1));

    // New map has entry removed
    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get("a"), None);
}

#[rstest]
fn test_remove_all_entries() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .remove("a")
        .remove("b");

    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

// =============================================================================
// TDD Cycle 5: contains_key operation
// =============================================================================

#[rstest]
fn test_contains_key_existing() {
    let map = PersistentHashMap::new().insert("key".to_string(), 42);

    assert!(map.contains_key("key"));
}

#[rstest]
fn test_contains_key_nonexistent() {
    let map = PersistentHashMap::new().insert("key".to_string(), 42);

    assert!(!map.contains_key("other"));
}

#[rstest]
fn test_contains_key_empty_map() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    assert!(!map.contains_key("key"));
}

// =============================================================================
// TDD Cycle 6: Hash collision handling
// =============================================================================

/// A type that always produces the same hash value for collision testing.
#[derive(Clone, PartialEq, Eq, Debug)]
struct CollidingKey {
    value: u32,
}

impl CollidingKey {
    fn new(value: u32) -> Self {
        CollidingKey { value }
    }
}

impl Hash for CollidingKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Always hash to the same value to force collisions
        42u64.hash(state);
    }
}

#[rstest]
fn test_hash_collision_insert() {
    let key1 = CollidingKey::new(1);
    let key2 = CollidingKey::new(2);
    let key3 = CollidingKey::new(3);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one".to_string())
        .insert(key2.clone(), "two".to_string())
        .insert(key3.clone(), "three".to_string());

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&key1), Some(&"one".to_string()));
    assert_eq!(map.get(&key2), Some(&"two".to_string()));
    assert_eq!(map.get(&key3), Some(&"three".to_string()));
}

#[rstest]
fn test_hash_collision_overwrite() {
    let key = CollidingKey::new(1);

    let map1 = PersistentHashMap::new().insert(key.clone(), "first".to_string());
    let map2 = map1.insert(key.clone(), "second".to_string());

    assert_eq!(map1.get(&key), Some(&"first".to_string()));
    assert_eq!(map2.get(&key), Some(&"second".to_string()));
    assert_eq!(map2.len(), 1);
}

#[rstest]
fn test_hash_collision_remove() {
    let key1 = CollidingKey::new(1);
    let key2 = CollidingKey::new(2);
    let key3 = CollidingKey::new(3);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one".to_string())
        .insert(key2.clone(), "two".to_string())
        .insert(key3.clone(), "three".to_string());

    let map = map.remove(&key2);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"one".to_string()));
    assert_eq!(map.get(&key2), None);
    assert_eq!(map.get(&key3), Some(&"three".to_string()));
}

// =============================================================================
// TDD Cycle 7: Update with function
// =============================================================================

#[rstest]
fn test_update_existing_key() {
    let map = PersistentHashMap::new().insert("count".to_string(), 10);
    let updated = map.update("count", |value| value + 1);

    assert!(updated.is_some());
    let updated = updated.unwrap();
    assert_eq!(updated.get("count"), Some(&11));
}

#[rstest]
fn test_update_nonexistent_key_returns_none() {
    let map = PersistentHashMap::new().insert("count".to_string(), 10);
    let result = map.update("nonexistent", |value| value + 1);

    assert!(result.is_none());
}

#[rstest]
fn test_update_does_not_modify_original() {
    let map1 = PersistentHashMap::new().insert("count".to_string(), 10);
    let map2 = map1.update("count", |value| value * 2).unwrap();

    assert_eq!(map1.get("count"), Some(&10));
    assert_eq!(map2.get("count"), Some(&20));
}

// =============================================================================
// TDD Cycle 8: Merge two maps
// =============================================================================

#[rstest]
fn test_merge_two_disjoint_maps() {
    let map1 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);
    let map2 = PersistentHashMap::new()
        .insert("c".to_string(), 3)
        .insert("d".to_string(), 4);

    let merged = map1.merge(&map2);

    assert_eq!(merged.len(), 4);
    assert_eq!(merged.get("a"), Some(&1));
    assert_eq!(merged.get("b"), Some(&2));
    assert_eq!(merged.get("c"), Some(&3));
    assert_eq!(merged.get("d"), Some(&4));
}

#[rstest]
fn test_merge_overlapping_maps_prefers_other() {
    let map1 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);
    let map2 = PersistentHashMap::new()
        .insert("b".to_string(), 20)
        .insert("c".to_string(), 3);

    let merged = map1.merge(&map2);

    assert_eq!(merged.len(), 3);
    assert_eq!(merged.get("a"), Some(&1));
    assert_eq!(merged.get("b"), Some(&20)); // From map2
    assert_eq!(merged.get("c"), Some(&3));
}

#[rstest]
fn test_merge_with_empty_map() {
    let map = PersistentHashMap::new().insert("a".to_string(), 1);
    let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();

    let merged1 = map.merge(&empty);
    let merged2 = empty.merge(&map);

    assert_eq!(merged1.len(), 1);
    assert_eq!(merged1.get("a"), Some(&1));
    assert_eq!(merged2.len(), 1);
    assert_eq!(merged2.get("a"), Some(&1));
}

// =============================================================================
// TDD Cycle 9: Iterator (keys, values, iter)
// =============================================================================

#[rstest]
fn test_iter_empty_map() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let collected: Vec<_> = map.iter().collect();
    assert!(collected.is_empty());
}

#[rstest]
fn test_iter_collects_all_entries() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);

    let mut collected: Vec<_> = map.iter().collect();
    collected.sort_by_key(|(key, _)| (*key).clone());

    assert_eq!(collected.len(), 3);
    assert_eq!(collected[0], (&"a".to_string(), &1));
    assert_eq!(collected[1], (&"b".to_string(), &2));
    assert_eq!(collected[2], (&"c".to_string(), &3));
}

#[rstest]
fn test_keys_iterator() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);

    let mut keys: Vec<_> = map.keys().cloned().collect();
    keys.sort();

    assert_eq!(
        keys,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[rstest]
fn test_values_iterator() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);

    let mut values: Vec<_> = map.values().copied().collect();
    values.sort();

    assert_eq!(values, vec![1, 2, 3]);
}

// =============================================================================
// TDD Cycle 10: FromIterator trait
// =============================================================================

#[rstest]
fn test_from_iter() {
    let entries = vec![
        ("a".to_string(), 1),
        ("b".to_string(), 2),
        ("c".to_string(), 3),
    ];
    let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

    assert_eq!(map.len(), 3);
    assert_eq!(map.get("a"), Some(&1));
    assert_eq!(map.get("b"), Some(&2));
    assert_eq!(map.get("c"), Some(&3));
}

#[rstest]
fn test_from_iter_with_duplicate_keys() {
    let entries = vec![
        ("key".to_string(), 1),
        ("key".to_string(), 2),
        ("key".to_string(), 3),
    ];
    let map: PersistentHashMap<String, i32> = entries.into_iter().collect();

    assert_eq!(map.len(), 1);
    assert_eq!(map.get("key"), Some(&3)); // Last value wins
}

// =============================================================================
// TDD Cycle 11: PartialEq, Eq, Debug traits
// =============================================================================

#[rstest]
fn test_partial_eq_same_entries() {
    let map1 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);
    let map2 = PersistentHashMap::new()
        .insert("b".to_string(), 2)
        .insert("a".to_string(), 1);

    assert_eq!(map1, map2);
}

#[rstest]
fn test_partial_eq_different_entries() {
    let map1 = PersistentHashMap::new().insert("a".to_string(), 1);
    let map2 = PersistentHashMap::new().insert("a".to_string(), 2);

    assert_ne!(map1, map2);
}

#[rstest]
fn test_partial_eq_different_length() {
    let map1 = PersistentHashMap::new().insert("a".to_string(), 1);
    let map2 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    assert_ne!(map1, map2);
}

#[rstest]
fn test_partial_eq_empty_maps() {
    let map1: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let map2: PersistentHashMap<String, i32> = PersistentHashMap::new();

    assert_eq!(map1, map2);
}

#[rstest]
fn test_debug_format() {
    let map = PersistentHashMap::new().insert("key".to_string(), 42);
    let debug_str = format!("{:?}", map);

    // Debug output should contain the key and value
    assert!(debug_str.contains("key") || debug_str.contains("42"));
}

// =============================================================================
// Large-scale tests
// =============================================================================

#[rstest]
fn test_large_map() {
    let mut map = PersistentHashMap::new();
    for index in 0..1000 {
        map = map.insert(index, index * 2);
    }

    assert_eq!(map.len(), 1000);

    for index in 0..1000 {
        assert_eq!(map.get(&index), Some(&(index * 2)));
    }
}

#[rstest]
fn test_large_map_with_removals() {
    let mut map = PersistentHashMap::new();
    for index in 0..500 {
        map = map.insert(index, index);
    }

    // Remove even numbers
    for index in (0..500).step_by(2) {
        map = map.remove(&index);
    }

    assert_eq!(map.len(), 250);

    for index in 0..500 {
        if index % 2 == 0 {
            assert_eq!(map.get(&index), None);
        } else {
            assert_eq!(map.get(&index), Some(&index));
        }
    }
}

// =============================================================================
// Borrow pattern tests
// =============================================================================

#[rstest]
fn test_get_with_borrowed_key() {
    let map = PersistentHashMap::new().insert("hello".to_string(), 42);

    // Can use &str to lookup String keys
    assert_eq!(map.get("hello"), Some(&42));
}

#[rstest]
fn test_contains_key_with_borrowed_key() {
    let map = PersistentHashMap::new().insert("hello".to_string(), 42);

    assert!(map.contains_key("hello"));
}

#[rstest]
fn test_remove_with_borrowed_key() {
    let map = PersistentHashMap::new().insert("hello".to_string(), 42);
    let map = map.remove("hello");

    assert!(map.is_empty());
}

// =============================================================================
// IntoIterator tests
// =============================================================================

#[rstest]
fn test_into_iter() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let mut collected: Vec<_> = map.into_iter().collect();
    collected.sort_by_key(|(key, _)| (*key).clone());

    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0], ("a".to_string(), 1));
    assert_eq!(collected[1], ("b".to_string(), 2));
}

// =============================================================================
// Default trait test
// =============================================================================

#[rstest]
fn test_default() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::default();
    assert!(map.is_empty());
}

// =============================================================================
// Coverage Tests: Controlled Hash Helper
// =============================================================================

/// A type that allows us to control the hash value for testing collision handling.
#[derive(Clone, PartialEq, Eq, Debug)]
struct ControlledHash {
    value: i32,
    hash: u64,
}

impl ControlledHash {
    fn new(value: i32, hash: u64) -> Self {
        ControlledHash { value, hash }
    }

    fn with_value(value: i32) -> Self {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        ControlledHash {
            value,
            hash: hasher.finish(),
        }
    }
}

impl Hash for ControlledHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Write the raw bytes directly to the hasher
        // This ensures that when DefaultHasher finishes, it produces
        // a hash value that can be used with our controlled bit patterns.
        // However, since we can't control the final hash value directly,
        // we rely on the fact that same hash field => same final hash.
        self.hash.hash(state);
    }
}

/// A hash key type that allows complete control over the final computed hash value.
/// This is used for testing specific code paths that depend on hash bit patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
struct DirectHashKey {
    identifier: i32,
    hash_value: u64,
}

#[allow(dead_code)]
impl DirectHashKey {
    fn new(identifier: i32, hash_value: u64) -> Self {
        DirectHashKey {
            identifier,
            hash_value,
        }
    }
}

/// Custom hasher that captures a single u64 value.
#[allow(dead_code)]
struct DirectHasher {
    value: u64,
}

#[allow(dead_code)]
impl DirectHasher {
    fn new() -> Self {
        DirectHasher { value: 0 }
    }
}

impl Hasher for DirectHasher {
    fn finish(&self) -> u64 {
        self.value
    }

    fn write(&mut self, bytes: &[u8]) {
        // Convert bytes to u64 if possible
        if bytes.len() == 8 {
            self.value = u64::from_ne_bytes(bytes.try_into().unwrap());
        }
    }
}

impl Hash for DirectHashKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Write raw bytes of hash_value directly
        state.write(&self.hash_value.to_ne_bytes());
    }
}

// =============================================================================
// Coverage Tests: Hash Collision Tests
// =============================================================================

#[rstest]
fn test_hash_collision_insert_controlled() {
    // Create two keys with the same hash but different values
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345); // Same hash as key1

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "value1")
        .insert(key2.clone(), "value2");

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"value1"));
    assert_eq!(map.get(&key2), Some(&"value2"));
}

#[rstest]
fn test_hash_collision_update_existing() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "value1")
        .insert(key2.clone(), "value2")
        .insert(key1.clone(), "updated_value1"); // Update key1

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"updated_value1"));
    assert_eq!(map.get(&key2), Some(&"value2"));
}

#[rstest]
fn test_hash_collision_three_keys() {
    let key1 = ControlledHash::new(1, 99999);
    let key2 = ControlledHash::new(2, 99999);
    let key3 = ControlledHash::new(3, 99999);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 100)
        .insert(key2.clone(), 200)
        .insert(key3.clone(), 300);

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&key1), Some(&100));
    assert_eq!(map.get(&key2), Some(&200));
    assert_eq!(map.get(&key3), Some(&300));
}

#[rstest]
fn test_hash_collision_remove_from_collision_node() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove from collision node with 3 entries -> 2 entries
    let map2 = map.remove(&key2);
    assert_eq!(map2.len(), 2);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert!(map2.get(&key2).is_none());
    assert_eq!(map2.get(&key3), Some(&"three"));
}

#[rstest]
fn test_hash_collision_remove_to_single_entry() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Remove from collision node with 2 entries -> single entry
    let map2 = map.remove(&key2);
    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert!(map2.get(&key2).is_none());
}

#[rstest]
fn test_hash_collision_remove_last_entry() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .remove(&key1)
        .remove(&key2);

    assert!(map.is_empty());
}

#[rstest]
fn test_hash_collision_get_not_found() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 12345); // Same hash but not in map

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    assert!(map.get(&key3).is_none());
}

// =============================================================================
// Coverage Tests: Deep Tree Tests
// =============================================================================

#[rstest]
fn test_deep_tree_insert_and_get() {
    // Insert many elements to create a deep tree
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..1000 {
        map = map.insert(index, index * 10);
    }

    assert_eq!(map.len(), 1000);
    for index in 0..1000 {
        assert_eq!(map.get(&index), Some(&(index * 10)));
    }
}

#[rstest]
fn test_deep_tree_remove() {
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..100 {
        map = map.insert(index, index);
    }

    // Remove all elements
    for index in 0..100 {
        map = map.remove(&index);
    }

    assert!(map.is_empty());
}

#[rstest]
fn test_deep_tree_remove_partial() {
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..100 {
        map = map.insert(index, index);
    }

    // Remove every other element
    for index in (0..100).step_by(2) {
        map = map.remove(&index);
    }

    assert_eq!(map.len(), 50);
    for index in 0..100 {
        if index % 2 == 0 {
            assert!(map.get(&index).is_none());
        } else {
            assert_eq!(map.get(&index), Some(&index));
        }
    }
}

// =============================================================================
// Coverage Tests: Edge Cases
// =============================================================================

#[rstest]
fn test_remove_from_empty_map() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let result = map.remove("key");
    assert!(result.is_empty());
}

#[rstest]
fn test_get_from_empty_map() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    assert!(map.get("key").is_none());
}

#[rstest]
fn test_merge_empty_maps() {
    let empty1: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let empty2: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let result = empty1.merge(&empty2);
    assert!(result.is_empty());
}

// =============================================================================
// Coverage Tests: Iterator Tests
// =============================================================================

#[rstest]
fn test_iter_size_hint() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let iter = map.iter();
    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 2);
    assert_eq!(upper, Some(2));
}

#[rstest]
fn test_iter_exact_size() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);

    let iter = map.iter();
    assert_eq!(iter.len(), 3);
}

#[rstest]
fn test_into_iter_size_hint() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let iter = map.into_iter();
    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 2);
    assert_eq!(upper, Some(2));
}

#[rstest]
fn test_into_iter_exact_size() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let iter = map.into_iter();
    assert_eq!(iter.len(), 2);
}

#[rstest]
fn test_ref_into_iterator() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let mut sum = 0;
    for (_, value) in &map {
        sum += value;
    }
    assert_eq!(sum, 3);
}

// =============================================================================
// Coverage Tests: Type Class Tests
// =============================================================================

#[rstest]
fn test_foldable_fold_right() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);

    let result = map.fold_right(0, |value, accumulator| value + accumulator);
    assert_eq!(result, 6);
}

#[rstest]
fn test_foldable_is_empty_trait() {
    let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let non_empty = PersistentHashMap::new().insert("key".to_string(), 42);

    assert!(Foldable::is_empty(&empty));
    assert!(!Foldable::is_empty(&non_empty));
}

#[rstest]
fn test_foldable_length_trait() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    assert_eq!(Foldable::length(&map), 2);
}

// =============================================================================
// Coverage Tests: PartialEq Tests
// =============================================================================

#[rstest]
fn test_eq_different_values() {
    let map1 = PersistentHashMap::new().insert("a".to_string(), 1);
    let map2 = PersistentHashMap::new().insert("a".to_string(), 2);

    assert_ne!(map1, map2);
}

#[rstest]
fn test_eq_different_keys() {
    let map1 = PersistentHashMap::new().insert("a".to_string(), 1);
    let map2 = PersistentHashMap::new().insert("b".to_string(), 1);

    assert_ne!(map1, map2);
}

// =============================================================================
// Coverage Tests: Collision Node with Different Hash Depths
// =============================================================================

#[rstest]
fn test_collision_with_different_hash_at_same_index() {
    // Create keys that share the same first 5 bits but differ later
    // This tests the case where we need to create subnodes
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();

    // Insert many keys to create complex tree structure
    for index in 0..64 {
        map = map.insert(index, index * 100);
    }

    // Verify all keys are accessible
    for index in 0..64 {
        assert_eq!(map.get(&index), Some(&(index * 100)));
    }
}

#[rstest]
fn test_insert_collision_into_existing_collision_node() {
    // Create a collision node first
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Now insert a key with a different hash
    let key3 = ControlledHash::with_value(100);
    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

#[rstest]
fn test_remove_not_in_collision() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 99999); // Different hash

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Try to remove a key with a different hash
    let result = map.remove(&key3);
    assert_eq!(result.len(), 2);
}

// =============================================================================
// Coverage Tests: find_key in Collision Node
// =============================================================================

#[rstest]
fn test_update_in_collision_node() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 10)
        .insert(key2.clone(), 20);

    let updated = map.update(&key1, |v| v + 5);
    assert!(updated.is_some());
    let updated_map = updated.unwrap();
    assert_eq!(updated_map.get(&key1), Some(&15));
    assert_eq!(updated_map.get(&key2), Some(&20));
}

// =============================================================================
// Coverage Tests: Bitmap Node Edge Cases
// =============================================================================

#[rstest]
fn test_bitmap_single_entry_simplification() {
    // Create a bitmap node with two children, then remove one
    let map = PersistentHashMap::new().insert(1, "one").insert(2, "two");

    let removed = map.remove(&1);
    assert_eq!(removed.len(), 1);
    assert_eq!(removed.get(&2), Some(&"two"));
}

#[rstest]
fn test_bitmap_remove_simplifies_to_entry() {
    // Insert keys that create different bitmap positions
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..5 {
        map = map.insert(index, index);
    }

    // Remove all but one
    for index in 1..5 {
        map = map.remove(&index);
    }

    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&0), Some(&0));
}

#[rstest]
fn test_remove_last_from_subnode() {
    // Create a structure where removing leads to empty subnode
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..100 {
        map = map.insert(index, index);
    }

    // Remove all elements
    for index in 0..100 {
        map = map.remove(&index);
    }

    assert!(map.is_empty());
}

// =============================================================================
// Coverage Tests: Iterator after partial consumption
// =============================================================================

#[rstest]
fn test_iter_after_partial_consumption() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);

    let mut iter = map.iter();
    iter.next(); // Consume one element

    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 2);
    assert_eq!(upper, Some(2));
    assert_eq!(iter.len(), 2);
}

#[rstest]
fn test_into_iter_after_partial_consumption() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);

    let mut iter = map.into_iter();
    iter.next(); // Consume one element

    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 2);
    assert_eq!(upper, Some(2));
    assert_eq!(iter.len(), 2);
}

#[rstest]
fn test_iter_fully_consumed() {
    let map = PersistentHashMap::new().insert("a".to_string(), 1);

    let mut iter = map.iter();
    iter.next();
    assert!(iter.next().is_none());
    assert_eq!(iter.len(), 0);
}

// =============================================================================
// Coverage Tests: Additional Collision Node Tests
// =============================================================================

#[rstest]
fn test_collision_insert_with_different_hash_same_index() {
    // This tests the case where we have a collision node and need to
    // insert a key with a different hash that has the same index at this depth
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    // key3 has a different hash but might share the same index at some depth
    let key3 = ControlledHash::new(3, 12345 + (1 << 5)); // Different hash, potentially same at depth 0

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&key1), Some(&"one"));
    assert_eq!(map.get(&key2), Some(&"two"));
    assert_eq!(map.get(&key3), Some(&"three"));
}

#[rstest]
fn test_update_nonexistent_key_in_collision_node() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 12345); // Same hash but not in map

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 10)
        .insert(key2.clone(), 20);

    let result = map.update(&key3, |value| value + 5);
    assert!(result.is_none());
}

#[rstest]
fn test_collision_node_iteration() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 100)
        .insert(key2.clone(), 200)
        .insert(key3.clone(), 300);

    let sum: i32 = map.values().sum();
    assert_eq!(sum, 600);
}

#[rstest]
fn test_remove_all_from_collision_leaves_empty() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .remove(&key1)
        .remove(&key2);

    assert!(map.is_empty());
}

// =============================================================================
// Coverage Tests: Deep Tree Removal Tests
// =============================================================================

#[rstest]
fn test_remove_causes_subnode_to_become_entry() {
    // Create a structure where removing causes a subnode to collapse to an entry
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();

    // Insert keys that create a deep tree structure
    for index in 0..50 {
        map = map.insert(index, index * 10);
    }

    // Remove most keys, leaving structure that needs simplification
    for index in 1..50 {
        map = map.remove(&index);
    }

    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&0), Some(&0));
}

#[rstest]
fn test_remove_from_bitmap_with_node_child() {
    // Create a structure where we remove from a bitmap that has Node children
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();

    // Insert enough keys to create subnodes
    for index in 0..100 {
        map = map.insert(index, index);
    }

    // Remove specific keys to trigger different removal paths
    for index in (0..100).step_by(3) {
        map = map.remove(&index);
    }

    // Verify remaining keys
    for index in 0..100 {
        if index % 3 == 0 {
            assert!(map.get(&index).is_none());
        } else {
            assert_eq!(map.get(&index), Some(&index));
        }
    }
}

// =============================================================================
// Coverage Tests: Entry to Bitmap Conversion Tests
// =============================================================================

#[rstest]
fn test_entry_with_same_hash_different_keys_creates_collision() {
    let key1 = ControlledHash::new(100, 99999);
    let key2 = ControlledHash::new(200, 99999); // Same hash

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "first")
        .insert(key2.clone(), "second");

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"first"));
    assert_eq!(map.get(&key2), Some(&"second"));
}

#[rstest]
fn test_entry_different_hash_same_index_creates_subnode() {
    // Keys with different hashes but same index at depth 0
    let hash1: u64 = 0b00001; // index 1 at depth 0
    let hash2: u64 = 0b100001; // index 1 at depth 0, different at depth 1

    let key1 = ControlledHash::new(1, hash1);
    let key2 = ControlledHash::new(2, hash2);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "first")
        .insert(key2.clone(), "second");

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"first"));
    assert_eq!(map.get(&key2), Some(&"second"));
}

// =============================================================================
// Coverage Tests: find_key Coverage Tests
// =============================================================================

#[rstest]
fn test_find_key_in_empty_map() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let result = map.update("nonexistent", |value| value + 1);
    assert!(result.is_none());
}

#[rstest]
fn test_find_key_in_entry_node_not_matching() {
    let map = PersistentHashMap::singleton("key".to_string(), 42);
    let result = map.update("other", |value| value + 1);
    assert!(result.is_none());
}

#[rstest]
fn test_find_key_in_bitmap_not_found() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);
    let result = map.update("c", |value| value + 1);
    assert!(result.is_none());
}

#[rstest]
fn test_find_key_in_bitmap_entry_not_matching() {
    let map = PersistentHashMap::new()
        .insert("key1".to_string(), 1)
        .insert("key2".to_string(), 2);
    let result = map.update("key3", |value| value + 1);
    assert!(result.is_none());
}

#[rstest]
fn test_find_key_traverses_subnode() {
    // Create enough entries to have subnodes
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..100 {
        map = map.insert(index, index);
    }

    // Update a key that requires traversing subnodes
    let updated = map.update(&50, |value| value + 1000);
    assert!(updated.is_some());
    assert_eq!(updated.unwrap().get(&50), Some(&1050));
}

#[rstest]
fn test_find_key_in_collision_node_controlled() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 10)
        .insert(key2.clone(), 20);

    // Update key in collision node
    let updated = map.update(&key2, |value| value + 100);
    assert!(updated.is_some());
    assert_eq!(updated.unwrap().get(&key2), Some(&120));
}

#[rstest]
fn test_find_key_collision_not_found() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 12345); // Same hash but not in map

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 10)
        .insert(key2.clone(), 20);

    let result = map.update(&key3, |value| value + 100);
    assert!(result.is_none());
}

// =============================================================================
// Coverage Tests: Collision with Different Hash Insertion
// =============================================================================

#[rstest]
fn test_collision_insert_different_hash_different_index() {
    // Create collision node first
    let key1 = ControlledHash::new(1, 0b00000_00000_00001); // index 1 at depth 0
    let key2 = ControlledHash::new(2, 0b00000_00000_00001); // Same hash -> collision

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Now insert key with different hash, different index
    let key3 = ControlledHash::new(3, 0b00000_00000_00010); // index 2 at depth 0

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

#[rstest]
fn test_collision_insert_different_hash_same_index() {
    // Create collision node first
    let key1 = ControlledHash::new(1, 0b00000_00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00000_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Insert key with different hash but same index at depth 0
    let key3 = ControlledHash::new(3, 0b00000_00001_00001); // index 1 at depth 0, different at depth 1

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Coverage Tests: Remove from Collision with Different Hash
// =============================================================================

#[rstest]
fn test_remove_from_collision_different_hash() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 99999); // Different hash

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Try to remove a key with different hash
    let result = map.remove(&key3);
    assert_eq!(result.len(), 2);
}

// =============================================================================
// Coverage Tests: Collision Node with Higher Index Position
// =============================================================================

#[rstest]
fn test_collision_insert_collision_index_greater_than_new_index() {
    // Create collision node with high index, then insert with lower index
    // collision_index > new_index branch
    let key1 = ControlledHash::new(1, 0b11111_00000); // high index at depth 0
    let key2 = ControlledHash::new(2, 0b11111_00000); // Same hash -> collision

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Insert with lower index
    let key3 = ControlledHash::new(3, 0b00001_00000); // lower index at depth 0

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

#[rstest]
fn test_collision_with_same_index_at_depth_requires_recursion() {
    // Create collision node, then insert key with different hash but same index at all depths
    // This tests the recursive case in collision handling
    let key1 = ControlledHash::new(1, 0b00001_00001_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001_00001); // Same hash -> collision

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Insert with same index at depth 0, but different later
    let key3 = ControlledHash::new(3, 0b00010_00001_00001); // Same at depth 0, different at depth 1

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Coverage Tests: Entry Node with Same Index at Depth (Recursive Case)
// =============================================================================

#[rstest]
fn test_entry_same_index_different_hash_creates_subnode() {
    // Two keys with different hashes but same index at depth 0
    // This tests the existing_index == new_index branch in Entry
    let key1 = ControlledHash::new(1, 0b00000_00001); // index 1 at depth 0
    let key2 = ControlledHash::new(2, 0b00001_00001); // same index at depth 0, different at depth 1

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "first")
        .insert(key2.clone(), "second");

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"first"));
    assert_eq!(map.get(&key2), Some(&"second"));
}

#[rstest]
fn test_entry_same_index_requires_deep_recursion() {
    // Keys that match at multiple levels
    let key1 = ControlledHash::new(1, 0b00000_00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00000_00001); // Same at depth 0 and 1

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"one"));
    assert_eq!(map.get(&key2), Some(&"two"));
}

// =============================================================================
// Coverage Tests: Bitmap Node with Child::Entry Hash Collision
// =============================================================================

#[rstest]
fn test_bitmap_child_entry_hash_collision() {
    // Create a bitmap node with an Entry child, then insert a key
    // that has the same hash as the existing Entry
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010); // Different index

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Now insert a key with same hash as key1 (creates collision in Child::Entry)
    let key3 = ControlledHash::new(3, 0b00000_00001); // Same hash as key1

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

#[rstest]
fn test_bitmap_child_entry_different_hash_same_index() {
    // Create bitmap with Entry child, insert with different hash but same index
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Insert with same index as key1 but different hash (needs subnode)
    let key3 = ControlledHash::new(3, 0b00001_00001);

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Coverage Tests: Remove Operations that Simplify Tree Structure
// =============================================================================

#[rstest]
fn test_remove_subnode_collapses_to_entry() {
    // Create structure where removing leaves a single Entry in a subnode
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001); // Same index at depth 0

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Remove key2, should collapse the subnode
    let map2 = map.remove(&key2);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert!(map2.get(&key2).is_none());
}

#[rstest]
fn test_remove_subnode_becomes_empty() {
    // Create deep structure where removing makes subnode empty
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010);
    let key3 = ControlledHash::new(3, 0b00001_00001); // Same index as key1 at depth 0

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove both keys with index 1 at depth 0
    let map2 = map.remove(&key1).remove(&key3);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key2), Some(&"two"));
}

#[rstest]
fn test_remove_from_bitmap_with_node_child_simplifies() {
    // Build a structure that has Node children in a Bitmap
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..64 {
        map = map.insert(index, index * 10);
    }

    // Remove elements to trigger simplification paths
    for index in 1..64 {
        map = map.remove(&index);
    }

    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&0), Some(&0));
}

// =============================================================================
// Coverage Tests: Get from Bitmap with Missing Child
// =============================================================================

#[rstest]
fn test_get_from_bitmap_slot_empty() {
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010);
    let query = ControlledHash::new(3, 0b00000_00011); // Different index, not in map

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    assert!(map.get(&query).is_none());
}

#[rstest]
fn test_get_from_bitmap_child_entry_not_matching() {
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010);
    // Query with same index as key1 but different key
    let query = ControlledHash::new(3, 0b00000_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // key1 and query have same hash but different keys
    assert!(map.get(&query).is_none());
}

// =============================================================================
// Coverage Tests: Entry with existing_index > new_index Branch
// =============================================================================

#[rstest]
fn test_entry_existing_index_greater_than_new_index() {
    // First key has higher index, second key has lower index
    let key1 = ControlledHash::new(1, 0b00000_11111); // high index at depth 0
    let key2 = ControlledHash::new(2, 0b00000_00001); // low index at depth 0

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "first")
        .insert(key2.clone(), "second");

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"first"));
    assert_eq!(map.get(&key2), Some(&"second"));
}

// =============================================================================
// Coverage Tests: Remove with removed = false path
// =============================================================================

#[rstest]
fn test_remove_from_node_child_returns_not_removed() {
    // Create structure where remove_from_node returns Some((node, false))
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    let nonexistent = ControlledHash::new(99, 0b00010_00001); // Same first index but different key

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Try to remove key that matches path but doesn't exist
    let result = map.remove(&nonexistent);

    assert_eq!(result.len(), 2);
    assert_eq!(result.get(&key1), Some(&"one"));
    assert_eq!(result.get(&key2), Some(&"two"));
}

// =============================================================================
// Coverage Tests: Iterator with Collision Nodes
// =============================================================================

#[rstest]
fn test_iter_with_collision_node_in_bitmap() {
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00001); // Same hash -> collision
    let key3 = ControlledHash::new(3, 0b00000_00010); // Different index

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 10)
        .insert(key2.clone(), 20)
        .insert(key3.clone(), 30);

    let sum: i32 = map.values().sum();
    assert_eq!(sum, 60);
    assert_eq!(map.len(), 3);
}

// =============================================================================
// Coverage Tests: Deep Removal with Multiple Simplification Steps
// =============================================================================

#[rstest]
fn test_remove_triggers_cascading_simplification() {
    // Create a deep tree structure
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..128 {
        map = map.insert(index, index);
    }

    // Remove most elements, triggering multiple simplification levels
    for index in 1..128 {
        map = map.remove(&index);
    }

    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&0), Some(&0));
}

// =============================================================================
// Tests for update_with method
// =============================================================================

#[rstest]
fn test_update_with_increment_existing_value() {
    let map = PersistentHashMap::new().insert("count".to_string(), 10);
    let updated = map.update_with("count", |maybe_value| maybe_value.map(|value| value + 1));

    assert_eq!(updated.get("count"), Some(&11));
    assert_eq!(map.get("count"), Some(&10)); // Original unchanged
}

#[rstest]
fn test_update_with_insert_if_absent() {
    let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let updated = map.update_with("new_key", |maybe_value| match maybe_value {
        Some(value) => Some(*value),
        None => Some(100),
    });

    assert_eq!(updated.get("new_key"), Some(&100));
    assert!(map.get("new_key").is_none()); // Original unchanged
}

#[rstest]
fn test_update_with_remove_by_returning_none() {
    let map = PersistentHashMap::new().insert("key".to_string(), 42);
    let updated = map.update_with("key", |_| None);

    assert!(updated.get("key").is_none());
    assert_eq!(map.get("key"), Some(&42)); // Original unchanged
}

#[rstest]
fn test_update_with_no_change_on_nonexistent_key() {
    let map = PersistentHashMap::new().insert("existing".to_string(), 10);
    let updated = map.update_with("nonexistent", |_| None);

    assert_eq!(updated.len(), 1);
    assert_eq!(updated.get("existing"), Some(&10));
}

#[rstest]
fn test_update_with_replace_value() {
    let map = PersistentHashMap::new().insert("key".to_string(), 5);
    let updated = map.update_with("key", |maybe_value| maybe_value.map(|value| value * 10));

    assert_eq!(updated.get("key"), Some(&50));
}

#[rstest]
fn test_update_with_conditional_update() {
    let map = PersistentHashMap::new()
        .insert("low".to_string(), 5)
        .insert("high".to_string(), 100);

    // Only update if value is less than 50
    let updated_low = map.update_with("low", |maybe_value| {
        maybe_value.map(|value| if *value < 50 { value + 10 } else { *value })
    });
    let updated_high = map.update_with("high", |maybe_value| {
        maybe_value.map(|value| if *value < 50 { value + 10 } else { *value })
    });

    assert_eq!(updated_low.get("low"), Some(&15)); // Updated
    assert_eq!(updated_high.get("high"), Some(&100)); // Unchanged
}

#[rstest]
fn test_update_with_upsert_pattern() {
    // Upsert: insert if not exists, update if exists
    let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let with_value = empty.insert("counter".to_string(), 1);

    let upsert = |maybe_value: Option<&i32>| -> Option<i32> {
        match maybe_value {
            Some(value) => Some(value + 1),
            None => Some(1),
        }
    };

    // First call: inserts 1
    let result1 = empty.update_with("counter", upsert);
    assert_eq!(result1.get("counter"), Some(&1));

    // Second call: increments to 2
    let result2 = with_value.update_with("counter", upsert);
    assert_eq!(result2.get("counter"), Some(&2));
}

#[rstest]
fn test_update_with_does_not_modify_other_entries() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);

    let updated = map.update_with("b", |maybe_value| maybe_value.map(|value| value * 10));

    assert_eq!(updated.get("a"), Some(&1));
    assert_eq!(updated.get("b"), Some(&20));
    assert_eq!(updated.get("c"), Some(&3));
}

#[rstest]
fn test_update_with_on_empty_map_insert() {
    let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let updated = empty.update_with("key", |_| Some(42));

    assert_eq!(updated.len(), 1);
    assert_eq!(updated.get("key"), Some(&42));
}

#[rstest]
fn test_update_with_on_empty_map_no_insert() {
    let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let updated = empty.update_with("key", |_| None);

    assert!(updated.is_empty());
}

#[rstest]
fn test_update_with_type_change() {
    // Value type can be complex
    let map = PersistentHashMap::new().insert("data".to_string(), vec![1, 2, 3]);

    let updated = map.update_with("data", |maybe_value| {
        maybe_value.map(|vector| {
            let mut new_vector = vector.clone();
            new_vector.push(4);
            new_vector
        })
    });

    assert_eq!(updated.get("data"), Some(&vec![1, 2, 3, 4]));
}

#[rstest]
fn test_update_with_preserves_length_on_update() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let updated = map.update_with("a", |maybe_value| maybe_value.map(|value| value + 10));

    assert_eq!(updated.len(), 2);
}

#[rstest]
fn test_update_with_length_decreases_on_remove() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let updated = map.update_with("a", |_| None);

    assert_eq!(updated.len(), 1);
    assert!(updated.get("a").is_none());
    assert_eq!(updated.get("b"), Some(&2));
}

#[rstest]
fn test_update_with_length_increases_on_insert() {
    let map = PersistentHashMap::new().insert("existing".to_string(), 1);

    let updated = map.update_with("new", |_| Some(2));

    assert_eq!(updated.len(), 2);
    assert_eq!(updated.get("existing"), Some(&1));
    assert_eq!(updated.get("new"), Some(&2));
}

#[rstest]
fn test_remove_all_but_one_from_each_subnode() {
    // Create structure with multiple subnodes
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..256 {
        map = map.insert(index, index * 2);
    }

    // Remove elements to leave one per original subnode
    for index in 1..256 {
        if index % 32 != 0 {
            map = map.remove(&index);
        }
    }

    // Check remaining elements
    for index in (0..256).step_by(32) {
        assert_eq!(map.get(&index), Some(&(index * 2)));
    }
}

// =============================================================================
// Coverage Tests: Collision Node Remove All But One
// =============================================================================

#[rstest]
fn test_collision_remove_all_but_one_entry() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 12345);
    let key4 = ControlledHash::new(4, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three")
        .insert(key4.clone(), "four");

    // Remove all but one
    let map2 = map.remove(&key1).remove(&key2).remove(&key3);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key4), Some(&"four"));
}

// =============================================================================
// Coverage Tests: Update in Bitmap with Node Child
// =============================================================================

#[rstest]
fn test_update_traverses_node_child() {
    // Create deep structure
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    for index in 0..100 {
        map = map.insert(index, index);
    }

    // Update an element deep in the tree
    let updated = map.update(&50, |value| value * 10);
    assert!(updated.is_some());
    assert_eq!(updated.unwrap().get(&50), Some(&500));
}

// =============================================================================
// Coverage Tests: find_key in Bitmap with Subnode
// =============================================================================

#[rstest]
fn test_find_key_in_subnode() {
    // Create structure with subnodes
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 100)
        .insert(key2.clone(), 200);

    // Update key in subnode
    let updated = map.update(&key2, |value| value + 50);
    assert!(updated.is_some());
    assert_eq!(updated.unwrap().get(&key2), Some(&250));
}

// =============================================================================
// Coverage Tests: Remove from Entry Node with Same Hash but Not Same Key
// =============================================================================

#[rstest]
fn test_remove_entry_same_hash_different_key() {
    let key1 = ControlledHash::new(1, 12345);
    let query = ControlledHash::new(99, 12345); // Same hash, different key

    let map = PersistentHashMap::singleton(key1.clone(), "value");

    // Try to remove with same hash but different key
    let result = map.remove(&query);

    assert_eq!(result.len(), 1);
    assert_eq!(result.get(&key1), Some(&"value"));
}

// =============================================================================
// Coverage Tests: Insert into Bitmap with Child::Node
// =============================================================================

#[rstest]
fn test_insert_into_bitmap_with_node_child() {
    // Create structure with Node children
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001); // Creates subnode

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Insert into the same subnode
    let key3 = ControlledHash::new(3, 0b00010_00001);
    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Coverage Tests: Collision Node with Key Update
// =============================================================================

#[rstest]
fn test_collision_node_key_update() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Update an existing key in collision node
    let map2 = map.insert(key2.clone(), "updated_two");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"updated_two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Coverage Tests: Empty Collision After All Removals
// =============================================================================

#[rstest]
fn test_collision_becomes_empty_after_all_removals() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    let result = map.remove(&key1).remove(&key2);

    assert!(result.is_empty());
}

// =============================================================================
// Additional Coverage Tests: Collision to Bitmap Conversion
// =============================================================================

#[rstest]
fn test_collision_to_bitmap_collision_index_less_than_new_index() {
    // collision_index < new_index: collision node comes first in children
    let key1 = ControlledHash::new(1, 0b00000_00001); // index 1 at depth 0
    let key2 = ControlledHash::new(2, 0b00000_00001); // Same hash -> collision

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Insert with higher index
    let key3 = ControlledHash::new(3, 0b00000_11111); // index 31 at depth 0

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

#[rstest]
fn test_collision_to_bitmap_collision_index_greater_than_new_index() {
    // collision_index > new_index: new entry comes first in children
    let key1 = ControlledHash::new(1, 0b00000_11111); // index 31 at depth 0
    let key2 = ControlledHash::new(2, 0b00000_11111); // Same hash -> collision

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Insert with lower index
    let key3 = ControlledHash::new(3, 0b00000_00001); // index 1 at depth 0

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Additional Coverage Tests: Entry to Bitmap with Different Index Ordering
// =============================================================================

#[rstest]
fn test_entry_to_bitmap_existing_index_less_than_new_index() {
    let key1 = ControlledHash::new(1, 0b00000_00001); // index 1 at depth 0
    let key2 = ControlledHash::new(2, 0b00000_11111); // index 31 at depth 0

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "first")
        .insert(key2.clone(), "second");

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"first"));
    assert_eq!(map.get(&key2), Some(&"second"));
}

#[rstest]
fn test_entry_to_bitmap_existing_index_greater_than_new_index() {
    let key1 = ControlledHash::new(1, 0b00000_11111); // index 31 at depth 0
    let key2 = ControlledHash::new(2, 0b00000_00001); // index 1 at depth 0

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "first")
        .insert(key2.clone(), "second");

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&"first"));
    assert_eq!(map.get(&key2), Some(&"second"));
}

// =============================================================================
// Additional Coverage Tests: Remove from Bitmap with Various Children Types
// =============================================================================

#[rstest]
fn test_remove_from_bitmap_subnode_becomes_entry() {
    // Create a structure where removing collapses subnode to entry
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001); // Creates subnode at index 1
    let key3 = ControlledHash::new(3, 0b00000_00010); // Different index

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove key1, should collapse subnode to entry containing only key2
    let map2 = map.remove(&key1);

    assert_eq!(map2.len(), 2);
    assert!(map2.get(&key1).is_none());
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

#[rstest]
fn test_remove_from_bitmap_subnode_becomes_empty() {
    // Create structure where subnode becomes empty
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Remove both
    let map2 = map.remove(&key1).remove(&key2);

    assert!(map2.is_empty());
}

#[rstest]
fn test_remove_simplifies_bitmap_to_single_entry() {
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    let map2 = map.remove(&key1);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key2), Some(&"two"));
}

// =============================================================================
// Additional Coverage Tests: Remove from Node Child in Bitmap
// =============================================================================

#[rstest]
fn test_remove_from_subnode_simplifies_to_entry() {
    // Create bitmap with node child, then remove to simplify
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    let key3 = ControlledHash::new(3, 0b00000_00010);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove key2, should collapse subnode to entry
    let map2 = map.remove(&key2);

    assert_eq!(map2.len(), 2);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert!(map2.get(&key2).is_none());
    assert_eq!(map2.get(&key3), Some(&"three"));
}

#[rstest]
fn test_remove_both_from_subnode() {
    // Create bitmap with node child containing two entries
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    let key3 = ControlledHash::new(3, 0b00000_00010);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove both keys from subnode
    let map2 = map.remove(&key1).remove(&key2);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Additional Coverage Tests: Bitmap Node Empty After Remove
// =============================================================================

#[rstest]
fn test_bitmap_becomes_empty_after_removing_all() {
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010);
    let key3 = ControlledHash::new(3, 0b00000_00011);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three")
        .remove(&key1)
        .remove(&key2)
        .remove(&key3);

    assert!(map.is_empty());
}

// =============================================================================
// Additional Coverage Tests: Subnode Returns Not Removed
// =============================================================================

#[rstest]
fn test_remove_from_subnode_key_not_found() {
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    let query = ControlledHash::new(99, 0b00010_00001); // Same index 1, but not in subnode

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    let result = map.remove(&query);

    assert_eq!(result.len(), 2);
}

// =============================================================================
// Additional Coverage Tests: Collision Node in Bitmap
// =============================================================================

#[rstest]
fn test_collision_node_stored_in_bitmap() {
    // Create a collision node, then add another key at different index
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00001); // Same hash -> collision
    let key3 = ControlledHash::new(3, 0b00000_00010); // Different index

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&key1), Some(&"one"));
    assert_eq!(map.get(&key2), Some(&"two"));
    assert_eq!(map.get(&key3), Some(&"three"));
}

// =============================================================================
// Additional Coverage Tests: Deep Structure with Node Children
// =============================================================================

#[rstest]
fn test_deep_structure_with_multiple_levels() {
    // Create keys that require multiple levels of depth
    let key1 = ControlledHash::new(1, 0b00001_00001_00001);
    let key2 = ControlledHash::new(2, 0b00010_00001_00001); // Same at depth 0, 1; different at depth 2
    let key3 = ControlledHash::new(3, 0b00011_00001_00001);
    let key4 = ControlledHash::new(4, 0b00001_00010_00001); // Same at depth 0; different at depth 1

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three")
        .insert(key4.clone(), "four");

    assert_eq!(map.len(), 4);
    assert_eq!(map.get(&key1), Some(&"one"));
    assert_eq!(map.get(&key2), Some(&"two"));
    assert_eq!(map.get(&key3), Some(&"three"));
    assert_eq!(map.get(&key4), Some(&"four"));
}

#[rstest]
fn test_remove_from_deep_structure() {
    let key1 = ControlledHash::new(1, 0b00001_00001_00001);
    let key2 = ControlledHash::new(2, 0b00010_00001_00001);
    let key3 = ControlledHash::new(3, 0b00011_00001_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    let map2 = map.remove(&key2);

    assert_eq!(map2.len(), 2);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert!(map2.get(&key2).is_none());
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Additional Coverage Tests: Update via find_key
// =============================================================================

#[rstest]
fn test_update_via_find_key_in_entry() {
    let map = PersistentHashMap::singleton("key".to_string(), 10);
    let updated = map.update("key", |v| v * 2);

    assert!(updated.is_some());
    assert_eq!(updated.unwrap().get("key"), Some(&20));
}

#[rstest]
fn test_update_via_find_key_in_bitmap() {
    let map = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let updated = map.update("b", |v| v + 10);

    assert!(updated.is_some());
    assert_eq!(updated.unwrap().get("b"), Some(&12));
}

#[rstest]
fn test_update_via_find_key_in_collision() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 100)
        .insert(key2.clone(), 200);

    let updated = map.update(&key2, |v| v + 50);

    assert!(updated.is_some());
    assert_eq!(updated.unwrap().get(&key2), Some(&250));
}

// =============================================================================
// Additional Coverage Tests: PartialEq
// =============================================================================

#[rstest]
fn test_partial_eq_value_mismatch() {
    let map1 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);
    let map2 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 999); // Different value

    assert_ne!(map1, map2);
}

#[rstest]
fn test_partial_eq_key_not_in_other() {
    let map1 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);
    let map2 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("c".to_string(), 2); // Different key

    assert_ne!(map1, map2);
}

// =============================================================================
// Additional Coverage Tests: Merge Operations
// =============================================================================

#[rstest]
fn test_merge_with_overlapping_keys() {
    let map1 = PersistentHashMap::new()
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2)
        .insert("c".to_string(), 3);
    let map2 = PersistentHashMap::new()
        .insert("b".to_string(), 20)
        .insert("c".to_string(), 30)
        .insert("d".to_string(), 4);

    let merged = map1.merge(&map2);

    assert_eq!(merged.len(), 4);
    assert_eq!(merged.get("a"), Some(&1));
    assert_eq!(merged.get("b"), Some(&20));
    assert_eq!(merged.get("c"), Some(&30));
    assert_eq!(merged.get("d"), Some(&4));
}

// =============================================================================
// Additional Coverage Tests: Collision Node Iteration
// =============================================================================

#[rstest]
fn test_iter_collision_node() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 10)
        .insert(key2.clone(), 20)
        .insert(key3.clone(), 30);

    let sum: i32 = map.values().sum();
    assert_eq!(sum, 60);
}

// =============================================================================
// Additional Coverage Tests: Collision with Same Index Recursion
// =============================================================================

#[rstest]
fn test_collision_same_index_recursion() {
    // Create collision, then insert with same index but different hash
    let key1 = ControlledHash::new(1, 0b00001_00001_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001_00001); // Same hash -> collision

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Insert with same index at depth 0, but different hash
    let key3 = ControlledHash::new(3, 0b00001_00010_00001);

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Additional Coverage Tests: Node Simplification to Entry when len == 1
// =============================================================================

#[rstest]
fn test_remove_simplifies_subnode_to_entry_when_len_1() {
    // Create a deep structure
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Remove key1, subnode should collapse
    let map2 = map.remove(&key1);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key2), Some(&"two"));
}

#[rstest]
fn test_remove_simplifies_bitmap_when_one_entry_remains() {
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010);
    let key3 = ControlledHash::new(3, 0b00000_00011);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove two entries, leaving one
    let map2 = map.remove(&key1).remove(&key2);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Final Coverage Tests: Collision Same Index Recursion
// =============================================================================

#[rstest]
fn test_collision_same_index_recursion_into_node() {
    // Create a collision, then insert key with same index at depth 0
    // This should trigger line 634-637 (collision_index == new_index)
    let collision_hash: u64 = 0b00001; // index 1 at depth 0
    let key1 = ControlledHash::new(1, collision_hash);
    let key2 = ControlledHash::new(2, collision_hash); // Same hash -> collision

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Now insert key with different hash but same index at depth 0
    // This triggers the collision_index == new_index branch
    let new_hash: u64 = 0b100001; // same index 1 at depth 0, different at depth 1
    let key3 = ControlledHash::new(3, new_hash);

    let map2 = map.insert(key3.clone(), "three");

    assert_eq!(map2.len(), 3);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Final Coverage Tests: Remove Returns Not Removed Clone
// =============================================================================

#[rstest]
fn test_remove_not_found_returns_clone() {
    // Test line 707: removed = false case returns clone
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    let nonexistent = ControlledHash::new(99, 0b00010_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Try to remove a key that exists in the same subnode path but doesn't exist
    let result = map.remove(&nonexistent);

    assert_eq!(result.len(), 2);
    assert_eq!(result.get(&key1), Some(&"one"));
    assert_eq!(result.get(&key2), Some(&"two"));
}

// =============================================================================
// Final Coverage Tests: Bitmap Entry Removal with Empty Result
// =============================================================================

#[rstest]
fn test_remove_entry_bitmap_becomes_empty() {
    // Test line 760: new_bitmap == 0 case
    let key = ControlledHash::new(1, 0b00000_00001);

    let map = PersistentHashMap::singleton(key.clone(), "value");
    let result = map.remove(&key);

    assert!(result.is_empty());
}

// =============================================================================
// Final Coverage Tests: Subnode Becomes Empty in Bitmap
// =============================================================================

#[rstest]
fn test_remove_from_subnode_becomes_empty() {
    // Test lines 806-835: subnode becomes empty after removal
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    let key3 = ControlledHash::new(3, 0b00000_00010);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove both keys from the subnode at index 1
    let map2 = map.remove(&key1).remove(&key2);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key3), Some(&"three"));
}

#[rstest]
fn test_remove_last_from_subnode_bitmap_simplifies() {
    // Create structure where removing last element from subnode triggers simplification
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001); // Same index at depth 0
    let key3 = ControlledHash::new(3, 0b00010_00001); // Same index at depth 0

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove key2 and key3, leaving only key1 in subnode
    let map2 = map.remove(&key2).remove(&key3);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key1), Some(&"one"));
}

#[rstest]
fn test_remove_from_subnode_with_only_node_child() {
    // Create a bitmap with only a Node child, then remove to make it empty
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Remove both to make entire structure empty
    let map2 = map.remove(&key1).remove(&key2);

    assert!(map2.is_empty());
}

// =============================================================================
// Final Coverage Tests: Collision Remove to Empty and Single
// =============================================================================

#[rstest]
fn test_collision_remove_to_empty() {
    // Test line 908: new_entries.is_empty() case
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    let result = map.remove(&key1).remove(&key2);

    assert!(result.is_empty());
}

#[rstest]
fn test_collision_remove_key_not_found() {
    // Test line 929: found_index is None case
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let nonexistent = ControlledHash::new(99, 12345); // Same hash but not in collision

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    let result = map.remove(&nonexistent);

    assert_eq!(result.len(), 2);
}

// =============================================================================
// Final Coverage Tests: find_key Various Paths
// =============================================================================

#[rstest]
fn test_find_key_empty_node() {
    // Test line 984: Node::Empty case
    let empty: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let result = empty.update("key", |v| v + 1);
    assert!(result.is_none());
}

#[rstest]
fn test_find_key_entry_not_matching() {
    // Test line 993: entry_hash == hash but entry_key != key
    let map = PersistentHashMap::singleton("key".to_string(), 10);
    let result = map.update("other", |v| v + 1);
    assert!(result.is_none());
}

#[rstest]
fn test_find_key_bitmap_slot_empty() {
    // Test line 1001: bitmap & bit == 0
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let query = ControlledHash::new(99, 0b00000_00010); // Different index

    let map = PersistentHashMap::singleton(key1.clone(), 10);
    let result = map.update(&query, |v| v + 1);
    assert!(result.is_none());
}

#[rstest]
fn test_find_key_bitmap_child_entry_not_matching() {
    // Test line 1009: child_key.borrow() != key
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00000_00010);
    let query = ControlledHash::new(99, 0b00000_00001); // Same index as key1

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 10)
        .insert(key2.clone(), 20);

    let result = map.update(&query, |v| v + 1);
    assert!(result.is_none());
}

#[rstest]
fn test_find_key_collision_not_found_update_returns_none() {
    // Test line 1022: key not found in collision entries
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let query = ControlledHash::new(99, 12345); // Same hash but not in collision

    let map = PersistentHashMap::new()
        .insert(key1.clone(), 10)
        .insert(key2.clone(), 20);

    let result = map.update(&query, |v| v + 1);
    assert!(result.is_none());
}

// =============================================================================
// Final Coverage Tests: Deep Subnode Operations
// =============================================================================

#[rstest]
fn test_remove_from_deep_subnode_simplifies_to_entry() {
    // Create a deep structure and remove to trigger simplification
    let key1 = ControlledHash::new(1, 0b00001_00001_00001);
    let key2 = ControlledHash::new(2, 0b00010_00001_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Remove key1, should simplify
    let map2 = map.remove(&key1);

    assert_eq!(map2.len(), 1);
    assert_eq!(map2.get(&key2), Some(&"two"));
}

#[rstest]
fn test_remove_from_deep_subnode_returns_empty() {
    let key1 = ControlledHash::new(1, 0b00001_00001_00001);
    let key2 = ControlledHash::new(2, 0b00010_00001_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    // Remove both
    let map2 = map.remove(&key1).remove(&key2);

    assert!(map2.is_empty());
}

// =============================================================================
// Final Coverage Tests: Bitmap with Multiple Node Children
// =============================================================================

#[rstest]
fn test_bitmap_multiple_node_children_removal() {
    // Create bitmap with multiple Node children
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    let key3 = ControlledHash::new(3, 0b00000_00010);
    let key4 = ControlledHash::new(4, 0b00001_00010);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three")
        .insert(key4.clone(), "four");

    // Remove from first subnode
    let map2 = map.remove(&key1);

    assert_eq!(map2.len(), 3);
    assert!(map2.get(&key1).is_none());
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
    assert_eq!(map2.get(&key4), Some(&"four"));
}

// =============================================================================
// Final Coverage Tests: Subnode Returns Not Removed
// =============================================================================

#[rstest]
fn test_subnode_remove_returns_not_removed() {
    // Create structure where remove_from_node on subnode returns Some(_, false)
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    // Query has same index at depth 0, but different path in subnode
    let query = ControlledHash::new(99, 0b00010_00001);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two");

    let result = map.remove(&query);

    // Should return clone since nothing was removed
    assert_eq!(result.len(), 2);
}

// =============================================================================
// Final Coverage Tests: Bitmap Simplification with Node Child
// =============================================================================

#[rstest]
fn test_bitmap_single_node_child_simplifies() {
    // Create a bitmap that has only one Node child after removal
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    let key3 = ControlledHash::new(3, 0b00000_00010);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove key3, leaving only subnode at index 1
    let map2 = map.remove(&key3);

    assert_eq!(map2.len(), 2);
    assert_eq!(map2.get(&key1), Some(&"one"));
    assert_eq!(map2.get(&key2), Some(&"two"));
}

#[rstest]
fn test_remove_causes_subnode_to_simplify_to_single_entry() {
    // Create structure where removing from subnode causes it to become single entry
    let key1 = ControlledHash::new(1, 0b00000_00001);
    let key2 = ControlledHash::new(2, 0b00001_00001);
    let key3 = ControlledHash::new(3, 0b00000_00010);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three");

    // Remove key1, subnode should simplify to single entry containing key2
    let map2 = map.remove(&key1);

    assert_eq!(map2.len(), 2);
    assert!(map2.get(&key1).is_none());
    assert_eq!(map2.get(&key2), Some(&"two"));
    assert_eq!(map2.get(&key3), Some(&"three"));
}

// =============================================================================
// Large Scale Coverage Tests: Stress Testing with Many Keys
// =============================================================================

#[rstest]
fn test_large_scale_insert_and_remove() {
    // Insert many keys to exercise all code paths statistically
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();

    // Insert 1000 keys
    for index in 0..1000 {
        map = map.insert(index, index * 10);
    }

    assert_eq!(map.len(), 1000);

    // Verify all keys
    for index in 0..1000 {
        assert_eq!(map.get(&index), Some(&(index * 10)));
    }

    // Remove half of the keys
    for index in (0..1000).step_by(2) {
        map = map.remove(&index);
    }

    assert_eq!(map.len(), 500);

    // Verify remaining keys
    for index in (1..1000).step_by(2) {
        assert_eq!(map.get(&index), Some(&(index * 10)));
    }

    // Remove remaining keys
    for index in (1..1000).step_by(2) {
        map = map.remove(&index);
    }

    assert!(map.is_empty());
}

#[rstest]
fn test_large_scale_collision_handling() {
    // Create many keys with controlled collisions
    let mut map: PersistentHashMap<ControlledHash, i32> = PersistentHashMap::new();

    // Insert keys with collision groups
    for group in 0..10 {
        let hash = (group * 1000) as u64;
        for index in 0..10 {
            let key = ControlledHash::new(group * 10 + index, hash);
            map = map.insert(key, group * 10 + index);
        }
    }

    assert_eq!(map.len(), 100);

    // Remove from each collision group
    for group in 0..10 {
        let hash = (group * 1000) as u64;
        for index in 0..5 {
            let key = ControlledHash::new(group * 10 + index, hash);
            map = map.remove(&key);
        }
    }

    assert_eq!(map.len(), 50);

    // Verify remaining keys
    for group in 0..10 {
        let hash = (group * 1000) as u64;
        for index in 5..10 {
            let key = ControlledHash::new(group * 10 + index, hash);
            assert_eq!(map.get(&key), Some(&(group * 10 + index)));
        }
    }
}

#[rstest]
fn test_large_scale_update() {
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();

    for index in 0..500 {
        map = map.insert(index, index);
    }

    // Update all keys
    for index in 0..500 {
        if let Some(updated) = map.update(&index, |v| v * 2) {
            map = updated;
        }
    }

    // Verify updates
    for index in 0..500 {
        assert_eq!(map.get(&index), Some(&(index * 2)));
    }
}

#[rstest]
fn test_mixed_operations_stress() {
    let mut map: PersistentHashMap<i32, String> = PersistentHashMap::new();

    // Interleave inserts, updates, and removes
    for index in 0..300 {
        map = map.insert(index, format!("value_{}", index));

        if index > 0 && index % 3 == 0 {
            // Remove every third key after some delay
            map = map.remove(&(index - 3));
        }

        if index > 5 && index % 5 == 0 {
            // Update every fifth key
            if let Some(updated) = map.update(&(index - 5), |v| format!("{}_updated", v)) {
                map = updated;
            }
        }
    }

    // Verify the map is in a consistent state
    let count = map.iter().count();
    assert_eq!(count, map.len());
}

#[rstest]
fn test_deep_tree_structure() {
    // Insert keys that will create deep tree structure
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();

    for index in 0..10000 {
        map = map.insert(index, index);
    }

    assert_eq!(map.len(), 10000);

    // Access random positions to ensure tree is navigable
    for index in (0..10000).step_by(100) {
        assert_eq!(map.get(&index), Some(&index));
    }

    // Remove in reverse order
    for index in (0..10000).rev() {
        map = map.remove(&index);
    }

    assert!(map.is_empty());
}

#[rstest]
fn test_collision_with_deep_recursion() {
    // Create collision groups at different depths
    let mut map: PersistentHashMap<ControlledHash, i32> = PersistentHashMap::new();

    // Group 1: same hash
    for index in 0..5 {
        map = map.insert(ControlledHash::new(index, 111111), index);
    }

    // Group 2: different first-level hash, creates separate subtree
    for index in 5..10 {
        map = map.insert(ControlledHash::new(index, 222222), index);
    }

    // Group 3: same hash as group 1 - more collision
    for index in 10..15 {
        map = map.insert(ControlledHash::new(index, 111111), index);
    }

    assert_eq!(map.len(), 15);

    // Remove from collision group
    for index in 0..5 {
        map = map.remove(&ControlledHash::new(index, 111111));
    }

    assert_eq!(map.len(), 10);
}

#[rstest]
fn test_merge_large_maps() {
    let mut map1: PersistentHashMap<i32, i32> = PersistentHashMap::new();
    let mut map2: PersistentHashMap<i32, i32> = PersistentHashMap::new();

    for index in 0..100 {
        map1 = map1.insert(index, index);
    }

    for index in 50..150 {
        map2 = map2.insert(index, index * 10);
    }

    let merged = map1.merge(&map2);

    // 0-49 from map1, 50-149 from map2
    assert_eq!(merged.len(), 150);

    for index in 0..50 {
        assert_eq!(merged.get(&index), Some(&index));
    }

    for index in 50..150 {
        assert_eq!(merged.get(&index), Some(&(index * 10)));
    }
}

#[rstest]
fn test_iterator_with_complex_structure() {
    let mut map: PersistentHashMap<i32, i32> = PersistentHashMap::new();

    for index in 0..500 {
        map = map.insert(index, index);
    }

    // Test iterator
    let sum: i32 = map.values().sum();
    assert_eq!(sum, (0..500).sum());

    let key_count = map.keys().count();
    assert_eq!(key_count, 500);

    let entry_count = map.iter().count();
    assert_eq!(entry_count, 500);
}

#[rstest]
fn test_from_iter_and_into_iter() {
    let entries: Vec<(i32, i32)> = (0..200).map(|index| (index, index * 2)).collect();
    let map: PersistentHashMap<i32, i32> = entries.clone().into_iter().collect();

    assert_eq!(map.len(), 200);

    let collected: Vec<(i32, i32)> = map.into_iter().collect();
    assert_eq!(collected.len(), 200);
}

// =============================================================================
// Phase 4: Collision Bucket Optimization Tests (SmallVec Spillover)
// =============================================================================

/// Tests collision node with exactly 4 entries (within SmallVec stack limit).
#[rstest]
fn test_collision_exactly_four_entries_within_stack() {
    let key1 = ControlledHash::new(1, 12345);
    let key2 = ControlledHash::new(2, 12345);
    let key3 = ControlledHash::new(3, 12345);
    let key4 = ControlledHash::new(4, 12345);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three")
        .insert(key4.clone(), "four");

    assert_eq!(map.len(), 4);
    assert_eq!(map.get(&key1), Some(&"one"));
    assert_eq!(map.get(&key2), Some(&"two"));
    assert_eq!(map.get(&key3), Some(&"three"));
    assert_eq!(map.get(&key4), Some(&"four"));
}

/// Tests collision node with 5+ entries (SmallVec spills to heap).
#[rstest]
fn test_collision_five_entries_heap_spillover() {
    let key1 = ControlledHash::new(1, 99999);
    let key2 = ControlledHash::new(2, 99999);
    let key3 = ControlledHash::new(3, 99999);
    let key4 = ControlledHash::new(4, 99999);
    let key5 = ControlledHash::new(5, 99999);

    let map = PersistentHashMap::new()
        .insert(key1.clone(), "one")
        .insert(key2.clone(), "two")
        .insert(key3.clone(), "three")
        .insert(key4.clone(), "four")
        .insert(key5.clone(), "five");

    assert_eq!(map.len(), 5);
    assert_eq!(map.get(&key1), Some(&"one"));
    assert_eq!(map.get(&key2), Some(&"two"));
    assert_eq!(map.get(&key3), Some(&"three"));
    assert_eq!(map.get(&key4), Some(&"four"));
    assert_eq!(map.get(&key5), Some(&"five"));
}

/// Tests collision node with 8 entries (well above SmallVec limit).
#[rstest]
fn test_collision_eight_entries_large_spillover() {
    let entries: Vec<_> = (1..=8)
        .map(|index| ControlledHash::new(index, 77777))
        .collect();

    let mut map = PersistentHashMap::new();
    for (index, key) in entries.iter().enumerate() {
        map = map.insert(key.clone(), (index + 1) as i32 * 10);
    }

    assert_eq!(map.len(), 8);
    for (index, key) in entries.iter().enumerate() {
        assert_eq!(map.get(key), Some(&((index + 1) as i32 * 10)));
    }
}

/// Tests updating a key in a large collision node (>4 entries).
#[rstest]
fn test_collision_update_in_large_bucket() {
    let entries: Vec<_> = (1..=6)
        .map(|index| ControlledHash::new(index, 55555))
        .collect();

    let mut map = PersistentHashMap::new();
    for (index, key) in entries.iter().enumerate() {
        map = map.insert(key.clone(), index as i32);
    }

    // Update an existing key
    map = map.insert(entries[2].clone(), 999);

    assert_eq!(map.len(), 6);
    assert_eq!(map.get(&entries[2]), Some(&999));

    // Other entries unchanged
    assert_eq!(map.get(&entries[0]), Some(&0));
    assert_eq!(map.get(&entries[5]), Some(&5));
}

/// Tests removing entries from a large collision node.
#[rstest]
fn test_collision_remove_from_large_bucket() {
    let entries: Vec<_> = (1..=6)
        .map(|index| ControlledHash::new(index, 44444))
        .collect();

    let mut map = PersistentHashMap::new();
    for (index, key) in entries.iter().enumerate() {
        map = map.insert(key.clone(), index as i32);
    }

    // Remove middle entry
    map = map.remove(&entries[3]);

    assert_eq!(map.len(), 5);
    assert!(map.get(&entries[3]).is_none());
    assert_eq!(map.get(&entries[0]), Some(&0));
    assert_eq!(map.get(&entries[5]), Some(&5));
}

/// Tests that collision node preserves immutability with large buckets.
#[rstest]
fn test_collision_immutability_large_bucket() {
    let entries: Vec<_> = (1..=5)
        .map(|index| ControlledHash::new(index, 33333))
        .collect();

    let mut map1 = PersistentHashMap::new();
    for (index, key) in entries.iter().enumerate() {
        map1 = map1.insert(key.clone(), index as i32);
    }

    let map1_snapshot = map1.clone();

    // Modify map1
    let map2 = map1.insert(entries[0].clone(), 100);
    let map3 = map2.remove(&entries[1]);

    // Original snapshot should be unchanged
    assert_eq!(map1_snapshot.len(), 5);
    assert_eq!(map1_snapshot.get(&entries[0]), Some(&0));
    assert_eq!(map1_snapshot.get(&entries[1]), Some(&1));

    // Modified maps should reflect changes
    assert_eq!(map2.get(&entries[0]), Some(&100));
    assert_eq!(map3.len(), 4);
    assert!(map3.get(&entries[1]).is_none());
}

/// Tests iterating over a large collision node.
#[rstest]
fn test_collision_iteration_large_bucket() {
    let entries: Vec<_> = (1..=7)
        .map(|index| ControlledHash::new(index, 22222))
        .collect();

    let mut map = PersistentHashMap::new();
    for (index, key) in entries.iter().enumerate() {
        map = map.insert(key.clone(), (index + 1) as i32);
    }

    let sum: i32 = map.values().sum();
    assert_eq!(sum, 1 + 2 + 3 + 4 + 5 + 6 + 7);
}

/// Tests removing entries until collision node becomes a single Entry node.
#[rstest]
fn test_collision_reduce_to_single_entry() {
    let entries: Vec<_> = (1..=5)
        .map(|index| ControlledHash::new(index, 11111))
        .collect();

    let mut map = PersistentHashMap::new();
    for key in &entries {
        map = map.insert(key.clone(), 1);
    }

    // Remove all but one
    for key in entries.iter().skip(1) {
        map = map.remove(key);
    }

    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&entries[0]), Some(&1));
}

/// Tests contains_key on a large collision node.
#[rstest]
fn test_collision_contains_key_large_bucket() {
    let entries: Vec<_> = (1..=6)
        .map(|index| ControlledHash::new(index, 88888))
        .collect();
    let missing_key = ControlledHash::new(100, 88888);

    let mut map = PersistentHashMap::new();
    for key in &entries {
        map = map.insert(key.clone(), 1);
    }

    for key in &entries {
        assert!(map.contains_key(key));
    }
    assert!(!map.contains_key(&missing_key));
}

// =============================================================================
// TASK-010: insert_without_cow Structural Sharing Tests
//
// These tests verify that:
// 1. insert_without_cow and insert produce equivalent results
// 2. transient modifications do not affect the original PersistentHashMap
// 3. Generation tokens remain consistent during operations
// 4. In-place updates only occur when the root is exclusively owned
// =============================================================================

use lambars::persistent::TransientHashMap;

/// Tests that structural sharing is correctly maintained.
///
/// When a PersistentHashMap is converted to a TransientHashMap and modified,
/// the original PersistentHashMap should remain unchanged.
#[rstest]
fn test_insert_without_cow_structural_sharing() {
    // Create a persistent map with some entries
    let persistent: PersistentHashMap<String, i32> = (0..100)
        .map(|index| (format!("key_{index}"), index))
        .collect();

    // Clone before creating transient to verify structural sharing
    let persistent_clone = persistent.clone();

    // Create transient and modify via insert_without_cow
    let mut transient = persistent.transient();
    for index in 100..150 {
        transient.insert_without_cow(format!("key_{index}"), index);
    }

    // Convert back to persistent
    let result = transient.persistent();

    // Original persistent should be unchanged
    assert_eq!(persistent_clone.len(), 100);
    for index in 0..100 {
        assert_eq!(persistent_clone.get(&format!("key_{index}")), Some(&index));
    }

    // Result should have all entries
    assert_eq!(result.len(), 150);
    for index in 0..150 {
        assert_eq!(result.get(&format!("key_{index}")), Some(&index));
    }
}

/// Tests that transient modifications are isolated from the original map.
///
/// Modifications via insert_without_cow on a transient should not affect
/// any PersistentHashMap instances derived from the same source.
#[rstest]
fn test_transient_isolation() {
    // Create a base persistent map
    let base: PersistentHashMap<String, i32> = vec![
        ("a".to_string(), 1),
        ("b".to_string(), 2),
        ("c".to_string(), 3),
    ]
    .into_iter()
    .collect();

    // Create two transients from the same base
    let mut transient1 = base.clone().transient();
    let mut transient2 = base.transient();

    // Modify each transient differently using insert_without_cow
    transient1.insert_without_cow("a".to_string(), 100);
    transient1.insert_without_cow("d".to_string(), 4);

    transient2.insert_without_cow("b".to_string(), 200);
    transient2.insert_without_cow("e".to_string(), 5);

    // Convert back to persistent
    let result1 = transient1.persistent();
    let result2 = transient2.persistent();

    // Results should be independent
    assert_eq!(result1.get("a"), Some(&100));
    assert_eq!(result1.get("d"), Some(&4));
    assert_eq!(result1.get("b"), Some(&2)); // Unchanged from original

    assert_eq!(result2.get("b"), Some(&200));
    assert_eq!(result2.get("e"), Some(&5));
    assert_eq!(result2.get("a"), Some(&1)); // Unchanged from original
}

/// Tests that generation tokens remain consistent during operations.
///
/// After multiple insert_without_cow operations, converting to persistent
/// and back to transient should maintain correct behavior.
#[rstest]
fn test_generation_consistency() {
    // Create initial transient
    let mut transient: TransientHashMap<i32, i32> = TransientHashMap::new();

    // Insert many entries to create a complex tree structure
    for index in 0..200 {
        transient.insert_without_cow(index, index * 10);
    }

    // Convert to persistent and back to transient
    let persistent = transient.persistent();
    let mut transient2 = persistent.transient();

    // Continue modifying
    for index in 200..300 {
        transient2.insert_without_cow(index, index * 10);
    }

    // Update some existing keys
    for index in 0..50 {
        transient2.insert_without_cow(index, index * 100);
    }

    // Final result should have all entries correctly
    let result = transient2.persistent();
    assert_eq!(result.len(), 300);

    // Verify updated values
    for index in 0..50 {
        assert_eq!(result.get(&index), Some(&(index * 100)));
    }

    // Verify unchanged values
    for index in 50..200 {
        assert_eq!(result.get(&index), Some(&(index * 10)));
    }

    // Verify new values
    for index in 200..300 {
        assert_eq!(result.get(&index), Some(&(index * 10)));
    }
}

/// Tests that in-place updates only occur when the root is exclusively owned.
///
/// When multiple references exist to the root (shared ownership), the fallback
/// COW path should be used to maintain referential transparency.
#[rstest]
fn test_insert_without_cow_inplace_only_on_exclusive() {
    // Create a persistent map
    let persistent: PersistentHashMap<String, i32> =
        vec![("existing".to_string(), 100)].into_iter().collect();

    // Keep a reference to the persistent map
    let persistent_clone = persistent.clone();

    // Create transient - the root is now shared between persistent_clone and transient
    let mut transient = persistent.transient();

    // insert_without_cow should handle shared root via fallback path
    transient.insert_without_cow("new".to_string(), 200);
    transient.insert_without_cow("existing".to_string(), 150);

    let result = transient.persistent();

    // Original clone should be unchanged (structural sharing preserved)
    assert_eq!(persistent_clone.len(), 1);
    assert_eq!(persistent_clone.get("existing"), Some(&100));

    // Result should have modifications
    assert_eq!(result.len(), 2);
    assert_eq!(result.get("existing"), Some(&150));
    assert_eq!(result.get("new"), Some(&200));
}
