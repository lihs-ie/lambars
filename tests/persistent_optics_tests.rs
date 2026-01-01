#![cfg(feature = "optics")]
//! Tests for persistent data structure optics integration.
//!
//! This module tests the integration between Optics (Optional, Traversal)
//! and persistent data structures (PersistentVector, PersistentHashMap, PersistentTreeMap).

#![forbid(unsafe_code)]

use lambars::optics::persistent_optics::{
    index_optional, key_optional_hashmap, key_optional_treemap, persistent_hashmap_traversal,
    persistent_treemap_traversal, persistent_vector_traversal,
};
use lambars::optics::{Optional, Traversal};
use lambars::persistent::{PersistentHashMap, PersistentTreeMap, PersistentVector};
use rstest::rstest;

// =============================================================================
// PersistentVector Optional Tests
// =============================================================================

mod persistent_vector_optional {
    use super::*;

    #[rstest]
    fn test_get_option_returns_some_for_valid_index() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = index_optional::<i32>(2);

        assert_eq!(optional.get_option(&vector), Some(&3));
    }

    #[rstest]
    fn test_get_option_returns_none_for_out_of_bounds_index() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = index_optional::<i32>(10);

        assert_eq!(optional.get_option(&vector), None);
    }

    #[rstest]
    fn test_get_option_returns_none_for_empty_vector() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let optional = index_optional::<i32>(0);

        assert_eq!(optional.get_option(&vector), None);
    }

    #[rstest]
    fn test_set_updates_element_at_valid_index() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = index_optional::<i32>(2);

        let updated = optional.set(vector.clone(), 100);

        assert_eq!(updated.get(2), Some(&100));
        // Original is unchanged
        assert_eq!(vector.get(2), Some(&3));
    }

    #[rstest]
    fn test_set_returns_unchanged_for_out_of_bounds_index() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = index_optional::<i32>(10);

        let result = optional.set(vector.clone(), 100);

        // Should return the original vector unchanged
        assert_eq!(result, vector);
    }

    #[rstest]
    fn test_modify_option_modifies_existing_element() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = index_optional::<i32>(1);

        let result = optional.modify_option(vector, |x| x * 10);

        assert!(result.is_some());
        let updated = result.unwrap();
        assert_eq!(updated.get(1), Some(&20));
    }

    #[rstest]
    fn test_modify_option_returns_none_for_out_of_bounds() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = index_optional::<i32>(10);

        let result = optional.modify_option(vector, |x| x * 10);

        assert!(result.is_none());
    }

    #[rstest]
    fn test_is_present_true_for_valid_index() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = index_optional::<i32>(0);

        assert!(optional.is_present(&vector));
    }

    #[rstest]
    fn test_is_present_false_for_invalid_index() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional = index_optional::<i32>(100);

        assert!(!optional.is_present(&vector));
    }

    #[rstest]
    fn test_first_element_access() {
        let vector: PersistentVector<i32> = (10..=15).collect();
        let optional = index_optional::<i32>(0);

        assert_eq!(optional.get_option(&vector), Some(&10));
    }

    #[rstest]
    fn test_last_element_access() {
        let vector: PersistentVector<i32> = (10..=15).collect();
        let optional = index_optional::<i32>(5);

        assert_eq!(optional.get_option(&vector), Some(&15));
    }
}

// =============================================================================
// PersistentVector Traversal Tests
// =============================================================================

mod persistent_vector_traversal {
    use super::*;

    #[rstest]
    fn test_get_all_returns_all_elements() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = persistent_vector_traversal::<i32>();

        let elements: Vec<&i32> = traversal.get_all(&vector).collect();

        assert_eq!(elements, vec![&1, &2, &3, &4, &5]);
    }

    #[rstest]
    fn test_get_all_returns_empty_for_empty_vector() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let traversal = persistent_vector_traversal::<i32>();

        let elements: Vec<&i32> = traversal.get_all(&vector).collect();

        assert!(elements.is_empty());
    }

    #[rstest]
    fn test_modify_all_doubles_all_elements() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = persistent_vector_traversal::<i32>();

        let doubled = traversal.modify_all(vector, |x| x * 2);

        let elements: Vec<&i32> = doubled.iter().collect();
        assert_eq!(elements, vec![&2, &4, &6, &8, &10]);
    }

    #[rstest]
    fn test_modify_all_on_empty_vector() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let traversal = persistent_vector_traversal::<i32>();

        let result = traversal.modify_all(vector, |x| x * 2);

        assert!(result.is_empty());
    }

    #[rstest]
    fn test_set_all_sets_all_elements_to_same_value() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = persistent_vector_traversal::<i32>();

        let result = traversal.set_all(vector, 0);

        let elements: Vec<&i32> = result.iter().collect();
        assert_eq!(elements, vec![&0, &0, &0, &0, &0]);
    }

    #[rstest]
    fn test_fold_sums_all_elements() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = persistent_vector_traversal::<i32>();

        let sum = traversal.fold(&vector, 0, |accumulator, element| accumulator + element);

        assert_eq!(sum, 15);
    }

    #[rstest]
    fn test_length_returns_element_count() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = persistent_vector_traversal::<i32>();

        assert_eq!(traversal.length(&vector), 5);
    }

    #[rstest]
    fn test_for_all_checks_all_elements() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = persistent_vector_traversal::<i32>();

        assert!(traversal.for_all(&vector, |x| *x > 0));
        assert!(!traversal.for_all(&vector, |x| *x > 3));
    }

    #[rstest]
    fn test_exists_checks_any_element() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let traversal = persistent_vector_traversal::<i32>();

        assert!(traversal.exists(&vector, |x| *x == 3));
        assert!(!traversal.exists(&vector, |x| *x == 10));
    }

    #[rstest]
    fn test_head_option_returns_first() {
        let vector: PersistentVector<i32> = (10..=15).collect();
        let traversal = persistent_vector_traversal::<i32>();

        assert_eq!(traversal.head_option(&vector), Some(&10));
    }

    #[rstest]
    fn test_head_option_returns_none_for_empty() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let traversal = persistent_vector_traversal::<i32>();

        assert_eq!(traversal.head_option(&vector), None);
    }
}

// =============================================================================
// PersistentHashMap Optional Tests
// =============================================================================

mod persistent_hashmap_optional {
    use super::*;

    #[rstest]
    fn test_get_option_returns_some_for_existing_key() {
        let map = PersistentHashMap::new()
            .insert("one".to_string(), 1)
            .insert("two".to_string(), 2);
        let optional = key_optional_hashmap::<String, i32>("one".to_string());

        assert_eq!(optional.get_option(&map), Some(&1));
    }

    #[rstest]
    fn test_get_option_returns_none_for_missing_key() {
        let map = PersistentHashMap::new()
            .insert("one".to_string(), 1)
            .insert("two".to_string(), 2);
        let optional = key_optional_hashmap::<String, i32>("three".to_string());

        assert_eq!(optional.get_option(&map), None);
    }

    #[rstest]
    fn test_get_option_returns_none_for_empty_map() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let optional = key_optional_hashmap::<String, i32>("any".to_string());

        assert_eq!(optional.get_option(&map), None);
    }

    #[rstest]
    fn test_set_updates_existing_key() {
        let map = PersistentHashMap::new()
            .insert("one".to_string(), 1)
            .insert("two".to_string(), 2);
        let optional = key_optional_hashmap::<String, i32>("one".to_string());

        let updated = optional.set(map.clone(), 100);

        assert_eq!(updated.get("one"), Some(&100));
        // Original is unchanged
        assert_eq!(map.get("one"), Some(&1));
    }

    #[rstest]
    fn test_set_inserts_for_missing_key() {
        let map = PersistentHashMap::new().insert("one".to_string(), 1);
        let optional = key_optional_hashmap::<String, i32>("two".to_string());

        let updated = optional.set(map, 2);

        assert_eq!(updated.get("two"), Some(&2));
        assert_eq!(updated.len(), 2);
    }

    #[rstest]
    fn test_modify_option_modifies_existing_value() {
        let map = PersistentHashMap::new().insert("count".to_string(), 10);
        let optional = key_optional_hashmap::<String, i32>("count".to_string());

        let result = optional.modify_option(map, |x| x + 5);

        assert!(result.is_some());
        assert_eq!(result.unwrap().get("count"), Some(&15));
    }

    #[rstest]
    fn test_modify_option_returns_none_for_missing_key() {
        let map = PersistentHashMap::new().insert("count".to_string(), 10);
        let optional = key_optional_hashmap::<String, i32>("missing".to_string());

        let result = optional.modify_option(map, |x| x + 5);

        assert!(result.is_none());
    }

    #[rstest]
    fn test_is_present_true_for_existing_key() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);
        let optional = key_optional_hashmap::<String, i32>("key".to_string());

        assert!(optional.is_present(&map));
    }

    #[rstest]
    fn test_is_present_false_for_missing_key() {
        let map = PersistentHashMap::new().insert("key".to_string(), 42);
        let optional = key_optional_hashmap::<String, i32>("missing".to_string());

        assert!(!optional.is_present(&map));
    }
}

// =============================================================================
// PersistentHashMap Traversal Tests
// =============================================================================

mod persistent_hashmap_traversal {
    use super::*;

    #[rstest]
    fn test_get_all_returns_all_values() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let traversal = persistent_hashmap_traversal::<String, i32>();

        let mut values: Vec<i32> = traversal.get_all(&map).copied().collect();
        values.sort();

        assert_eq!(values, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_get_all_returns_empty_for_empty_map() {
        let map: PersistentHashMap<String, i32> = PersistentHashMap::new();
        let traversal = persistent_hashmap_traversal::<String, i32>();

        let values: Vec<&i32> = traversal.get_all(&map).collect();

        assert!(values.is_empty());
    }

    #[rstest]
    fn test_modify_all_doubles_all_values() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let traversal = persistent_hashmap_traversal::<String, i32>();

        let doubled = traversal.modify_all(map, |x| x * 2);

        assert_eq!(doubled.get("a"), Some(&2));
        assert_eq!(doubled.get("b"), Some(&4));
        assert_eq!(doubled.get("c"), Some(&6));
    }

    #[rstest]
    fn test_fold_sums_all_values() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let traversal = persistent_hashmap_traversal::<String, i32>();

        let sum = traversal.fold(&map, 0, |accumulator, value| accumulator + value);

        assert_eq!(sum, 6);
    }

    #[rstest]
    fn test_length_returns_entry_count() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2);
        let traversal = persistent_hashmap_traversal::<String, i32>();

        assert_eq!(traversal.length(&map), 2);
    }

    #[rstest]
    fn test_for_all_checks_all_values() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 5)
            .insert("b".to_string(), 10)
            .insert("c".to_string(), 15);
        let traversal = persistent_hashmap_traversal::<String, i32>();

        assert!(traversal.for_all(&map, |x| *x > 0));
        assert!(traversal.for_all(&map, |x| *x % 5 == 0));
        assert!(!traversal.for_all(&map, |x| *x > 10));
    }

    #[rstest]
    fn test_exists_checks_any_value() {
        let map = PersistentHashMap::new()
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .insert("c".to_string(), 3);
        let traversal = persistent_hashmap_traversal::<String, i32>();

        assert!(traversal.exists(&map, |x| *x == 2));
        assert!(!traversal.exists(&map, |x| *x == 10));
    }
}

// =============================================================================
// PersistentTreeMap Optional Tests
// =============================================================================

mod persistent_treemap_optional {
    use super::*;

    #[rstest]
    fn test_get_option_returns_some_for_existing_key() {
        let map = PersistentTreeMap::new().insert(1, "one").insert(2, "two");
        let optional = key_optional_treemap::<i32, &str>(1);

        assert_eq!(optional.get_option(&map), Some(&"one"));
    }

    #[rstest]
    fn test_get_option_returns_none_for_missing_key() {
        let map = PersistentTreeMap::new().insert(1, "one").insert(2, "two");
        let optional = key_optional_treemap::<i32, &str>(3);

        assert_eq!(optional.get_option(&map), None);
    }

    #[rstest]
    fn test_get_option_returns_none_for_empty_map() {
        let map: PersistentTreeMap<i32, &str> = PersistentTreeMap::new();
        let optional = key_optional_treemap::<i32, &str>(1);

        assert_eq!(optional.get_option(&map), None);
    }

    #[rstest]
    fn test_set_updates_existing_key() {
        let map = PersistentTreeMap::new().insert(1, "one").insert(2, "two");
        let optional = key_optional_treemap::<i32, &str>(1);

        let updated = optional.set(map.clone(), "ONE");

        assert_eq!(updated.get(&1), Some(&"ONE"));
        // Original is unchanged
        assert_eq!(map.get(&1), Some(&"one"));
    }

    #[rstest]
    fn test_set_inserts_for_missing_key() {
        let map = PersistentTreeMap::new().insert(1, "one");
        let optional = key_optional_treemap::<i32, &str>(2);

        let updated = optional.set(map, "two");

        assert_eq!(updated.get(&2), Some(&"two"));
        assert_eq!(updated.len(), 2);
    }

    #[rstest]
    fn test_modify_option_modifies_existing_value() {
        let map = PersistentTreeMap::new().insert(1, 10);
        let optional = key_optional_treemap::<i32, i32>(1);

        let result = optional.modify_option(map, |x| x * 2);

        assert!(result.is_some());
        assert_eq!(result.unwrap().get(&1), Some(&20));
    }

    #[rstest]
    fn test_modify_option_returns_none_for_missing_key() {
        let map = PersistentTreeMap::new().insert(1, 10);
        let optional = key_optional_treemap::<i32, i32>(2);

        let result = optional.modify_option(map, |x| x * 2);

        assert!(result.is_none());
    }

    #[rstest]
    fn test_is_present_true_for_existing_key() {
        let map = PersistentTreeMap::new().insert(42, "answer");
        let optional = key_optional_treemap::<i32, &str>(42);

        assert!(optional.is_present(&map));
    }

    #[rstest]
    fn test_is_present_false_for_missing_key() {
        let map = PersistentTreeMap::new().insert(42, "answer");
        let optional = key_optional_treemap::<i32, &str>(99);

        assert!(!optional.is_present(&map));
    }
}

// =============================================================================
// PersistentTreeMap Traversal Tests
// =============================================================================

mod persistent_treemap_traversal {
    use super::*;

    #[rstest]
    fn test_get_all_returns_all_values_in_key_order() {
        let map = PersistentTreeMap::new()
            .insert(3, "three")
            .insert(1, "one")
            .insert(2, "two");
        let traversal = persistent_treemap_traversal::<i32, &str>();

        let values: Vec<&&str> = traversal.get_all(&map).collect();

        // Values should be in key order (1, 2, 3)
        assert_eq!(values, vec![&"one", &"two", &"three"]);
    }

    #[rstest]
    fn test_get_all_returns_empty_for_empty_map() {
        let map: PersistentTreeMap<i32, &str> = PersistentTreeMap::new();
        let traversal = persistent_treemap_traversal::<i32, &str>();

        let values: Vec<&&str> = traversal.get_all(&map).collect();

        assert!(values.is_empty());
    }

    #[rstest]
    fn test_modify_all_transforms_all_values() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);
        let traversal = persistent_treemap_traversal::<i32, i32>();

        let doubled = traversal.modify_all(map, |x| x * 2);

        assert_eq!(doubled.get(&1), Some(&20));
        assert_eq!(doubled.get(&2), Some(&40));
        assert_eq!(doubled.get(&3), Some(&60));
    }

    #[rstest]
    fn test_fold_sums_all_values() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);
        let traversal = persistent_treemap_traversal::<i32, i32>();

        let sum = traversal.fold(&map, 0, |accumulator, value| accumulator + value);

        assert_eq!(sum, 60);
    }

    #[rstest]
    fn test_length_returns_entry_count() {
        let map = PersistentTreeMap::new()
            .insert(1, "one")
            .insert(2, "two")
            .insert(3, "three");
        let traversal = persistent_treemap_traversal::<i32, &str>();

        assert_eq!(traversal.length(&map), 3);
    }

    #[rstest]
    fn test_for_all_checks_all_values() {
        let map = PersistentTreeMap::new()
            .insert(1, 5)
            .insert(2, 10)
            .insert(3, 15);
        let traversal = persistent_treemap_traversal::<i32, i32>();

        assert!(traversal.for_all(&map, |x| *x > 0));
        assert!(traversal.for_all(&map, |x| *x % 5 == 0));
        assert!(!traversal.for_all(&map, |x| *x > 10));
    }

    #[rstest]
    fn test_exists_checks_any_value() {
        let map = PersistentTreeMap::new()
            .insert(1, 10)
            .insert(2, 20)
            .insert(3, 30);
        let traversal = persistent_treemap_traversal::<i32, i32>();

        assert!(traversal.exists(&map, |x| *x == 20));
        assert!(!traversal.exists(&map, |x| *x == 100));
    }

    #[rstest]
    fn test_head_option_returns_value_of_minimum_key() {
        let map = PersistentTreeMap::new()
            .insert(3, "three")
            .insert(1, "one")
            .insert(2, "two");
        let traversal = persistent_treemap_traversal::<i32, &str>();

        // First value should be for key 1
        assert_eq!(traversal.head_option(&map), Some(&"one"));
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

mod integration {
    use super::*;

    #[rstest]
    fn test_vector_optional_and_traversal_compose() {
        // Create a vector of vectors
        let inner1: PersistentVector<i32> = (1..=3).collect();
        let inner2: PersistentVector<i32> = (4..=6).collect();
        let outer: PersistentVector<PersistentVector<i32>> =
            PersistentVector::new().push_back(inner1).push_back(inner2);

        // Access first inner vector, then traverse its elements
        let outer_optional = index_optional::<PersistentVector<i32>>(0);
        let inner_vector = outer_optional.get_option(&outer);

        assert!(inner_vector.is_some());

        let inner_traversal = persistent_vector_traversal::<i32>();
        let elements: Vec<&i32> = inner_traversal.get_all(inner_vector.unwrap()).collect();

        assert_eq!(elements, vec![&1, &2, &3]);
    }

    #[rstest]
    fn test_hashmap_with_vector_values() {
        // Map of String -> PersistentVector<i32>
        let vec1: PersistentVector<i32> = (1..=3).collect();
        let vec2: PersistentVector<i32> = (4..=6).collect();
        let map = PersistentHashMap::new()
            .insert("first".to_string(), vec1)
            .insert("second".to_string(), vec2);

        // Get the "first" vector using Optional
        let optional = key_optional_hashmap::<String, PersistentVector<i32>>("first".to_string());
        let vector = optional.get_option(&map);

        assert!(vector.is_some());
        assert_eq!(vector.unwrap().len(), 3);
    }

    #[rstest]
    fn test_treemap_ordered_traversal() {
        let map = PersistentTreeMap::new()
            .insert(100, "hundred")
            .insert(50, "fifty")
            .insert(75, "seventy-five")
            .insert(25, "twenty-five");

        let traversal = persistent_treemap_traversal::<i32, &str>();
        let values: Vec<&&str> = traversal.get_all(&map).collect();

        // Should be ordered by key
        assert_eq!(
            values,
            vec![&"twenty-five", &"fifty", &"seventy-five", &"hundred"]
        );
    }

    #[rstest]
    fn test_chained_modifications() {
        let vector: PersistentVector<i32> = (1..=5).collect();
        let optional0 = index_optional::<i32>(0);
        let optional2 = index_optional::<i32>(2);
        let optional4 = index_optional::<i32>(4);

        // Chain multiple set operations
        let result = optional0.set(optional2.set(optional4.set(vector, 50), 30), 10);

        assert_eq!(result.get(0), Some(&10));
        assert_eq!(result.get(1), Some(&2)); // Unchanged
        assert_eq!(result.get(2), Some(&30));
        assert_eq!(result.get(3), Some(&4)); // Unchanged
        assert_eq!(result.get(4), Some(&50));
    }
}
