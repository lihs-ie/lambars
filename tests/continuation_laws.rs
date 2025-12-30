//! Property-based tests for Continuation<R, A> laws.
//!
//! This module verifies that Continuation implementations satisfy:
//!
//! - **Functor Laws**: identity and composition
//! - **Monad Laws**: left identity, right identity, associativity

#![cfg(feature = "control")]

use functional_rusty::control::Continuation;
use proptest::prelude::*;

// =============================================================================
// Helper Functions for Tests
// =============================================================================

fn add_one(n: i32) -> i32 {
    n.wrapping_add(1)
}

fn multiply_two(n: i32) -> i32 {
    n.wrapping_mul(2)
}

fn multiply_three(n: i32) -> i32 {
    n.wrapping_mul(3)
}

fn to_string_fn(n: i32) -> String {
    n.to_string()
}

fn string_length(s: String) -> usize {
    s.len()
}

fn cont_add_one(x: i32) -> Continuation<i32, i32> {
    Continuation::pure(x.wrapping_add(1))
}

fn cont_multiply_two(x: i32) -> Continuation<i32, i32> {
    Continuation::pure(x.wrapping_mul(2))
}

fn cont_multiply_two_generic<R: 'static>(x: i32) -> Continuation<R, i32> {
    Continuation::pure(x.wrapping_mul(2))
}

fn cont_to_string<R: 'static>(x: i32) -> Continuation<R, String> {
    Continuation::pure(x.to_string())
}

fn cont_string_length<R: 'static>(s: String) -> Continuation<R, usize> {
    Continuation::pure(s.len())
}

// =============================================================================
// Functor Laws
// =============================================================================

proptest! {
    /// Functor Identity Law: cont.map(|x| x).run(k) == cont.run(k)
    #[test]
    fn prop_continuation_functor_identity(value in any::<i32>()) {
        let cont1: Continuation<i32, i32> = Continuation::pure(value);
        let cont2: Continuation<i32, i32> = Continuation::pure(value);

        let left = cont1.map(|x| x).run(|x| x);
        let right = cont2.run(|x| x);

        prop_assert_eq!(left, right);
    }
}

proptest! {
    /// Functor Composition Law:
    /// cont.map(f).map(g).run(k) == cont.map(|x| g(f(x))).run(k)
    #[test]
    fn prop_continuation_functor_composition(value in any::<i32>()) {
        let cont1: Continuation<i32, i32> = Continuation::pure(value);
        let cont2: Continuation<i32, i32> = Continuation::pure(value);

        let left = cont1.map(add_one).map(multiply_two).run(|x| x);
        let right = cont2.map(|x| multiply_two(add_one(x))).run(|x| x);

        prop_assert_eq!(left, right);
    }
}

proptest! {
    /// Functor composition with type changes
    #[test]
    fn prop_continuation_functor_composition_type_change(value in any::<i32>()) {
        let cont1: Continuation<usize, i32> = Continuation::pure(value);
        let cont2: Continuation<usize, i32> = Continuation::pure(value);

        let left = cont1.map(to_string_fn).map(string_length).run(|x| x);
        let right = cont2.map(|x| string_length(to_string_fn(x))).run(|x| x);

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Monad Laws
// =============================================================================

proptest! {
    /// Monad Left Identity:
    /// Continuation::pure(a).flat_map(f).run(k) == f(a).run(k)
    #[test]
    fn prop_continuation_monad_left_identity(value in any::<i32>()) {
        let left: i32 = Continuation::pure(value).flat_map(cont_multiply_two).run(|x| x);
        let right: i32 = cont_multiply_two(value).run(|x| x);

        prop_assert_eq!(left, right);
    }
}

proptest! {
    /// Monad Left Identity with different result type
    #[test]
    fn prop_continuation_monad_left_identity_type_change(value in any::<i32>()) {
        let left: String = Continuation::pure(value).flat_map(cont_to_string::<String>).run(|x| x);
        let right: String = cont_to_string::<String>(value).run(|x| x);

        prop_assert_eq!(left, right);
    }
}

proptest! {
    /// Monad Right Identity:
    /// m.flat_map(Continuation::pure).run(k) == m.run(k)
    #[test]
    fn prop_continuation_monad_right_identity(value in any::<i32>()) {
        let cont1: Continuation<i32, i32> = Continuation::pure(value);
        let cont2: Continuation<i32, i32> = Continuation::pure(value);

        let left = cont1.flat_map(Continuation::pure).run(|x| x);
        let right = cont2.run(|x| x);

        prop_assert_eq!(left, right);
    }
}

proptest! {
    /// Monad Associativity:
    /// m.flat_map(f).flat_map(g).run(k) == m.flat_map(|x| f(x).flat_map(g)).run(k)
    #[test]
    fn prop_continuation_monad_associativity(value in any::<i32>()) {
        let cont1: Continuation<i32, i32> = Continuation::pure(value);
        let cont2: Continuation<i32, i32> = Continuation::pure(value);

        let left = cont1.flat_map(cont_add_one).flat_map(cont_multiply_two).run(|x| x);
        let right = cont2.flat_map(|x| cont_add_one(x).flat_map(cont_multiply_two)).run(|x| x);

        prop_assert_eq!(left, right);
    }
}

proptest! {
    /// Monad Associativity with different types
    #[test]
    fn prop_continuation_monad_associativity_types(value in any::<i32>()) {
        let cont1: Continuation<usize, i32> = Continuation::pure(value);
        let cont2: Continuation<usize, i32> = Continuation::pure(value);

        let left = cont1.flat_map(cont_to_string::<usize>).flat_map(cont_string_length::<usize>).run(|x| x);
        let right = cont2.flat_map(|x| cont_to_string::<usize>(x).flat_map(cont_string_length::<usize>)).run(|x| x);

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// and_then / flat_map equivalence
// =============================================================================

proptest! {
    /// and_then is an alias for flat_map
    #[test]
    fn prop_continuation_and_then_flat_map_equivalence(value in any::<i32>()) {
        let cont1: Continuation<i32, i32> = Continuation::pure(value);
        let cont2: Continuation<i32, i32> = Continuation::pure(value);

        let left = cont1.flat_map(cont_multiply_two_generic::<i32>).run(|x| x);
        let right = cont2.and_then(cont_multiply_two_generic::<i32>).run(|x| x);

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// map via flat_map
// =============================================================================

proptest! {
    /// map(f) == flat_map(|x| pure(f(x)))
    #[test]
    fn prop_continuation_map_via_flat_map(value in any::<i32>()) {
        let cont1: Continuation<i32, i32> = Continuation::pure(value);
        let cont2: Continuation<i32, i32> = Continuation::pure(value);

        let mapped = cont1.map(multiply_three).run(|x| x);
        let flat_mapped = cont2.flat_map(|x| Continuation::pure(multiply_three(x))).run(|x| x);

        prop_assert_eq!(mapped, flat_mapped);
    }
}

// =============================================================================
// then behavior
// =============================================================================

proptest! {
    /// then discards the first value
    #[test]
    fn prop_continuation_then_discards_first(
        value1 in any::<i32>(),
        value2 in any::<i32>()
    ) {
        let first: Continuation<i32, i32> = Continuation::pure(value1);
        let second: Continuation<i32, i32> = Continuation::pure(value2);
        let result = first.then(second).run(|x| x);

        prop_assert_eq!(result, value2);
    }
}

// =============================================================================
// Continuation-specific properties
// =============================================================================

proptest! {
    /// pure value passes through unchanged
    #[test]
    fn prop_continuation_pure_passes_through(value in any::<i32>()) {
        let cont: Continuation<i32, i32> = Continuation::pure(value);
        let result = cont.run(|x| x);

        prop_assert_eq!(result, value);
    }
}

proptest! {
    /// Continuation::new with identity passes value through
    #[test]
    fn prop_continuation_new_identity(value in any::<i32>()) {
        let cont: Continuation<i32, i32> = Continuation::new(move |k| k(value));
        let result = cont.run(|x| x);

        prop_assert_eq!(result, value);
    }
}

proptest! {
    /// Continuation can transform in the final run
    #[test]
    fn prop_continuation_final_transform(value in any::<i32>()) {
        let cont: Continuation<String, i32> = Continuation::pure(value);
        let result = cont.run(|x| x.to_string());

        prop_assert_eq!(result, value.to_string());
    }
}

// =============================================================================
// call_with_current_continuation_once properties
// =============================================================================

proptest! {
    /// call_cc without exit behaves like pure
    #[test]
    fn prop_continuation_call_cc_no_exit_like_pure(value in any::<i32>()) {
        let cont = Continuation::call_with_current_continuation_once(move |_exit| {
            Continuation::pure(value)
        });
        let pure_cont: Continuation<i32, i32> = Continuation::pure(value);

        prop_assert_eq!(cont.run(|x| x), pure_cont.run(|x| x));
    }
}

proptest! {
    /// call_cc with immediate exit returns the exit value
    #[test]
    fn prop_continuation_call_cc_immediate_exit(value in any::<i32>()) {
        let cont: Continuation<i32, i32> =
            Continuation::call_with_current_continuation_once(move |exit| exit(value));

        prop_assert_eq!(cont.run(|x| x), value);
    }
}

proptest! {
    /// Conditional exit works correctly (condition true)
    #[test]
    fn prop_continuation_call_cc_conditional_true(value in 11i32..100i32) {
        // value > 10, so exit should be called
        let cont = Continuation::call_with_current_continuation_once(move |exit| {
            Continuation::pure(value).flat_map(move |x| {
                if x > 10 {
                    exit(x * 100)
                } else {
                    Continuation::pure(x + 5)
                }
            })
        });

        prop_assert_eq!(cont.run(|x| x), value * 100);
    }
}

proptest! {
    /// Conditional exit works correctly (condition false)
    #[test]
    fn prop_continuation_call_cc_conditional_false(value in -100i32..10i32) {
        // value <= 10, so exit should NOT be called
        let cont = Continuation::call_with_current_continuation_once(move |exit| {
            Continuation::pure(value).flat_map(move |x| {
                if x > 10 {
                    exit(x * 100)
                } else {
                    Continuation::pure(x + 5)
                }
            })
        });

        prop_assert_eq!(cont.run(|x| x), value + 5);
    }
}

// =============================================================================
// Complex compositions
// =============================================================================

proptest! {
    /// Complex composition produces correct result
    #[test]
    fn prop_continuation_complex_composition(value in any::<i32>()) {
        let result: i32 = Continuation::pure(value)
            .flat_map(|x| Continuation::pure(x.wrapping_add(5)))
            .flat_map(|x| Continuation::pure(x.wrapping_mul(2)))
            .map(|x| x.wrapping_sub(1))
            .run(|x| x);

        let expected = (value.wrapping_add(5)).wrapping_mul(2).wrapping_sub(1);
        prop_assert_eq!(result, expected);
    }
}
