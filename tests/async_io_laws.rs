#![cfg(feature = "async")]
//! Property-based tests for AsyncIO Monad laws.
//!
//! This module verifies that the AsyncIO type satisfies the Monad laws:
//! - Left Identity: pure(a).flat_map(f) == f(a)
//! - Right Identity: m.flat_map(pure) == m
//! - Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
//!
//! Also verifies Functor and Applicative laws.

use lambars::effect::AsyncIO;
use proptest::prelude::*;

// =============================================================================
// Monad Laws
// =============================================================================

proptest! {
    /// Left Identity Law: pure(a).flat_map(f) == f(a)
    ///
    /// Wrapping a value in pure and then flat_mapping over it with a function
    /// is the same as just applying the function to the value.
    #[test]
    fn prop_async_io_monad_left_identity(value: i32) {
        let function = |n: i32| AsyncIO::pure(n.wrapping_mul(2));

        // We need to run the async tests in a tokio runtime
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(value).flat_map(function).run_async().await
        });
        let right_result = runtime.block_on(async {
            function(value).run_async().await
        });

        prop_assert_eq!(left_result, right_result);
    }

    /// Right Identity Law: m.flat_map(pure) == m
    ///
    /// flat_mapping a monad with pure returns the original monad.
    #[test]
    fn prop_async_io_monad_right_identity(value: i32) {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(value).flat_map(AsyncIO::pure).run_async().await
        });
        let right_result = value;

        prop_assert_eq!(left_result, right_result);
    }

    /// Associativity Law: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    ///
    /// The order of flat_map composition doesn't matter (modulo grouping).
    #[test]
    fn prop_async_io_monad_associativity(value: i32) {
        let function1 = |n: i32| AsyncIO::pure(n.wrapping_add(1));
        let function2 = |n: i32| AsyncIO::pure(n.wrapping_mul(2));

        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(value)
                .flat_map(function1)
                .flat_map(function2)
                .run_async()
                .await
        });
        let right_result = runtime.block_on(async {
            AsyncIO::pure(value)
                .flat_map(move |x| function1(x).flat_map(function2))
                .run_async()
                .await
        });

        prop_assert_eq!(left_result, right_result);
    }
}

// =============================================================================
// Functor Laws
// =============================================================================

proptest! {
    /// Functor Identity Law: fmap(id) == id
    ///
    /// Mapping the identity function over an AsyncIO returns the same AsyncIO.
    #[test]
    fn prop_async_io_functor_identity(value: i32) {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(value).fmap(|x| x).run_async().await
        });
        let right_result = value;

        prop_assert_eq!(left_result, right_result);
    }

    /// Functor Composition Law: fmap(f . g) == fmap(f) . fmap(g)
    ///
    /// Mapping a composed function is the same as composing the maps.
    #[test]
    fn prop_async_io_functor_composition(value: i32) {
        let function1 = |x: i32| x.wrapping_add(1);
        let function2 = |x: i32| x.wrapping_mul(2);

        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(value)
                .fmap(move |x| function2(function1(x)))
                .run_async()
                .await
        });
        let right_result = runtime.block_on(async {
            AsyncIO::pure(value)
                .fmap(function1)
                .fmap(function2)
                .run_async()
                .await
        });

        prop_assert_eq!(left_result, right_result);
    }
}

// =============================================================================
// Additional Properties
// =============================================================================

proptest! {
    /// and_then is an alias for flat_map
    #[test]
    fn prop_async_io_and_then_equals_flat_map(value: i32) {
        let function = |n: i32| AsyncIO::pure(n.wrapping_add(10));

        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(value).and_then(function).run_async().await
        });
        let right_result = runtime.block_on(async {
            AsyncIO::pure(value).flat_map(function).run_async().await
        });

        prop_assert_eq!(left_result, right_result);
    }

    /// map2 is consistent with flat_map and fmap
    #[test]
    fn prop_async_io_map2_consistency(a: i32, b: i32) {
        let combine = |x: i32, y: i32| x.wrapping_add(y);

        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(a).map2(AsyncIO::pure(b), combine).run_async().await
        });
        let right_result = runtime.block_on(async {
            AsyncIO::pure(a)
                .flat_map(move |x| {
                    let b_copy = b;
                    AsyncIO::pure(b_copy).fmap(move |y| combine(x, y))
                })
                .run_async()
                .await
        });

        prop_assert_eq!(left_result, right_result);
    }

    /// product is consistent with map2
    #[test]
    fn prop_async_io_product_consistency(a: i32, b: i32) {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(a).product(AsyncIO::pure(b)).run_async().await
        });
        let right_result = runtime.block_on(async {
            AsyncIO::pure(a).map2(AsyncIO::pure(b), |x, y| (x, y)).run_async().await
        });

        prop_assert_eq!(left_result, right_result);
    }

    /// then discards the first value
    #[test]
    fn prop_async_io_then_discards_first(a: i32, b: i32) {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(a).then(AsyncIO::pure(b)).run_async().await
        });
        let right_result = runtime.block_on(async {
            let b_copy = b;
            AsyncIO::pure(a).flat_map(move |_| AsyncIO::pure(b_copy)).run_async().await
        });

        prop_assert_eq!(left_result, right_result);
    }
}

// =============================================================================
// Deferred Execution Properties
// =============================================================================

#[test]
fn test_async_io_pure_is_referentially_transparent() {
    // Multiple calls to run_async on equivalent AsyncIOs should give the same result
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let value = 42;

    let result1 = runtime.block_on(async { AsyncIO::pure(value).run_async().await });
    let result2 = runtime.block_on(async { AsyncIO::pure(value).run_async().await });

    assert_eq!(result1, result2);
}

#[test]
fn test_async_io_chained_operations_are_referentially_transparent() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let result1 = runtime.block_on(async {
        AsyncIO::pure(10)
            .fmap(|x| x * 2)
            .flat_map(|x| AsyncIO::pure(x + 5))
            .run_async()
            .await
    });
    let result2 = runtime.block_on(async {
        AsyncIO::pure(10)
            .fmap(|x| x * 2)
            .flat_map(|x| AsyncIO::pure(x + 5))
            .run_async()
            .await
    });

    assert_eq!(result1, result2);
}

// =============================================================================
// Applicative Laws
// =============================================================================

proptest! {
    /// Applicative Identity Law: pure(id) <*> v == v
    #[test]
    fn prop_async_io_applicative_identity(value: i32) {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let identity_fn: fn(i32) -> i32 = |x| x;
        let left_result = runtime.block_on(async {
            AsyncIO::pure(value)
                .apply(AsyncIO::pure(identity_fn))
                .run_async()
                .await
        });
        let right_result = value;

        prop_assert_eq!(left_result, right_result);
    }

    /// Applicative Homomorphism Law: pure(f) <*> pure(x) == pure(f(x))
    #[test]
    fn prop_async_io_applicative_homomorphism(value: i32) {
        let function: fn(i32) -> i32 = |x| x.wrapping_mul(2);

        let runtime = tokio::runtime::Runtime::new().unwrap();

        let left_result = runtime.block_on(async {
            AsyncIO::pure(value)
                .apply(AsyncIO::pure(function))
                .run_async()
                .await
        });
        let right_result = runtime.block_on(async {
            AsyncIO::pure(function(value)).run_async().await
        });

        prop_assert_eq!(left_result, right_result);
    }
}
