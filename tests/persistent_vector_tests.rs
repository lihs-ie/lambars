#![cfg(feature = "persistent")]
//! Unit tests for PersistentVector.
//!
//! This module contains comprehensive tests for the PersistentVector implementation,
//! organized by TDD cycles.

use lambars::persistent::PersistentVector;
use lambars::typeclass::{Foldable, FunctorMut, TypeConstructor};
use rstest::rstest;

// =============================================================================
// Cycle 1: Basic structure and new()
// =============================================================================

#[rstest]
fn test_new_creates_empty_vector() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    assert!(vector.is_empty());
    assert_eq!(vector.len(), 0);
}

#[rstest]
fn test_get_on_empty_returns_none() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    assert_eq!(vector.get(0), None);
}

// =============================================================================
// Cycle 2: push_back (append to tail)
// =============================================================================

#[rstest]
fn test_push_back_single() {
    let vector = PersistentVector::new().push_back(42);
    assert_eq!(vector.len(), 1);
    assert_eq!(vector.get(0), Some(&42));
}

#[rstest]
fn test_push_back_multiple() {
    let vector = PersistentVector::new()
        .push_back(1)
        .push_back(2)
        .push_back(3);
    assert_eq!(vector.len(), 3);
    assert_eq!(vector.get(0), Some(&1));
    assert_eq!(vector.get(1), Some(&2));
    assert_eq!(vector.get(2), Some(&3));
}

#[rstest]
fn test_push_back_does_not_modify_original() {
    let vector1 = PersistentVector::new().push_back(1);
    let vector2 = vector1.push_back(2);

    assert_eq!(vector1.len(), 1);
    assert_eq!(vector1.get(0), Some(&1));
    assert_eq!(vector1.get(1), None);

    assert_eq!(vector2.len(), 2);
    assert_eq!(vector2.get(0), Some(&1));
    assert_eq!(vector2.get(1), Some(&2));
}

#[rstest]
fn test_push_back_beyond_tail_capacity() {
    // Push more than 32 elements to trigger tail overflow
    let mut vector = PersistentVector::new();
    for index in 0..40 {
        vector = vector.push_back(index);
    }

    assert_eq!(vector.len(), 40);
    for index in 0..40 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

#[rstest]
fn test_push_back_large_number_of_elements() {
    let mut vector = PersistentVector::new();
    for index in 0..1000 {
        vector = vector.push_back(index);
    }

    assert_eq!(vector.len(), 1000);
    for index in 0..1000 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// Cycle 3: get (random access)
// =============================================================================

#[rstest]
fn test_get_within_tail() {
    let vector: PersistentVector<i32> = (0..20).collect();
    for index in 0..20 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

#[rstest]
fn test_get_beyond_tail() {
    // 32+ elements use root node
    let vector: PersistentVector<i32> = (0..100).collect();
    for index in 0..100 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

#[rstest]
fn test_get_out_of_bounds() {
    let vector: PersistentVector<i32> = (0..10).collect();
    assert_eq!(vector.get(10), None);
    assert_eq!(vector.get(100), None);
}

#[rstest]
fn test_get_deep_tree() {
    // Create a vector large enough to have multiple levels
    let vector: PersistentVector<i32> = (0..2000).collect();
    for index in 0..2000 {
        assert_eq!(
            vector.get(index),
            Some(&(index as i32)),
            "Failed at index {}",
            index
        );
    }
}

// =============================================================================
// Cycle 4: update (element update)
// =============================================================================

#[rstest]
fn test_update_in_tail() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let updated = vector.update(5, 100).unwrap();

    assert_eq!(updated.get(5), Some(&100));
    assert_eq!(vector.get(5), Some(&5)); // Original unchanged
}

#[rstest]
fn test_update_in_root() {
    let vector: PersistentVector<i32> = (0..100).collect();
    let updated = vector.update(10, 999).unwrap();

    assert_eq!(updated.get(10), Some(&999));
    assert_eq!(vector.get(10), Some(&10)); // Original unchanged
}

#[rstest]
fn test_update_out_of_bounds() {
    let vector: PersistentVector<i32> = (0..10).collect();
    assert!(vector.update(10, 100).is_none());
    assert!(vector.update(100, 100).is_none());
}

#[rstest]
fn test_update_preserves_other_elements() {
    let vector: PersistentVector<i32> = (0..50).collect();
    let updated = vector.update(25, 999).unwrap();

    for index in 0..50 {
        if index == 25 {
            assert_eq!(updated.get(index), Some(&999));
        } else {
            assert_eq!(updated.get(index), Some(&(index as i32)));
        }
    }
}

// =============================================================================
// Cycle 5: pop_back (remove from tail)
// =============================================================================

#[rstest]
fn test_pop_back_single_element() {
    let vector = PersistentVector::new().push_back(42);
    let (remaining, element) = vector.pop_back().unwrap();

    assert_eq!(element, 42);
    assert!(remaining.is_empty());
}

#[rstest]
fn test_pop_back_multiple_elements() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let (remaining, element) = vector.pop_back().unwrap();

    assert_eq!(element, 5);
    assert_eq!(remaining.len(), 4);
    for index in 0..4 {
        assert_eq!(remaining.get(index), Some(&((index + 1) as i32)));
    }
}

#[rstest]
fn test_pop_back_empty() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    assert!(vector.pop_back().is_none());
}

#[rstest]
fn test_pop_back_does_not_modify_original() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let (remaining, _) = vector.pop_back().unwrap();

    assert_eq!(vector.len(), 5);
    assert_eq!(remaining.len(), 4);
}

#[rstest]
fn test_pop_back_from_root() {
    // Pop from a vector that has elements in root
    let vector: PersistentVector<i32> = (0..50).collect();
    let (remaining, element) = vector.pop_back().unwrap();

    assert_eq!(element, 49);
    assert_eq!(remaining.len(), 49);
    for index in 0..49 {
        assert_eq!(remaining.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// Cycle 6: push_front (prepend)
// =============================================================================

#[rstest]
fn test_push_front_single() {
    let vector = PersistentVector::new().push_front(42);
    assert_eq!(vector.len(), 1);
    assert_eq!(vector.get(0), Some(&42));
}

#[rstest]
fn test_push_front_multiple() {
    let vector = PersistentVector::new()
        .push_front(3)
        .push_front(2)
        .push_front(1);
    assert_eq!(vector.len(), 3);
    assert_eq!(vector.get(0), Some(&1));
    assert_eq!(vector.get(1), Some(&2));
    assert_eq!(vector.get(2), Some(&3));
}

#[rstest]
fn test_push_front_does_not_modify_original() {
    let vector1 = PersistentVector::new().push_front(2);
    let vector2 = vector1.push_front(1);

    assert_eq!(vector1.len(), 1);
    assert_eq!(vector1.get(0), Some(&2));

    assert_eq!(vector2.len(), 2);
    assert_eq!(vector2.get(0), Some(&1));
    assert_eq!(vector2.get(1), Some(&2));
}

#[rstest]
fn test_push_front_on_non_empty() {
    let vector: PersistentVector<i32> = (1..=3).collect();
    let with_zero = vector.push_front(0);

    assert_eq!(with_zero.len(), 4);
    assert_eq!(with_zero.get(0), Some(&0));
    assert_eq!(with_zero.get(1), Some(&1));
    assert_eq!(with_zero.get(2), Some(&2));
    assert_eq!(with_zero.get(3), Some(&3));
}

// =============================================================================
// Cycle 7: pop_front (remove from front)
// =============================================================================

#[rstest]
fn test_pop_front_single_element() {
    let vector = PersistentVector::new().push_back(42);
    let (remaining, element) = vector.pop_front().unwrap();

    assert_eq!(element, 42);
    assert!(remaining.is_empty());
}

#[rstest]
fn test_pop_front_multiple_elements() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let (remaining, element) = vector.pop_front().unwrap();

    assert_eq!(element, 1);
    assert_eq!(remaining.len(), 4);
    for index in 0..4 {
        assert_eq!(remaining.get(index), Some(&((index + 2) as i32)));
    }
}

#[rstest]
fn test_pop_front_empty() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    assert!(vector.pop_front().is_none());
}

#[rstest]
fn test_pop_front_does_not_modify_original() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let (remaining, _) = vector.pop_front().unwrap();

    assert_eq!(vector.len(), 5);
    assert_eq!(remaining.len(), 4);
}

// =============================================================================
// Cycle 8: first/last methods
// =============================================================================

#[rstest]
fn test_first_non_empty() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    assert_eq!(vector.first(), Some(&1));
}

#[rstest]
fn test_first_empty() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    assert_eq!(vector.first(), None);
}

#[rstest]
fn test_last_non_empty() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    assert_eq!(vector.last(), Some(&5));
}

#[rstest]
fn test_last_empty() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    assert_eq!(vector.last(), None);
}

#[rstest]
fn test_last_in_tail() {
    let vector: PersistentVector<i32> = (0..10).collect();
    assert_eq!(vector.last(), Some(&9));
}

#[rstest]
fn test_last_in_root() {
    let vector: PersistentVector<i32> = (0..100).collect();
    assert_eq!(vector.last(), Some(&99));
}

// =============================================================================
// Cycle 9: iter (iterator)
// =============================================================================

#[rstest]
fn test_iter_collects_all_elements() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let collected: Vec<&i32> = vector.iter().collect();
    assert_eq!(collected, vec![&1, &2, &3, &4, &5]);
}

#[rstest]
fn test_iter_empty() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    let collected: Vec<&i32> = vector.iter().collect();
    assert!(collected.is_empty());
}

#[rstest]
fn test_iter_large_vector() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    let collected: Vec<_> = vector.iter().copied().collect();
    let expected: Vec<i32> = (0..1000).collect();
    assert_eq!(collected, expected);
}

#[rstest]
fn test_iter_sum() {
    let vector: PersistentVector<i32> = (1..=100).collect();
    let sum: i32 = vector.iter().sum();
    assert_eq!(sum, 5050); // Sum of 1 to 100
}

// =============================================================================
// Cycle 10: FromIterator
// =============================================================================

#[rstest]
fn test_from_iter_range() {
    let vector: PersistentVector<i32> = (0..10).collect();
    assert_eq!(vector.len(), 10);
    for index in 0..10 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

#[rstest]
fn test_from_iter_vec() {
    let source = vec![1, 2, 3, 4, 5];
    let vector: PersistentVector<i32> = source.into_iter().collect();
    assert_eq!(vector.len(), 5);
    let collected: Vec<_> = vector.iter().copied().collect();
    assert_eq!(collected, vec![1, 2, 3, 4, 5]);
}

#[rstest]
fn test_from_iter_empty() {
    let empty: Vec<i32> = vec![];
    let vector: PersistentVector<i32> = empty.into_iter().collect();
    assert!(vector.is_empty());
}

// =============================================================================
// Cycle 11: IntoIterator
// =============================================================================

#[rstest]
fn test_into_iter() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let collected: Vec<i32> = vector.into_iter().collect();
    assert_eq!(collected, vec![1, 2, 3, 4, 5]);
}

#[rstest]
fn test_into_iter_empty() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    let collected: Vec<i32> = vector.into_iter().collect();
    assert!(collected.is_empty());
}

#[rstest]
fn test_into_iter_large() {
    let vector: PersistentVector<i32> = (0..500).collect();
    let collected: Vec<i32> = vector.into_iter().collect();
    let expected: Vec<i32> = (0..500).collect();
    assert_eq!(collected, expected);
}

// =============================================================================
// Cycle 12: append (concatenation)
// =============================================================================

#[rstest]
fn test_append_two_vectors() {
    let vector1: PersistentVector<i32> = (1..=3).collect();
    let vector2: PersistentVector<i32> = (4..=6).collect();
    let combined = vector1.append(&vector2);

    assert_eq!(combined.len(), 6);
    let collected: Vec<_> = combined.iter().copied().collect();
    assert_eq!(collected, vec![1, 2, 3, 4, 5, 6]);
}

#[rstest]
fn test_append_with_empty() {
    let vector: PersistentVector<i32> = (1..=3).collect();
    let empty: PersistentVector<i32> = PersistentVector::new();

    let result1 = vector.append(&empty);
    let collected1: Vec<_> = result1.iter().copied().collect();
    assert_eq!(collected1, vec![1, 2, 3]);

    let result2 = empty.append(&vector);
    let collected2: Vec<_> = result2.iter().copied().collect();
    assert_eq!(collected2, vec![1, 2, 3]);
}

#[rstest]
fn test_append_does_not_modify_original() {
    let vector1: PersistentVector<i32> = (1..=3).collect();
    let vector2: PersistentVector<i32> = (4..=6).collect();
    let _combined = vector1.append(&vector2);

    assert_eq!(vector1.len(), 3);
    assert_eq!(vector2.len(), 3);
}

// =============================================================================
// Cycle 13: singleton
// =============================================================================

#[rstest]
fn test_singleton() {
    let vector = PersistentVector::singleton(42);
    assert_eq!(vector.len(), 1);
    assert_eq!(vector.get(0), Some(&42));
}

// =============================================================================
// Cycle 14: PartialEq, Eq, Debug, Default
// =============================================================================

#[rstest]
fn test_eq_same_elements() {
    let vector1: PersistentVector<i32> = (1..=5).collect();
    let vector2: PersistentVector<i32> = (1..=5).collect();
    assert_eq!(vector1, vector2);
}

#[rstest]
fn test_ne_different_elements() {
    let vector1: PersistentVector<i32> = (1..=5).collect();
    let vector2: PersistentVector<i32> = (1..=6).collect();
    assert_ne!(vector1, vector2);
}

#[rstest]
fn test_eq_empty() {
    let empty1: PersistentVector<i32> = PersistentVector::new();
    let empty2: PersistentVector<i32> = PersistentVector::new();
    assert_eq!(empty1, empty2);
}

#[rstest]
fn test_debug() {
    let vector: PersistentVector<i32> = (1..=3).collect();
    let debug_str = format!("{:?}", vector);
    assert!(debug_str.contains("1"));
    assert!(debug_str.contains("2"));
    assert!(debug_str.contains("3"));
}

#[rstest]
fn test_default() {
    let vector: PersistentVector<i32> = PersistentVector::default();
    assert!(vector.is_empty());
}

// =============================================================================
// Cycle 15: TypeConstructor
// =============================================================================

#[rstest]
fn test_type_constructor() {
    fn assert_type_constructor<T: TypeConstructor<Inner = i32>>() {}
    assert_type_constructor::<PersistentVector<i32>>();
}

// =============================================================================
// Cycle 16: Functor/FunctorMut
// =============================================================================

#[rstest]
fn test_fmap_mut() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let doubled: PersistentVector<i32> = vector.fmap_mut(|x| x * 2);
    let collected: Vec<_> = doubled.iter().copied().collect();
    assert_eq!(collected, vec![2, 4, 6, 8, 10]);
}

#[rstest]
fn test_fmap_mut_type_change() {
    let vector: PersistentVector<i32> = (1..=3).collect();
    let strings: PersistentVector<String> = vector.fmap_mut(|x| x.to_string());
    let collected: Vec<_> = strings.iter().cloned().collect();
    assert_eq!(collected, vec!["1", "2", "3"]);
}

#[rstest]
fn test_fmap_ref_mut() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let doubled: PersistentVector<i32> = vector.fmap_ref_mut(|x| x * 2);
    let collected: Vec<_> = doubled.iter().copied().collect();
    assert_eq!(collected, vec![2, 4, 6, 8, 10]);

    // Original should be unchanged
    let original: Vec<_> = vector.iter().copied().collect();
    assert_eq!(original, vec![1, 2, 3, 4, 5]);
}

#[rstest]
fn test_functor_identity_law() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let mapped: PersistentVector<i32> = vector.clone().fmap_mut(|x| x);
    assert_eq!(vector, mapped);
}

#[rstest]
fn test_functor_composition_law() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let function1 = |x: i32| x + 1;
    let function2 = |x: i32| x * 2;

    let left: PersistentVector<i32> = vector.clone().fmap_mut(function1).fmap_mut(function2);
    let right: PersistentVector<i32> = vector.fmap_mut(|x| function2(function1(x)));

    assert_eq!(left, right);
}

// =============================================================================
// Cycle 17: Foldable
// =============================================================================

#[rstest]
fn test_fold_left() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let sum = vector.fold_left(0, |accumulator, element| accumulator + element);
    assert_eq!(sum, 15);
}

#[rstest]
fn test_fold_right() {
    let vector: PersistentVector<i32> = (1..=3).collect();
    let result = vector.fold_right(String::new(), |element, accumulator| {
        format!("{}{}", element, accumulator)
    });
    assert_eq!(result, "123");
}

#[rstest]
fn test_foldable_length() {
    let vector: PersistentVector<i32> = (1..=10).collect();
    assert_eq!(vector.length(), 10);
}

#[rstest]
fn test_foldable_is_empty() {
    let empty: PersistentVector<i32> = PersistentVector::new();
    let non_empty: PersistentVector<i32> = (1..=5).collect();

    assert!(Foldable::is_empty(&empty));
    assert!(!Foldable::is_empty(&non_empty));
}

#[rstest]
fn test_foldable_find() {
    let vector: PersistentVector<i32> = (1..=10).collect();
    assert_eq!(vector.find(|element| *element > 5), Some(6));
}

#[rstest]
fn test_foldable_exists() {
    let vector: PersistentVector<i32> = (1..=10).collect();
    assert!(vector.exists(|element| *element > 5));
    assert!(!vector.exists(|element| *element > 100));
}

#[rstest]
fn test_foldable_for_all() {
    let vector: PersistentVector<i32> = (1..=10).collect();
    assert!(vector.for_all(|element| *element > 0));
    assert!(!vector.for_all(|element| *element > 5));
}

// =============================================================================
// Cycle 18: Performance tests
// =============================================================================

#[rstest]
fn test_large_vector_operations() {
    // Build a large vector
    let mut vector = PersistentVector::new();
    for index in 0..10_000 {
        vector = vector.push_back(index);
    }
    assert_eq!(vector.len(), 10_000);

    // Verify all elements
    for index in 0..10_000 {
        assert_eq!(
            vector.get(index),
            Some(&(index as i32)),
            "Failed at index {}",
            index
        );
    }

    // Update an element
    let updated = vector.update(5000, -1).unwrap();
    assert_eq!(updated.get(5000), Some(&-1));
    assert_eq!(vector.get(5000), Some(&5000)); // Original unchanged
}

#[rstest]
fn test_structural_sharing() {
    // Create a base vector
    let base: PersistentVector<i32> = (0..1000).collect();

    // Create multiple versions
    let version1 = base.push_back(1000);
    let version2 = base.push_back(2000);
    let version3 = base.update(500, -1).unwrap();

    // All versions should be independent
    assert_eq!(base.len(), 1000);
    assert_eq!(version1.len(), 1001);
    assert_eq!(version2.len(), 1001);
    assert_eq!(version3.len(), 1000);

    assert_eq!(base.get(999), Some(&999));
    assert_eq!(version1.get(1000), Some(&1000));
    assert_eq!(version2.get(1000), Some(&2000));
    assert_eq!(version3.get(500), Some(&-1));
    assert_eq!(base.get(500), Some(&500)); // Original unchanged
}

// =============================================================================
// Coverage Tests: Clone Trait
// =============================================================================

#[rstest]
fn test_clone() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let cloned = vector.clone();
    assert_eq!(vector, cloned);
}

// =============================================================================
// Coverage Tests: Iterator size_hint and ExactSizeIterator
// =============================================================================

#[rstest]
fn test_iter_size_hint() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let iter = vector.iter();
    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 5);
    assert_eq!(upper, Some(5));
}

#[rstest]
fn test_iter_exact_size() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let iter = vector.iter();
    assert_eq!(iter.len(), 5);
}

#[rstest]
fn test_into_iter_size_hint() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let iter = vector.into_iter();
    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 5);
    assert_eq!(upper, Some(5));
}

#[rstest]
fn test_into_iter_exact_size() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let iter = vector.into_iter();
    assert_eq!(iter.len(), 5);
}

#[rstest]
fn test_ref_into_iterator() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let mut sum = 0;
    for element in &vector {
        sum += element;
    }
    assert_eq!(sum, 15);
}

// =============================================================================
// Coverage Tests: pop_back from root (complex case)
// =============================================================================

#[rstest]
fn test_pop_back_to_tail_only() {
    // Vector with 33 elements: 32 in root + 1 in tail
    let mut vector = PersistentVector::new();
    for index in 0..33 {
        vector = vector.push_back(index);
    }
    assert_eq!(vector.len(), 33);

    // Pop to go back to exactly 32 elements (all in tail)
    let (popped, element) = vector.pop_back().unwrap();
    assert_eq!(element, 32);
    assert_eq!(popped.len(), 32);

    // Pop again to have 31 elements in tail
    let (popped2, element2) = popped.pop_back().unwrap();
    assert_eq!(element2, 31);
    assert_eq!(popped2.len(), 31);
}

#[rstest]
fn test_pop_back_repeatedly() {
    let vector: PersistentVector<i32> = (0..100).collect();
    let mut current = vector;

    for expected in (0..100).rev() {
        let (remaining, element) = current.pop_back().unwrap();
        assert_eq!(element, expected);
        current = remaining;
    }

    assert!(current.is_empty());
}

// =============================================================================
// Coverage Tests: Large pop_front operations
// =============================================================================

#[rstest]
fn test_pop_front_repeatedly() {
    let vector: PersistentVector<i32> = (0..50).collect();
    let mut current = vector;

    for expected in 0..50 {
        let (remaining, element) = current.pop_front().unwrap();
        assert_eq!(element, expected);
        current = remaining;
    }

    assert!(current.is_empty());
}

// =============================================================================
// Coverage Tests: fold_map
// =============================================================================

#[rstest]
fn test_fold_map() {
    use lambars::typeclass::Sum;

    let vector: PersistentVector<i32> = (1..=5).collect();
    let result: Sum<i32> = vector.fold_map(Sum::new);
    assert_eq!(result.into_inner(), 15);
}

#[rstest]
fn test_fold_map_empty() {
    use lambars::typeclass::Sum;

    let vector: PersistentVector<i32> = PersistentVector::new();
    let result: Sum<i32> = vector.fold_map(Sum::new);
    assert_eq!(result.into_inner(), 0);
}

// =============================================================================
// Coverage Tests: update boundary cases
// =============================================================================

#[rstest]
fn test_update_at_first_element() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let updated = vector.update(0, 100).unwrap();
    assert_eq!(updated.get(0), Some(&100));
    assert_eq!(vector.get(0), Some(&0)); // Original unchanged
}

#[rstest]
fn test_update_at_last_element() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let updated = vector.update(9, 100).unwrap();
    assert_eq!(updated.get(9), Some(&100));
    assert_eq!(vector.get(9), Some(&9)); // Original unchanged
}

#[rstest]
fn test_update_at_boundary() {
    // Vector with exactly 32 elements (all in tail)
    let vector: PersistentVector<i32> = (0..32).collect();
    let updated = vector.update(31, 999).unwrap();
    assert_eq!(updated.get(31), Some(&999));
}

#[rstest]
fn test_update_boundary_between_root_and_tail() {
    // Vector with 33 elements: 32 in root + 1 in tail
    let vector: PersistentVector<i32> = (0..33).collect();

    // Update element at index 31 (last in root)
    let updated31 = vector.update(31, 999).unwrap();
    assert_eq!(updated31.get(31), Some(&999));

    // Update element at index 32 (first in tail)
    let updated32 = vector.update(32, 888).unwrap();
    assert_eq!(updated32.get(32), Some(&888));
}

// =============================================================================
// Coverage Tests: Iterator after partial consumption
// =============================================================================

#[rstest]
fn test_iter_after_partial_consumption() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let mut iter = vector.iter();

    iter.next(); // Consume first element

    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 4);
    assert_eq!(upper, Some(4));
    assert_eq!(iter.len(), 4);
}

#[rstest]
fn test_into_iter_after_partial_consumption() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let mut iter = vector.into_iter();

    iter.next(); // Consume first element

    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 4);
    assert_eq!(upper, Some(4));
    assert_eq!(iter.len(), 4);
}

// =============================================================================
// Coverage Tests: Large tree operations
// =============================================================================

#[rstest]
fn test_large_tree_depth() {
    // Create a large enough vector to have multiple tree levels
    let vector: PersistentVector<i32> = (0..10000).collect();

    // Verify random access works at all positions
    assert_eq!(vector.get(0), Some(&0));
    assert_eq!(vector.get(1000), Some(&1000));
    assert_eq!(vector.get(5000), Some(&5000));
    assert_eq!(vector.get(9999), Some(&9999));
}

#[rstest]
fn test_large_tree_update() {
    let vector: PersistentVector<i32> = (0..10000).collect();

    // Update at various positions
    let updated1 = vector.update(0, -1).unwrap();
    let updated2 = updated1.update(5000, -2).unwrap();
    let updated3 = updated2.update(9999, -3).unwrap();

    assert_eq!(updated3.get(0), Some(&-1));
    assert_eq!(updated3.get(5000), Some(&-2));
    assert_eq!(updated3.get(9999), Some(&-3));

    // Original unchanged
    assert_eq!(vector.get(0), Some(&0));
    assert_eq!(vector.get(5000), Some(&5000));
    assert_eq!(vector.get(9999), Some(&9999));
}

// =============================================================================
// Coverage Tests: Append with various sizes
// =============================================================================

#[rstest]
fn test_append_large_vectors() {
    let vector1: PersistentVector<i32> = (0..100).collect();
    let vector2: PersistentVector<i32> = (100..200).collect();
    let combined = vector1.append(&vector2);

    assert_eq!(combined.len(), 200);
    for index in 0..200 {
        assert_eq!(combined.get(index), Some(&(index as i32)));
    }
}

#[rstest]
fn test_append_small_to_large() {
    let large: PersistentVector<i32> = (0..1000).collect();
    let small: PersistentVector<i32> = (0..3).collect();
    let combined = large.append(&small);

    assert_eq!(combined.len(), 1003);
}

// =============================================================================
// Coverage Tests: Edge cases in tree navigation
// =============================================================================

#[rstest]
fn test_get_in_deep_tree() {
    // Create a very large vector that requires deep tree navigation
    let vector: PersistentVector<i32> = (0..50000).collect();

    // Test access at various depths
    assert_eq!(vector.get(100), Some(&100));
    assert_eq!(vector.get(1000), Some(&1000));
    assert_eq!(vector.get(10000), Some(&10000));
    assert_eq!(vector.get(49999), Some(&49999));
}

#[rstest]
fn test_update_in_deep_tree() {
    let vector: PersistentVector<i32> = (0..50000).collect();

    let updated = vector.update(25000, -1).unwrap();
    assert_eq!(updated.get(25000), Some(&-1));
    assert_eq!(vector.get(25000), Some(&25000)); // Original unchanged
}

// =============================================================================
// Coverage Tests: Foldable additional methods
// =============================================================================

#[rstest]
fn test_foldable_to_list() {
    let vector: PersistentVector<i32> = (1..=5).collect();
    let as_vec: Vec<_> = vector.fold_left(Vec::new(), |mut accumulator, element| {
        accumulator.push(element);
        accumulator
    });
    assert_eq!(as_vec, vec![1, 2, 3, 4, 5]);
}

#[rstest]
fn test_foldable_count() {
    let vector: PersistentVector<i32> = (0..100).collect();
    let count = vector.fold_left(0, |accumulator, _| accumulator + 1);
    assert_eq!(count, 100);
}

// =============================================================================
// Coverage Tests: pop_back with Tree Depth Reduction
// =============================================================================

#[rstest]
fn test_pop_back_reduces_tree_depth() {
    // Create a vector large enough to have increased tree depth
    // Then pop elements until tree depth is reduced
    let mut vector: PersistentVector<i32> = (0..1100).collect();

    // Pop elements to reduce tree size
    for _ in 0..1068 {
        let (remaining, _) = vector.pop_back().unwrap();
        vector = remaining;
    }

    // After popping, should still work correctly
    assert_eq!(vector.len(), 32);
    for index in 0..32 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

#[rstest]
fn test_pop_back_from_single_leaf_in_root() {
    // Create vector with 33 elements (one leaf in root, one in tail)
    let vector: PersistentVector<i32> = (0..33).collect();

    // Pop to 32 elements
    let (vector32, element32) = vector.pop_back().unwrap();
    assert_eq!(element32, 32);
    assert_eq!(vector32.len(), 32);

    // Pop down to 31 (still in tail-only mode)
    let (vector31, element31) = vector32.pop_back().unwrap();
    assert_eq!(element31, 31);
    assert_eq!(vector31.len(), 31);
}

#[rstest]
fn test_pop_back_transitions_from_root_to_tail() {
    // Create vector with 64 elements, then pop down
    let mut vector: PersistentVector<i32> = (0..64).collect();

    // Pop 32 elements
    for expected in (32..64).rev() {
        let (remaining, element) = vector.pop_back().unwrap();
        assert_eq!(element, expected);
        vector = remaining;
    }

    // Now we should have 32 elements
    assert_eq!(vector.len(), 32);
    for index in 0..32 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// Coverage Tests: push_back with Root Overflow
// =============================================================================

#[rstest]
fn test_push_back_causes_root_overflow() {
    // Create vector near capacity that requires new root level
    let vector: PersistentVector<i32> = (0..1024).collect();

    // Add one more to trigger potential overflow handling
    let extended = vector.push_back(1024);
    assert_eq!(extended.len(), 1025);
    assert_eq!(extended.get(1024), Some(&1024));
}

#[rstest]
fn test_push_back_creates_deep_tree() {
    // Push enough elements to create multiple tree levels
    let mut vector = PersistentVector::new();
    for index in 0..2000 {
        vector = vector.push_back(index);
    }

    // Verify tree integrity
    for index in 0..2000 {
        assert_eq!(
            vector.get(index),
            Some(&(index as i32)),
            "Failed at index {}",
            index
        );
    }
}

// =============================================================================
// Coverage Tests: get_from_root Edge Cases
// =============================================================================

#[rstest]
fn test_get_from_root_with_none_child() {
    // Create sparse-ish vector and test edge cases
    let vector: PersistentVector<i32> = (0..100).collect();
    assert!(vector.get(100).is_none());
    assert!(vector.get(1000).is_none());
}

#[rstest]
fn test_get_from_deeply_nested_root() {
    let vector: PersistentVector<i32> = (0..5000).collect();

    // Access at various depths
    assert_eq!(vector.get(0), Some(&0));
    assert_eq!(vector.get(32), Some(&32));
    assert_eq!(vector.get(1024), Some(&1024));
    assert_eq!(vector.get(4999), Some(&4999));
}

// =============================================================================
// Coverage Tests: update_in_root Edge Cases
// =============================================================================

#[rstest]
fn test_update_in_root_at_various_depths() {
    let vector: PersistentVector<i32> = (0..2000).collect();

    // Update at first element in root
    let updated1 = vector.update(0, -1).unwrap();
    assert_eq!(updated1.get(0), Some(&-1));

    // Update at middle of root
    let updated2 = updated1.update(1000, -2).unwrap();
    assert_eq!(updated2.get(1000), Some(&-2));

    // Update at last element before tail
    let updated3 = updated2.update(1967, -3).unwrap();
    assert_eq!(updated3.get(1967), Some(&-3));
}

#[rstest]
fn test_update_at_leaf_boundary() {
    let vector: PersistentVector<i32> = (0..100).collect();

    // Update at positions 31 and 32 (boundary between leaves)
    let updated = vector.update(31, 100).unwrap().update(32, 200).unwrap();

    assert_eq!(updated.get(31), Some(&100));
    assert_eq!(updated.get(32), Some(&200));
}

// =============================================================================
// Coverage Tests: Functor fmap (FnOnce version)
// =============================================================================

use lambars::typeclass::Functor;

#[rstest]
fn test_fmap_on_empty_vector() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    let mapped: PersistentVector<String> = vector.fmap(|x| x.to_string());
    assert!(mapped.is_empty());
}

#[rstest]
fn test_fmap_on_singleton() {
    let vector = PersistentVector::singleton(42);
    let mapped: PersistentVector<String> = vector.fmap(|x| format!("value: {}", x));
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped.get(0), Some(&"value: 42".to_string()));
}

#[rstest]
fn test_fmap_ref_on_empty_vector() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    let mapped: PersistentVector<String> = vector.fmap_ref(|x| x.to_string());
    assert!(mapped.is_empty());
}

#[rstest]
fn test_fmap_ref_on_singleton() {
    let vector = PersistentVector::singleton(42);
    let mapped: PersistentVector<String> = vector.fmap_ref(|x| format!("value: {}", x));
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped.get(0), Some(&"value: 42".to_string()));
}

// =============================================================================
// Coverage Tests: build_persistent_vector_from_vec
// =============================================================================

#[rstest]
fn test_fmap_mut_creates_large_vector() {
    // This exercises build_persistent_vector_from_vec with many elements
    let vector: PersistentVector<i32> = (0..100).collect();
    let mapped: PersistentVector<i32> = vector.fmap_mut(|x| x * 2);

    assert_eq!(mapped.len(), 100);
    for index in 0..100 {
        assert_eq!(mapped.get(index), Some(&((index as i32) * 2)));
    }
}

#[rstest]
fn test_fmap_mut_with_exactly_32_elements() {
    // Test boundary case for tail-only vector
    let vector: PersistentVector<i32> = (0..32).collect();
    let mapped: PersistentVector<i32> = vector.fmap_mut(|x| x + 1);

    assert_eq!(mapped.len(), 32);
    for index in 0..32 {
        assert_eq!(mapped.get(index), Some(&((index as i32) + 1)));
    }
}

#[rstest]
fn test_fmap_mut_with_exactly_33_elements() {
    // Test boundary case where root starts being used
    let vector: PersistentVector<i32> = (0..33).collect();
    let mapped: PersistentVector<i32> = vector.fmap_mut(|x| x + 100);

    assert_eq!(mapped.len(), 33);
    for index in 0..33 {
        assert_eq!(mapped.get(index), Some(&((index as i32) + 100)));
    }
}

// =============================================================================
// Coverage Tests: build_root_from_elements
// =============================================================================

#[rstest]
fn test_build_large_vector_from_collect() {
    // This tests build_root_from_elements with multiple levels
    let vector: PersistentVector<i32> = (0..2000).collect();

    assert_eq!(vector.len(), 2000);
    for index in [0, 31, 32, 1000, 1999] {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

#[rstest]
fn test_build_vector_with_single_leaf_in_root() {
    // 64 elements = 32 in root leaf + 32 in tail
    let vector: PersistentVector<i32> = (0..64).collect();

    assert_eq!(vector.len(), 64);
    for index in 0..64 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// Coverage Tests: get_leaf_at Edge Cases
// =============================================================================

#[rstest]
fn test_pop_back_get_leaf_at() {
    // Create vector where pop_back needs to fetch leaf from root
    let vector: PersistentVector<i32> = (0..65).collect();

    // Pop twice to go from 65 -> 64 -> 63
    let (v64, _) = vector.pop_back().unwrap();
    let (v63, _) = v64.pop_back().unwrap();

    assert_eq!(v63.len(), 63);
    assert_eq!(v63.last(), Some(&62));
}

#[rstest]
fn test_pop_back_multiple_levels() {
    // Create a vector with multiple tree levels
    let vector: PersistentVector<i32> = (0..1100).collect();

    // Pop from the back multiple times
    let (v1099, e1099) = vector.pop_back().unwrap();
    assert_eq!(e1099, 1099);

    let (v1098, e1098) = v1099.pop_back().unwrap();
    assert_eq!(e1098, 1098);

    // Verify remaining elements
    for index in 0..1098 {
        assert_eq!(v1098.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// Coverage Tests: do_pop_tail Edge Cases
// =============================================================================

#[rstest]
fn test_pop_back_cascading_empty_nodes() {
    // Create vector then pop extensively to trigger cascading cleanup
    let mut vector: PersistentVector<i32> = (0..100).collect();

    for expected in (0..100).rev() {
        let (remaining, element) = vector.pop_back().unwrap();
        assert_eq!(element, expected);
        vector = remaining;
    }

    assert!(vector.is_empty());
}

#[rstest]
fn test_pop_back_at_boundaries() {
    // Test pop at specific boundaries
    for size in [32_usize, 33, 64, 65, 96, 97, 128, 129] {
        let vector: PersistentVector<i32> = (0..size as i32).collect();
        let (remaining, element) = vector.pop_back().unwrap();

        assert_eq!(element, (size - 1) as i32);
        assert_eq!(remaining.len(), size - 1);
    }
}

// =============================================================================
// Coverage Tests: push_tail_into_node with None Child
// =============================================================================

#[rstest]
fn test_push_back_creates_new_path() {
    // Push elements that require creating new paths in the tree
    let mut vector = PersistentVector::new();

    // Push 33 elements to trigger path creation
    for index in 0..33 {
        vector = vector.push_back(index);
    }

    assert_eq!(vector.len(), 33);

    // Continue to trigger more path creations
    for index in 33..100 {
        vector = vector.push_back(index);
    }

    // Verify all elements
    for index in 0..100 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// Coverage Tests: new_path Function
// =============================================================================

#[rstest]
fn test_push_back_triggers_new_path_at_multiple_levels() {
    // Push enough elements to trigger new_path at multiple levels
    let mut vector = PersistentVector::new();

    for index in 0..2000 {
        vector = vector.push_back(index);
    }

    // Verify structure is correct
    for index in [0, 500, 1000, 1500, 1999] {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// Coverage Tests: Empty Vector Edge Cases
// =============================================================================

#[rstest]
fn test_append_two_empty_vectors() {
    let empty1: PersistentVector<i32> = PersistentVector::new();
    let empty2: PersistentVector<i32> = PersistentVector::new();

    let combined = empty1.append(&empty2);
    assert!(combined.is_empty());
}

#[rstest]
fn test_operations_on_empty_vector() {
    let empty: PersistentVector<i32> = PersistentVector::new();

    assert!(empty.first().is_none());
    assert!(empty.last().is_none());
    assert!(empty.get(0).is_none());
    assert!(empty.pop_back().is_none());
    assert!(empty.pop_front().is_none());
    assert!(empty.update(0, 1).is_none());
}

// =============================================================================
// Coverage Tests: Iterator Exhaustion
// =============================================================================

#[rstest]
fn test_iter_fully_exhausted() {
    let vector: PersistentVector<i32> = (1..=3).collect();
    let mut iter = vector.iter();

    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next(), Some(&2));
    assert_eq!(iter.next(), Some(&3));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None); // Already exhausted

    assert_eq!(iter.len(), 0);
}

#[rstest]
fn test_into_iter_fully_exhausted() {
    let vector: PersistentVector<i32> = (1..=3).collect();
    let mut iter = vector.into_iter();

    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(2));
    assert_eq!(iter.next(), Some(3));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);

    assert_eq!(iter.len(), 0);
}

// =============================================================================
// Coverage Tests: Semigroup and Monoid
// =============================================================================

use lambars::typeclass::{Monoid, Semigroup};

#[rstest]
fn test_semigroup_combine_empty_with_non_empty() {
    let empty: PersistentVector<i32> = PersistentVector::new();
    let non_empty: PersistentVector<i32> = (1..=3).collect();

    let combined1 = empty.clone().combine(non_empty.clone());
    let combined2 = non_empty.clone().combine(empty);

    assert_eq!(combined1, non_empty);
    assert_eq!(combined2, non_empty);
}

#[rstest]
fn test_monoid_combine_all() {
    let vectors: Vec<PersistentVector<i32>> =
        vec![(1..=3).collect(), (4..=6).collect(), (7..=9).collect()];

    let combined: PersistentVector<i32> = vectors
        .into_iter()
        .fold(PersistentVector::empty(), |acc, v| acc.combine(v));

    let collected: Vec<_> = combined.iter().copied().collect();
    assert_eq!(collected, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

// =============================================================================
// Coverage Tests: PartialEq Edge Cases
// =============================================================================

#[rstest]
fn test_eq_different_lengths() {
    let v1: PersistentVector<i32> = (0..10).collect();
    let v2: PersistentVector<i32> = (0..20).collect();

    assert_ne!(v1, v2);
}

#[rstest]
fn test_eq_same_length_different_content() {
    let v1: PersistentVector<i32> = (0..10).collect();
    let v2: PersistentVector<i32> = (10..20).collect();

    assert_ne!(v1, v2);
}

// =============================================================================
// Coverage Tests: pop_tail_from_root
// =============================================================================

#[rstest]
fn test_pop_back_reduces_single_child_to_new_root() {
    // Create a vector that when popped will have a single child in root
    let vector: PersistentVector<i32> = (0..1025).collect();

    // Pop until we might trigger depth reduction
    let mut current = vector;
    for _ in 0..993 {
        let (remaining, _) = current.pop_back().unwrap();
        current = remaining;
    }

    // Should still work correctly
    assert_eq!(current.len(), 32);
    for index in 0..32 {
        assert_eq!(current.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// Coverage Tests: Large Vector Stress Test
// =============================================================================

#[rstest]
fn test_very_large_vector() {
    // Create a very large vector
    let vector: PersistentVector<i32> = (0..100_000).collect();

    // Spot check values
    assert_eq!(vector.get(0), Some(&0));
    assert_eq!(vector.get(50_000), Some(&50_000));
    assert_eq!(vector.get(99_999), Some(&99_999));
}

// =============================================================================
// Coverage Tests: Update with Child None Branch
// =============================================================================

#[rstest]
fn test_update_non_existent_index() {
    let vector: PersistentVector<i32> = (0..50).collect();

    // Try to update beyond the length
    assert!(vector.update(50, 100).is_none());
    assert!(vector.update(100, 100).is_none());
}

// =============================================================================
// Coverage Tests: Tail Size Edge Cases
// =============================================================================

#[rstest]
fn test_vector_with_full_tail() {
    // Create vector with exactly 32 elements (full tail)
    let vector: PersistentVector<i32> = (0..32).collect();

    assert_eq!(vector.len(), 32);
    assert_eq!(vector.last(), Some(&31));

    // Push one more to trigger tail overflow
    let extended = vector.push_back(32);
    assert_eq!(extended.len(), 33);
    assert_eq!(extended.last(), Some(&32));
}

#[rstest]
fn test_vector_with_empty_tail() {
    // After certain operations, tail might have specific sizes
    let vector: PersistentVector<i32> = (0..64).collect();

    // Pop to make tail have single element
    let (v63, _) = vector.pop_back().unwrap();

    // The new tail should be fetched from root
    assert_eq!(v63.len(), 63);
}

// =============================================================================
// Coverage Tests: fmap_ref_mut
// =============================================================================

#[rstest]
fn test_fmap_ref_mut_empty() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    let mapped: PersistentVector<i32> = vector.fmap_ref_mut(|x| x * 2);
    assert!(mapped.is_empty());
}

#[rstest]
fn test_fmap_ref_mut_large() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    let mapped: PersistentVector<i32> = vector.fmap_ref_mut(|x| x + 1);

    for index in 0..1000 {
        assert_eq!(mapped.get(index), Some(&((index as i32) + 1)));
    }

    // Original unchanged
    for index in 0..1000 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }
}

// =============================================================================
// Tests for slice method
// =============================================================================

#[rstest]
fn test_slice_basic() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(2, 5);

    assert_eq!(sliced.len(), 3);
    assert_eq!(sliced.get(0), Some(&2));
    assert_eq!(sliced.get(1), Some(&3));
    assert_eq!(sliced.get(2), Some(&4));
}

#[rstest]
fn test_slice_from_start() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(0, 3);

    assert_eq!(sliced.len(), 3);
    assert_eq!(sliced.get(0), Some(&0));
    assert_eq!(sliced.get(1), Some(&1));
    assert_eq!(sliced.get(2), Some(&2));
}

#[rstest]
fn test_slice_to_end() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(7, 10);

    assert_eq!(sliced.len(), 3);
    assert_eq!(sliced.get(0), Some(&7));
    assert_eq!(sliced.get(1), Some(&8));
    assert_eq!(sliced.get(2), Some(&9));
}

#[rstest]
fn test_slice_entire_vector() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(0, 10);

    assert_eq!(sliced.len(), 10);
    for index in 0..10 {
        assert_eq!(sliced.get(index), Some(&(index as i32)));
    }
}

#[rstest]
fn test_slice_empty_range() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(5, 5);

    assert!(sliced.is_empty());
}

#[rstest]
fn test_slice_invalid_range_start_greater_than_end() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(7, 3);

    assert!(sliced.is_empty());
}

#[rstest]
fn test_slice_start_out_of_bounds() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(20, 30);

    assert!(sliced.is_empty());
}

#[rstest]
fn test_slice_end_clamped_to_length() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(5, 100);

    assert_eq!(sliced.len(), 5);
    for index in 0..5 {
        assert_eq!(sliced.get(index), Some(&((index + 5) as i32)));
    }
}

#[rstest]
fn test_slice_on_empty_vector() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    let sliced = vector.slice(0, 5);

    assert!(sliced.is_empty());
}

#[rstest]
fn test_slice_does_not_modify_original() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(2, 5);

    // Original unchanged
    assert_eq!(vector.len(), 10);
    for index in 0..10 {
        assert_eq!(vector.get(index), Some(&(index as i32)));
    }

    // Slice is independent
    assert_eq!(sliced.len(), 3);
}

#[rstest]
fn test_slice_large_vector() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    let sliced = vector.slice(100, 200);

    assert_eq!(sliced.len(), 100);
    for index in 0..100 {
        assert_eq!(sliced.get(index), Some(&((index + 100) as i32)));
    }
}

#[rstest]
fn test_slice_single_element() {
    let vector: PersistentVector<i32> = (0..10).collect();
    let sliced = vector.slice(5, 6);

    assert_eq!(sliced.len(), 1);
    assert_eq!(sliced.get(0), Some(&5));
}

// =============================================================================
// Cycle 19: Optimized Iterator Tests
// =============================================================================

#[rstest]
fn test_optimized_iterator_empty_vector() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    let mut iterator = vector.iter();

    assert_eq!(iterator.next(), None);
    assert_eq!(iterator.len(), 0);
}

#[rstest]
fn test_optimized_iterator_single_element() {
    let vector = PersistentVector::singleton(42);
    let collected: Vec<&i32> = vector.iter().collect();

    assert_eq!(collected, vec![&42]);
}

#[rstest]
fn test_optimized_iterator_tail_only() {
    let vector: PersistentVector<i32> = (0..20).collect();
    let collected: Vec<i32> = vector.iter().copied().collect();
    let expected: Vec<i32> = (0..20).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_iterator_exactly_32_elements() {
    let vector: PersistentVector<i32> = (0..32).collect();
    let collected: Vec<i32> = vector.iter().copied().collect();
    let expected: Vec<i32> = (0..32).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_iterator_33_elements() {
    let vector: PersistentVector<i32> = (0..33).collect();
    let collected: Vec<i32> = vector.iter().copied().collect();
    let expected: Vec<i32> = (0..33).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_iterator_large_vector() {
    let vector: PersistentVector<i32> = (0..10000).collect();
    let collected: Vec<i32> = vector.iter().copied().collect();
    let expected: Vec<i32> = (0..10000).collect();

    assert_eq!(collected, expected);
}

#[rstest]
#[case(64)] // 32 in root + 32 in tail
#[case(65)] // 64 in root + 1 in tail
#[case(100)] // 96 in root + 4 in tail
#[case(1000)] // 992 in root + 8 in tail
fn test_optimized_iterator_with_partial_tail(#[case] size: usize) {
    #[allow(clippy::cast_possible_wrap)]
    let vector: PersistentVector<i32> = (0..size as i32).collect();
    let collected: Vec<i32> = vector.iter().copied().collect();
    #[allow(clippy::cast_possible_wrap)]
    let expected: Vec<i32> = (0..size as i32).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_iterator_size_hint() {
    let vector: PersistentVector<i32> = (0..100).collect();
    let mut iterator = vector.iter();

    assert_eq!(iterator.size_hint(), (100, Some(100)));

    // After consuming 50 elements
    for _ in 0..50 {
        iterator.next();
    }
    assert_eq!(iterator.size_hint(), (50, Some(50)));

    // After consuming all elements
    for _ in 0..50 {
        iterator.next();
    }
    assert_eq!(iterator.size_hint(), (0, Some(0)));
}

#[rstest]
fn test_optimized_iterator_exact_size() {
    let vector: PersistentVector<i32> = (0..100).collect();
    let mut iterator = vector.iter();

    assert_eq!(iterator.len(), 100);

    for expected_remaining in (0..100).rev() {
        iterator.next();
        assert_eq!(iterator.len(), expected_remaining);
    }
}

#[rstest]
fn test_optimized_into_iterator_correctness() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    let collected: Vec<i32> = vector.into_iter().collect();
    let expected: Vec<i32> = (0..1000).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_iterator_tree_boundary() {
    // 32^2 = 1024 elements (tree level 2)
    let vector: PersistentVector<i32> = (0..1024).collect();
    let collected: Vec<i32> = vector.iter().copied().collect();
    let expected: Vec<i32> = (0..1024).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_iterator_deep_tree() {
    // 100,000 elements for deep tree structure test
    let vector: PersistentVector<i32> = (0..100_000).collect();
    let collected: Vec<i32> = vector.iter().copied().collect();
    let expected: Vec<i32> = (0..100_000).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_into_iterator_empty() {
    let vector: PersistentVector<i32> = PersistentVector::new();
    let mut iterator = vector.into_iter();

    assert_eq!(iterator.next(), None);
    assert_eq!(iterator.len(), 0);
}

#[rstest]
fn test_optimized_into_iterator_single_element() {
    let vector = PersistentVector::singleton(42);
    let collected: Vec<i32> = vector.into_iter().collect();

    assert_eq!(collected, vec![42]);
}

#[rstest]
fn test_optimized_into_iterator_tail_only() {
    let vector: PersistentVector<i32> = (0..20).collect();
    let collected: Vec<i32> = vector.into_iter().collect();
    let expected: Vec<i32> = (0..20).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_into_iterator_exactly_32_elements() {
    let vector: PersistentVector<i32> = (0..32).collect();
    let collected: Vec<i32> = vector.into_iter().collect();
    let expected: Vec<i32> = (0..32).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_into_iterator_33_elements() {
    let vector: PersistentVector<i32> = (0..33).collect();
    let collected: Vec<i32> = vector.into_iter().collect();
    let expected: Vec<i32> = (0..33).collect();

    assert_eq!(collected, expected);
}

#[rstest]
fn test_optimized_into_iterator_size_hint() {
    let vector: PersistentVector<i32> = (0..100).collect();
    let mut iterator = vector.into_iter();

    assert_eq!(iterator.size_hint(), (100, Some(100)));

    // After consuming 50 elements
    for _ in 0..50 {
        iterator.next();
    }
    assert_eq!(iterator.size_hint(), (50, Some(50)));
}

#[rstest]
fn test_optimized_into_iterator_exact_size() {
    let vector: PersistentVector<i32> = (0..100).collect();
    let mut iterator = vector.into_iter();

    assert_eq!(iterator.len(), 100);

    for expected_remaining in (0..100).rev() {
        iterator.next();
        assert_eq!(iterator.len(), expected_remaining);
    }
}

#[rstest]
fn test_optimized_into_iterator_deep_tree() {
    // 100,000 elements for deep tree structure test
    let vector: PersistentVector<i32> = (0..100_000).collect();
    let collected: Vec<i32> = vector.into_iter().collect();
    let expected: Vec<i32> = (0..100_000).collect();

    assert_eq!(collected, expected);
}

// =============================================================================
// Cycle 20: push_tail_to_root Optimization Tests
// =============================================================================

mod push_tail_to_root_optimization_tests {
    use super::*;

    #[rstest]
    fn test_push_back_fills_tail() {
        // 32 elements (tail only)
        let vector: PersistentVector<i32> = (0..32).collect();
        assert_eq!(vector.len(), 32);
        let collected: Vec<i32> = vector.iter().copied().collect();
        let expected: Vec<i32> = (0..32).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_push_back_overflows_tail() {
        // 33 elements (push_tail_to_root is called)
        let vector: PersistentVector<i32> = (0..33).collect();
        assert_eq!(vector.len(), 33);
        let collected: Vec<i32> = vector.iter().copied().collect();
        let expected: Vec<i32> = (0..33).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_push_back_multiple_tail_overflows() {
        // 65 elements (push_tail_to_root is called multiple times)
        let vector: PersistentVector<i32> = (0..65).collect();
        assert_eq!(vector.len(), 65);
        let collected: Vec<i32> = vector.iter().copied().collect();
        let expected: Vec<i32> = (0..65).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_push_back_deep_tree() {
        // 1025 elements (tree level increases)
        let vector: PersistentVector<i32> = (0..1025).collect();
        assert_eq!(vector.len(), 1025);
        let collected: Vec<i32> = vector.iter().copied().collect();
        let expected: Vec<i32> = (0..1025).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_push_back_preserves_sharing() {
        // Original vector is not modified
        let original: PersistentVector<i32> = (0..32).collect();
        let extended = original.push_back(32);

        assert_eq!(original.len(), 32);
        assert_eq!(extended.len(), 33);
        assert_eq!(original.get(31), Some(&31));
        assert_eq!(extended.get(32), Some(&32));
    }
}

// =============================================================================
// Cycle 21: FromIterator Optimization Tests
// =============================================================================

mod from_iter_optimization_tests {
    use super::*;

    #[rstest]
    fn test_from_iter_empty() {
        let vector: PersistentVector<i32> = std::iter::empty().collect();
        assert!(vector.is_empty());
        assert_eq!(vector.len(), 0);
    }

    #[rstest]
    fn test_from_iter_single() {
        let vector: PersistentVector<i32> = std::iter::once(42).collect();
        assert_eq!(vector.len(), 1);
        assert_eq!(vector.get(0), Some(&42));
    }

    #[rstest]
    fn test_from_iter_tail_only() {
        let vector: PersistentVector<i32> = (0..20).collect();
        assert_eq!(vector.len(), 20);
        let collected: Vec<i32> = vector.iter().copied().collect();
        let expected: Vec<i32> = (0..20).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_from_iter_exactly_branching_factor() {
        let vector: PersistentVector<i32> = (0..32).collect();
        assert_eq!(vector.len(), 32);
        let collected: Vec<i32> = vector.iter().copied().collect();
        let expected: Vec<i32> = (0..32).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_from_iter_multiple_levels() {
        let vector: PersistentVector<i32> = (0..1000).collect();
        assert_eq!(vector.len(), 1000);
        let collected: Vec<i32> = vector.iter().copied().collect();
        let expected: Vec<i32> = (0..1000).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_from_iter_large() {
        let vector: PersistentVector<i32> = (0..10000).collect();
        assert_eq!(vector.len(), 10000);
        let collected: Vec<i32> = vector.iter().copied().collect();
        let expected: Vec<i32> = (0..10000).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_from_iter_preserves_order() {
        let source: Vec<i32> = vec![10, 20, 30, 40, 50];
        let vector: PersistentVector<i32> = source.iter().copied().collect();
        let collected: Vec<i32> = vector.iter().copied().collect();
        assert_eq!(collected, source);
    }

    #[rstest]
    fn test_from_iter_with_strings() {
        let source: Vec<String> = vec!["hello".to_string(), "world".to_string()];
        let vector: PersistentVector<String> = source.iter().cloned().collect();
        assert_eq!(vector.len(), 2);
        assert_eq!(vector.get(0), Some(&"hello".to_string()));
    }
}

// =============================================================================
// Cycle 22: push_back_many Tests
// =============================================================================

mod push_back_many_tests {
    use super::*;

    #[rstest]
    fn test_push_back_many_empty_iter() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        let extended = vector.push_back_many(Vec::<i32>::new());
        assert_eq!(extended.len(), 3);
        let collected: Vec<i32> = extended.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_push_back_many_to_empty_vector() {
        let vector: PersistentVector<i32> = PersistentVector::new();
        let extended = vector.push_back_many(vec![1, 2, 3]);
        assert_eq!(extended.len(), 3);
        let collected: Vec<i32> = extended.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_push_back_many_single_element() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        let extended = vector.push_back_many(vec![4]);
        assert_eq!(extended.len(), 4);
        let collected: Vec<i32> = extended.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3, 4]);
    }

    #[rstest]
    fn test_push_back_many_multiple_elements() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        let extended = vector.push_back_many(4..=6);
        assert_eq!(extended.len(), 6);
        let collected: Vec<i32> = extended.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3, 4, 5, 6]);
    }

    #[rstest]
    fn test_push_back_many_large() {
        let vector: PersistentVector<i32> = (0..100).collect();
        let extended = vector.push_back_many(100..200);
        assert_eq!(extended.len(), 200);
        let collected: Vec<i32> = extended.iter().copied().collect();
        let expected: Vec<i32> = (0..200).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_push_back_many_preserves_original() {
        let original: PersistentVector<i32> = (1..=3).collect();
        let _ = original.push_back_many(vec![4, 5, 6]);
        // Original vector is not modified
        assert_eq!(original.len(), 3);
        let collected: Vec<i32> = original.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_push_back_many_fills_tail() {
        // Tail is filled case
        let vector: PersistentVector<i32> = (0..30).collect();
        let extended = vector.push_back_many(30..32);
        assert_eq!(extended.len(), 32);
        let collected: Vec<i32> = extended.iter().copied().collect();
        let expected: Vec<i32> = (0..32).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_push_back_many_overflows_tail() {
        // Tail overflows case
        let vector: PersistentVector<i32> = (0..30).collect();
        let extended = vector.push_back_many(30..40);
        assert_eq!(extended.len(), 40);
        let collected: Vec<i32> = extended.iter().copied().collect();
        let expected: Vec<i32> = (0..40).collect();
        assert_eq!(collected, expected);
    }

    #[rstest]
    fn test_push_back_many_with_iterator() {
        let vector: PersistentVector<i32> = (1..=3).collect();
        let extended = vector.push_back_many((4..=6).map(|x| x * 2));
        let collected: Vec<i32> = extended.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3, 8, 10, 12]);
    }

    #[rstest]
    fn test_push_back_many_few_elements_uses_push_back() {
        // 4 or fewer elements use individual push_back
        let vector: PersistentVector<i32> = (1..=3).collect();
        let extended = vector.push_back_many(vec![4, 5]);
        assert_eq!(extended.len(), 5);
        let collected: Vec<i32> = extended.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3, 4, 5]);
    }
}

// =============================================================================
// Cycle 23: from_slice Tests
// =============================================================================

mod from_slice_tests {
    use super::*;

    #[rstest]
    fn test_from_slice_empty() {
        let vector = PersistentVector::<i32>::from_slice(&[]);
        assert!(vector.is_empty());
        assert_eq!(vector.len(), 0);
    }

    #[rstest]
    fn test_from_slice_single() {
        let vector = PersistentVector::from_slice(&[42]);
        assert_eq!(vector.len(), 1);
        assert_eq!(vector.get(0), Some(&42));
    }

    #[rstest]
    fn test_from_slice_multiple() {
        let vector = PersistentVector::from_slice(&[1, 2, 3, 4, 5]);
        assert_eq!(vector.len(), 5);
        let collected: Vec<i32> = vector.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3, 4, 5]);
    }

    #[rstest]
    fn test_from_slice_exactly_branching_factor() {
        let source: Vec<i32> = (0..32).collect();
        let vector = PersistentVector::from_slice(&source);
        assert_eq!(vector.len(), 32);
        let collected: Vec<i32> = vector.iter().copied().collect();
        assert_eq!(collected, source);
    }

    #[rstest]
    fn test_from_slice_large() {
        let source: Vec<i32> = (0..10000).collect();
        let vector = PersistentVector::from_slice(&source);
        assert_eq!(vector.len(), 10000);
        let collected: Vec<i32> = vector.iter().copied().collect();
        assert_eq!(collected, source);
    }

    #[rstest]
    fn test_from_slice_preserves_order() {
        let source = [10, 20, 30, 40, 50];
        let vector = PersistentVector::from_slice(&source);
        let collected: Vec<i32> = vector.iter().copied().collect();
        assert_eq!(collected, source.to_vec());
    }

    #[rstest]
    fn test_from_slice_equals_from_iter() {
        let source = vec![1, 2, 3, 4, 5];
        let from_slice = PersistentVector::from_slice(&source);
        let from_iter: PersistentVector<i32> = source.into_iter().collect();
        assert_eq!(from_slice, from_iter);
    }

    #[rstest]
    fn test_from_slice_with_strings() {
        let source = ["hello", "world", "rust"];
        let vector = PersistentVector::from_slice(&source);
        assert_eq!(vector.len(), 3);
        assert_eq!(vector.get(0), Some(&"hello"));
    }
}
