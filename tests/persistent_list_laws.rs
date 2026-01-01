//! Property-based tests for PersistentList.
//!
//! These tests verify that PersistentList satisfies the algebraic laws
//! for the type classes it implements.

use lambars::persistent::PersistentList;
use lambars::typeclass::{Foldable, FunctorMut, Monoid, Semigroup, Sum};
use proptest::prelude::*;

// =============================================================================
// Strategy for generating PersistentList
// =============================================================================

/// Generates a `PersistentList<i32>` with up to `max_size` elements.
fn persistent_list_strategy(max_size: usize) -> impl Strategy<Value = PersistentList<i32>> {
    prop::collection::vec(any::<i32>(), 0..max_size).prop_map(|vector| vector.into_iter().collect())
}

/// Generates a small `PersistentList<i32>` for faster tests.
fn small_list() -> impl Strategy<Value = PersistentList<i32>> {
    persistent_list_strategy(20)
}

proptest! {
    // =========================================================================
    // Basic Properties
    // =========================================================================

    #[test]
    fn prop_len_matches_iter_count(list in small_list()) {
        prop_assert_eq!(list.len(), list.iter().count());
    }

    #[test]
    fn prop_is_empty_matches_len_zero(list in small_list()) {
        prop_assert_eq!(list.is_empty(), list.len() == 0);
    }

    #[test]
    fn prop_cons_increases_len_by_one(list in small_list(), element: i32) {
        let new_list = list.cons(element);
        prop_assert_eq!(new_list.len(), list.len() + 1);
    }

    #[test]
    fn prop_cons_puts_element_at_head(list in small_list(), element: i32) {
        let new_list = list.cons(element);
        prop_assert_eq!(new_list.head(), Some(&element));
    }

    #[test]
    fn prop_tail_decreases_len_by_one(list in persistent_list_strategy(20).prop_filter("non-empty", |list| !list.is_empty())) {
        let tail = list.tail();
        prop_assert_eq!(tail.len(), list.len() - 1);
    }

    #[test]
    fn prop_uncons_returns_head_and_tail(list in persistent_list_strategy(20).prop_filter("non-empty", |list| !list.is_empty())) {
        if let Some((head, tail)) = list.uncons() {
            prop_assert_eq!(list.head(), Some(head));
            prop_assert_eq!(tail.len(), list.len() - 1);
        }
    }

    #[test]
    fn prop_get_within_bounds_returns_some(list in persistent_list_strategy(20).prop_filter("non-empty", |list| !list.is_empty())) {
        let index = 0; // Always valid for non-empty list
        prop_assert!(list.get(index).is_some());
    }

    #[test]
    fn prop_get_out_of_bounds_returns_none(list in small_list()) {
        prop_assert_eq!(list.get(list.len()), None);
        prop_assert_eq!(list.get(list.len() + 100), None);
    }

    // =========================================================================
    // Structural Sharing Properties
    // =========================================================================

    #[test]
    fn prop_tail_preserves_structure(list in persistent_list_strategy(20).prop_filter("non-empty", |list| !list.is_empty())) {
        let with_element = list.cons(999);
        let tail_of_new = with_element.tail();
        // tail should be equal to the original list
        prop_assert_eq!(tail_of_new, list);
    }

    // =========================================================================
    // Reverse Properties
    // =========================================================================

    #[test]
    fn prop_reverse_reverse_is_identity(list in small_list()) {
        let reversed_twice = list.clone().reverse().reverse();
        prop_assert_eq!(reversed_twice, list);
    }

    #[test]
    fn prop_reverse_preserves_length(list in small_list()) {
        let reversed = list.reverse();
        prop_assert_eq!(reversed.len(), list.len());
    }

    #[test]
    fn prop_reverse_empty_is_empty(_: ()) {
        let empty: PersistentList<i32> = PersistentList::new();
        let reversed = empty.reverse();
        prop_assert!(reversed.is_empty());
    }

    #[test]
    fn prop_reverse_singleton_is_same(element: i32) {
        let singleton = PersistentList::singleton(element);
        let reversed = singleton.reverse();
        prop_assert_eq!(reversed.head(), Some(&element));
    }

    // =========================================================================
    // Append Properties (Semigroup Laws)
    // =========================================================================

    #[test]
    fn prop_semigroup_associativity(
        list1 in small_list(),
        list2 in small_list(),
        list3 in small_list()
    ) {
        // (a + b) + c == a + (b + c)
        let left = list1.clone().combine(list2.clone()).combine(list3.clone());
        let right = list1.combine(list2.combine(list3));
        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_append_length(list1 in small_list(), list2 in small_list()) {
        let combined = list1.append(&list2);
        prop_assert_eq!(combined.len(), list1.len() + list2.len());
    }

    #[test]
    fn prop_append_empty_left_identity(list in small_list()) {
        let empty: PersistentList<i32> = PersistentList::new();
        let result = empty.append(&list);
        prop_assert_eq!(result, list);
    }

    #[test]
    fn prop_append_empty_right_identity(list in small_list()) {
        let empty: PersistentList<i32> = PersistentList::new();
        let result = list.append(&empty);
        prop_assert_eq!(result, list);
    }

    // =========================================================================
    // Monoid Laws
    // =========================================================================

    #[test]
    fn prop_monoid_left_identity(list in small_list()) {
        let empty: PersistentList<i32> = PersistentList::empty();
        let result = empty.combine(list.clone());
        prop_assert_eq!(result, list);
    }

    #[test]
    fn prop_monoid_right_identity(list in small_list()) {
        let empty: PersistentList<i32> = PersistentList::empty();
        let result = list.clone().combine(empty);
        prop_assert_eq!(result, list);
    }

    // =========================================================================
    // Functor Laws (using FunctorMut)
    // =========================================================================

    #[test]
    fn prop_functor_identity(list in small_list()) {
        // fmap id == id
        let mapped = list.clone().fmap_mut(|element| element);
        prop_assert_eq!(mapped, list);
    }

    #[test]
    fn prop_functor_composition(list in small_list()) {
        // fmap (g . f) == fmap g . fmap f
        let function1 = |element: i32| element.wrapping_add(1);
        let function2 = |element: i32| element.wrapping_mul(2);

        let left = list.clone().fmap_mut(function1).fmap_mut(function2);
        let right = list.fmap_mut(|element| function2(function1(element)));

        prop_assert_eq!(left, right);
    }

    // =========================================================================
    // Foldable Laws
    // =========================================================================

    #[test]
    fn prop_fold_left_sum_matches_iter_sum(list in small_list()) {
        // Use wrapping addition to avoid overflow
        let fold_sum = list.clone().fold_left(0i64, |accumulator, element| {
            accumulator.wrapping_add(i64::from(element))
        });
        let iter_sum: i64 = list.iter().map(|&element| i64::from(element)).sum();
        prop_assert_eq!(fold_sum, iter_sum);
    }

    #[test]
    fn prop_fold_map_sum(list in small_list()) {
        // fold_map with Sum should equal the sum of elements
        let fold_map_result: Sum<i64> = list.clone().fmap_mut(|element| Sum(i64::from(element))).fold_left(Sum(0), |accumulator, element| accumulator.combine(element));
        let direct_sum: i64 = list.iter().map(|&element| i64::from(element)).sum();
        prop_assert_eq!(fold_map_result.0, direct_sum);
    }

    #[test]
    fn prop_to_list_roundtrip(list in small_list()) {
        // Converting to Vec and back should preserve elements
        let as_vec: Vec<i32> = list.clone().into_iter().collect();
        let back_to_list: PersistentList<i32> = as_vec.into_iter().collect();
        prop_assert_eq!(back_to_list, list);
    }

    #[test]
    fn prop_length_matches_fold(list in small_list()) {
        let fold_count = list.clone().fold_left(0usize, |count, _| count + 1);
        prop_assert_eq!(fold_count, list.len());
    }

    // =========================================================================
    // FromIterator / IntoIterator Properties
    // =========================================================================

    #[test]
    fn prop_from_iter_preserves_order(elements in prop::collection::vec(any::<i32>(), 0..20)) {
        let list: PersistentList<i32> = elements.clone().into_iter().collect();
        let back_to_vec: Vec<i32> = list.into_iter().collect();
        prop_assert_eq!(back_to_vec, elements);
    }

    #[test]
    fn prop_into_iter_yields_all_elements(list in small_list()) {
        let collected: Vec<i32> = list.clone().into_iter().collect();
        prop_assert_eq!(collected.len(), list.len());
    }

    #[test]
    fn prop_iter_yields_same_as_into_iter(list in small_list()) {
        let iter_collected: Vec<&i32> = list.iter().collect();
        let into_iter_collected: Vec<i32> = list.clone().into_iter().collect();
        prop_assert_eq!(iter_collected.len(), into_iter_collected.len());
        for (reference, value) in iter_collected.iter().zip(into_iter_collected.iter()) {
            prop_assert_eq!(*reference, value);
        }
    }

    // =========================================================================
    // Equality Properties
    // =========================================================================

    #[test]
    fn prop_eq_reflexive(list in small_list()) {
        prop_assert_eq!(list.clone(), list);
    }

    #[test]
    fn prop_eq_symmetric(list1 in small_list(), list2 in small_list()) {
        prop_assert_eq!(list1 == list2, list2 == list1);
    }

    #[test]
    fn prop_clone_equals_original(list in small_list()) {
    let cloned = list.clone();
        prop_assert_eq!(cloned, list);
    }

    // =========================================================================
    // Additional Properties
    // =========================================================================

    #[test]
    fn prop_singleton_has_len_one(element: i32) {
        let singleton = PersistentList::singleton(element);
        prop_assert_eq!(singleton.len(), 1);
    }

    #[test]
    fn prop_head_of_singleton_is_element(element: i32) {
        let singleton = PersistentList::singleton(element);
        prop_assert_eq!(singleton.head(), Some(&element));
    }

    #[test]
    fn prop_tail_of_singleton_is_empty(element: i32) {
        let singleton = PersistentList::singleton(element);
        let tail = singleton.tail();
        prop_assert!(tail.is_empty());
    }

    #[test]
    fn prop_get_zero_equals_head(list in persistent_list_strategy(20).prop_filter("non-empty", |list| !list.is_empty())) {
        prop_assert_eq!(list.get(0), list.head());
    }
}
