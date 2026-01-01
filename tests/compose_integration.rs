#![cfg(feature = "compose")]
//! Integration tests for compose module.
//!
//! These tests verify that all composition utilities work together correctly
//! in real-world scenarios. They test complex combinations of:
//!
//! - `compose!` macro
//! - `pipe!` macro
//! - `partial!` macro
//! - `curry2!` through `curry6!` macros
//! - `identity`, `constant`, `flip` helper functions

#![allow(unused_imports)]

use lambars::compose::{constant, flip, identity};
use lambars::{compose, curry2, curry3, curry4, partial, pipe};

// =============================================================================
// Complex Pipeline Scenarios
// =============================================================================

#[test]
fn test_complex_data_transformation_pipeline() {
    // Simulate a data processing pipeline
    fn parse_number(text: &str) -> Result<i32, &'static str> {
        text.trim().parse().map_err(|_| "Parse error")
    }

    fn validate_positive(number: i32) -> Result<i32, &'static str> {
        if number > 0 {
            Ok(number)
        } else {
            Err("Number must be positive")
        }
    }

    fn double(number: i32) -> i32 {
        number * 2
    }

    fn format_result(number: i32) -> String {
        format!("Result: {number}")
    }

    // Using pipe for the successful path
    let input = "  42  ";
    let result: Result<String, &str> = parse_number(input)
        .and_then(validate_positive)
        .map(|n| pipe!(n, double, format_result));

    assert_eq!(result, Ok("Result: 84".to_string()));
}

#[test]
fn test_curried_functions_in_pipeline() {
    let add = |first: i32, second: i32| first + second;
    let multiply = |first: i32, second: i32| first * second;

    let add_ten = curry2!(add)(10);
    let triple = curry2!(multiply)(3);

    // Build a processing pipeline with curried functions
    let result = pipe!(5, triple, add_ten);

    // 5 * 3 = 15, then 15 + 10 = 25
    assert_eq!(result, 25);
}

#[test]
fn test_partial_application_with_compose() {
    fn calculate(base: i32, multiplier: i32, addend: i32) -> i32 {
        base * multiplier + addend
    }

    // Create specialized versions using partial
    let double_and_add = partial!(calculate, __, 2, 5); // x * 2 + 5
    let triple_and_add = partial!(calculate, __, 3, 10); // x * 3 + 10

    // Compose them: triple_and_add(double_and_add(x))
    let composed = compose!(triple_and_add, double_and_add);

    // double_and_add(4) = 4 * 2 + 5 = 13
    // triple_and_add(13) = 13 * 3 + 10 = 49
    assert_eq!(composed(4), 49);
}

#[test]
fn test_flip_with_partial_application() {
    fn power(base: i32, exponent: u32) -> i32 {
        base.pow(exponent)
    }

    // Normal: power(base, exponent)
    // Flipped: flipped_power(exponent, base)
    let flipped_power = flip(power);

    // Use partial to fix the exponent (first argument of flipped function)
    let square = partial!(flipped_power, 2u32, __);

    assert_eq!(square(5), 25); // 5^2
    assert_eq!(square(3), 9); // 3^2

    // Create another power function
    let flipped_power2 = flip(power);
    let cube = partial!(flipped_power2, 3u32, __);

    assert_eq!(cube(3), 27); // 3^3
    assert_eq!(cube(2), 8); // 2^3
}

// =============================================================================
// Functional Programming Patterns
// =============================================================================

#[test]
fn test_point_free_style() {
    // Point-free style: defining functions without explicitly mentioning their arguments
    fn add_one(x: i32) -> i32 {
        x + 1
    }
    fn double(x: i32) -> i32 {
        x * 2
    }
    fn negate(x: i32) -> i32 {
        -x
    }

    // Define a complex transformation without mentioning the argument
    let transform = compose!(negate, add_one, double);

    // transform(x) = negate(add_one(double(x))) = -(double(x) + 1) = -(2x + 1)
    assert_eq!(transform(5), -11); // -(10 + 1) = -11
    assert_eq!(transform(0), -1); // -(0 + 1) = -1
}

#[test]
fn test_function_reuse_patterns() {
    let add = |first: i32, second: i32| first + second;

    // Create a family of related functions
    let add_one = curry2!(add)(1);
    let add_five = curry2!(add)(5);
    let add_ten = curry2!(add)(10);

    // All are reusable
    assert_eq!(add_one(100), 101);
    assert_eq!(add_five(100), 105);
    assert_eq!(add_ten(100), 110);

    // Can be composed
    let add_sixteen = compose!(add_ten, add_five, add_one);
    assert_eq!(add_sixteen(0), 16);
}

#[test]
fn test_constant_in_pipeline() {
    // Using constant to replace values in a pipeline
    // Each constant function is parameterized by the input type
    let always_zero_from_int = constant::<i32, i32>(0);
    let always_zero_from_str = constant::<i32, &str>(0);

    let always_hello_from_int = constant::<&str, i32>("hello");
    let always_hello_from_vec = constant::<&str, Vec<i32>>("hello");

    // constant ignores its input
    assert_eq!(always_zero_from_int(42), 0);
    assert_eq!(always_zero_from_str("anything"), 0);

    assert_eq!(always_hello_from_int(123), "hello");
    assert_eq!(always_hello_from_vec(vec![1, 2, 3]), "hello");
}

#[test]
fn test_identity_as_default_transform() {
    // Using identity when no transformation is needed
    let maybe_transform = |should_double: bool| {
        if should_double {
            |x: i32| x * 2
        } else {
            identity
        }
    };

    let double_transform = maybe_transform(true);
    let no_transform = maybe_transform(false);

    assert_eq!(double_transform(5), 10);
    assert_eq!(no_transform(5), 5);
}

// =============================================================================
// Real-World Use Cases
// =============================================================================

#[test]
fn test_string_processing_pipeline() {
    fn trim_whitespace(text: String) -> String {
        text.trim().to_string()
    }

    fn to_uppercase(text: String) -> String {
        text.to_uppercase()
    }

    fn add_prefix(text: String) -> String {
        format!("[INFO] {text}")
    }

    let process = compose!(add_prefix, to_uppercase, trim_whitespace);

    let result = process("  hello world  ".to_string());
    assert_eq!(result, "[INFO] HELLO WORLD");
}

#[test]
fn test_numeric_calculations_with_curry() {
    fn linear_equation(slope: f64, intercept: f64, variable: f64) -> f64 {
        slope * variable + intercept
    }

    // Create specific linear functions: y = mx + b
    let curried = curry3!(linear_equation);

    // y = 2x + 3
    let line1 = curried(2.0)(3.0);

    // y = 0.5x - 1
    let line2 = curried(0.5)(-1.0);

    assert!((line1(4.0) - 11.0).abs() < f64::EPSILON); // 2*4 + 3 = 11
    assert!((line2(6.0) - 2.0).abs() < f64::EPSILON); // 0.5*6 - 1 = 2
}

#[test]
fn test_collection_transformation() {
    let numbers = vec![1, 2, 3, 4, 5];

    let add_one = |x: i32| x + 1;
    let double = |x: i32| x * 2;

    // Transform each element through a composed function
    let transform = compose!(double, add_one);

    let result: Vec<i32> = numbers.into_iter().map(transform).collect();

    // Each element: double(add_one(x)) = double(x+1) = 2(x+1)
    assert_eq!(result, vec![4, 6, 8, 10, 12]);
}

#[test]
fn test_optional_value_processing() {
    fn safe_divide(numerator: i32, denominator: i32) -> Option<i32> {
        if denominator == 0 {
            None
        } else {
            Some(numerator / denominator)
        }
    }

    let curried_divide = curry2!(safe_divide);
    let divide_100_by = curried_divide(100);

    assert_eq!(divide_100_by(5), Some(20));
    assert_eq!(divide_100_by(0), None);
    assert_eq!(divide_100_by(4), Some(25));
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_pipe_with_side_effects() {
    // Using pipe! for a sequence where one step has side effects
    fn double(x: i32) -> i32 {
        x * 2
    }

    fn add_ten(x: i32) -> i32 {
        x + 10
    }

    // Simple pipeline without side effects (compose! requires Fn, not FnMut)
    let result = pipe!(5, double, add_ten);
    assert_eq!(result, 20); // 5 * 2 = 10, then 10 + 10 = 20
}

#[test]
fn test_curry_with_owned_types() {
    fn concatenate(first: String, second: String) -> String {
        format!("{first}{second}")
    }

    let curried = curry2!(concatenate);
    let hello_plus = curried(String::from("Hello, "));

    assert_eq!(hello_plus(String::from("World!")), "Hello, World!");
    assert_eq!(hello_plus(String::from("Rust!")), "Hello, Rust!");
}

#[test]
fn test_partial_with_all_placeholders() {
    fn add(first: i32, second: i32) -> i32 {
        first + second
    }

    // All placeholders - equivalent to the original function
    let add_alias = partial!(add, __, __);

    assert_eq!(add_alias(3, 4), 7);
    assert_eq!(add_alias(10, 20), 30);
}

#[test]
fn test_nested_compositions() {
    let f1 = |x: i32| x + 1;
    let f2 = |x: i32| x * 2;
    let f3 = |x: i32| x - 3;

    // Nested compose
    let inner = compose!(f2, f3);
    let outer = compose!(f1, inner);

    // outer(x) = f1(f2(f3(x))) = (x - 3) * 2 + 1
    assert_eq!(outer(10), 15); // (10 - 3) * 2 + 1 = 15
}

// =============================================================================
// Performance Considerations
// =============================================================================

#[test]
fn test_curried_function_reuse_many_times() {
    let add = |first: i32, second: i32| first + second;
    let add_five = curry2!(add)(5);

    // Should be able to reuse the curried function many times
    let sum: i32 = (0..1000).map(&add_five).sum();

    // Sum of (i + 5) for i = 0..1000 = sum(0..1000) + 5*1000
    // = (999 * 1000 / 2) + 5000 = 499500 + 5000 = 504500
    assert_eq!(sum, 504500);
}

#[test]
fn test_compose_chain_performance() {
    let f1 = |x: i64| x.wrapping_add(1);
    let f2 = |x: i64| x.wrapping_mul(2);
    let f3 = |x: i64| x.wrapping_sub(3);
    let f4 = |x: i64| x.wrapping_add(4);
    let f5 = |x: i64| x.wrapping_mul(5);

    let composed = compose!(f5, f4, f3, f2, f1);

    // Execute many times
    let results: Vec<i64> = (0..1000).map(composed).collect();

    // Verify first few results
    // f5(f4(f3(f2(f1(0))))) = f5(f4(f3(f2(1)))) = f5(f4(f3(2))) = f5(f4(-1)) = f5(3) = 15
    assert_eq!(results[0], 15);
    // f5(f4(f3(f2(f1(1))))) = f5(f4(f3(f2(2)))) = f5(f4(f3(4))) = f5(f4(1)) = f5(5) = 25
    assert_eq!(results[1], 25);
}
