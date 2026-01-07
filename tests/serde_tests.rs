#![cfg(feature = "serde")]

//! Integration tests for serde support in lambars.
//!
//! These tests verify that all data structures correctly serialize and deserialize
//! with various serde formats.

use lambars::control::Either;
use lambars::persistent::{
    PersistentHashMap, PersistentHashSet, PersistentList, PersistentTreeMap, PersistentVector,
};
use rstest::rstest;

// =============================================================================
// Either Integration Tests
// =============================================================================

#[rstest]
fn test_either_json_roundtrip() {
    let left: Either<String, i32> = Either::Left("error".to_string());
    let right: Either<String, i32> = Either::Right(42);

    let left_json = serde_json::to_string(&left).unwrap();
    let right_json = serde_json::to_string(&right).unwrap();

    let restored_left: Either<String, i32> = serde_json::from_str(&left_json).unwrap();
    let restored_right: Either<String, i32> = serde_json::from_str(&right_json).unwrap();

    assert_eq!(left, restored_left);
    assert_eq!(right, restored_right);
}

// =============================================================================
// PersistentList Integration Tests
// =============================================================================

#[rstest]
fn test_list_json_roundtrip() {
    let list: PersistentList<i32> = (1..=10).collect();
    let json = serde_json::to_string(&list).unwrap();
    let restored: PersistentList<i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(list, restored);
}

#[rstest]
fn test_list_nested_structures() {
    let inner1: PersistentList<i32> = (1..=3).collect();
    let inner2: PersistentList<i32> = (4..=6).collect();
    let outer: PersistentList<PersistentList<i32>> = vec![inner1, inner2].into_iter().collect();

    let json = serde_json::to_string(&outer).unwrap();
    let restored: PersistentList<PersistentList<i32>> = serde_json::from_str(&json).unwrap();

    assert_eq!(outer.len(), restored.len());
    for (original, restored_inner) in outer.iter().zip(restored.iter()) {
        assert_eq!(original, restored_inner);
    }
}

// =============================================================================
// PersistentVector Integration Tests
// =============================================================================

#[rstest]
fn test_vector_json_roundtrip() {
    let vector: PersistentVector<i32> = (1..=100).collect();
    let json = serde_json::to_string(&vector).unwrap();
    let restored: PersistentVector<i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(vector, restored);
}

#[rstest]
fn test_vector_nested_structures() {
    let inner1: PersistentVector<i32> = (1..=3).collect();
    let inner2: PersistentVector<i32> = (4..=6).collect();
    let outer: PersistentVector<PersistentVector<i32>> = vec![inner1, inner2].into_iter().collect();

    let json = serde_json::to_string(&outer).unwrap();
    let restored: PersistentVector<PersistentVector<i32>> = serde_json::from_str(&json).unwrap();

    assert_eq!(outer, restored);
}

// =============================================================================
// PersistentHashSet Integration Tests
// =============================================================================

#[rstest]
fn test_hashset_json_roundtrip() {
    let set: PersistentHashSet<i32> = (1..=100).collect();
    let json = serde_json::to_string(&set).unwrap();
    let restored: PersistentHashSet<i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(set, restored);
}

#[rstest]
fn test_hashset_with_strings() {
    let set: PersistentHashSet<String> = ["hello", "world", "rust"]
        .into_iter()
        .map(String::from)
        .collect();

    let json = serde_json::to_string(&set).unwrap();
    let restored: PersistentHashSet<String> = serde_json::from_str(&json).unwrap();
    assert_eq!(set, restored);
}

// =============================================================================
// PersistentHashMap Integration Tests
// =============================================================================

#[rstest]
fn test_hashmap_json_roundtrip() {
    let mut map: PersistentHashMap<String, i32> = PersistentHashMap::new();
    for element_index in 0..100 {
        map = map.insert(format!("key{element_index}"), element_index);
    }
    let json = serde_json::to_string(&map).unwrap();
    let restored: PersistentHashMap<String, i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(map, restored);
}

#[rstest]
fn test_hashmap_with_nested_values() {
    let map = PersistentHashMap::new()
        .insert("list".to_string(), (1..=3).collect::<Vec<_>>())
        .insert("empty".to_string(), vec![]);

    let json = serde_json::to_string(&map).unwrap();
    let restored: PersistentHashMap<String, Vec<i32>> = serde_json::from_str(&json).unwrap();
    assert_eq!(map.get("list"), restored.get("list"));
    assert_eq!(map.get("empty"), restored.get("empty"));
}

// =============================================================================
// PersistentTreeMap Integration Tests
// =============================================================================

#[rstest]
fn test_treemap_json_roundtrip() {
    let mut map: PersistentTreeMap<String, i32> = PersistentTreeMap::new();
    for element_index in 0..100 {
        map = map.insert(format!("key{element_index:03}"), element_index);
    }
    let json = serde_json::to_string(&map).unwrap();
    let restored: PersistentTreeMap<String, i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(map, restored);
}

#[rstest]
fn test_treemap_preserves_order_in_json() {
    let map = PersistentTreeMap::new()
        .insert("c".to_string(), 3)
        .insert("a".to_string(), 1)
        .insert("b".to_string(), 2);

    let json = serde_json::to_string(&map).unwrap();
    assert_eq!(json, r#"{"a":1,"b":2,"c":3}"#);
}

// =============================================================================
// Cross-type Integration Tests
// =============================================================================

#[rstest]
fn test_either_with_persistent_structures() {
    let left: Either<PersistentList<i32>, PersistentVector<i32>> = Either::Left((1..=5).collect());
    let right: Either<PersistentList<i32>, PersistentVector<i32>> =
        Either::Right((6..=10).collect());

    let left_json = serde_json::to_string(&left).unwrap();
    let right_json = serde_json::to_string(&right).unwrap();

    let restored_left: Either<PersistentList<i32>, PersistentVector<i32>> =
        serde_json::from_str(&left_json).unwrap();
    let restored_right: Either<PersistentList<i32>, PersistentVector<i32>> =
        serde_json::from_str(&right_json).unwrap();

    assert_eq!(left, restored_left);
    assert_eq!(right, restored_right);
}

#[rstest]
fn test_map_with_set_values() {
    let set1: PersistentHashSet<i32> = (1..=3).collect();
    let set2: PersistentHashSet<i32> = (4..=6).collect();

    let map = PersistentHashMap::new()
        .insert("first".to_string(), set1.clone())
        .insert("second".to_string(), set2.clone());

    let json = serde_json::to_string(&map).unwrap();
    let restored: PersistentHashMap<String, PersistentHashSet<i32>> =
        serde_json::from_str(&json).unwrap();

    assert_eq!(map.get("first"), restored.get("first"));
    assert_eq!(map.get("second"), restored.get("second"));
}

#[rstest]
fn test_treemap_with_vector_values() {
    let map = PersistentTreeMap::new()
        .insert(
            "first".to_string(),
            (1..=3).collect::<PersistentVector<i32>>(),
        )
        .insert(
            "second".to_string(),
            (4..=6).collect::<PersistentVector<i32>>(),
        );

    let json = serde_json::to_string(&map).unwrap();
    let restored: PersistentTreeMap<String, PersistentVector<i32>> =
        serde_json::from_str(&json).unwrap();

    assert_eq!(map, restored);
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[rstest]
fn test_empty_structures() {
    let empty_list: PersistentList<i32> = PersistentList::new();
    let empty_vector: PersistentVector<i32> = PersistentVector::new();
    let empty_set: PersistentHashSet<i32> = PersistentHashSet::new();
    let empty_hashmap: PersistentHashMap<String, i32> = PersistentHashMap::new();
    let empty_treemap: PersistentTreeMap<String, i32> = PersistentTreeMap::new();

    assert_eq!(serde_json::to_string(&empty_list).unwrap(), "[]");
    assert_eq!(serde_json::to_string(&empty_vector).unwrap(), "[]");
    assert_eq!(serde_json::to_string(&empty_set).unwrap(), "[]");
    assert_eq!(serde_json::to_string(&empty_hashmap).unwrap(), "{}");
    assert_eq!(serde_json::to_string(&empty_treemap).unwrap(), "{}");
}

#[rstest]
fn test_singleton_structures() {
    let list = PersistentList::singleton(42);
    let vector = PersistentVector::singleton(42);
    let set = PersistentHashSet::singleton(42);
    let hashmap = PersistentHashMap::singleton("key".to_string(), 42);
    let treemap = PersistentTreeMap::singleton("key".to_string(), 42);

    assert_eq!(serde_json::to_string(&list).unwrap(), "[42]");
    assert_eq!(serde_json::to_string(&vector).unwrap(), "[42]");
    assert_eq!(serde_json::to_string(&set).unwrap(), "[42]");
    assert_eq!(serde_json::to_string(&hashmap).unwrap(), r#"{"key":42}"#);
    assert_eq!(serde_json::to_string(&treemap).unwrap(), r#"{"key":42}"#);
}

// =============================================================================
// Type Mismatch Error Tests (for expecting() coverage)
// =============================================================================

#[rstest]
fn test_list_type_mismatch_error() {
    let json = r#""not an array""#;
    let result: Result<PersistentList<i32>, _> = serde_json::from_str(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("a sequence"));
}

#[rstest]
fn test_vector_type_mismatch_error() {
    let json = r#"{"key": "value"}"#;
    let result: Result<PersistentVector<i32>, _> = serde_json::from_str(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("a sequence"));
}

#[rstest]
fn test_hashset_type_mismatch_error() {
    let json = r#"42"#;
    let result: Result<PersistentHashSet<i32>, _> = serde_json::from_str(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("a sequence"));
}

#[rstest]
fn test_hashmap_type_mismatch_error() {
    let json = r#"[1, 2, 3]"#;
    let result: Result<PersistentHashMap<String, i32>, _> = serde_json::from_str(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("a map"));
}

#[rstest]
fn test_treemap_type_mismatch_error() {
    let json = r#""not a map""#;
    let result: Result<PersistentTreeMap<String, i32>, _> = serde_json::from_str(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("a map"));
}
