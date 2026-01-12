#![cfg(feature = "effect")]
//! Property-based tests for IO Monad laws.
//!
//! This module verifies that the IO type satisfies the Monad laws:
//! - Left Identity: pure(a).flat_map(f) == f(a)
//! - Right Identity: m.flat_map(pure) == m
//! - Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))

use lambars::effect::IO;
use lambars::typeclass::{Applicative, Functor, Monad};
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
    fn prop_io_left_identity(value: i32) {
        let function = |n: i32| IO::pure(n.wrapping_mul(2));

        let left_result = IO::pure(value).flat_map(function).run_unsafe();
        let right_result = function(value).run_unsafe();

        prop_assert_eq!(left_result, right_result);
    }

    /// Right Identity Law: m.flat_map(pure) == m
    ///
    /// flat_mapping a monad with pure returns the original monad.
    #[test]
    fn prop_io_right_identity(value: i32) {
        let left_result = IO::pure(value).flat_map(IO::pure).run_unsafe();
        let right_result = value;

        prop_assert_eq!(left_result, right_result);
    }

    /// Associativity Law: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    ///
    /// The order of flat_map composition doesn't matter (modulo grouping).
    #[test]
    fn prop_io_associativity(value: i32) {
        let function1 = |n: i32| IO::pure(n.wrapping_add(1));
        let function2 = |n: i32| IO::pure(n.wrapping_mul(2));

        let left_result = IO::pure(value)
            .flat_map(function1)
            .flat_map(function2)
            .run_unsafe();
        let right_result = IO::pure(value)
            .flat_map(move |x| function1(x).flat_map(function2))
            .run_unsafe();

        prop_assert_eq!(left_result, right_result);
    }
}

// =============================================================================
// Functor Laws
// =============================================================================

proptest! {
    /// Functor Identity Law: fmap(id) == id
    ///
    /// Mapping the identity function over an IO returns the same IO.
    #[test]
    fn prop_io_functor_identity(value: i32) {
        let left_result = IO::pure(value).fmap(|x| x).run_unsafe();
        let right_result = value;

        prop_assert_eq!(left_result, right_result);
    }

    /// Functor Composition Law: fmap(f . g) == fmap(f) . fmap(g)
    ///
    /// Mapping a composed function is the same as composing the maps.
    #[test]
    fn prop_io_functor_composition(value: i32) {
        let function1 = |x: i32| x.wrapping_add(1);
        let function2 = |x: i32| x.wrapping_mul(2);

        let left_result = IO::pure(value)
            .fmap(move |x| function2(function1(x)))
            .run_unsafe();
        let right_result = IO::pure(value)
            .fmap(function1)
            .fmap(function2)
            .run_unsafe();

        prop_assert_eq!(left_result, right_result);
    }
}

// =============================================================================
// Additional Properties
// =============================================================================

proptest! {
    /// and_then is an alias for flat_map
    #[test]
    fn prop_io_and_then_equals_flat_map(value: i32) {
        let function = |n: i32| IO::pure(n.wrapping_add(10));

        let left_result = IO::pure(value).and_then(function).run_unsafe();
        let right_result = IO::pure(value).flat_map(function).run_unsafe();

        prop_assert_eq!(left_result, right_result);
    }

    /// map2 is consistent with flat_map and fmap
    #[test]
    fn prop_io_map2_consistency(a: i32, b: i32) {
        let combine = |x: i32, y: i32| x.wrapping_add(y);

        let left_result = IO::pure(a).map2(IO::pure(b), combine).run_unsafe();
        let right_result = IO::pure(a)
            .flat_map(move |x| {
                let b_copy = b;
                IO::pure(b_copy).fmap(move |y| combine(x, y))
            })
            .run_unsafe();

        prop_assert_eq!(left_result, right_result);
    }

    /// product is consistent with map2
    #[test]
    fn prop_io_product_consistency(a: i32, b: i32) {
        let left_result = IO::pure(a).product(IO::pure(b)).run_unsafe();
        let right_result = IO::pure(a).map2(IO::pure(b), |x, y| (x, y)).run_unsafe();

        prop_assert_eq!(left_result, right_result);
    }

    /// then discards the first value
    #[test]
    fn prop_io_then_discards_first(a: i32, b: i32) {
        let left_result = IO::pure(a).then(IO::pure(b)).run_unsafe();
        let right_result = {
            let b_copy = b;
            IO::pure(a).flat_map(move |_| IO::pure(b_copy)).run_unsafe()
        };

        prop_assert_eq!(left_result, right_result);
    }
}

// =============================================================================
// Deferred Execution Properties
// =============================================================================

#[test]
fn test_io_pure_is_referentially_transparent() {
    // Multiple calls to run_unsafe on equivalent IOs should give the same result
    let value = 42;
    let io1 = IO::pure(value);
    let io2 = IO::pure(value);

    assert_eq!(io1.run_unsafe(), io2.run_unsafe());
}

#[test]
fn test_io_chained_operations_are_referentially_transparent() {
    let io1 = IO::pure(10).fmap(|x| x * 2).flat_map(|x| IO::pure(x + 5));
    let io2 = IO::pure(10).fmap(|x| x * 2).flat_map(|x| IO::pure(x + 5));

    assert_eq!(io1.run_unsafe(), io2.run_unsafe());
}
