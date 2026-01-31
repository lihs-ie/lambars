//! Property-based tests for the `for_async!` macro.
//!
//! These tests verify that `for_async!` behaves consistently with `for_!`
//! and follows expected properties.

#![cfg(feature = "async")]

use lambars::effect::AsyncIO;
use lambars::for_;
use lambars::for_async;
use proptest::prelude::*;

// =============================================================================
// Property Tests: Equivalence with for_!
// =============================================================================

proptest! {
    /// Single iteration should produce the same result as `for_!`
    #[test]
    fn prop_single_iteration_equivalence(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let f = |x: i32| x.wrapping_mul(2);

        let elements_clone = elements.clone();
        let sync_result = for_! {
            x <= elements_clone;
            yield f(x)
        };

        let async_result = runtime.block_on(async {
            for_async! {
                x <= elements.clone();
                yield f(x)
            }.await
        });

        prop_assert_eq!(sync_result, async_result);
    }

    /// Nested iteration should produce the same result as `for_!`
    #[test]
    fn prop_nested_iteration_equivalence(
        xs in prop::collection::vec(any::<i32>(), 0..10),
        ys in prop::collection::vec(any::<i32>(), 0..10)
    ) {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let ys_clone = ys.clone();
        let xs_clone = xs.clone();
        let sync_result = for_! {
            x <= xs_clone;
            y <= ys_clone.clone();
            yield (x, y)
        };

        let ys_clone2 = ys.clone();
        let async_result = runtime.block_on(async {
            for_async! {
                x <= xs.clone();
                y <= ys_clone2.clone();
                yield (x, y)
            }.await
        });

        prop_assert_eq!(sync_result, async_result);
    }

    /// AsyncIO::pure bind should be equivalent to direct value use
    #[test]
    fn prop_async_bind_pure_equivalence(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let f = |x: i32| x.wrapping_mul(2);

        // Direct computation
        let elements_clone = elements.clone();
        let direct_result = for_! {
            x <= elements_clone;
            yield f(x)
        };

        // Via AsyncIO::pure
        let async_result = runtime.block_on(async {
            for_async! {
                x <= elements.clone();
                doubled <~ AsyncIO::pure(f(x));
                yield doubled
            }.await
        });

        prop_assert_eq!(direct_result, async_result);
    }

    /// Empty collection should always produce empty result
    #[test]
    fn prop_empty_collection_produces_empty_result(_dummy in any::<i32>()) {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let empty: Vec<i32> = vec![];
        let result = runtime.block_on(async {
            for_async! {
                x <= empty;
                yield x * 2
            }.await
        });

        prop_assert!(result.is_empty());
    }

    /// Let binding should not affect the result
    #[test]
    fn prop_let_binding_no_effect(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        // Without let binding
        let elements_clone = elements.clone();
        let without_let = runtime.block_on(async {
            for_async! {
                x <= elements_clone;
                yield x.wrapping_mul(2)
            }.await
        });

        // With let binding
        let with_let = runtime.block_on(async {
            for_async! {
                x <= elements.clone();
                let doubled = x.wrapping_mul(2);
                yield doubled
            }.await
        });

        prop_assert_eq!(without_let, with_let);
    }

    /// Result length should equal product of collection lengths
    #[test]
    fn prop_nested_result_length(
        xs in prop::collection::vec(any::<i32>(), 0..10),
        ys in prop::collection::vec(any::<i32>(), 0..10)
    ) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let expected_len = xs.len() * ys.len();

        let ys_clone = ys.clone();
        let result = runtime.block_on(async {
            for_async! {
                _x <= xs;
                _y <= ys_clone.clone();
                yield ()
            }.await
        });

        prop_assert_eq!(result.len(), expected_len);
    }
}
