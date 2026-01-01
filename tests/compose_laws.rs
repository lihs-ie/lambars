#![cfg(feature = "compose")]
//! Property-based tests for function composition laws.
//!
//! This module verifies that composition utilities satisfy the required laws:
//!
//! ## Composition Laws
//! - **Associativity**: `compose!(f, compose!(g, h)) == compose!(compose!(f, g), h)`
//! - **Left Identity**: `compose!(identity, f) == f`
//! - **Right Identity**: `compose!(f, identity) == f`
//!
//! ## Pipe Laws
//! - **Consistency with Compose**: `pipe!(x, f, g) == compose!(g, f)(x)`
//!
//! ## Flip Laws
//! - **Double Flip Identity**: `flip(flip(f)) == f`
//! - **Flip Definition**: `flip(f)(a, b) == f(b, a)`
//!
//! ## Curry Laws
//! - **Equivalence**: `curry2!(f)(a)(b) == f(a, b)`
//!
//! Using proptest, we generate random inputs to thoroughly verify these laws
//! across a wide range of values.

#![allow(unused_imports)]

use lambars::compose::constant;
use lambars::compose::flip;
use lambars::compose::identity;
use lambars::{compose, curry2, curry3, partial, pipe};
use proptest::prelude::*;

// =============================================================================
// Composition Laws
// =============================================================================

proptest! {
    /// Left Identity Law: compose!(identity, f)(x) == f(x)
    #[test]
    fn prop_compose_left_identity(x in any::<i32>()) {
        let function = |n: i32| n.wrapping_mul(2);

        let composed = compose!(identity, function);

        prop_assert_eq!(composed(x), function(x));
    }

    /// Right Identity Law: compose!(f, identity)(x) == f(x)
    #[test]
    fn prop_compose_right_identity(x in any::<i32>()) {
        let function = |n: i32| n.wrapping_mul(2);

        let composed = compose!(function, identity);

        prop_assert_eq!(composed(x), function(x));
    }

    /// Associativity Law: compose!(f, compose!(g, h)) == compose!(compose!(f, g), h)
    #[test]
    fn prop_compose_associativity(x in any::<i32>()) {
        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);
        let function3 = |n: i32| n.wrapping_sub(3);

        // compose!(f, compose!(g, h))
        let inner_right = compose!(function2, function3);
        let left_associative = compose!(function1, inner_right);

        // compose!(compose!(f, g), h)
        let inner_left = compose!(function1, function2);
        let right_associative = compose!(inner_left, function3);

        prop_assert_eq!(left_associative(x), right_associative(x));
    }
}

// =============================================================================
// Pipe Laws
// =============================================================================

proptest! {
    /// Pipe consistency with compose: pipe!(x, f, g) == compose!(g, f)(x)
    #[test]
    fn prop_pipe_compose_consistency(x in any::<i32>()) {
        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);

        let pipe_result = pipe!(x, function1, function2);
        let compose_result = compose!(function2, function1)(x);

        prop_assert_eq!(pipe_result, compose_result);
    }

    /// Pipe with identity: pipe!(x, identity) == x
    #[test]
    fn prop_pipe_identity(x in any::<i32>()) {
        let result = pipe!(x, identity);
        prop_assert_eq!(result, x);
    }

    /// Pipe single function: pipe!(x, f) == f(x)
    #[test]
    fn prop_pipe_single(x in any::<i32>()) {
        let function = |n: i32| n.wrapping_mul(2);

        let pipe_result = pipe!(x, function);

        prop_assert_eq!(pipe_result, function(x));
    }

    /// Pipe multiple functions with compose equivalence
    #[test]
    fn prop_pipe_three_functions(x in any::<i32>()) {
        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);
        let function3 = |n: i32| n.wrapping_sub(3);

        let pipe_result = pipe!(x, function1, function2, function3);
        let composed = compose!(function3, function2, function1);

        prop_assert_eq!(pipe_result, composed(x));
    }
}

// =============================================================================
// Identity Function Laws
// =============================================================================

proptest! {
    /// Identity function returns input unchanged (i32)
    #[test]
    fn prop_identity_i32(x in any::<i32>()) {
        prop_assert_eq!(identity(x), x);
    }

    /// Identity function returns input unchanged (String)
    #[test]
    fn prop_identity_string(x in any::<String>()) {
        prop_assert_eq!(identity(x.clone()), x);
    }
}

// =============================================================================
// Constant Function Laws
// =============================================================================

proptest! {
    /// Constant function always returns the same value
    #[test]
    fn prop_constant_ignores_input(constant_value in any::<i32>(), input in any::<i32>()) {
        let always_constant = constant::<i32, i32>(constant_value);
        prop_assert_eq!(always_constant(input), constant_value);
    }

    /// Constant function works with different input types
    #[test]
    fn prop_constant_different_input_type(constant_value in any::<i32>(), input in any::<String>()) {
        let always_constant = constant::<i32, String>(constant_value);
        prop_assert_eq!(always_constant(input), constant_value);
    }
}

// =============================================================================
// Flip Laws
// =============================================================================

proptest! {
    /// Double flip is identity: flip(flip(f))(a, b) == f(a, b)
    #[test]
    fn prop_flip_double_identity(a in any::<i32>(), b in any::<i32>()) {
        let function = |x: i32, y: i32| x.wrapping_sub(y);

        let flipped_once = flip(function);
        let flipped_twice = flip(flipped_once);

        prop_assert_eq!(flipped_twice(a, b), function(a, b));
    }

    /// Flip definition: flip(f)(a, b) == f(b, a)
    #[test]
    fn prop_flip_definition(a in any::<i32>(), b in any::<i32>()) {
        let function = |x: i32, y: i32| x.wrapping_sub(y);

        let flipped = flip(function);

        prop_assert_eq!(flipped(a, b), function(b, a));
    }

    /// Flip preserves function semantics
    #[test]
    fn prop_flip_division(a in 1i32..1000, b in 1i32..1000) {
        let divide = |x: i32, y: i32| x / y;
        let flipped_divide = flip(divide);

        // flipped_divide(a, b) = divide(b, a)
        prop_assert_eq!(flipped_divide(a, b), divide(b, a));
    }
}

// =============================================================================
// Curry Laws
// =============================================================================

proptest! {
    /// Curry2 equivalence: curry2!(f)(a)(b) == f(a, b)
    #[test]
    fn prop_curry2_equivalence(a in any::<i32>(), b in any::<i32>()) {
        let function = |x: i32, y: i32| x.wrapping_add(y);

        let curried = curry2!(function);

        prop_assert_eq!(curried(a)(b), function(a, b));
    }

    /// Curry3 equivalence: curry3!(f)(a)(b)(c) == f(a, b, c)
    #[test]
    fn prop_curry3_equivalence(a in any::<i32>(), b in any::<i32>(), c in any::<i32>()) {
        let function = |x: i32, y: i32, z: i32| x.wrapping_add(y).wrapping_add(z);

        let curried = curry3!(function);

        prop_assert_eq!(curried(a)(b)(c), function(a, b, c));
    }

    /// Curried function can be partially applied and reused
    #[test]
    fn prop_curry2_partial_reuse(a in any::<i32>(), b1 in any::<i32>(), b2 in any::<i32>()) {
        let function = |x: i32, y: i32| x.wrapping_add(y);

        let curried = curry2!(function);
        let partial = curried(a);

        // Same partial can be called with different arguments
        prop_assert_eq!(partial(b1), function(a, b1));
        prop_assert_eq!(partial(b2), function(a, b2));
    }
}

// =============================================================================
// Partial Application Laws
// =============================================================================

proptest! {
    /// Partial with first argument fixed: partial!(f, a, __)(b) == f(a, b)
    #[test]
    fn prop_partial_first_fixed(a in any::<i32>(), b in any::<i32>()) {
        let function = |x: i32, y: i32| x.wrapping_sub(y);

        let partial_function = partial!(function, a, __);

        prop_assert_eq!(partial_function(b), function(a, b));
    }

    /// Partial with second argument fixed: partial!(f, __, b)(a) == f(a, b)
    #[test]
    fn prop_partial_second_fixed(a in any::<i32>(), b in any::<i32>()) {
        let function = |x: i32, y: i32| x.wrapping_sub(y);

        let partial_function = partial!(function, __, b);

        prop_assert_eq!(partial_function(a), function(a, b));
    }

    /// Partial with all arguments fixed: partial!(f, a, b)() == f(a, b)
    #[test]
    fn prop_partial_all_fixed(a in any::<i32>(), b in any::<i32>()) {
        let function = |x: i32, y: i32| x.wrapping_add(y);

        let partial_function = partial!(function, a, b);

        prop_assert_eq!(partial_function(), function(a, b));
    }
}

// =============================================================================
// Integration Laws
// =============================================================================

proptest! {
    /// Curried and composed functions work together
    #[test]
    fn prop_curry_compose_integration(x in any::<i32>()) {
        let add = |a: i32, b: i32| a.wrapping_add(b);
        let multiply = |a: i32, b: i32| a.wrapping_mul(b);

        let add_five = curry2!(add)(5);
        let double = curry2!(multiply)(2);

        // compose!(add_five, double)(x) = add_five(double(x)) = double(x) + 5
        let composed = compose!(add_five, double);
        let expected = x.wrapping_mul(2).wrapping_add(5);

        prop_assert_eq!(composed(x), expected);
    }

    /// Partial application and compose work together
    #[test]
    fn prop_partial_compose_integration(x in any::<i32>()) {
        let subtract = |a: i32, b: i32| a.wrapping_sub(b);

        let subtract_five = partial!(subtract, __, 5);  // x - 5
        let five_minus = partial!(subtract, 5, __);      // 5 - x

        let composed = compose!(five_minus, subtract_five);

        // composed(x) = five_minus(subtract_five(x)) = 5 - (x - 5) = 10 - x
        prop_assert_eq!(composed(x), 10i32.wrapping_sub(x));
    }
}
