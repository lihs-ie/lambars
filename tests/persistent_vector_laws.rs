//! Property-based tests for PersistentVector laws.
//!
//! This module verifies the algebraic laws and invariants of PersistentVector
//! using proptest.

use functional_rusty::persistent::PersistentVector;
use functional_rusty::typeclass::{Foldable, FunctorMut};
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
        let index = (elements[0].abs() as usize) % length;
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
        let update_index = (elements[0].abs() as usize) % length;
        let check_index = ((elements[1].abs() as usize) % (length - 1) + update_index + 1) % length;
        let new_value = 99999;

        if update_index != check_index {
            if let Some(updated) = vector.update(update_index, new_value) {
                prop_assert_eq!(
                    updated.get(check_index),
                    vector.get(check_index),
                    "Update at {} should not affect index {}",
                    update_index,
                    check_index
                );
            }
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
        let index = (elements[0].abs() as usize) % original.len();
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
