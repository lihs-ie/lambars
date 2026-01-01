#![cfg(feature = "persistent")]
//! Property-based tests for PersistentHashSet laws.
//!
//! These tests verify that PersistentHashSet satisfies the mathematical
//! properties expected of a set data structure.

use lambars::persistent::PersistentHashSet;
use lambars::typeclass::Foldable;
use proptest::prelude::*;

// =============================================================================
// Insert-Contains Law
// Description: An inserted element is always contained in the set
// =============================================================================

proptest! {
    #[test]
    fn prop_insert_contains_law(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        new_element: i32
    ) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let with_element = set.insert(new_element);

        prop_assert!(with_element.contains(&new_element));
    }
}

// =============================================================================
// Remove-Contains Law
// Description: A removed element is never contained in the result set
// =============================================================================

proptest! {
    #[test]
    fn prop_remove_contains_law(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        element_to_remove: i32
    ) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let without_element = set.remove(&element_to_remove);

        prop_assert!(!without_element.contains(&element_to_remove));
    }
}

// =============================================================================
// Union Identity Law
// Description: Union with empty set is identity
// =============================================================================

proptest! {
    #[test]
    fn prop_union_identity_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let empty: PersistentHashSet<i32> = PersistentHashSet::new();

        let union_with_empty = set.union(&empty);
        let empty_union_with_set = empty.union(&set);

        prop_assert_eq!(union_with_empty, set.clone());
        prop_assert_eq!(empty_union_with_set, set);
    }
}

// =============================================================================
// Union Commutativity Law
// Description: A ∪ B = B ∪ A
// =============================================================================

proptest! {
    #[test]
    fn prop_union_commutativity_law(
        elements_a in prop::collection::vec(any::<i32>(), 0..30),
        elements_b in prop::collection::vec(any::<i32>(), 0..30)
    ) {
        let set_a: PersistentHashSet<i32> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i32> = elements_b.into_iter().collect();

        let a_union_b = set_a.union(&set_b);
        let b_union_a = set_b.union(&set_a);

        prop_assert_eq!(a_union_b, b_union_a);
    }
}

// =============================================================================
// Union Associativity Law
// Description: (A ∪ B) ∪ C = A ∪ (B ∪ C)
// =============================================================================

proptest! {
    #[test]
    fn prop_union_associativity_law(
        elements_a in prop::collection::vec(any::<i32>(), 0..20),
        elements_b in prop::collection::vec(any::<i32>(), 0..20),
        elements_c in prop::collection::vec(any::<i32>(), 0..20)
    ) {
        let set_a: PersistentHashSet<i32> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i32> = elements_b.into_iter().collect();
        let set_c: PersistentHashSet<i32> = elements_c.into_iter().collect();

        let left = set_a.union(&set_b).union(&set_c);
        let right = set_a.union(&set_b.union(&set_c));

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Intersection Identity Law
// Description: Intersection with self is identity
// =============================================================================

proptest! {
    #[test]
    fn prop_intersection_identity_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let intersection = set.intersection(&set);

        prop_assert_eq!(intersection, set);
    }
}

// =============================================================================
// Intersection Commutativity Law
// Description: A ∩ B = B ∩ A
// =============================================================================

proptest! {
    #[test]
    fn prop_intersection_commutativity_law(
        elements_a in prop::collection::vec(any::<i32>(), 0..30),
        elements_b in prop::collection::vec(any::<i32>(), 0..30)
    ) {
        let set_a: PersistentHashSet<i32> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i32> = elements_b.into_iter().collect();

        let a_intersect_b = set_a.intersection(&set_b);
        let b_intersect_a = set_b.intersection(&set_a);

        prop_assert_eq!(a_intersect_b, b_intersect_a);
    }
}

// =============================================================================
// Intersection Associativity Law
// Description: (A ∩ B) ∩ C = A ∩ (B ∩ C)
// =============================================================================

proptest! {
    #[test]
    fn prop_intersection_associativity_law(
        elements_a in prop::collection::vec(any::<i32>(), 0..20),
        elements_b in prop::collection::vec(any::<i32>(), 0..20),
        elements_c in prop::collection::vec(any::<i32>(), 0..20)
    ) {
        let set_a: PersistentHashSet<i32> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i32> = elements_b.into_iter().collect();
        let set_c: PersistentHashSet<i32> = elements_c.into_iter().collect();

        let left = set_a.intersection(&set_b).intersection(&set_c);
        let right = set_a.intersection(&set_b.intersection(&set_c));

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Difference Self Law
// Description: A - A = ∅ (difference with self is empty)
// =============================================================================

proptest! {
    #[test]
    fn prop_difference_self_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let difference = set.difference(&set);

        prop_assert!(difference.is_empty());
    }
}

// =============================================================================
// Difference Empty Law
// Description: A - ∅ = A
// =============================================================================

proptest! {
    #[test]
    fn prop_difference_empty_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let empty: PersistentHashSet<i32> = PersistentHashSet::new();

        let difference = set.difference(&empty);

        prop_assert_eq!(difference, set);
    }
}

// =============================================================================
// Symmetric Difference Self Law
// Description: A Δ A = ∅ (symmetric difference with self is empty)
// =============================================================================

proptest! {
    #[test]
    fn prop_symmetric_difference_self_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let symmetric_difference = set.symmetric_difference(&set);

        prop_assert!(symmetric_difference.is_empty());
    }
}

// =============================================================================
// Symmetric Difference Commutativity Law
// Description: A Δ B = B Δ A
// =============================================================================

proptest! {
    #[test]
    fn prop_symmetric_difference_commutativity_law(
        elements_a in prop::collection::vec(any::<i32>(), 0..30),
        elements_b in prop::collection::vec(any::<i32>(), 0..30)
    ) {
        let set_a: PersistentHashSet<i32> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i32> = elements_b.into_iter().collect();

        let a_sym_diff_b = set_a.symmetric_difference(&set_b);
        let b_sym_diff_a = set_b.symmetric_difference(&set_a);

        prop_assert_eq!(a_sym_diff_b, b_sym_diff_a);
    }
}

// =============================================================================
// Subset Reflexivity Law
// Description: A ⊆ A (every set is a subset of itself)
// =============================================================================

proptest! {
    #[test]
    fn prop_subset_reflexivity_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();

        prop_assert!(set.is_subset(&set));
    }
}

// =============================================================================
// Subset-Superset Duality Law
// Description: A ⊆ B ⟺ B ⊇ A
// =============================================================================

proptest! {
    #[test]
    fn prop_subset_superset_duality_law(
        elements_a in prop::collection::vec(any::<i32>(), 0..30),
        elements_b in prop::collection::vec(any::<i32>(), 0..30)
    ) {
        let set_a: PersistentHashSet<i32> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i32> = elements_b.into_iter().collect();

        prop_assert_eq!(set_a.is_subset(&set_b), set_b.is_superset(&set_a));
    }
}

// =============================================================================
// Disjoint Symmetry Law
// Description: A and B are disjoint ⟺ B and A are disjoint
// =============================================================================

proptest! {
    #[test]
    fn prop_disjoint_symmetry_law(
        elements_a in prop::collection::vec(any::<i32>(), 0..30),
        elements_b in prop::collection::vec(any::<i32>(), 0..30)
    ) {
        let set_a: PersistentHashSet<i32> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i32> = elements_b.into_iter().collect();

        prop_assert_eq!(set_a.is_disjoint(&set_b), set_b.is_disjoint(&set_a));
    }
}

// =============================================================================
// Disjoint-Intersection Law
// Description: A and B are disjoint ⟺ A ∩ B = ∅
// =============================================================================

proptest! {
    #[test]
    fn prop_disjoint_intersection_law(
        elements_a in prop::collection::vec(any::<i32>(), 0..30),
        elements_b in prop::collection::vec(any::<i32>(), 0..30)
    ) {
        let set_a: PersistentHashSet<i32> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i32> = elements_b.into_iter().collect();

        let intersection = set_a.intersection(&set_b);
        let is_disjoint = set_a.is_disjoint(&set_b);

        prop_assert_eq!(is_disjoint, intersection.is_empty());
    }
}

// =============================================================================
// Length Consistency Law
// Description: len() equals the count of iterated elements
// =============================================================================

proptest! {
    #[test]
    fn prop_length_consistency_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let iterator_count = set.iter().count();

        prop_assert_eq!(set.len(), iterator_count);
    }
}

// =============================================================================
// Insert Length Law
// Description: insert on new element increases length by 1
// =============================================================================

proptest! {
    #[test]
    fn prop_insert_length_law(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        new_element: i32
    ) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let had_element = set.contains(&new_element);
        let with_element = set.insert(new_element);

        if had_element {
            prop_assert_eq!(with_element.len(), set.len());
        } else {
            prop_assert_eq!(with_element.len(), set.len() + 1);
        }
    }
}

// =============================================================================
// Remove Length Law
// Description: remove on existing element decreases length by 1
// =============================================================================

proptest! {
    #[test]
    fn prop_remove_length_law(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        element_to_remove: i32
    ) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let had_element = set.contains(&element_to_remove);
        let without_element = set.remove(&element_to_remove);

        if had_element {
            prop_assert_eq!(without_element.len(), set.len() - 1);
        } else {
            prop_assert_eq!(without_element.len(), set.len());
        }
    }
}

// =============================================================================
// Immutability Law
// Description: Operations do not modify the original set
// =============================================================================

proptest! {
    #[test]
    fn prop_immutability_law(
        elements in prop::collection::vec(any::<i32>(), 1..50),
        new_element: i32
    ) {
        let original: PersistentHashSet<i32> = elements.clone().into_iter().collect();
        let original_length = original.len();

        // Perform various operations
        let _inserted = original.insert(new_element);
        let _removed = original.remove(&new_element);

        if let Some(first) = elements.first() {
            let _removed_existing = original.remove(first);
        }

        // Original should remain unchanged
        prop_assert_eq!(original.len(), original_length);

        for element in &elements {
            prop_assert!(original.contains(element));
        }
    }
}

// =============================================================================
// Union-Intersection Distributivity Law
// Description: A ∪ (B ∩ C) = (A ∪ B) ∩ (A ∪ C)
// =============================================================================

proptest! {
    #[test]
    fn prop_union_intersection_distributivity_law(
        elements_a in prop::collection::vec(any::<i16>(), 0..15),
        elements_b in prop::collection::vec(any::<i16>(), 0..15),
        elements_c in prop::collection::vec(any::<i16>(), 0..15)
    ) {
        let set_a: PersistentHashSet<i16> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i16> = elements_b.into_iter().collect();
        let set_c: PersistentHashSet<i16> = elements_c.into_iter().collect();

        let left = set_a.union(&set_b.intersection(&set_c));
        let right = set_a.union(&set_b).intersection(&set_a.union(&set_c));

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// De Morgan's Laws
// Description: Verification of De Morgan's laws for sets
// (A - B) = (A ∩ B^c) is harder to test without universal set,
// so we test related properties
// =============================================================================

proptest! {
    #[test]
    fn prop_union_difference_law(
        elements_a in prop::collection::vec(any::<i16>(), 0..20),
        elements_b in prop::collection::vec(any::<i16>(), 0..20)
    ) {
        let set_a: PersistentHashSet<i16> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i16> = elements_b.into_iter().collect();

        // A = (A ∩ B) ∪ (A - B)
        let intersection = set_a.intersection(&set_b);
        let difference = set_a.difference(&set_b);
        let reconstructed = intersection.union(&difference);

        prop_assert_eq!(reconstructed, set_a);
    }
}

// =============================================================================
// Symmetric Difference Definition Law
// Description: A Δ B = (A - B) ∪ (B - A)
// =============================================================================

proptest! {
    #[test]
    fn prop_symmetric_difference_definition_law(
        elements_a in prop::collection::vec(any::<i32>(), 0..30),
        elements_b in prop::collection::vec(any::<i32>(), 0..30)
    ) {
        let set_a: PersistentHashSet<i32> = elements_a.into_iter().collect();
        let set_b: PersistentHashSet<i32> = elements_b.into_iter().collect();

        let symmetric_difference = set_a.symmetric_difference(&set_b);
        let manual_symmetric_difference = set_a.difference(&set_b).union(&set_b.difference(&set_a));

        prop_assert_eq!(symmetric_difference, manual_symmetric_difference);
    }
}

// =============================================================================
// Foldable Length Consistency Law
// Description: Foldable::length equals len()
// =============================================================================

proptest! {
    #[test]
    fn prop_foldable_length_consistency_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();

        prop_assert_eq!(Foldable::length(&set), set.len());
    }
}

// =============================================================================
// FromIterator-IntoIterator Roundtrip Law
// Description: Collecting from iterator produces equivalent set
// =============================================================================

proptest! {
    #[test]
    fn prop_iter_roundtrip_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let original: PersistentHashSet<i32> = elements.into_iter().collect();
        let roundtripped: PersistentHashSet<i32> = original.clone().into_iter().collect();

        prop_assert_eq!(original, roundtripped);
    }
}

// =============================================================================
// Clone Equivalence Law
// Description: Cloned set equals original
// =============================================================================

proptest! {
    #[test]
    fn prop_clone_equivalence_law(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let set: PersistentHashSet<i32> = elements.into_iter().collect();
        let cloned = set.clone();

        prop_assert_eq!(set, cloned);
    }
}
