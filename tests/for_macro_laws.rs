//! Property-based tests for the for_! macro.
//!
//! These tests verify that the for_! macro behaves equivalently
//! to explicit iterator operations (map, flat_map, collect).

use lambars::for_;
use proptest::prelude::*;

// =============================================================================
// Law 1: Single iteration is equivalent to map
// =============================================================================

proptest! {
    /// A single iteration with for_! should be equivalent to
    /// iterator's map followed by collect.
    ///
    /// for_! { x <= xs; yield f(x) } == xs.into_iter().map(f).collect()
    #[test]
    fn prop_single_iteration_equals_map(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let function = |x: i32| x.wrapping_mul(2);

        let for_result = for_! {
            x <= elements.clone();
            yield function(x)
        };

        let map_result: Vec<i32> = elements.into_iter().map(function).collect();

        prop_assert_eq!(for_result, map_result);
    }
}

// =============================================================================
// Law 2: Nested iteration is equivalent to flat_map chain
// =============================================================================

proptest! {
    /// Nested iteration with for_! should be equivalent to
    /// chained flat_map operations.
    ///
    /// for_! { x <= xs; y <= ys; yield (x, y) }
    /// ==
    /// xs.into_iter().flat_map(|x| ys.clone().into_iter().map(move |y| (x, y))).collect()
    #[test]
    fn prop_nested_iteration_equals_flat_map(
        xs in prop::collection::vec(any::<i32>(), 0..10),
        ys in prop::collection::vec(any::<i32>(), 0..10)
    ) {
        let ys_clone = ys.clone();
        let for_result = for_! {
            x <= xs.clone();
            y <= ys_clone.clone();
            yield (x, y)
        };

        let flat_map_result: Vec<(i32, i32)> = xs.into_iter()
            .flat_map(|x| ys.clone().into_iter().map(move |y| (x, y)))
            .collect();

        prop_assert_eq!(for_result, flat_map_result);
    }
}

// =============================================================================
// Law 3: Let binding is a pure computation
// =============================================================================

proptest! {
    /// Let binding in for_! should be equivalent to computing
    /// the value inline in yield.
    ///
    /// for_! { x <= xs; let y = f(x); yield y }
    /// ==
    /// for_! { x <= xs; yield f(x) }
    #[test]
    fn prop_let_binding_pure(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let for_with_let = for_! {
            x <= elements.clone();
            let doubled = x.wrapping_mul(2);
            yield doubled
        };

        let for_without_let = for_! {
            x <= elements.clone();
            yield x.wrapping_mul(2)
        };

        let direct_result: Vec<i32> = elements.into_iter()
            .map(|x| x.wrapping_mul(2))
            .collect();

        prop_assert_eq!(for_with_let.clone(), for_without_let);
        prop_assert_eq!(for_with_let, direct_result);
    }
}

// =============================================================================
// Law 4: Multiple let bindings are sequential
// =============================================================================

proptest! {
    /// Multiple let bindings should be evaluated sequentially.
    ///
    /// for_! { x <= xs; let a = f(x); let b = g(a); yield b }
    /// ==
    /// xs.into_iter().map(|x| { let a = f(x); let b = g(a); b }).collect()
    #[test]
    fn prop_multiple_let_bindings_sequential(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let for_result = for_! {
            x <= elements.clone();
            let doubled = x.wrapping_mul(2);
            let squared = doubled.wrapping_mul(doubled);
            yield squared
        };

        let direct_result: Vec<i32> = elements.into_iter()
            .map(|x| {
                let doubled = x.wrapping_mul(2);
                doubled.wrapping_mul(doubled)
            })
            .collect();

        prop_assert_eq!(for_result, direct_result);
    }
}

// =============================================================================
// Law 5: Empty collection yields empty result
// =============================================================================

proptest! {
    /// An empty source collection should always yield an empty result.
    #[test]
    fn prop_empty_collection_yields_empty(
        _seed in any::<u64>()
    ) {
        let empty: Vec<i32> = vec![];

        let result = for_! {
            x <= empty;
            yield x.wrapping_mul(2)
        };

        prop_assert!(result.is_empty());
    }
}

// =============================================================================
// Law 6: Three-level nesting is equivalent to triple flat_map
// =============================================================================

proptest! {
    /// Three-level nesting should be equivalent to triple flat_map.
    #[test]
    fn prop_three_level_nesting(
        xs in prop::collection::vec(any::<i8>(), 0..5),
        ys in prop::collection::vec(any::<i8>(), 0..5),
        zs in prop::collection::vec(any::<i8>(), 0..5)
    ) {
        let ys_for = ys.clone();
        let zs_for = zs.clone();
        let for_result = for_! {
            x <= xs.clone();
            y <= ys_for.clone();
            z <= zs_for.clone();
            yield (x, y, z)
        };

        let flat_map_result: Vec<(i8, i8, i8)> = xs.into_iter()
            .flat_map(|x| {
                let zs_inner = zs.clone();
                ys.clone().into_iter().flat_map(move |y| {
                    zs_inner.clone().into_iter().map(move |z| (x, y, z))
                })
            })
            .collect();

        prop_assert_eq!(for_result, flat_map_result);
    }
}

// =============================================================================
// Law 7: Result length is product of input lengths (for independent iterations)
// =============================================================================

proptest! {
    /// For independent nested iterations, the result length should be
    /// the product of the input lengths.
    #[test]
    fn prop_result_length_is_product(
        xs in prop::collection::vec(any::<i32>(), 0..20),
        ys in prop::collection::vec(any::<i32>(), 0..20)
    ) {
        let expected_length = xs.len() * ys.len();
        let ys_for = ys.clone();

        let result = for_! {
            x <= xs;
            y <= ys_for.clone();
            yield (x, y)
        };

        prop_assert_eq!(result.len(), expected_length);
    }
}

// =============================================================================
// Law 8: Tuple pattern destructuring
// =============================================================================

proptest! {
    /// Tuple pattern in for_! should correctly destructure elements.
    #[test]
    fn prop_tuple_pattern_destructuring(
        pairs in prop::collection::vec((any::<i32>(), any::<i32>()), 0..50)
    ) {
        let for_result = for_! {
            (a, b) <= pairs.clone();
            yield a.wrapping_add(b)
        };

        let direct_result: Vec<i32> = pairs.into_iter()
            .map(|(a, b)| a.wrapping_add(b))
            .collect();

        prop_assert_eq!(for_result, direct_result);
    }
}

// =============================================================================
// Law 9: Let tuple binding
// =============================================================================

proptest! {
    /// Let tuple binding should correctly destructure.
    #[test]
    fn prop_let_tuple_binding(
        pairs in prop::collection::vec((any::<i32>(), any::<i32>()), 0..50)
    ) {
        let for_result = for_! {
            pair <= pairs.clone();
            let (a, b) = pair;
            yield a.wrapping_add(b)
        };

        let direct_result: Vec<i32> = pairs.into_iter()
            .map(|(a, b)| a.wrapping_add(b))
            .collect();

        prop_assert_eq!(for_result, direct_result);
    }
}

// =============================================================================
// Law 10: Wildcard pattern ignores value
// =============================================================================

proptest! {
    /// Wildcard pattern should ignore values and execute the body
    /// for each element.
    #[test]
    fn prop_wildcard_ignores_value(
        elements in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let expected_length = elements.len();

        let result = for_! {
            _ <= elements;
            yield 42
        };

        prop_assert_eq!(result.len(), expected_length);
        prop_assert!(result.iter().all(|&x| x == 42));
    }
}
