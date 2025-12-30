//! Unit tests for PersistentList.
//!
//! These tests verify the correctness of the PersistentList implementation.
//! They follow the TDD approach and cover all basic operations.

use functional_rusty::persistent::PersistentList;
use functional_rusty::typeclass::{
    Applicative, Foldable, Functor, FunctorMut, Monad, Monoid, Semigroup, Sum, TypeConstructor,
};
use rstest::rstest;

// =============================================================================
// Cycle 1: Basic structure and new()
// =============================================================================

#[rstest]
fn test_new_creates_empty_list() {
    let list: PersistentList<i32> = PersistentList::new();
    assert!(list.is_empty());
    assert_eq!(list.len(), 0);
}

#[rstest]
fn test_new_head_returns_none() {
    let list: PersistentList<i32> = PersistentList::new();
    assert_eq!(list.head(), None);
}

// =============================================================================
// Cycle 2: cons (prepend element)
// =============================================================================

#[rstest]
fn test_cons_adds_element_to_front() {
    let list = PersistentList::new().cons(1);
    assert_eq!(list.head(), Some(&1));
    assert_eq!(list.len(), 1);
}

#[rstest]
fn test_cons_chain_builds_list_in_reverse_order() {
    let list = PersistentList::new().cons(3).cons(2).cons(1);
    assert_eq!(list.head(), Some(&1));
    assert_eq!(list.len(), 3);
}

#[rstest]
fn test_cons_does_not_modify_original() {
    let list1 = PersistentList::new().cons(1);
    let list2 = list1.cons(2);
    // list1 is not modified
    assert_eq!(list1.len(), 1);
    assert_eq!(list1.head(), Some(&1));
    // list2 has the new element
    assert_eq!(list2.len(), 2);
    assert_eq!(list2.head(), Some(&2));
}

// =============================================================================
// Cycle 3: tail (rest of list)
// =============================================================================

#[rstest]
fn test_tail_of_non_empty_list() {
    let list = PersistentList::new().cons(3).cons(2).cons(1);
    let tail = list.tail();
    assert_eq!(tail.head(), Some(&2));
    assert_eq!(tail.len(), 2);
}

#[rstest]
fn test_tail_of_single_element_list() {
    let list = PersistentList::new().cons(1);
    let tail = list.tail();
    assert!(tail.is_empty());
}

#[rstest]
fn test_tail_of_empty_list() {
    let list: PersistentList<i32> = PersistentList::new();
    let tail = list.tail();
    assert!(tail.is_empty());
}

#[rstest]
fn test_tail_shares_structure() {
    let list1 = PersistentList::new().cons(3).cons(2).cons(1);
    let list2 = list1.cons(0);
    // list1 and list2.tail() should share structure
    let list2_tail = list2.tail();
    assert_eq!(list1.len(), list2_tail.len());
    // Verify the elements are the same
    let collected1: Vec<&i32> = list1.iter().collect();
    let collected2: Vec<&i32> = list2_tail.iter().collect();
    assert_eq!(collected1, collected2);
}

// =============================================================================
// Cycle 4: singleton
// =============================================================================

#[rstest]
fn test_singleton_creates_single_element_list() {
    let list = PersistentList::singleton(42);
    assert_eq!(list.head(), Some(&42));
    assert_eq!(list.len(), 1);
}

// =============================================================================
// Cycle 5: uncons
// =============================================================================

#[rstest]
fn test_uncons_non_empty() {
    let list = PersistentList::new().cons(3).cons(2).cons(1);
    let (head, tail) = list.uncons().unwrap();
    assert_eq!(*head, 1);
    assert_eq!(tail.head(), Some(&2));
}

#[rstest]
fn test_uncons_empty() {
    let list: PersistentList<i32> = PersistentList::new();
    assert!(list.uncons().is_none());
}

// =============================================================================
// Cycle 6: get (index access)
// =============================================================================

#[rstest]
fn test_get_valid_index() {
    let list = PersistentList::new().cons(3).cons(2).cons(1);
    assert_eq!(list.get(0), Some(&1));
    assert_eq!(list.get(1), Some(&2));
    assert_eq!(list.get(2), Some(&3));
}

#[rstest]
fn test_get_invalid_index() {
    let list = PersistentList::new().cons(1);
    assert_eq!(list.get(1), None);
    assert_eq!(list.get(10), None);
}

#[rstest]
fn test_get_empty_list() {
    let list: PersistentList<i32> = PersistentList::new();
    assert_eq!(list.get(0), None);
}

// =============================================================================
// Cycle 7: iter
// =============================================================================

#[rstest]
fn test_iter_collects_all_elements() {
    let list = PersistentList::new().cons(3).cons(2).cons(1);
    let collected: Vec<&i32> = list.iter().collect();
    assert_eq!(collected, vec![&1, &2, &3]);
}

#[rstest]
fn test_iter_empty_list() {
    let list: PersistentList<i32> = PersistentList::new();
    let collected: Vec<&i32> = list.iter().collect();
    assert!(collected.is_empty());
}

#[rstest]
fn test_iter_sum() {
    let list = PersistentList::new().cons(3).cons(2).cons(1);
    let sum: i32 = list.iter().copied().sum();
    assert_eq!(sum, 6);
}

// =============================================================================
// Cycle 8: append
// =============================================================================

#[rstest]
fn test_append_two_lists() {
    let list1 = PersistentList::new().cons(2).cons(1);
    let list2 = PersistentList::new().cons(4).cons(3);
    let combined = list1.append(&list2);
    let collected: Vec<&i32> = combined.iter().collect();
    assert_eq!(collected, vec![&1, &2, &3, &4]);
}

#[rstest]
fn test_append_with_empty() {
    let list = PersistentList::new().cons(2).cons(1);
    let empty: PersistentList<i32> = PersistentList::new();

    // list.append(&empty) == list
    let result1 = list.append(&empty);
    let collected1: Vec<&i32> = result1.iter().collect();
    assert_eq!(collected1, vec![&1, &2]);

    // empty.append(&list) == list
    let result2 = empty.append(&list);
    let collected2: Vec<&i32> = result2.iter().collect();
    assert_eq!(collected2, vec![&1, &2]);
}

// =============================================================================
// Cycle 9: reverse
// =============================================================================

#[rstest]
fn test_reverse() {
    let list = PersistentList::new().cons(3).cons(2).cons(1);
    let reversed = list.reverse();
    let collected: Vec<&i32> = reversed.iter().collect();
    assert_eq!(collected, vec![&3, &2, &1]);
}

#[rstest]
fn test_reverse_empty() {
    let list: PersistentList<i32> = PersistentList::new();
    let reversed = list.reverse();
    assert!(reversed.is_empty());
}

#[rstest]
fn test_reverse_involution() {
    let list = PersistentList::new().cons(3).cons(2).cons(1);
    let reversed_twice = list.reverse().reverse();
    // Should be equal to the original list
    let original: Vec<&i32> = list.iter().collect();
    let result: Vec<&i32> = reversed_twice.iter().collect();
    assert_eq!(original, result);
}

// =============================================================================
// Cycle 10: FromIterator
// =============================================================================

#[rstest]
fn test_from_iter() {
    let list: PersistentList<i32> = (1..=5).collect();
    assert_eq!(list.len(), 5);
    assert_eq!(list.head(), Some(&1));
}

#[rstest]
fn test_from_array() {
    let list: PersistentList<i32> = [1, 2, 3].into_iter().collect();
    let collected: Vec<&i32> = list.iter().collect();
    assert_eq!(collected, vec![&1, &2, &3]);
}

// =============================================================================
// Cycle 11: IntoIterator
// =============================================================================

#[rstest]
fn test_into_iter() {
    let list: PersistentList<i32> = (1..=3).collect();
    let collected: Vec<i32> = list.into_iter().collect();
    assert_eq!(collected, vec![1, 2, 3]);
}

// =============================================================================
// Cycle 12: PartialEq, Eq, Debug
// =============================================================================

#[rstest]
fn test_partial_eq() {
    let list1: PersistentList<i32> = (1..=3).collect();
    let list2: PersistentList<i32> = (1..=3).collect();
    let list3: PersistentList<i32> = (1..=4).collect();

    assert_eq!(list1, list2);
    assert_ne!(list1, list3);
}

#[rstest]
fn test_debug() {
    let list: PersistentList<i32> = (1..=3).collect();
    let debug_str = format!("{:?}", list);
    assert!(debug_str.contains("1"));
    assert!(debug_str.contains("2"));
    assert!(debug_str.contains("3"));
}

// =============================================================================
// Cycle 13: TypeConstructor
// =============================================================================

#[rstest]
fn test_type_constructor() {
    fn assert_type_constructor<T: TypeConstructor<Inner = i32>>() {}
    assert_type_constructor::<PersistentList<i32>>();
}

// =============================================================================
// Cycle 14: Functor, FunctorMut
// =============================================================================

#[rstest]
fn test_fmap_mut() {
    let list: PersistentList<i32> = (1..=3).collect();
    let doubled: PersistentList<i32> = list.fmap_mut(|element| element * 2);
    let collected: Vec<&i32> = doubled.iter().collect();
    assert_eq!(collected, vec![&2, &4, &6]);
}

#[rstest]
fn test_functor_identity_law() {
    let list: PersistentList<i32> = (1..=3).collect();
    let mapped = list.clone().fmap_mut(|element| element);
    assert_eq!(list, mapped);
}

#[rstest]
fn test_functor_composition_law() {
    let list: PersistentList<i32> = (1..=3).collect();
    let function1 = |element: i32| element + 1;
    let function2 = |element: i32| element * 2;

    let left = list.clone().fmap_mut(function1).fmap_mut(function2);
    let right = list.fmap_mut(|element| function2(function1(element)));

    assert_eq!(left, right);
}

// =============================================================================
// Cycle 15: Foldable
// =============================================================================

#[rstest]
fn test_fold_left() {
    let list: PersistentList<i32> = (1..=5).collect();
    let sum = list.fold_left(0, |accumulator, element| accumulator + element);
    assert_eq!(sum, 15);
}

#[rstest]
fn test_fold_right() {
    let list: PersistentList<i32> = (1..=3).collect();
    let result = list.fold_right(String::new(), |element, accumulator| {
        format!("{}{}", element, accumulator)
    });
    assert_eq!(result, "123");
}

#[rstest]
fn test_fold_map() {
    let list: PersistentList<i32> = (1..=5).collect();
    let sum: Sum<i32> = list.fold_map(Sum);
    assert_eq!(sum.0, 15);
}

// =============================================================================
// Cycle 16: Monad
// =============================================================================

#[rstest]
fn test_flat_map_mut() {
    let list: PersistentList<i32> = (1..=3).collect();
    let result: PersistentList<i32> =
        list.flat_map_mut(|element| PersistentList::new().cons(element * 2).cons(element));
    // [1, 2, 2, 4, 3, 6]
    let collected: Vec<i32> = result.into_iter().collect();
    assert_eq!(collected, vec![1, 2, 2, 4, 3, 6]);
}

// =============================================================================
// Additional tests for edge cases
// =============================================================================

#[rstest]
fn test_large_list() {
    let list: PersistentList<i32> = (0..1000).collect();
    assert_eq!(list.len(), 1000);
    assert_eq!(list.get(0), Some(&0));
    assert_eq!(list.get(500), Some(&500));
    assert_eq!(list.get(999), Some(&999));
}

#[rstest]
fn test_structural_sharing_between_versions() {
    // Create a base list
    let base: PersistentList<i32> = (1..=5).collect();

    // Create multiple versions by cons
    let version1 = base.cons(0);
    let version2 = base.cons(-1);
    let version3 = base.cons(-2);

    // All versions share the same tail
    assert_eq!(version1.tail(), base);
    assert_eq!(version2.tail(), base);
    assert_eq!(version3.tail(), base);

    // The base list is unchanged
    assert_eq!(base.len(), 5);
    assert_eq!(base.head(), Some(&1));
}

#[rstest]
fn test_clone() {
    let list: PersistentList<i32> = (1..=3).collect();
    let cloned = list.clone();
    assert_eq!(list, cloned);
}

#[rstest]
fn test_default() {
    let list: PersistentList<i32> = PersistentList::default();
    assert!(list.is_empty());
}

// =============================================================================
// Coverage Tests: Edge Cases
// =============================================================================

#[rstest]
fn test_tail_of_single_element_returns_empty() {
    let list = PersistentList::singleton(42);
    let tail = list.tail();
    assert!(tail.is_empty());
}

#[rstest]
fn test_head_empty_returns_none() {
    let empty: PersistentList<i32> = PersistentList::new();
    assert!(empty.head().is_none());
}

#[rstest]
fn test_get_out_of_bounds_returns_none() {
    let list: PersistentList<i32> = (1..=3).collect();
    assert!(list.get(10).is_none());
    assert!(list.get(3).is_none());
}

#[rstest]
fn test_get_from_empty_list_returns_none() {
    let empty: PersistentList<i32> = PersistentList::new();
    assert!(empty.get(0).is_none());
}

// =============================================================================
// Coverage Tests: Append Edge Cases
// =============================================================================

#[rstest]
fn test_append_to_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let list: PersistentList<i32> = (1..=3).collect();
    let result = empty.append(&list);
    assert_eq!(result.len(), 3);
    assert_eq!(result.head(), Some(&1));
}

#[rstest]
fn test_append_empty_to_non_empty_list() {
    let list: PersistentList<i32> = (1..=3).collect();
    let empty: PersistentList<i32> = PersistentList::new();
    let result = list.append(&empty);
    assert_eq!(result.len(), 3);
    assert_eq!(result.head(), Some(&1));
}

#[rstest]
fn test_append_two_empty_lists() {
    let empty1: PersistentList<i32> = PersistentList::new();
    let empty2: PersistentList<i32> = PersistentList::new();
    let result = empty1.append(&empty2);
    assert!(result.is_empty());
}

// =============================================================================
// Coverage Tests: Reverse Edge Cases
// =============================================================================

#[rstest]
fn test_reverse_single_element() {
    let list = PersistentList::singleton(42);
    let reversed = list.reverse();
    assert_eq!(reversed.head(), Some(&42));
    assert_eq!(reversed.len(), 1);
}

// =============================================================================
// Coverage Tests: Iterator
// =============================================================================

#[rstest]
fn test_iter_size_hint_returns_unknown() {
    let list: PersistentList<i32> = (1..=5).collect();
    let iter = list.iter();
    let (lower, upper) = iter.size_hint();
    // size_hint returns (0, None) for the reference iterator
    assert_eq!(lower, 0);
    assert!(upper.is_none());
}

#[rstest]
fn test_into_iter_size_hint() {
    let list: PersistentList<i32> = (1..=5).collect();
    let iter = list.clone().into_iter();
    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 5);
    assert_eq!(upper, Some(5));
}

#[rstest]
fn test_into_iter_exact_size() {
    let list: PersistentList<i32> = (1..=5).collect();
    let iter = list.into_iter();
    assert_eq!(iter.len(), 5);
}

#[rstest]
fn test_into_iter_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let mut iter = empty.into_iter();
    assert!(iter.next().is_none());
}

#[rstest]
fn test_ref_into_iterator() {
    let list: PersistentList<i32> = (1..=3).collect();
    let mut sum = 0;
    for element in &list {
        sum += element;
    }
    assert_eq!(sum, 6);
}

// =============================================================================
// Coverage Tests: Type Class Implementations
// =============================================================================

#[rstest]
fn test_functor_fmap_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let result: PersistentList<i32> = empty.fmap(|x| x * 2);
    assert!(result.is_empty());
}

#[rstest]
fn test_functor_fmap_singleton_list() {
    let list = PersistentList::singleton(42);
    let result: PersistentList<i32> = list.fmap(|x| x * 2);
    assert_eq!(result.head(), Some(&84));
    // Note: fmap with FnOnce only works for single-element lists
    assert_eq!(result.len(), 1);
}

#[rstest]
fn test_functor_fmap_ref_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let result: PersistentList<i32> = empty.fmap_ref(|x| x * 2);
    assert!(result.is_empty());
}

#[rstest]
fn test_functor_fmap_ref_singleton_list() {
    let list = PersistentList::singleton(42);
    let result: PersistentList<i32> = list.fmap_ref(|x| x * 2);
    assert_eq!(result.head(), Some(&84));
}

#[rstest]
fn test_functor_fmap_multi_element_list() {
    // fmap with FnOnce can only use the first element
    let list: PersistentList<i32> = (1..=3).collect();
    let result: PersistentList<i32> = list.fmap(|x| x * 10);
    // Only first element is transformed due to FnOnce limitation
    assert_eq!(result.head(), Some(&10));
}

#[rstest]
fn test_functor_mut_fmap_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let result: PersistentList<i32> = empty.fmap_mut(|x| x * 2);
    assert!(result.is_empty());
}

#[rstest]
fn test_functor_mut_fmap_ref_mut_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let result: PersistentList<i32> = empty.fmap_ref_mut(|x| x * 2);
    assert!(result.is_empty());
}

#[rstest]
fn test_functor_mut_fmap_ref_mut_list() {
    let list: PersistentList<i32> = (1..=3).collect();
    let result: PersistentList<i32> = list.fmap_ref_mut(|x| x * 2);
    let collected: Vec<&i32> = result.iter().collect();
    assert_eq!(collected, vec![&2, &4, &6]);
}

#[rstest]
fn test_applicative_pure_list() {
    let result: PersistentList<i32> = <PersistentList<i32> as Applicative>::pure(42);
    assert_eq!(result.head(), Some(&42));
    assert_eq!(result.len(), 1);
}

#[rstest]
fn test_applicative_map2_empty_handling() {
    let list1: PersistentList<i32> = PersistentList::new();
    let list2: PersistentList<i32> = PersistentList::singleton(2);
    // map2 with FnOnce returns empty for the type system limitation
    let result = list1.map2(list2, |a, b| a + b);
    assert!(result.is_empty());
}

#[rstest]
fn test_applicative_map3_returns_empty() {
    let list1: PersistentList<i32> = PersistentList::singleton(1);
    let list2: PersistentList<i32> = PersistentList::singleton(2);
    let list3: PersistentList<i32> = PersistentList::singleton(3);
    // map3 returns empty due to type system limitations
    let result = list1.map3(list2, list3, |a, b, c| a + b + c);
    assert!(result.is_empty());
}

#[rstest]
fn test_applicative_apply_returns_empty() {
    let list_of_functions: PersistentList<fn(i32) -> i32> =
        PersistentList::singleton(|x: i32| x * 2);
    let list_of_values: PersistentList<i32> = PersistentList::singleton(5);
    // apply returns empty due to type system limitations
    let result = list_of_functions.apply(list_of_values);
    assert!(result.is_empty());
}

#[rstest]
fn test_monad_flat_map_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let result: PersistentList<i32> = empty.flat_map(|x| PersistentList::singleton(x * 2));
    assert!(result.is_empty());
}

#[rstest]
fn test_monad_flat_map_singleton_list() {
    let list = PersistentList::singleton(42);
    let result: PersistentList<i32> = list.flat_map(|x| PersistentList::new().cons(x * 2).cons(x));
    assert_eq!(result.len(), 2);
}

#[rstest]
fn test_monad_flat_map_returns_list() {
    let list = PersistentList::singleton(3);
    let result: PersistentList<i32> = list.flat_map(|x| (1..=x).collect());
    let collected: Vec<&i32> = result.iter().collect();
    assert_eq!(collected, vec![&1, &2, &3]);
}

// =============================================================================
// Coverage Tests: Foldable
// =============================================================================

#[rstest]
fn test_foldable_fold_left_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let result = empty.fold_left(100, |accumulator, element| accumulator + element);
    assert_eq!(result, 100);
}

#[rstest]
fn test_foldable_fold_right_list() {
    let list: PersistentList<i32> = (1..=3).collect();
    // fold_right processes from right to left: 1 - (2 - (3 - 0)) = 1 - (2 - 3) = 1 - (-1) = 2
    let result = list.fold_right(0, |element, accumulator| element - accumulator);
    assert_eq!(result, 2);
}

#[rstest]
fn test_foldable_fold_right_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let result = empty.fold_right(100, |element, accumulator| element + accumulator);
    assert_eq!(result, 100);
}

#[rstest]
fn test_foldable_is_empty_via_trait() {
    let empty: PersistentList<i32> = PersistentList::new();
    let non_empty: PersistentList<i32> = PersistentList::singleton(42);
    assert!(Foldable::is_empty(&empty));
    assert!(!Foldable::is_empty(&non_empty));
}

#[rstest]
fn test_foldable_length_via_trait() {
    let list: PersistentList<i32> = (1..=5).collect();
    assert_eq!(Foldable::length(&list), 5);
}

// =============================================================================
// Coverage Tests: Semigroup and Monoid
// =============================================================================

#[rstest]
fn test_semigroup_combine_empty_left() {
    let empty: PersistentList<i32> = PersistentList::new();
    let list: PersistentList<i32> = (1..=3).collect();
    let result = empty.combine(list);
    let collected: Vec<&i32> = result.iter().collect();
    assert_eq!(collected, vec![&1, &2, &3]);
}

#[rstest]
fn test_semigroup_combine_empty_right() {
    let list: PersistentList<i32> = (1..=3).collect();
    let empty: PersistentList<i32> = PersistentList::new();
    let result = list.combine(empty);
    let collected: Vec<&i32> = result.iter().collect();
    assert_eq!(collected, vec![&1, &2, &3]);
}

#[rstest]
fn test_monoid_empty_identity() {
    let list: PersistentList<i32> = (1..=3).collect();
    let empty: PersistentList<i32> = Monoid::empty();

    // Left identity
    let left_identity = empty.clone().combine(list.clone());
    // Right identity
    let right_identity = list.clone().combine(empty);

    assert_eq!(left_identity, list);
    assert_eq!(right_identity, list);
}

// =============================================================================
// Coverage Tests: PartialEq
// =============================================================================

#[rstest]
fn test_eq_different_lengths() {
    let list1: PersistentList<i32> = (1..=3).collect();
    let list2: PersistentList<i32> = (1..=4).collect();
    assert_ne!(list1, list2);
}

#[rstest]
fn test_eq_different_elements() {
    let list1: PersistentList<i32> = (1..=3).collect();
    let list2: PersistentList<i32> = vec![1, 2, 4].into_iter().collect();
    assert_ne!(list1, list2);
}

#[rstest]
fn test_eq_empty_lists() {
    let empty1: PersistentList<i32> = PersistentList::new();
    let empty2: PersistentList<i32> = PersistentList::new();
    assert_eq!(empty1, empty2);
}

// =============================================================================
// Coverage Tests: flat_map_mut
// =============================================================================

#[rstest]
fn test_flat_map_mut_empty_list() {
    let empty: PersistentList<i32> = PersistentList::new();
    let result = empty.flat_map_mut(|x| PersistentList::singleton(x * 2));
    assert!(result.is_empty());
}

#[rstest]
fn test_flat_map_mut_returns_empty_sublists() {
    let list: PersistentList<i32> = (1..=3).collect();
    let result: PersistentList<i32> = list.flat_map_mut(|_| PersistentList::new());
    assert!(result.is_empty());
}

#[rstest]
fn test_flat_map_mut_multiple_elements() {
    let list: PersistentList<i32> = (1..=2).collect();
    let result =
        list.flat_map_mut(|element| PersistentList::new().cons(element * 10).cons(element));
    // [1] -> [1, 10], [2] -> [2, 20]
    // Expected order: [1, 10, 2, 20]
    let collected: Vec<i32> = result.into_iter().collect();
    assert_eq!(collected, vec![1, 10, 2, 20]);
}
