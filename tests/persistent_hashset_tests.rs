//! Unit tests for PersistentHashSet.
//!
//! These tests follow the TDD approach, testing all API methods
//! and edge cases for the PersistentHashSet implementation.

use functional_rusty::persistent::PersistentHashSet;
use rstest::rstest;

// =============================================================================
// TDD Cycle 1: Empty set creation
// =============================================================================

#[rstest]
fn test_new_creates_empty_set() {
    let set: PersistentHashSet<i32> = PersistentHashSet::new();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[rstest]
fn test_default_creates_empty_set() {
    let set: PersistentHashSet<i32> = PersistentHashSet::default();
    assert!(set.is_empty());
}

// =============================================================================
// TDD Cycle 2: Insert and contains basic operations
// =============================================================================

#[rstest]
fn test_singleton_creates_single_element_set() {
    let set = PersistentHashSet::singleton(42);
    assert_eq!(set.len(), 1);
    assert!(set.contains(&42));
}

#[rstest]
fn test_insert_single_element() {
    let set = PersistentHashSet::new().insert(42);
    assert_eq!(set.len(), 1);
    assert!(set.contains(&42));
}

#[rstest]
fn test_insert_multiple_elements() {
    let set = PersistentHashSet::new().insert(1).insert(2).insert(3);

    assert_eq!(set.len(), 3);
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
    assert!(!set.contains(&4));
}

#[rstest]
fn test_insert_duplicate_does_not_increase_length() {
    let set1 = PersistentHashSet::new().insert(42);
    let set2 = set1.insert(42);

    assert_eq!(set1.len(), 1);
    assert_eq!(set2.len(), 1);
}

#[rstest]
fn test_insert_does_not_modify_original() {
    let set1 = PersistentHashSet::new().insert(1);
    let set2 = set1.insert(2);

    assert_eq!(set1.len(), 1);
    assert!(set1.contains(&1));
    assert!(!set1.contains(&2));

    assert_eq!(set2.len(), 2);
    assert!(set2.contains(&1));
    assert!(set2.contains(&2));
}

#[rstest]
fn test_contains_with_borrow() {
    let set = PersistentHashSet::new()
        .insert("hello".to_string())
        .insert("world".to_string());

    // Test using &str to look up String
    assert!(set.contains("hello"));
    assert!(set.contains("world"));
    assert!(!set.contains("other"));
}

// =============================================================================
// TDD Cycle 3: Remove operation
// =============================================================================

#[rstest]
fn test_remove_existing_element() {
    let set = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let removed = set.remove(&2);

    assert_eq!(removed.len(), 2);
    assert!(removed.contains(&1));
    assert!(!removed.contains(&2));
    assert!(removed.contains(&3));
}

#[rstest]
fn test_remove_non_existing_element() {
    let set = PersistentHashSet::new().insert(1).insert(2);
    let removed = set.remove(&3);

    assert_eq!(removed.len(), 2);
    assert!(removed.contains(&1));
    assert!(removed.contains(&2));
}

#[rstest]
fn test_remove_does_not_modify_original() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = set1.remove(&1);

    assert_eq!(set1.len(), 2);
    assert!(set1.contains(&1));

    assert_eq!(set2.len(), 1);
    assert!(!set2.contains(&1));
}

#[rstest]
fn test_remove_all_elements() {
    let set = PersistentHashSet::new().insert(42);
    let empty = set.remove(&42);

    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
}

// =============================================================================
// TDD Cycle 4: Union (set union)
// =============================================================================

#[rstest]
fn test_union_of_two_sets() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(2).insert(3);
    let union = set1.union(&set2);

    assert_eq!(union.len(), 3);
    assert!(union.contains(&1));
    assert!(union.contains(&2));
    assert!(union.contains(&3));
}

#[rstest]
fn test_union_with_empty_set() {
    let set = PersistentHashSet::new().insert(1).insert(2);
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();

    let union1 = set.union(&empty);
    let union2 = empty.union(&set);

    assert_eq!(union1.len(), 2);
    assert_eq!(union2.len(), 2);
}

#[rstest]
fn test_union_of_disjoint_sets() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(3).insert(4);
    let union = set1.union(&set2);

    assert_eq!(union.len(), 4);
    assert!(union.contains(&1));
    assert!(union.contains(&2));
    assert!(union.contains(&3));
    assert!(union.contains(&4));
}

#[rstest]
fn test_union_does_not_modify_originals() {
    let set1 = PersistentHashSet::new().insert(1);
    let set2 = PersistentHashSet::new().insert(2);
    let _union = set1.union(&set2);

    assert_eq!(set1.len(), 1);
    assert_eq!(set2.len(), 1);
}

// =============================================================================
// TDD Cycle 5: Intersection (set intersection)
// =============================================================================

#[rstest]
fn test_intersection_of_overlapping_sets() {
    let set1 = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let set2 = PersistentHashSet::new().insert(2).insert(3).insert(4);
    let intersection = set1.intersection(&set2);

    assert_eq!(intersection.len(), 2);
    assert!(intersection.contains(&2));
    assert!(intersection.contains(&3));
    assert!(!intersection.contains(&1));
    assert!(!intersection.contains(&4));
}

#[rstest]
fn test_intersection_of_disjoint_sets() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(3).insert(4);
    let intersection = set1.intersection(&set2);

    assert!(intersection.is_empty());
}

#[rstest]
fn test_intersection_with_empty_set() {
    let set = PersistentHashSet::new().insert(1).insert(2);
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();

    let intersection = set.intersection(&empty);

    assert!(intersection.is_empty());
}

#[rstest]
fn test_intersection_with_self() {
    let set = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let intersection = set.intersection(&set);

    assert_eq!(intersection.len(), 3);
    assert!(intersection.contains(&1));
    assert!(intersection.contains(&2));
    assert!(intersection.contains(&3));
}

// =============================================================================
// TDD Cycle 6: Difference (set difference)
// =============================================================================

#[rstest]
fn test_difference_of_overlapping_sets() {
    let set1 = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let set2 = PersistentHashSet::new().insert(2).insert(3).insert(4);
    let difference = set1.difference(&set2);

    assert_eq!(difference.len(), 1);
    assert!(difference.contains(&1));
    assert!(!difference.contains(&2));
    assert!(!difference.contains(&3));
}

#[rstest]
fn test_difference_of_disjoint_sets() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(3).insert(4);
    let difference = set1.difference(&set2);

    assert_eq!(difference.len(), 2);
    assert!(difference.contains(&1));
    assert!(difference.contains(&2));
}

#[rstest]
fn test_difference_with_empty_set() {
    let set = PersistentHashSet::new().insert(1).insert(2);
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();

    let difference = set.difference(&empty);

    assert_eq!(difference.len(), 2);
    assert!(difference.contains(&1));
    assert!(difference.contains(&2));
}

#[rstest]
fn test_difference_from_empty_set() {
    let set = PersistentHashSet::new().insert(1).insert(2);
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();

    let difference = empty.difference(&set);

    assert!(difference.is_empty());
}

#[rstest]
fn test_difference_with_self() {
    let set = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let difference = set.difference(&set);

    assert!(difference.is_empty());
}

// =============================================================================
// TDD Cycle 7: Symmetric difference
// =============================================================================

#[rstest]
fn test_symmetric_difference_of_overlapping_sets() {
    let set1 = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let set2 = PersistentHashSet::new().insert(2).insert(3).insert(4);
    let symmetric_difference = set1.symmetric_difference(&set2);

    assert_eq!(symmetric_difference.len(), 2);
    assert!(symmetric_difference.contains(&1));
    assert!(symmetric_difference.contains(&4));
    assert!(!symmetric_difference.contains(&2));
    assert!(!symmetric_difference.contains(&3));
}

#[rstest]
fn test_symmetric_difference_of_disjoint_sets() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(3).insert(4);
    let symmetric_difference = set1.symmetric_difference(&set2);

    assert_eq!(symmetric_difference.len(), 4);
}

#[rstest]
fn test_symmetric_difference_with_self() {
    let set = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let symmetric_difference = set.symmetric_difference(&set);

    assert!(symmetric_difference.is_empty());
}

#[rstest]
fn test_symmetric_difference_with_empty() {
    let set = PersistentHashSet::new().insert(1).insert(2);
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();

    let symmetric_difference = set.symmetric_difference(&empty);

    assert_eq!(symmetric_difference.len(), 2);
}

// =============================================================================
// TDD Cycle 8: is_subset, is_superset, is_disjoint
// =============================================================================

#[rstest]
fn test_is_subset_true() {
    let subset = PersistentHashSet::new().insert(1).insert(2);
    let superset = PersistentHashSet::new().insert(1).insert(2).insert(3);

    assert!(subset.is_subset(&superset));
}

#[rstest]
fn test_is_subset_false() {
    let set1 = PersistentHashSet::new().insert(1).insert(4);
    let set2 = PersistentHashSet::new().insert(1).insert(2).insert(3);

    assert!(!set1.is_subset(&set2));
}

#[rstest]
fn test_is_subset_self() {
    let set = PersistentHashSet::new().insert(1).insert(2);

    assert!(set.is_subset(&set));
}

#[rstest]
fn test_is_subset_empty() {
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();
    let set = PersistentHashSet::new().insert(1).insert(2);

    assert!(empty.is_subset(&set));
    assert!(empty.is_subset(&empty));
}

#[rstest]
fn test_is_superset_true() {
    let superset = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let subset = PersistentHashSet::new().insert(1).insert(2);

    assert!(superset.is_superset(&subset));
}

#[rstest]
fn test_is_superset_false() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(1).insert(2).insert(3);

    assert!(!set1.is_superset(&set2));
}

#[rstest]
fn test_is_superset_empty() {
    let set = PersistentHashSet::new().insert(1).insert(2);
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();

    assert!(set.is_superset(&empty));
}

#[rstest]
fn test_is_disjoint_true() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(3).insert(4);

    assert!(set1.is_disjoint(&set2));
    assert!(set2.is_disjoint(&set1));
}

#[rstest]
fn test_is_disjoint_false() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(2).insert(3);

    assert!(!set1.is_disjoint(&set2));
}

#[rstest]
fn test_is_disjoint_with_empty() {
    let set = PersistentHashSet::new().insert(1).insert(2);
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();

    assert!(set.is_disjoint(&empty));
    assert!(empty.is_disjoint(&set));
}

// =============================================================================
// TDD Cycle 9: Iterator
// =============================================================================

#[rstest]
fn test_iter_empty_set() {
    let set: PersistentHashSet<i32> = PersistentHashSet::new();
    let collected: Vec<_> = set.iter().collect();

    assert!(collected.is_empty());
}

#[rstest]
fn test_iter_non_empty_set() {
    let set = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let mut collected: Vec<_> = set.iter().cloned().collect();
    collected.sort();

    assert_eq!(collected, vec![1, 2, 3]);
}

#[rstest]
fn test_iter_size_hint() {
    let set = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let iterator = set.iter();

    assert_eq!(iterator.size_hint(), (3, Some(3)));
}

#[rstest]
fn test_into_iter() {
    let set = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let mut collected: Vec<_> = set.into_iter().collect();
    collected.sort();

    assert_eq!(collected, vec![1, 2, 3]);
}

// =============================================================================
// TDD Cycle 10: FromIterator
// =============================================================================

#[rstest]
fn test_from_iter_empty() {
    let empty: Vec<i32> = vec![];
    let set: PersistentHashSet<i32> = empty.into_iter().collect();

    assert!(set.is_empty());
}

#[rstest]
fn test_from_iter_with_elements() {
    let set: PersistentHashSet<i32> = vec![1, 2, 3].into_iter().collect();

    assert_eq!(set.len(), 3);
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
}

#[rstest]
fn test_from_iter_with_duplicates() {
    let set: PersistentHashSet<i32> = vec![1, 2, 2, 3, 3, 3].into_iter().collect();

    assert_eq!(set.len(), 3);
}

#[rstest]
fn test_from_array() {
    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();

    assert_eq!(set.len(), 3);
}

#[rstest]
fn test_from_range() {
    let set: PersistentHashSet<i32> = (1..=5).collect();

    assert_eq!(set.len(), 5);
    for element in 1..=5 {
        assert!(set.contains(&element));
    }
}

// =============================================================================
// TDD Cycle 11: PartialEq, Eq, Debug
// =============================================================================

#[rstest]
fn test_eq_same_elements() {
    let set1 = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let set2 = PersistentHashSet::new().insert(3).insert(1).insert(2);

    assert_eq!(set1, set2);
}

#[rstest]
fn test_eq_different_elements() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(1).insert(3);

    assert_ne!(set1, set2);
}

#[rstest]
fn test_eq_different_lengths() {
    let set1 = PersistentHashSet::new().insert(1).insert(2);
    let set2 = PersistentHashSet::new().insert(1);

    assert_ne!(set1, set2);
}

#[rstest]
fn test_eq_empty_sets() {
    let set1: PersistentHashSet<i32> = PersistentHashSet::new();
    let set2: PersistentHashSet<i32> = PersistentHashSet::new();

    assert_eq!(set1, set2);
}

#[rstest]
fn test_debug_format() {
    let set = PersistentHashSet::new().insert(1).insert(2);
    let debug_string = format!("{:?}", set);

    // Check that it contains curly braces (set notation)
    assert!(debug_string.contains('{'));
    assert!(debug_string.contains('}'));
}

// =============================================================================
// TDD Cycle 12: Foldable trait
// =============================================================================

#[rstest]
fn test_fold_left_sum() {
    use functional_rusty::typeclass::Foldable;

    let set: PersistentHashSet<i32> = [1, 2, 3, 4, 5].into_iter().collect();
    let sum = set.fold_left(0, |accumulator, element| accumulator + element);

    assert_eq!(sum, 15);
}

#[rstest]
fn test_fold_left_empty() {
    use functional_rusty::typeclass::Foldable;

    let set: PersistentHashSet<i32> = PersistentHashSet::new();
    let sum = set.fold_left(0, |accumulator, element| accumulator + element);

    assert_eq!(sum, 0);
}

#[rstest]
fn test_foldable_is_empty() {
    use functional_rusty::typeclass::Foldable;

    let empty: PersistentHashSet<i32> = PersistentHashSet::new();
    let non_empty = PersistentHashSet::new().insert(42);

    assert!(Foldable::is_empty(&empty));
    assert!(!Foldable::is_empty(&non_empty));
}

#[rstest]
fn test_foldable_length() {
    use functional_rusty::typeclass::Foldable;

    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();

    assert_eq!(Foldable::length(&set), 3);
}

// =============================================================================
// Edge cases and stress tests
// =============================================================================

#[rstest]
fn test_large_set() {
    let set: PersistentHashSet<i32> = (0..10000).collect();

    assert_eq!(set.len(), 10000);

    for element in 0..10000 {
        assert!(set.contains(&element));
    }
    assert!(!set.contains(&10000));
}

#[rstest]
fn test_many_insertions_and_removals() {
    let mut set: PersistentHashSet<i32> = PersistentHashSet::new();

    for element in 0..1000 {
        set = set.insert(element);
    }
    assert_eq!(set.len(), 1000);

    for element in 0..500 {
        set = set.remove(&element);
    }
    assert_eq!(set.len(), 500);

    for element in 500..1000 {
        assert!(set.contains(&element));
    }
}

#[rstest]
fn test_structural_sharing() {
    let set1 = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let set2 = set1.insert(4);
    let set3 = set1.remove(&1);

    // All three sets should be valid and independent
    assert_eq!(set1.len(), 3);
    assert_eq!(set2.len(), 4);
    assert_eq!(set3.len(), 2);

    assert!(set1.contains(&1));
    assert!(!set3.contains(&1));
    assert!(set2.contains(&4));
}

#[rstest]
fn test_clone() {
    let set1 = PersistentHashSet::new().insert(1).insert(2).insert(3);
    let set2 = set1.clone();

    assert_eq!(set1, set2);
    assert_eq!(set1.len(), set2.len());
}

// =============================================================================
// Type inference and generic tests
// =============================================================================

#[rstest]
fn test_with_string_elements() {
    let set = PersistentHashSet::new()
        .insert("apple".to_string())
        .insert("banana".to_string())
        .insert("cherry".to_string());

    assert_eq!(set.len(), 3);
    assert!(set.contains("apple"));
    assert!(set.contains("banana"));
    assert!(set.contains("cherry"));
}

#[rstest]
fn test_with_tuple_elements() {
    let set = PersistentHashSet::new()
        .insert((1, "one"))
        .insert((2, "two"))
        .insert((3, "three"));

    assert_eq!(set.len(), 3);
    assert!(set.contains(&(1, "one")));
    assert!(set.contains(&(2, "two")));
}

// =============================================================================
// Coverage Tests: Iterator
// =============================================================================

#[rstest]
fn test_iter_exact_size() {
    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let iter = set.iter();
    assert_eq!(iter.len(), 3);
}

#[rstest]
fn test_iter_after_partial_consumption() {
    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let mut iter = set.iter();
    iter.next(); // Consume one element

    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 2);
    assert_eq!(upper, Some(2));
    assert_eq!(iter.len(), 2);
}

#[rstest]
fn test_into_iter_size_hint() {
    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let iter = set.into_iter();
    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 3);
    assert_eq!(upper, Some(3));
}

#[rstest]
fn test_into_iter_exact_size() {
    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let iter = set.into_iter();
    assert_eq!(iter.len(), 3);
}

#[rstest]
fn test_into_iter_after_partial_consumption() {
    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let mut iter = set.into_iter();
    iter.next(); // Consume one element

    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, 2);
    assert_eq!(upper, Some(2));
    assert_eq!(iter.len(), 2);
}

#[rstest]
fn test_ref_into_iterator() {
    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let mut sum = 0;
    for element in &set {
        sum += element;
    }
    assert_eq!(sum, 6);
}

// =============================================================================
// Coverage Tests: Foldable trait additional methods
// =============================================================================

#[rstest]
fn test_foldable_fold_right() {
    use functional_rusty::typeclass::Foldable;

    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let sum = set.fold_right(0, |element, accumulator| element + accumulator);
    assert_eq!(sum, 6);
}

#[rstest]
fn test_foldable_fold_left_with_string() {
    use functional_rusty::typeclass::Foldable;

    let set: PersistentHashSet<i32> = [1].into_iter().collect();
    let result = set.fold_left(String::new(), |mut accumulator, element| {
        accumulator.push_str(&element.to_string());
        accumulator
    });
    assert_eq!(result, "1");
}

// =============================================================================
// Coverage Tests: Remove edge cases
// =============================================================================

#[rstest]
fn test_remove_from_empty_set() {
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();
    let result = empty.remove(&42);
    assert!(result.is_empty());
}

// =============================================================================
// Coverage Tests: Set operations edge cases
// =============================================================================

#[rstest]
fn test_union_same_elements() {
    let set1: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let set2: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let union = set1.union(&set2);

    assert_eq!(union.len(), 3);
}

#[rstest]
fn test_intersection_empty_with_non_empty() {
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();
    let non_empty: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();

    let intersection = empty.intersection(&non_empty);
    assert!(intersection.is_empty());
}

#[rstest]
fn test_symmetric_difference_empty_with_non_empty() {
    let empty: PersistentHashSet<i32> = PersistentHashSet::new();
    let non_empty: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();

    let symmetric_difference = empty.symmetric_difference(&non_empty);
    assert_eq!(symmetric_difference.len(), 3);
}

// =============================================================================
// Coverage Tests: is_subset/is_superset edge cases
// =============================================================================

#[rstest]
fn test_empty_is_subset_of_empty() {
    let empty1: PersistentHashSet<i32> = PersistentHashSet::new();
    let empty2: PersistentHashSet<i32> = PersistentHashSet::new();

    assert!(empty1.is_subset(&empty2));
    assert!(empty2.is_subset(&empty1));
}

#[rstest]
fn test_empty_is_superset_of_empty() {
    let empty1: PersistentHashSet<i32> = PersistentHashSet::new();
    let empty2: PersistentHashSet<i32> = PersistentHashSet::new();

    assert!(empty1.is_superset(&empty2));
    assert!(empty2.is_superset(&empty1));
}

#[rstest]
fn test_is_disjoint_empty_sets() {
    let empty1: PersistentHashSet<i32> = PersistentHashSet::new();
    let empty2: PersistentHashSet<i32> = PersistentHashSet::new();

    assert!(empty1.is_disjoint(&empty2));
}

// =============================================================================
// Coverage Tests: Large set operations
// =============================================================================

#[rstest]
fn test_large_set_operations() {
    let set1: PersistentHashSet<i32> = (0..1000).collect();
    let set2: PersistentHashSet<i32> = (500..1500).collect();

    let union = set1.union(&set2);
    assert_eq!(union.len(), 1500);

    let intersection = set1.intersection(&set2);
    assert_eq!(intersection.len(), 500);

    let difference = set1.difference(&set2);
    assert_eq!(difference.len(), 500);

    let symmetric_difference = set1.symmetric_difference(&set2);
    assert_eq!(symmetric_difference.len(), 1000);
}

// =============================================================================
// Coverage Tests: Foldable find, exists, for_all
// =============================================================================

#[rstest]
fn test_foldable_find() {
    use functional_rusty::typeclass::Foldable;

    let set: PersistentHashSet<i32> = [10, 20, 30].into_iter().collect();
    let found = set.find(|element| *element > 15);
    assert!(found.is_some());
    assert!(found.unwrap() > 15);
}

#[rstest]
fn test_foldable_find_not_found() {
    use functional_rusty::typeclass::Foldable;

    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let found = set.find(|element| *element > 100);
    assert!(found.is_none());
}

#[rstest]
fn test_foldable_exists() {
    use functional_rusty::typeclass::Foldable;

    let set: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    assert!(set.exists(|element| *element == 2));
    assert!(!set.exists(|element| *element == 100));
}

#[rstest]
fn test_foldable_for_all() {
    use functional_rusty::typeclass::Foldable;

    let set: PersistentHashSet<i32> = [2, 4, 6].into_iter().collect();
    assert!(set.for_all(|element| element % 2 == 0));
    assert!(!set.for_all(|element| *element < 5));
}

// =============================================================================
// Coverage Tests: Debug format
// =============================================================================

#[rstest]
fn test_debug_format_empty() {
    let set: PersistentHashSet<i32> = PersistentHashSet::new();
    let debug_string = format!("{:?}", set);
    assert!(debug_string.contains('{'));
    assert!(debug_string.contains('}'));
}

// =============================================================================
// Coverage Tests: Clone
// =============================================================================

#[rstest]
fn test_clone_and_modify() {
    let set1: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
    let set2 = set1.clone();
    let set3 = set2.insert(4);

    assert_eq!(set1.len(), 3);
    assert_eq!(set2.len(), 3);
    assert_eq!(set3.len(), 4);
}
