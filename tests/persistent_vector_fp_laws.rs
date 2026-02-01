#![cfg(feature = "persistent")]
//! Property tests verifying PersistentVector adheres to FP principles:
//! referential transparency, purity, and immutability.

use lambars::persistent::PersistentVector;
use lambars::typeclass::{Foldable, FunctorMut};
use proptest::prelude::*;

proptest! {
    /// push_back is a pure function: same input always produces same output.
    #[test]
    fn prop_push_back_referential_transparency(
        elements in prop::collection::vec(any::<i32>(), 0..100),
        new_element: i32
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();

        // Call push_back multiple times with same input
        let result1 = vector.push_back(new_element);
        let result2 = vector.push_back(new_element);
        let result3 = vector.push_back(new_element);

        // All results should be identical
        prop_assert_eq!(&result1, &result2, "push_back should be deterministic (1 vs 2)");
        prop_assert_eq!(&result2, &result3, "push_back should be deterministic (2 vs 3)");

        // Verify the content is correct
        prop_assert_eq!(result1.len(), elements.len() + 1);
        prop_assert_eq!(result1.get(elements.len()), Some(&new_element));
    }

    /// push_back preserves all existing elements at their original indices.
    #[test]
    fn prop_push_back_preserves_elements(
        elements in prop::collection::vec(any::<i32>(), 0..100),
        new_element: i32
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let result = vector.push_back(new_element);

        // All original elements should be preserved at their original indices
        for (index, &expected) in elements.iter().enumerate() {
            prop_assert_eq!(
                result.get(index),
                Some(&expected),
                "Element at index {} should be preserved after push_back",
                index
            );
        }
    }

    /// Sequential push_back operations are equivalent to batch push_back_many.
    #[test]
    fn prop_push_back_sequential_equals_batch(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        additions in prop::collection::vec(any::<i32>(), 1..20)
    ) {
        let base: PersistentVector<i32> = elements.iter().copied().collect();

        // Sequential push_back
        let mut sequential = base.clone();
        for &element in &additions {
            sequential = sequential.push_back(element);
        }

        // Batch push_back_many
        let batch = base.push_back_many(additions.iter().copied());

        // Results should be identical
        prop_assert_eq!(
            sequential.len(),
            batch.len(),
            "Length should match between sequential and batch push_back"
        );
        for index in 0..sequential.len() {
            prop_assert_eq!(
                sequential.get(index),
                batch.get(index),
                "Elements at index {} should match",
                index
            );
        }
    }

    /// push_back never modifies the original vector.
    #[test]
    fn prop_push_back_immutability(
        elements in prop::collection::vec(any::<i32>(), 0..100),
        new_element: i32
    ) {
        let original: PersistentVector<i32> = elements.iter().copied().collect();
        let original_len = original.len();

        // Capture original state
        let original_elements: Vec<i32> = original.iter().copied().collect();

        // Perform push_back (which should not modify original)
        let _new_vector = original.push_back(new_element);

        // Verify original is completely unchanged
        prop_assert_eq!(
            original.len(),
            original_len,
            "Original length should not change after push_back"
        );

        for (index, &expected) in original_elements.iter().enumerate() {
            prop_assert_eq!(
                original.get(index),
                Some(&expected),
                "Original element at index {} should not change",
                index
            );
        }
    }

    /// Multiple push_back calls from same base create independent branches.
    #[test]
    fn prop_push_back_branches_independent(
        elements in prop::collection::vec(any::<i32>(), 1..50)
    ) {
        let base: PersistentVector<i32> = elements.iter().copied().collect();

        // Create multiple branches from same base
        let branch1 = base.push_back(1111);
        let branch2 = base.push_back(2222);
        let branch3 = base.push_back(3333);

        // All branches should have correct length
        prop_assert_eq!(branch1.len(), elements.len() + 1);
        prop_assert_eq!(branch2.len(), elements.len() + 1);
        prop_assert_eq!(branch3.len(), elements.len() + 1);

        // Each branch should have its own distinct last element
        prop_assert_eq!(branch1.get(elements.len()), Some(&1111));
        prop_assert_eq!(branch2.get(elements.len()), Some(&2222));
        prop_assert_eq!(branch3.get(elements.len()), Some(&3333));

        // All branches should share the same prefix
        for index in 0..elements.len() {
            prop_assert_eq!(branch1.get(index), branch2.get(index));
            prop_assert_eq!(branch2.get(index), branch3.get(index));
        }
    }

    /// Nested operations do not affect ancestor vectors.
    #[test]
    fn prop_push_back_nested_immutability(
        elements in prop::collection::vec(any::<i32>(), 1..30)
    ) {
        let base: PersistentVector<i32> = elements.iter().copied().collect();
        let base_snapshot: Vec<i32> = base.iter().copied().collect();

        // Create a chain of operations
        let level1 = base.push_back(100);
        let level2 = level1.push_back(200);
        let level3 = level2.push_back(300);

        // Verify base is unchanged
        prop_assert_eq!(base.len(), elements.len());
        for (index, &expected) in base_snapshot.iter().enumerate() {
            prop_assert_eq!(base.get(index), Some(&expected));
        }

        // Verify level1 is unchanged
        prop_assert_eq!(level1.len(), elements.len() + 1);
        prop_assert_eq!(level1.get(elements.len()), Some(&100));

        // Verify level2 is unchanged
        prop_assert_eq!(level2.len(), elements.len() + 2);
        prop_assert_eq!(level2.get(elements.len() + 1), Some(&200));

        // Verify level3 has all elements
        prop_assert_eq!(level3.len(), elements.len() + 3);
        prop_assert_eq!(level3.get(elements.len() + 2), Some(&300));
    }

    /// Functor Identity Law: fmap(id) == id.
    #[test]
    fn prop_functor_identity_law_extended(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let mapped: PersistentVector<i32> = vector.clone().fmap_mut(|x| x);

        prop_assert_eq!(&vector, &mapped, "fmap(id) should equal id");
    }

    /// Functor Composition Law: fmap(f).fmap(g) == fmap(g . f).
    #[test]
    fn prop_functor_composition_law_extended(
        elements in prop::collection::vec(-1000i32..1000i32, 0..100)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();

        // Use saturating operations to avoid overflow
        let function1 = |x: i32| x.saturating_add(1);
        let function2 = |x: i32| x.saturating_mul(2);

        // fmap(f).fmap(g)
        let left: PersistentVector<i32> = vector.clone().fmap_mut(function1).fmap_mut(function2);

        // fmap(g . f)
        let right: PersistentVector<i32> = vector.fmap_mut(|x| function2(function1(x)));

        prop_assert_eq!(&left, &right, "Functor composition law should hold");
    }

    /// fmap preserves length and structure.
    #[test]
    fn prop_functor_preserves_structure(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();
        let mapped: PersistentVector<i64> = vector.clone().fmap_mut(|x| i64::from(x) * 2);

        prop_assert_eq!(
            vector.len(),
            mapped.len(),
            "Functor should preserve length"
        );

        // Verify each element was transformed correctly
        for (index, &original) in elements.iter().enumerate() {
            let expected = i64::from(original) * 2;
            prop_assert_eq!(
                mapped.get(index),
                Some(&expected),
                "Element at index {} should be correctly transformed",
                index
            );
        }
    }

    /// push_back has no observable side effects.
    #[test]
    fn prop_push_back_purity_no_side_effects(
        elements in prop::collection::vec(any::<i32>(), 0..50),
        additions in prop::collection::vec(any::<i32>(), 1..10)
    ) {
        let base: PersistentVector<i32> = elements.iter().copied().collect();

        // Apply additions in forward order
        let mut forward = base.clone();
        for &element in &additions {
            forward = forward.push_back(element);
        }

        // Apply same additions again from the same base
        let mut forward_again = base.clone();
        for &element in &additions {
            forward_again = forward_again.push_back(element);
        }

        // Results should be identical
        prop_assert_eq!(
            &forward,
            &forward_again,
            "Repeated push_back operations should produce identical results"
        );
    }

    /// fold_left is referentially transparent.
    #[test]
    fn prop_fold_left_referential_transparency(
        elements in prop::collection::vec(-1000i32..1000i32, 0..50)
    ) {
        let vector: PersistentVector<i32> = elements.iter().copied().collect();

        // Call fold_left multiple times (note: fold_left consumes self, so we clone)
        let sum1 = vector.clone().fold_left(0i64, |acc, x| acc + i64::from(x));
        let sum2 = vector.clone().fold_left(0i64, |acc, x| acc + i64::from(x));
        let sum3 = vector.fold_left(0i64, |acc, x| acc + i64::from(x));

        // All results should be identical
        prop_assert_eq!(sum1, sum2);
        prop_assert_eq!(sum2, sum3);

        // Verify correctness
        let expected: i64 = elements.iter().map(|&x| i64::from(x)).sum();
        prop_assert_eq!(sum1, expected);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Large scale push_back maintains FP properties.
    #[test]
    fn prop_large_scale_push_back_fp_properties(
        initial_size in 100usize..500,
        additions in 100usize..300
    ) {
        let initial: PersistentVector<i32> = (0..initial_size as i32).collect();
        let initial_snapshot: Vec<i32> = initial.iter().copied().collect();

        // Perform many push_back operations
        let mut result = initial.clone();
        for index in 0..additions {
            result = result.push_back(index as i32 + 10000);
        }

        // Verify initial vector is unchanged (immutability)
        prop_assert_eq!(initial.len(), initial_size);
        for (index, &expected) in initial_snapshot.iter().enumerate() {
            prop_assert_eq!(
                initial.get(index),
                Some(&expected),
                "Initial vector should be unchanged at index {}",
                index
            );
        }

        // Verify result has correct structure (referential transparency)
        prop_assert_eq!(result.len(), initial_size + additions);

        // Verify all elements are correct
        for index in 0..initial_size {
            prop_assert_eq!(
                result.get(index),
                Some(&(index as i32)),
                "Original elements should be preserved"
            );
        }
        for index in 0..additions {
            prop_assert_eq!(
                result.get(initial_size + index),
                Some(&(index as i32 + 10000)),
                "Added elements should be at correct positions"
            );
        }
    }

    /// Large scale branching: multiple branches from same base are independent.
    #[test]
    fn prop_large_scale_branching(
        initial_size in 100usize..300,
        branch_count in 5usize..20
    ) {
        let base: PersistentVector<i32> = (0..initial_size as i32).collect();

        // Create multiple branches
        let branches: Vec<PersistentVector<i32>> = (0..branch_count)
            .map(|index| base.push_back(index as i32 * 1000))
            .collect();

        // Verify base is unchanged
        prop_assert_eq!(base.len(), initial_size);

        // Verify all branches are independent
        for (branch_index, branch) in branches.iter().enumerate() {
            prop_assert_eq!(branch.len(), initial_size + 1);
            prop_assert_eq!(
                branch.get(initial_size),
                Some(&(branch_index as i32 * 1000)),
                "Branch {} should have correct last element",
                branch_index
            );

            // Verify prefix is shared
            for index in 0..initial_size {
                prop_assert_eq!(
                    branch.get(index),
                    base.get(index),
                    "Branch {} should share prefix with base at index {}",
                    branch_index,
                    index
                );
            }
        }
    }
}

// =============================================================================
// Root Overflow Boundary Tests (1056/1057 boundary where tree depth increases)
// =============================================================================
// BRANCHING_FACTOR = 32, BITS_PER_LEVEL = 5, so:
// - Level 0 (leaf): 32 elements
// - Level 5: 32^2 = 1024 elements in root subtree
// - Level 10: 32^3 = 32768 elements
//
// The root_overflow branch in push_tail_to_root is triggered when:
//   (tail_offset >> shift) >= BRANCHING_FACTOR
// where tail_offset = ((length - 1) >> 5) << 5
//
// For shift = 5, root_overflow occurs when tail_offset >= 1024.
// tail_offset = ((length - 1) >> 5) << 5:
//   - length = 1056: tail_offset = ((1055) >> 5) << 5 = 32 << 5 = 1024
//   - At this point, tail has 32 elements (full, indices 1024-1055)
// When push_back is called on a vector of length 1056:
//   - tail is full, so push_tail_to_root is called
//   - root_overflow = (1024 >> 5) >= 32 = true
//   - Tree depth increases, and the new element becomes the first in a new tail

/// The exact boundary where root_overflow occurs.
/// At this length, the tail is full and the next push_back will trigger tree depth increase.
const ROOT_OVERFLOW_BOUNDARY: usize = 1056;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    /// Tests push_back at the root_overflow boundary (1056 -> 1057 elements).
    /// This triggers tree depth increase from level 5 to level 10.
    #[test]
    fn prop_root_overflow_boundary_immutability(
        extra_elements in 1usize..100
    ) {
        // Create a vector with exactly ROOT_OVERFLOW_BOUNDARY elements
        let base: PersistentVector<i32> = (0..ROOT_OVERFLOW_BOUNDARY as i32).collect();
        let base_snapshot: Vec<i32> = base.iter().copied().collect();

        // Push elements across the root_overflow boundary
        let mut result = base.clone();
        for index in 0..extra_elements {
            result = result.push_back(ROOT_OVERFLOW_BOUNDARY as i32 + index as i32);
        }

        // Verify base is completely unchanged (immutability)
        prop_assert_eq!(base.len(), ROOT_OVERFLOW_BOUNDARY);
        for (index, &expected) in base_snapshot.iter().enumerate() {
            prop_assert_eq!(
                base.get(index),
                Some(&expected),
                "Base element at index {} should be unchanged after root_overflow",
                index
            );
        }

        // Verify result has correct length
        prop_assert_eq!(result.len(), ROOT_OVERFLOW_BOUNDARY + extra_elements);

        // Verify all original elements preserved
        for index in 0..ROOT_OVERFLOW_BOUNDARY {
            prop_assert_eq!(
                result.get(index),
                Some(&(index as i32)),
                "Original element at index {} should be preserved after root_overflow",
                index
            );
        }

        // Verify new elements are correct
        for index in 0..extra_elements {
            prop_assert_eq!(
                result.get(ROOT_OVERFLOW_BOUNDARY + index),
                Some(&(ROOT_OVERFLOW_BOUNDARY as i32 + index as i32)),
                "New element at index {} should be correct",
                ROOT_OVERFLOW_BOUNDARY + index
            );
        }
    }

    /// Tests referential transparency across root_overflow boundary.
    #[test]
    fn prop_root_overflow_referential_transparency(
        offset in 0usize..64
    ) {
        // Start near the boundary
        let start_size = ROOT_OVERFLOW_BOUNDARY - offset;
        let base: PersistentVector<i32> = (0..start_size as i32).collect();

        // Push elements across the boundary multiple times
        let mut result1 = base.clone();
        let mut result2 = base.clone();

        // Push enough elements to cross the boundary
        let additions = offset + 64;
        for index in 0..additions {
            result1 = result1.push_back(start_size as i32 + index as i32);
            result2 = result2.push_back(start_size as i32 + index as i32);
        }

        // Both results should be identical (referential transparency)
        prop_assert_eq!(result1.len(), result2.len());
        for index in 0..result1.len() {
            prop_assert_eq!(
                result1.get(index),
                result2.get(index),
                "Results should be identical at index {} after root_overflow",
                index
            );
        }
    }

    /// Tests branching independence across root_overflow boundary.
    #[test]
    fn prop_root_overflow_branches_independent(
        branch_count in 2usize..10
    ) {
        // Create a vector at the root_overflow boundary
        let base: PersistentVector<i32> = (0..ROOT_OVERFLOW_BOUNDARY as i32).collect();

        // Create multiple branches that each trigger root_overflow
        let branches: Vec<PersistentVector<i32>> = (0..branch_count)
            .map(|index| base.push_back(10000 + index as i32))
            .collect();

        // Verify base is unchanged
        prop_assert_eq!(base.len(), ROOT_OVERFLOW_BOUNDARY);

        // Verify all branches are independent and correct
        for (branch_index, branch) in branches.iter().enumerate() {
            prop_assert_eq!(branch.len(), ROOT_OVERFLOW_BOUNDARY + 1);
            prop_assert_eq!(
                branch.get(ROOT_OVERFLOW_BOUNDARY),
                Some(&(10000 + branch_index as i32)),
                "Branch {} should have correct element at root_overflow position",
                branch_index
            );

            // Verify prefix is shared with base
            for index in 0..ROOT_OVERFLOW_BOUNDARY {
                prop_assert_eq!(
                    branch.get(index),
                    base.get(index),
                    "Branch {} should share prefix with base at index {}",
                    branch_index,
                    index
                );
            }
        }
    }
}
