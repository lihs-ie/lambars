#![cfg(feature = "persistent")]
//! Property-based tests for PersistentVector laws.
//!
//! This module verifies the algebraic laws and invariants of PersistentVector
//! using proptest.

use lambars::persistent::PersistentVector;
use lambars::typeclass::{Foldable, FunctorMut};
use proptest::prelude::*;

// =============================================================================
// Basic Laws
// =============================================================================

proptest! {
    /// Get-Update Law: update した要素は get で取得できる
    #[test]
    fn prop_get_update_law(
        elements in prop::collection::vec(any::<i32>(), 1..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let length = vector.len();

        // Pick a random valid index
        let index = (elements[0].unsigned_abs() as usize) % length;
        let new_value = 99999;

        if let Some(updated) = vector.update(index, new_value) {
            prop_assert_eq!(updated.get(index), Some(&new_value));
        }
    }

    /// Get-Update-Other Law: update は他のインデックスに影響しない
    #[test]
    fn prop_get_update_other_law(
        elements in prop::collection::vec(any::<i32>(), 2..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let length = vector.len();

        // Pick two different indices
        let update_index = (elements[0].unsigned_abs() as usize) % length;
        let check_index = ((elements[1].unsigned_abs() as usize) % (length - 1) + update_index + 1) % length;
        let new_value = 99999;

        if update_index != check_index
            && let Some(updated) = vector.update(update_index, new_value)
        {
            prop_assert_eq!(
                updated.get(check_index),
                vector.get(check_index),
                "Update at {} should not affect index {}",
                update_index,
                check_index
            );
        }
    }

    /// Push-Pop Law: push_back と pop_back は逆操作
    #[test]
    fn prop_push_pop_back_law(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        new_element: i32
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let with_element = vector.push_back(new_element);

        if let Some((remaining, popped)) = with_element.pop_back() {
            prop_assert_eq!(popped, new_element);
            prop_assert_eq!(remaining, vector);
        }
    }

    /// Push-Pop Front Law: push_front と pop_front は逆操作
    #[test]
    fn prop_push_pop_front_law(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        new_element: i32
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let with_element = vector.push_front(new_element);

        if let Some((remaining, popped)) = with_element.pop_front() {
            prop_assert_eq!(popped, new_element);
            prop_assert_eq!(remaining, vector);
        }
    }

    /// Length Law: push_back は長さを 1 増やす
    #[test]
    fn prop_push_back_length_law(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        new_element: i32
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let original_length = vector.len();
        let with_element = vector.push_back(new_element);

        prop_assert_eq!(with_element.len(), original_length + 1);
    }

    /// Length Law: push_front は長さを 1 増やす
    #[test]
    fn prop_push_front_length_law(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        new_element: i32
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let original_length = vector.len();
        let with_element = vector.push_front(new_element);

        prop_assert_eq!(with_element.len(), original_length + 1);
    }

    /// Append Identity Law (left): 空ベクターとの連結は恒等操作
    #[test]
    fn prop_append_identity_left(
        elements in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let empty: PersistentVector<i32> = PersistentVector::new();

        let result = empty.append(&vector);
        prop_assert_eq!(result, vector);
    }

    /// Append Identity Law (right): 空ベクターとの連結は恒等操作
    #[test]
    fn prop_append_identity_right(
        elements in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let empty: PersistentVector<i32> = PersistentVector::new();

        let result = vector.append(&empty);
        prop_assert_eq!(result, vector);
    }

    /// Append Associativity Law: (a.append(b)).append(c) == a.append(b.append(c))
    #[test]
    fn prop_append_associativity(
        elements_a in prop::collection::vec(any::<i32>(), 0..20),
        elements_b in prop::collection::vec(any::<i32>(), 0..20),
        elements_c in prop::collection::vec(any::<i32>(), 0..20)
    ) {
        let vector_a: PersistentVector<i32> = elements_a.into_iter().collect();
        let vector_b: PersistentVector<i32> = elements_b.into_iter().collect();
        let vector_c: PersistentVector<i32> = elements_c.into_iter().collect();

        let left = vector_a.clone().append(&vector_b).append(&vector_c);
        let right = vector_a.append(&vector_b.append(&vector_c));

        prop_assert_eq!(left, right);
    }

    /// Iter collects all elements in order
    #[test]
    fn prop_iter_preserves_order(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let collected: Vec<i32> = vector.iter().copied().collect();

        prop_assert_eq!(collected, elements);
    }

    /// IntoIterator collects all elements in order
    #[test]
    fn prop_into_iter_preserves_order(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let collected: Vec<i32> = vector.into_iter().collect();

        prop_assert_eq!(collected, elements);
    }

    /// From iterator round-trip
    #[test]
    fn prop_from_iter_round_trip(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let collected: Vec<i32> = vector.into_iter().collect();

        prop_assert_eq!(collected, elements);
    }
}

// =============================================================================
// Functor Laws
// =============================================================================

proptest! {
    /// Functor Identity Law: fmap(id) == id
    #[test]
    fn prop_functor_identity_law(
        elements in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let mapped: PersistentVector<i32> = vector.clone().fmap_mut(|x| x);

        prop_assert_eq!(vector, mapped);
    }

    /// Functor Composition Law: fmap(f).fmap(g) == fmap(g . f)
    #[test]
    fn prop_functor_composition_law(
        elements in prop::collection::vec(-1000i32..1000i32, 0..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let function1 = |x: i32| x.saturating_add(1);
        let function2 = |x: i32| x.saturating_mul(2);

        let left: PersistentVector<i32> = vector.clone().fmap_mut(function1).fmap_mut(function2);
        let right: PersistentVector<i32> = vector.fmap_mut(|x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Foldable Laws
// =============================================================================

proptest! {
    /// Fold left equals sum for addition
    #[test]
    fn prop_fold_left_sum(
        elements in prop::collection::vec(-1000i32..1000i32, 0..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let fold_result = vector.fold_left(0i64, |accumulator, element| accumulator + i64::from(element));
        let expected: i64 = elements.iter().map(|&x| i64::from(x)).sum();

        prop_assert_eq!(fold_result, expected);
    }

    /// Foldable length equals len()
    #[test]
    fn prop_foldable_length_equals_len(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        prop_assert_eq!(vector.length(), vector.len());
    }

    /// Foldable is_empty equals is_empty()
    #[test]
    fn prop_foldable_is_empty(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        prop_assert_eq!(Foldable::is_empty(&vector), vector.is_empty());
    }

    /// Foldable to_list preserves elements
    #[test]
    fn prop_foldable_to_list(
        elements in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let list = vector.to_list();

        prop_assert_eq!(list, elements);
    }
}

// =============================================================================
// Structural Sharing / Persistence Laws
// =============================================================================

proptest! {
    /// Push back does not modify original
    #[test]
    fn prop_push_back_persistence(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        new_element: i32
    ) {
        let original: PersistentVector<i32> = elements.iter().copied().collect();
        let original_len = original.len();
        let _new_version = original.push_back(new_element);

        // Original should be unchanged
        prop_assert_eq!(original.len(), original_len);
        for (index, element) in elements.iter().enumerate() {
            prop_assert_eq!(original.get(index), Some(element));
        }
    }

    /// Push front does not modify original
    #[test]
    fn prop_push_front_persistence(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        new_element: i32
    ) {
        let original: PersistentVector<i32> = elements.iter().copied().collect();
        let original_len = original.len();
        let _new_version = original.push_front(new_element);

        // Original should be unchanged
        prop_assert_eq!(original.len(), original_len);
        for (index, element) in elements.iter().enumerate() {
            prop_assert_eq!(original.get(index), Some(element));
        }
    }

    /// Update does not modify original
    #[test]
    fn prop_update_persistence(
        elements in prop::collection::vec(any::<i32>(), 1..50)
    ) {
        let original: PersistentVector<i32> = elements.iter().copied().collect();
        let index = (elements[0].unsigned_abs() as usize) % original.len();
        let _updated = original.update(index, 99999);

        // Original should be unchanged
        for (i, element) in elements.iter().enumerate() {
            prop_assert_eq!(original.get(i), Some(element));
        }
    }

    /// Multiple versions can coexist
    #[test]
    fn prop_multiple_versions_coexist(
        elements in prop::collection::vec(any::<i32>(), 5..20)
    ) {
        let base: PersistentVector<i32> = elements.iter().copied().collect();

        let version1 = base.push_back(1000);
        let version2 = base.push_back(2000);
        let version3 = base.push_front(-1);

        // All versions should be independent
        prop_assert_eq!(base.len(), elements.len());
        prop_assert_eq!(version1.len(), elements.len() + 1);
        prop_assert_eq!(version2.len(), elements.len() + 1);
        prop_assert_eq!(version3.len(), elements.len() + 1);

        // Check that base is unchanged
        for (index, element) in elements.iter().enumerate() {
            prop_assert_eq!(base.get(index), Some(element));
        }

        // Check new versions
        prop_assert_eq!(version1.get(elements.len()), Some(&1000));
        prop_assert_eq!(version2.get(elements.len()), Some(&2000));
        prop_assert_eq!(version3.get(0), Some(&-1));
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

proptest! {
    /// Empty vector operations
    #[test]
    fn prop_empty_vector_get_always_none(index: usize) {
        let vector: PersistentVector<i32> = PersistentVector::new();
        prop_assert_eq!(vector.get(index), None);
    }

    /// Single element vector
    #[test]
    fn prop_singleton_operations(element: i32) {
        let vector = PersistentVector::singleton(element);

        prop_assert_eq!(vector.len(), 1);
        prop_assert_eq!(vector.get(0), Some(&element));
        prop_assert_eq!(vector.first(), Some(&element));
        prop_assert_eq!(vector.last(), Some(&element));

        if let Some((remaining, popped)) = vector.pop_back() {
            prop_assert_eq!(popped, element);
            prop_assert!(remaining.is_empty());
        }

        if let Some((remaining, popped)) = vector.pop_front() {
            prop_assert_eq!(popped, element);
            prop_assert!(remaining.is_empty());
        }
    }

    /// First and last are consistent with get
    #[test]
    fn prop_first_last_consistent_with_get(
        elements in prop::collection::vec(any::<i32>(), 1..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();

        prop_assert_eq!(vector.first(), vector.get(0));
        prop_assert_eq!(vector.last(), vector.get(vector.len() - 1));
    }
}

// =============================================================================
// Optimized Iterator Laws
// =============================================================================

proptest! {
    /// Iterator completeness: all elements returned in correct order
    #[test]
    fn prop_optimized_iterator_completeness(
        elements in prop::collection::vec(any::<i32>(), 0..1000)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let collected: Vec<i32> = vector.iter().copied().collect();

        prop_assert_eq!(collected, elements);
    }

    /// Iterator length: count equals vector length
    #[test]
    fn prop_optimized_iterator_length(
        elements in prop::collection::vec(any::<i32>(), 0..500)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();

        prop_assert_eq!(vector.iter().count(), vector.len());
        prop_assert_eq!(vector.iter().len(), vector.len());
    }

    /// IntoIterator equivalence: iter and into_iter return same elements
    #[test]
    fn prop_optimized_into_iterator_equivalence(
        elements in prop::collection::vec(any::<i32>(), 0..500)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let from_iter: Vec<i32> = vector.clone().into_iter().collect();
        let from_ref_iter: Vec<i32> = vector.iter().copied().collect();

        prop_assert_eq!(from_iter, from_ref_iter);
    }

    /// size_hint accuracy: always returns correct remaining count
    #[test]
    fn prop_optimized_iterator_size_hint_accuracy(
        elements in prop::collection::vec(any::<i32>(), 0..200),
        consume_count in 0_usize..201
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let mut iterator = vector.iter();

        let to_consume = consume_count.min(elements.len());
        for _ in 0..to_consume {
            iterator.next();
        }

        let expected_remaining = elements.len().saturating_sub(to_consume);
        let (lower, upper) = iterator.size_hint();

        prop_assert_eq!(lower, expected_remaining);
        prop_assert_eq!(upper, Some(expected_remaining));
    }

    /// Iterator at tree boundaries
    #[test]
    fn prop_optimized_iterator_tree_boundaries(
        // Test sizes that are near boundaries (32, 64, 1024, etc.)
        size in prop::sample::select(vec![
            31_usize, 32, 33, 63, 64, 65, 1023, 1024, 1025
        ])
    ) {
        #[allow(clippy::cast_possible_wrap)]
        let vector: PersistentVector<i32> = (0..size as i32).collect();
        let collected: Vec<i32> = vector.iter().copied().collect();
        #[allow(clippy::cast_possible_wrap)]
        let expected: Vec<i32> = (0..size as i32).collect();

        prop_assert_eq!(collected, expected);
    }

    /// IntoIterator size_hint accuracy
    #[test]
    fn prop_optimized_into_iterator_size_hint_accuracy(
        elements in prop::collection::vec(any::<i32>(), 0..200),
        consume_count in 0_usize..201
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let mut iterator = vector.into_iter();

        let to_consume = consume_count.min(elements.len());
        for _ in 0..to_consume {
            iterator.next();
        }

        let expected_remaining = elements.len().saturating_sub(to_consume);
        let (lower, upper) = iterator.size_hint();

        prop_assert_eq!(lower, expected_remaining);
        prop_assert_eq!(upper, Some(expected_remaining));
    }
}

// =============================================================================
// push_back Optimization Properties
// =============================================================================

proptest! {
    /// from_iter preserves all elements in order
    #[test]
    fn prop_from_iter_preserves_elements(
        elements in prop::collection::vec(any::<i32>(), 0..500)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let collected: Vec<i32> = vector.iter().copied().collect();
        prop_assert_eq!(collected, elements);
    }

    /// from_iter preserves length
    #[test]
    fn prop_from_iter_preserves_length(
        elements in prop::collection::vec(any::<i32>(), 0..500)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        prop_assert_eq!(vector.len(), elements.len());
    }

    /// from_slice equals from_iter
    #[test]
    fn prop_from_slice_equals_from_iter(
        elements in prop::collection::vec(any::<i32>(), 0..200)
    ) {
        let from_slice = PersistentVector::from_slice(&elements);
        let from_iter: PersistentVector<i32> = elements.into_iter().collect();
        prop_assert_eq!(from_slice, from_iter);
    }

    /// push_back_many equals multiple push_back
    #[test]
    fn prop_push_back_many_equals_multiple_push_back(
        base_elements in prop::collection::vec(any::<i32>(), 0..100),
        new_elements in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        let base: PersistentVector<i32> = base_elements.iter().copied().collect();

        let from_many = base.push_back_many(new_elements.iter().copied());
        let mut from_individual = base.clone();
        for element in &new_elements {
            from_individual = from_individual.push_back(*element);
        }

        prop_assert_eq!(from_many, from_individual);
    }

    /// push_back_many preserves length
    #[test]
    fn prop_push_back_many_preserves_length(
        base_elements in prop::collection::vec(any::<i32>(), 0..100),
        new_elements in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        let base: PersistentVector<i32> = base_elements.iter().copied().collect();
        let extended = base.push_back_many(new_elements.iter().copied());

        prop_assert_eq!(
            extended.len(),
            base_elements.len() + new_elements.len()
        );
    }

    /// push_back_many preserves order
    #[test]
    fn prop_push_back_many_preserves_order(
        base_elements in prop::collection::vec(any::<i32>(), 0..100),
        new_elements in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        let base: PersistentVector<i32> = base_elements.iter().copied().collect();
        let extended = base.push_back_many(new_elements.iter().copied());
        let collected: Vec<i32> = extended.iter().copied().collect();

        let mut expected = base_elements.clone();
        expected.extend(new_elements.iter().copied());

        prop_assert_eq!(collected, expected);
    }

    /// push_tail_to_root correctness
    #[test]
    fn prop_push_tail_to_root_correctness(
        n in 0_usize..5000
    ) {
        #[allow(clippy::cast_possible_wrap)]
        let vector: PersistentVector<i32> = (0..n as i32).collect();
        #[allow(clippy::cast_possible_wrap)]
        let extended = vector.push_back(n as i32);

        prop_assert_eq!(extended.len(), n + 1);
        #[allow(clippy::cast_possible_wrap)]
        {
            prop_assert_eq!(extended.get(n), Some(&(n as i32)));
        }

        // All elements are correctly retained
        for i in 0..n {
            #[allow(clippy::cast_possible_wrap)]
            {
                prop_assert_eq!(extended.get(i), Some(&(i as i32)));
            }
        }
    }
}

// =============================================================================
// Hash Laws
// =============================================================================

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn calculate_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

proptest! {
    #[test]
    fn prop_hash_eq_consistency(elements in prop::collection::vec(any::<i32>(), 0..100)) {
        let vector1: PersistentVector<i32> = elements.iter().cloned().collect();
        let vector2: PersistentVector<i32> = elements.iter().cloned().collect();

        prop_assert_eq!(&vector1, &vector2);
        prop_assert_eq!(calculate_hash(&vector1), calculate_hash(&vector2));
    }

    /// Hash determinism: the same vector always has the same hash value
    #[test]
    fn prop_hash_deterministic(elements in prop::collection::vec(any::<i32>(), 0..100)) {
        let vector: PersistentVector<i32> = elements.iter().cloned().collect();

        let hash1 = calculate_hash(&vector);
        let hash2 = calculate_hash(&vector);

        prop_assert_eq!(hash1, hash2);
    }

    /// Cloned vectors have the same hash value
    #[test]
    fn prop_hash_clone_consistency(elements in prop::collection::vec(any::<i32>(), 0..100)) {
        let vector: PersistentVector<i32> = elements.iter().cloned().collect();
        let cloned = vector.clone();

        prop_assert_eq!(calculate_hash(&vector), calculate_hash(&cloned));
    }

    /// Vectors with different element orders have different hash values with high probability
    #[test]
    fn prop_hash_order_sensitive(elements in prop::collection::vec(any::<i32>(), 2..20)) {
        let vector1: PersistentVector<i32> = elements.iter().cloned().collect();
        let reversed: Vec<i32> = elements.iter().cloned().rev().collect();
        let vector2: PersistentVector<i32> = reversed.iter().cloned().collect();

        // Only test if the original vector and the reversed vector are different
        if vector1 != vector2 {
            // Hash collisions are possible but unlikely
            prop_assert_ne!(calculate_hash(&vector1), calculate_hash(&vector2));
        }
    }

    /// Vectors with different lengths have different hash values with high probability
    #[test]
    fn prop_hash_length_sensitive(
        elements in prop::collection::vec(any::<i32>(), 1..50),
        extra in any::<i32>()
    ) {
        let vector1: PersistentVector<i32> = elements.iter().cloned().collect();
        let mut extended = elements.clone();
        extended.push(extra);
        let vector2: PersistentVector<i32> = extended.iter().cloned().collect();

        prop_assert_ne!(calculate_hash(&vector1), calculate_hash(&vector2));
    }
}

// =============================================================================
// Ord Laws
// =============================================================================

/// Generates a `PersistentVector<i32>` with up to `max_size` elements.
fn persistent_vector_strategy(max_size: usize) -> impl Strategy<Value = PersistentVector<i32>> {
    prop::collection::vec(any::<i32>(), 0..max_size).prop_map(|vector| vector.into_iter().collect())
}

/// Generates a small `PersistentVector<i32>` for faster tests.
fn small_vector() -> impl Strategy<Value = PersistentVector<i32>> {
    persistent_vector_strategy(20)
}

proptest! {
    #[test]
    fn prop_ord_reflexivity(vector in small_vector()) {
        prop_assert_eq!(vector.cmp(&vector), std::cmp::Ordering::Equal);
    }

    #[test]
    fn prop_ord_antisymmetry(vector1 in small_vector(), vector2 in small_vector()) {
        if vector1 <= vector2 && vector2 <= vector1 {
            prop_assert_eq!(vector1, vector2);
        }
    }

    #[test]
    fn prop_ord_transitivity(
        vector1 in small_vector(),
        vector2 in small_vector(),
        vector3 in small_vector()
    ) {
        if vector1 <= vector2 && vector2 <= vector3 {
            prop_assert!(vector1 <= vector3);
        }
    }

    #[test]
    fn prop_ord_totality(vector1 in small_vector(), vector2 in small_vector()) {
        use std::cmp::Ordering::{Equal, Greater, Less};
        match vector1.cmp(&vector2) {
            Less => {
                prop_assert!(vector1 < vector2);
                prop_assert!(vector1 != vector2);
                prop_assert!(vector1 <= vector2);
            }
            Equal => {
                prop_assert!(vector1 >= vector2);
                prop_assert!(vector1 == vector2);
                prop_assert!(vector1 <= vector2);
            }
            Greater => {
                prop_assert!(vector1 >= vector2);
                prop_assert!(vector1 != vector2);
                prop_assert!(vector1 > vector2);
            }
        }
    }

    #[test]
    fn prop_ord_consistency_with_partial_ord(vector1 in small_vector(), vector2 in small_vector()) {
        prop_assert_eq!(vector1.partial_cmp(&vector2), Some(vector1.cmp(&vector2)));
    }

    #[test]
    fn prop_ord_consistency_with_eq(vector1 in small_vector(), vector2 in small_vector()) {
        prop_assert_eq!(vector1.cmp(&vector2) == std::cmp::Ordering::Equal, vector1 == vector2);
    }
}

// =============================================================================
// Concat Laws
// =============================================================================

proptest! {
    /// concat preserves the length: len(v1.concat(v2)) = len(v1) + len(v2)
    #[test]
    fn prop_concat_preserves_length(
        elements1 in prop::collection::vec(any::<i32>(), 0..100),
        elements2 in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector1: PersistentVector<i32> = elements1.iter().cloned().collect();
        let vector2: PersistentVector<i32> = elements2.iter().cloned().collect();
        let concatenated = vector1.concat(&vector2);

        prop_assert_eq!(concatenated.len(), vector1.len() + vector2.len());
    }

    /// concat preserves the order: concatenated[i] = v1[i] for i < len(v1),
    /// concatenated[i] = v2[i - len(v1)] for i >= len(v1)
    #[test]
    fn prop_concat_preserves_order(
        elements1 in prop::collection::vec(any::<i32>(), 0..50),
        elements2 in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        let vector1: PersistentVector<i32> = elements1.iter().cloned().collect();
        let vector2: PersistentVector<i32> = elements2.iter().cloned().collect();
        let concatenated = vector1.concat(&vector2);

        // Verify first part
        for (index, expected) in elements1.iter().enumerate() {
            prop_assert_eq!(concatenated.get(index), Some(expected));
        }

        // Verify second part
        for (index, expected) in elements2.iter().enumerate() {
            prop_assert_eq!(concatenated.get(elements1.len() + index), Some(expected));
        }
    }

    /// concat is associative: (v1.concat(v2)).concat(v3) == v1.concat(v2.concat(v3))
    #[test]
    fn prop_concat_associativity(
        elements1 in prop::collection::vec(any::<i32>(), 0..30),
        elements2 in prop::collection::vec(any::<i32>(), 0..30),
        elements3 in prop::collection::vec(any::<i32>(), 0..30)
    ) {
        let vector1: PersistentVector<i32> = elements1.iter().cloned().collect();
        let vector2: PersistentVector<i32> = elements2.iter().cloned().collect();
        let vector3: PersistentVector<i32> = elements3.iter().cloned().collect();

        let left = vector1.concat(&vector2).concat(&vector3);
        let right = vector1.concat(&vector2.concat(&vector3));

        prop_assert_eq!(left, right);
    }

    /// concat with empty vector is identity: v.concat(empty) == v and empty.concat(v) == v
    #[test]
    fn prop_concat_identity(elements in prop::collection::vec(any::<i32>(), 0..100)) {
        let vector: PersistentVector<i32> = elements.iter().cloned().collect();
        let empty: PersistentVector<i32> = PersistentVector::new();

        prop_assert_eq!(vector.concat(&empty), vector.clone());
        prop_assert_eq!(empty.concat(&vector), vector);
    }

    /// concat result equals Vec concatenation
    #[test]
    fn prop_concat_equals_vec_concat(
        elements1 in prop::collection::vec(any::<i32>(), 0..100),
        elements2 in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector1: PersistentVector<i32> = elements1.iter().cloned().collect();
        let vector2: PersistentVector<i32> = elements2.iter().cloned().collect();
        let concatenated = vector1.concat(&vector2);

        let mut expected = elements1.clone();
        expected.extend(elements2.iter().cloned());

        let result: Vec<i32> = concatenated.iter().cloned().collect();
        prop_assert_eq!(result, expected);
    }

    // =========================================================================
    // Concat Preservation Laws (Phase 10)
    // =========================================================================

    /// concat preserves all elements from both vectors
    #[test]
    fn prop_concat_preserves_all_elements(
        elements1 in prop::collection::vec(any::<i32>(), 0..200),
        elements2 in prop::collection::vec(any::<i32>(), 0..200)
    ) {
        let vector1: PersistentVector<i32> = elements1.iter().cloned().collect();
        let vector2: PersistentVector<i32> = elements2.iter().cloned().collect();
        let concatenated = vector1.concat(&vector2);

        prop_assert_eq!(concatenated.len(), elements1.len() + elements2.len());

        for (index, &expected) in elements1.iter().enumerate() {
            prop_assert_eq!(concatenated.get(index), Some(&expected));
        }
        for (index, &expected) in elements2.iter().enumerate() {
            prop_assert_eq!(concatenated.get(elements1.len() + index), Some(&expected));
        }
    }

    /// concat preserves original vectors (immutability)
    #[test]
    fn prop_concat_preserves_original_vectors(
        elements1 in prop::collection::vec(any::<i32>(), 1..100),
        elements2 in prop::collection::vec(any::<i32>(), 1..100)
    ) {
        let vector1: PersistentVector<i32> = elements1.iter().cloned().collect();
        let vector2: PersistentVector<i32> = elements2.iter().cloned().collect();
        let _concatenated = vector1.concat(&vector2);

        prop_assert_eq!(vector1.len(), elements1.len());
        prop_assert_eq!(vector2.len(), elements2.len());

        for (index, &expected) in elements1.iter().enumerate() {
            prop_assert_eq!(vector1.get(index), Some(&expected));
        }
        for (index, &expected) in elements2.iter().enumerate() {
            prop_assert_eq!(vector2.get(index), Some(&expected));
        }
    }

    // =========================================================================
    // Transient Roundtrip Laws (Phase 10)
    // =========================================================================

    /// transient -> persistent roundtrip preserves all elements
    #[test]
    fn prop_transient_roundtrip_preserves_elements(
        elements in prop::collection::vec(any::<i32>(), 0..500)
    ) {
        let vector: PersistentVector<i32> = elements.iter().cloned().collect();
        let roundtripped = vector.clone().transient().persistent();

        prop_assert_eq!(roundtripped.len(), vector.len());
        for index in 0..vector.len() {
            prop_assert_eq!(roundtripped.get(index), vector.get(index));
        }
    }

    /// transient modifications are persisted correctly
    #[test]
    fn prop_transient_modifications_persisted(
        elements in prop::collection::vec(any::<i32>(), 10..200),
        new_value in any::<i32>()
    ) {
        let vector: PersistentVector<i32> = elements.iter().cloned().collect();
        let mut transient = vector.transient();

        let update_index = elements.len() / 2;
        transient.update(update_index, new_value);

        let result = transient.persistent();

        prop_assert_eq!(result.get(update_index), Some(&new_value));

        for (index, element) in elements.iter().enumerate() {
            if index != update_index {
                prop_assert_eq!(result.get(index), Some(element));
            }
        }
    }

    /// transient batch push equals persistent sequential push
    #[test]
    fn prop_transient_batch_push_equals_persistent(
        initial in prop::collection::vec(any::<i32>(), 0..100),
        additions in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector: PersistentVector<i32> = initial.iter().cloned().collect();

        let mut transient = vector.clone().transient();
        for &element in &additions {
            transient.push_back(element);
        }
        let transient_result = transient.persistent();

        let persistent_result = vector.push_back_many(additions.iter().cloned());

        prop_assert_eq!(transient_result.len(), persistent_result.len());
        for index in 0..transient_result.len() {
            prop_assert_eq!(transient_result.get(index), persistent_result.get(index));
        }
    }
}
