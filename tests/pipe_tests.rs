//! Unit tests for the pipe! macro.
//!
//! Tests for left-to-right function pipeline composition.

#![cfg(feature = "compose")]

use functional_rusty::pipe;

// =============================================================================
// Basic pipe! tests
// =============================================================================

#[test]
fn test_pipe_value_only() {
    let result = pipe!(42);
    assert_eq!(result, 42);
}

#[test]
fn test_pipe_value_only_string() {
    let result = pipe!(String::from("hello"));
    assert_eq!(result, "hello");
}

#[test]
fn test_pipe_single_function() {
    fn double(value: i32) -> i32 {
        value * 2
    }
    let result = pipe!(5, double);
    assert_eq!(result, 10);
}

#[test]
fn test_pipe_two_functions() {
    fn add_one(value: i32) -> i32 {
        value + 1
    }
    fn double(value: i32) -> i32 {
        value * 2
    }

    // pipe!(x, f, g) = g(f(x)) = add_one(double(5)) = add_one(10) = 11
    let result = pipe!(5, double, add_one);
    assert_eq!(result, 11);
}

#[test]
fn test_pipe_three_functions() {
    fn add_one(value: i32) -> i32 {
        value + 1
    }
    fn double(value: i32) -> i32 {
        value * 2
    }
    fn square(value: i32) -> i32 {
        value * value
    }

    // pipe!(x, f, g, h) = h(g(f(x)))
    // square(3) = 9, double(9) = 18, add_one(18) = 19
    let result = pipe!(3, square, double, add_one);
    assert_eq!(result, 19);
}

#[test]
fn test_pipe_many_functions() {
    let add_one = |value: i32| value + 1;
    let double = |value: i32| value * 2;
    let square = |value: i32| value * value;
    let negate = |value: i32| -value;
    let add_hundred = |value: i32| value + 100;

    // Starting with 2:
    // 2 -> add_one -> 3 -> double -> 6 -> square -> 36 -> negate -> -36 -> add_hundred -> 64
    let result = pipe!(2, add_one, double, square, negate, add_hundred);
    assert_eq!(result, 64);
}

// =============================================================================
// Type conversion through pipe
// =============================================================================

#[test]
fn test_pipe_with_type_conversion() {
    fn to_string(value: i32) -> String {
        value.to_string()
    }
    fn get_length(text: String) -> usize {
        text.len()
    }

    let result = pipe!(12345, to_string, get_length);
    assert_eq!(result, 5);
}

#[test]
fn test_pipe_complex_type_chain() {
    fn parse_number(text: &str) -> Option<i32> {
        text.parse().ok()
    }
    fn double_option(opt: Option<i32>) -> Option<i32> {
        opt.map(|value| value * 2)
    }
    fn option_to_string(opt: Option<i32>) -> String {
        match opt {
            Some(value) => value.to_string(),
            None => String::from("None"),
        }
    }

    let result = pipe!("42", parse_number, double_option, option_to_string);
    assert_eq!(result, "84");
}

// =============================================================================
// Ownership and consuming functions
// =============================================================================

#[test]
fn test_pipe_consuming_functions() {
    fn consume_and_double(values: Vec<i32>) -> Vec<i32> {
        values.into_iter().map(|value| value * 2).collect()
    }

    fn consume_and_filter(values: Vec<i32>) -> Vec<i32> {
        values.into_iter().filter(|value| *value > 5).collect()
    }

    let result = pipe!(vec![1, 2, 3, 4, 5], consume_and_double, consume_and_filter);
    assert_eq!(result, vec![6, 8, 10]);
}

#[test]
fn test_pipe_with_fn_once_closure() {
    let captured_string = String::from("captured");
    let consume_closure = move |_: i32| captured_string;

    let result = pipe!(42, consume_closure);
    assert_eq!(result, "captured");
}

// =============================================================================
// Closure with captured environment
// =============================================================================

#[test]
fn test_pipe_with_closures() {
    let multiplier = 3;
    let offset = 10;

    let multiply = |value: i32| value * multiplier;
    let add_offset = |value: i32| value + offset;

    // 5 -> multiply by 3 -> 15 -> add 10 -> 25
    let result = pipe!(5, multiply, add_offset);
    assert_eq!(result, 25);
}

// =============================================================================
// Trailing comma support
// =============================================================================

#[test]
fn test_pipe_with_trailing_comma() {
    fn double(value: i32) -> i32 {
        value * 2
    }
    fn add_one(value: i32) -> i32 {
        value + 1
    }

    // Should accept trailing comma
    let result = pipe!(5, double, add_one,);
    assert_eq!(result, 11);
}

// =============================================================================
// Reference handling
// =============================================================================

#[test]
fn test_pipe_with_references() {
    fn to_uppercase(text: &str) -> String {
        text.to_uppercase()
    }
    fn add_exclamation(text: String) -> String {
        format!("{}!", text)
    }

    let result = pipe!("hello", to_uppercase, add_exclamation);
    assert_eq!(result, "HELLO!");
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_pipe_with_identity() {
    use functional_rusty::compose::identity;

    let result = pipe!(42, identity);
    assert_eq!(result, 42);
}

#[test]
fn test_pipe_with_constant() {
    use functional_rusty::compose::constant;

    let result = pipe!(42, constant(100));
    assert_eq!(result, 100);
}

// =============================================================================
// Equivalence with compose
// =============================================================================

mod compose_equivalence {
    use functional_rusty::{compose, pipe};

    #[test]
    fn test_pipe_compose_equivalence_two_functions() {
        fn add_one(value: i32) -> i32 {
            value + 1
        }
        fn double(value: i32) -> i32 {
            value * 2
        }

        let pipe_result = pipe!(5, double, add_one);
        let compose_result = compose!(add_one, double)(5);

        assert_eq!(pipe_result, compose_result);
    }

    #[test]
    fn test_pipe_compose_equivalence_three_functions() {
        fn add_one(value: i32) -> i32 {
            value + 1
        }
        fn double(value: i32) -> i32 {
            value * 2
        }
        fn square(value: i32) -> i32 {
            value * value
        }

        let pipe_result = pipe!(3, square, double, add_one);
        let compose_result = compose!(add_one, double, square)(3);

        assert_eq!(pipe_result, compose_result);
    }

    #[test]
    fn test_pipe_compose_equivalence_with_closures() {
        let f = |value: i32| value + 1;
        let g = |value: i32| value * 2;
        let h = |value: i32| value - 3;

        // pipe!(x, f, g, h) should equal compose!(h, g, f)(x)
        for input in -10..=10 {
            let pipe_result = pipe!(input, f, g, h);
            let compose_result = compose!(h, g, f)(input);
            assert_eq!(pipe_result, compose_result, "Failed for input: {}", input);
        }
    }
}
