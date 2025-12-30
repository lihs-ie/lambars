//! Property-based tests for Trampoline<A> laws.
//!
//! This module verifies that Trampoline implementations satisfy:
//!
//! - **Stack Safety**: Deep recursion does not overflow the stack
//! - **Functor Laws**: identity and composition
//! - **Monad Laws**: left identity, right identity, associativity

#![cfg(feature = "control")]

use functional_rusty::control::Trampoline;
use proptest::prelude::*;

// =============================================================================
// Stack Safety
// =============================================================================

proptest! {
    /// Stack safety: deep recursion using suspend does not overflow
    #[test]
    fn prop_trampoline_stack_safety_suspend(depth in 1000u64..10000u64) {
        fn count_down(n: u64) -> Trampoline<u64> {
            if n == 0 {
                Trampoline::done(0)
            } else {
                Trampoline::suspend(move || count_down(n - 1))
            }
        }

        let result = count_down(depth).run();
        prop_assert_eq!(result, 0);
    }
}

proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(32))]
    /// Stack safety: deep flat_map chains do not overflow
    #[test]
    fn prop_trampoline_stack_safety_flat_map(depth in 50u64..200u64) {
        fn nested(n: u64) -> Trampoline<u64> {
            if n == 0 {
                Trampoline::done(0)
            } else {
                Trampoline::suspend(move || nested(n - 1))
                    .flat_map(|x| Trampoline::done(x + 1))
            }
        }

        let result = nested(depth).run();
        prop_assert_eq!(result, depth);
    }
}

proptest! {
    /// Stack safety: mutual recursion does not overflow
    #[test]
    fn prop_trampoline_stack_safety_mutual_recursion(n in 1000u64..5000u64) {
        fn is_even(n: u64) -> Trampoline<bool> {
            if n == 0 {
                Trampoline::done(true)
            } else {
                Trampoline::suspend(move || is_odd(n - 1))
            }
        }

        fn is_odd(n: u64) -> Trampoline<bool> {
            if n == 0 {
                Trampoline::done(false)
            } else {
                Trampoline::suspend(move || is_even(n - 1))
            }
        }

        let result = is_even(n).run();
        prop_assert_eq!(result, n % 2 == 0);
    }
}

// =============================================================================
// Functor Laws
// =============================================================================

proptest! {
    /// Functor Identity Law: trampoline.map(|x| x).run() == trampoline.run()
    #[test]
    fn prop_trampoline_functor_identity(value in any::<i32>()) {
        let trampoline = Trampoline::done(value);
        let mapped = Trampoline::done(value).map(|x| x);

        prop_assert_eq!(trampoline.run(), mapped.run());
    }
}

proptest! {
    /// Functor Identity Law with suspend
    #[test]
    fn prop_trampoline_functor_identity_suspend(value in any::<i32>()) {
        let trampoline = Trampoline::suspend(move || Trampoline::done(value));
        let mapped = Trampoline::suspend(move || Trampoline::done(value)).map(|x| x);

        prop_assert_eq!(trampoline.run(), mapped.run());
    }
}

proptest! {
    /// Functor Composition Law:
    /// trampoline.map(f).map(g).run() == trampoline.map(|x| g(f(x))).run()
    #[test]
    fn prop_trampoline_functor_composition(value in any::<i32>()) {
        fn function1(n: i32) -> i32 { n.wrapping_add(1) }
        fn function2(n: i32) -> i32 { n.wrapping_mul(2) }

        let left = Trampoline::done(value).map(function1).map(function2);
        let right = Trampoline::done(value).map(|x| function2(function1(x)));

        prop_assert_eq!(left.run(), right.run());
    }
}

proptest! {
    /// Functor Composition Law with suspend
    #[test]
    fn prop_trampoline_functor_composition_suspend(value in any::<i32>()) {
        fn function1(n: i32) -> i32 { n.wrapping_add(1) }
        fn function2(n: i32) -> i32 { n.wrapping_mul(2) }

        let left = Trampoline::suspend(move || Trampoline::done(value))
            .map(function1)
            .map(function2);
        let right = Trampoline::suspend(move || Trampoline::done(value))
            .map(|x| function2(function1(x)));

        prop_assert_eq!(left.run(), right.run());
    }
}

// =============================================================================
// Monad Laws
// =============================================================================

proptest! {
    /// Monad Left Identity: Trampoline::done(a).flat_map(f).run() == f(a).run()
    #[test]
    fn prop_trampoline_monad_left_identity(value in any::<i32>()) {
        fn function(x: i32) -> Trampoline<i32> { Trampoline::done(x.wrapping_mul(2)) }

        let left = Trampoline::done(value).flat_map(function);
        let right = function(value);

        prop_assert_eq!(left.run(), right.run());
    }
}

proptest! {
    /// Monad Left Identity with suspend in function result
    #[test]
    fn prop_trampoline_monad_left_identity_suspend(value in any::<i32>()) {
        fn function(x: i32) -> Trampoline<i32> {
            Trampoline::suspend(move || Trampoline::done(x.wrapping_mul(2)))
        }

        let left = Trampoline::done(value).flat_map(function);
        let right = function(value);

        prop_assert_eq!(left.run(), right.run());
    }
}

proptest! {
    /// Monad Right Identity: m.flat_map(Trampoline::done).run() == m.run()
    #[test]
    fn prop_trampoline_monad_right_identity(value in any::<i32>()) {
        let trampoline = Trampoline::done(value);
        let flat_mapped = Trampoline::done(value).flat_map(Trampoline::done);

        prop_assert_eq!(trampoline.run(), flat_mapped.run());
    }
}

proptest! {
    /// Monad Right Identity with suspend
    #[test]
    fn prop_trampoline_monad_right_identity_suspend(value in any::<i32>()) {
        let trampoline = Trampoline::suspend(move || Trampoline::done(value));
        let flat_mapped = Trampoline::suspend(move || Trampoline::done(value))
            .flat_map(Trampoline::done);

        prop_assert_eq!(trampoline.run(), flat_mapped.run());
    }
}

proptest! {
    /// Monad Associativity:
    /// m.flat_map(f).flat_map(g).run() == m.flat_map(|x| f(x).flat_map(g)).run()
    #[test]
    fn prop_trampoline_monad_associativity(value in any::<i32>()) {
        fn function1(x: i32) -> Trampoline<i32> { Trampoline::done(x.wrapping_add(1)) }
        fn function2(x: i32) -> Trampoline<i32> { Trampoline::done(x.wrapping_mul(2)) }

        let left = Trampoline::done(value).flat_map(function1).flat_map(function2);
        let right = Trampoline::done(value).flat_map(|x| function1(x).flat_map(function2));

        prop_assert_eq!(left.run(), right.run());
    }
}

proptest! {
    /// Monad Associativity with suspend
    #[test]
    fn prop_trampoline_monad_associativity_suspend(value in any::<i32>()) {
        fn function1(x: i32) -> Trampoline<i32> {
            Trampoline::suspend(move || Trampoline::done(x.wrapping_add(1)))
        }
        fn function2(x: i32) -> Trampoline<i32> {
            Trampoline::suspend(move || Trampoline::done(x.wrapping_mul(2)))
        }

        let left = Trampoline::done(value).flat_map(function1).flat_map(function2);
        let right = Trampoline::done(value).flat_map(|x| function1(x).flat_map(function2));

        prop_assert_eq!(left.run(), right.run());
    }
}

// =============================================================================
// pure / done equivalence
// =============================================================================

proptest! {
    /// pure and done produce identical results
    #[test]
    fn prop_trampoline_pure_done_equivalence(value in any::<i32>()) {
        let from_done = Trampoline::done(value);
        let from_pure = Trampoline::pure(value);

        prop_assert_eq!(from_done.run(), from_pure.run());
    }
}

// =============================================================================
// and_then / flat_map equivalence
// =============================================================================

proptest! {
    /// and_then is an alias for flat_map
    #[test]
    fn prop_trampoline_and_then_flat_map_equivalence(value in any::<i32>()) {
        fn function(x: i32) -> Trampoline<i32> { Trampoline::done(x.wrapping_mul(2)) }

        let from_flat_map = Trampoline::done(value).flat_map(function);
        let from_and_then = Trampoline::done(value).and_then(function);

        prop_assert_eq!(from_flat_map.run(), from_and_then.run());
    }
}

// =============================================================================
// map via flat_map
// =============================================================================

proptest! {
    /// map(f) == flat_map(|x| done(f(x)))
    #[test]
    fn prop_trampoline_map_via_flat_map(value in any::<i32>()) {
        fn function(x: i32) -> i32 { x.wrapping_mul(3) }

        let mapped = Trampoline::done(value).map(function);
        let flat_mapped = Trampoline::done(value).flat_map(|x| Trampoline::done(function(x)));

        prop_assert_eq!(mapped.run(), flat_mapped.run());
    }
}

// =============================================================================
// then behavior
// =============================================================================

proptest! {
    /// then discards the first value
    #[test]
    fn prop_trampoline_then_discards_first(
        value1 in any::<i32>(),
        value2 in any::<i32>()
    ) {
        let first = Trampoline::done(value1);
        let second = Trampoline::done(value2);
        let result = first.then(second);

        prop_assert_eq!(result.run(), value2);
    }
}

// =============================================================================
// Recursive computations
// =============================================================================

proptest! {
    /// Factorial produces correct results
    #[test]
    fn prop_trampoline_factorial(n in 0u64..20u64) {
        fn factorial(n: u64) -> Trampoline<u64> {
            factorial_helper(n, 1)
        }

        fn factorial_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
            if n <= 1 {
                Trampoline::done(accumulator)
            } else {
                Trampoline::suspend(move || factorial_helper(n - 1, n * accumulator))
            }
        }

        fn expected_factorial(n: u64) -> u64 {
            (1..=n).product()
        }

        let result = factorial(n).run();
        let expected = expected_factorial(n);

        prop_assert_eq!(result, expected);
    }
}

proptest! {
    /// Sum produces correct results
    #[test]
    fn prop_trampoline_sum(n in 0u64..1000u64) {
        fn sum_to(n: u64) -> Trampoline<u64> {
            sum_to_helper(n, 0)
        }

        fn sum_to_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
            if n == 0 {
                Trampoline::done(accumulator)
            } else {
                Trampoline::suspend(move || sum_to_helper(n - 1, accumulator + n))
            }
        }

        let result = sum_to(n).run();
        let expected = n * (n + 1) / 2;

        prop_assert_eq!(result, expected);
    }
}

// =============================================================================
// resume behavior
// =============================================================================

proptest! {
    /// resume on done returns Right
    #[test]
    fn prop_trampoline_resume_done_is_right(value in any::<i32>()) {
        use functional_rusty::control::Either;

        let trampoline = Trampoline::done(value);
        let resumed = trampoline.resume();

        match resumed {
            Either::Right(v) => prop_assert_eq!(v, value),
            Either::Left(_) => prop_assert!(false, "Expected Right, got Left"),
        }
    }
}

proptest! {
    /// resume on suspend returns Left
    #[test]
    fn prop_trampoline_resume_suspend_is_left(value in any::<i32>()) {
        use functional_rusty::control::Either;

        let trampoline = Trampoline::suspend(move || Trampoline::done(value));
        let resumed = trampoline.resume();

        match resumed {
            Either::Left(thunk) => {
                // Execute the thunk and check it returns the value
                let next = thunk();
                prop_assert_eq!(next.run(), value);
            }
            Either::Right(_) => prop_assert!(false, "Expected Left, got Right"),
        }
    }
}
