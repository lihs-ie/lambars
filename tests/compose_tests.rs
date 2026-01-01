//! Unit tests for function composition utilities.
//!
//! Tests for identity, constant, flip functions and compose! macro.

#![cfg(feature = "compose")]

use lambars::compose::{constant, flip, identity};

// =============================================================================
// identity function tests
// =============================================================================

#[test]
fn test_identity_returns_same_integer() {
    assert_eq!(identity(42), 42);
    assert_eq!(identity(-100), -100);
    assert_eq!(identity(0), 0);
}

#[test]
fn test_identity_returns_same_string() {
    assert_eq!(identity("hello"), "hello");
    assert_eq!(identity(String::from("world")), String::from("world"));
}

#[test]
fn test_identity_returns_same_vector() {
    assert_eq!(identity(vec![1, 2, 3]), vec![1, 2, 3]);
    let empty: Vec<i32> = vec![];
    assert_eq!(identity(empty.clone()), empty);
}

#[test]
fn test_identity_with_custom_type() {
    #[derive(Debug, Clone, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    let point = Point { x: 1, y: 2 };
    assert_eq!(identity(point.clone()), point);
}

#[test]
fn test_identity_preserves_ownership() {
    let owned = String::from("owned string");
    let result = identity(owned);
    assert_eq!(result, "owned string");
}

// =============================================================================
// constant function tests
// =============================================================================

#[test]
fn test_constant_always_returns_same_integer() {
    let always_five = constant(5);
    assert_eq!(always_five(100), 5);
    assert_eq!(always_five(-50), 5);
    assert_eq!(always_five(0), 5);
}

#[test]
fn test_constant_ignores_string_input() {
    let always_five = constant(5);
    assert_eq!(always_five("ignored"), 5);
    assert_eq!(always_five("anything"), 5);
}

#[test]
fn test_constant_ignores_unit_input() {
    let always_five = constant(5);
    assert_eq!(always_five(()), 5);
}

#[test]
fn test_constant_ignores_vector_input() {
    let always_five = constant(5);
    assert_eq!(always_five(vec![1, 2, 3]), 5);
}

#[test]
fn test_constant_with_string() {
    let always_hello = constant(String::from("hello"));
    assert_eq!(always_hello(42), "hello");
    assert_eq!(always_hello(100), "hello");
}

#[test]
fn test_constant_with_map() {
    let values: Vec<i32> = vec![1, 2, 3].into_iter().map(constant(0)).collect();
    assert_eq!(values, vec![0, 0, 0]);
}

#[test]
fn test_constant_can_be_called_multiple_times() {
    let always_ten = constant(10);
    // Call multiple times to verify Clone works correctly
    for _ in 0..100 {
        assert_eq!(always_ten(0), 10);
    }
}

// =============================================================================
// flip function tests
// =============================================================================

#[test]
fn test_flip_swaps_arguments_divide() {
    fn divide(numerator: f64, denominator: f64) -> f64 {
        numerator / denominator
    }

    let flipped_divide = flip(divide);

    // divide(10.0, 2.0) = 5.0
    assert_eq!(divide(10.0, 2.0), 5.0);
    // flipped_divide(10.0, 2.0) = divide(2.0, 10.0) = 0.2
    assert!((flipped_divide(10.0, 2.0) - 0.2).abs() < f64::EPSILON);
}

#[test]
fn test_flip_swaps_arguments_subtract() {
    fn subtract(minuend: i32, subtrahend: i32) -> i32 {
        minuend - subtrahend
    }

    let flipped_subtract = flip(subtract);

    // subtract(10, 3) = 7
    assert_eq!(subtract(10, 3), 7);
    // flipped_subtract(10, 3) = subtract(3, 10) = -7
    assert_eq!(flipped_subtract(10, 3), -7);
}

#[test]
fn test_flip_double_flip_is_identity() {
    fn subtract(minuend: i32, subtrahend: i32) -> i32 {
        minuend - subtrahend
    }

    let flipped_once = flip(subtract);
    let flipped_twice = flip(flipped_once);

    // Double flip should be equivalent to original
    assert_eq!(subtract(10, 3), flipped_twice(10, 3));
    assert_eq!(subtract(5, 8), flipped_twice(5, 8));
}

#[test]
fn test_flip_with_different_argument_types() {
    fn repeat(count: usize, text: &str) -> String {
        text.repeat(count)
    }

    let flipped_repeat = flip(repeat);

    // repeat(3, "ab") = "ababab"
    assert_eq!(repeat(3, "ab"), "ababab");
    // flipped_repeat("ab", 3) = repeat(3, "ab") = "ababab"
    assert_eq!(flipped_repeat("ab", 3), "ababab");
}

#[test]
fn test_flip_with_closure() {
    let concat = |first: &str, second: &str| format!("{}{}", first, second);
    let flipped_concat = flip(concat);

    assert_eq!(concat("hello", "world"), "helloworld");
    assert_eq!(flipped_concat("hello", "world"), "worldhello");
}

// =============================================================================
// compose! macro tests
// =============================================================================

mod compose_macro_tests {
    use lambars::compose;

    #[test]
    fn test_compose_single_function() {
        fn double(value: i32) -> i32 {
            value * 2
        }
        let composed = compose!(double);
        assert_eq!(composed(5), 10);
    }

    #[test]
    fn test_compose_two_functions() {
        fn add_one(value: i32) -> i32 {
            value + 1
        }
        fn double(value: i32) -> i32 {
            value * 2
        }

        // compose!(f, g)(x) = f(g(x)) = add_one(double(5)) = add_one(10) = 11
        let composed = compose!(add_one, double);
        assert_eq!(composed(5), 11);
    }

    #[test]
    fn test_compose_three_functions() {
        fn add_one(value: i32) -> i32 {
            value + 1
        }
        fn double(value: i32) -> i32 {
            value * 2
        }
        fn square(value: i32) -> i32 {
            value * value
        }

        // compose!(f, g, h)(x) = f(g(h(x))) = add_one(double(square(3)))
        // = add_one(double(9)) = add_one(18) = 19
        let composed = compose!(add_one, double, square);
        assert_eq!(composed(3), 19);
    }

    #[test]
    fn test_compose_four_functions() {
        let add_one = |value: i32| value + 1;
        let double = |value: i32| value * 2;
        let square = |value: i32| value * value;
        let negate = |value: i32| -value;

        // negate(add_one(double(square(2)))) = negate(add_one(double(4)))
        // = negate(add_one(8)) = negate(9) = -9
        let composed = compose!(negate, add_one, double, square);
        assert_eq!(composed(2), -9);
    }

    #[test]
    fn test_compose_five_functions() {
        let f1 = |x: i32| x + 1;
        let f2 = |x: i32| x * 2;
        let f3 = |x: i32| x - 3;
        let f4 = |x: i32| x * x;
        let f5 = |x: i32| x + 10;

        // f1(f2(f3(f4(f5(1))))) = f1(f2(f3(f4(11)))) = f1(f2(f3(121)))
        // = f1(f2(118)) = f1(236) = 237
        let composed = compose!(f1, f2, f3, f4, f5);
        assert_eq!(composed(1), 237);
    }

    #[test]
    fn test_compose_immediate_application() {
        fn add_one(value: i32) -> i32 {
            value + 1
        }
        fn double(value: i32) -> i32 {
            value * 2
        }

        let result = compose!(add_one, double)(5);
        assert_eq!(result, 11);
    }

    #[test]
    fn test_compose_with_type_conversion() {
        fn to_string(value: i32) -> String {
            value.to_string()
        }
        fn get_length(text: String) -> usize {
            text.len()
        }

        let composed = compose!(get_length, to_string);
        assert_eq!(composed(12345), 5);
        assert_eq!(composed(1), 1);
        assert_eq!(composed(1000000), 7);
    }

    #[test]
    fn test_compose_with_closures_capturing_environment() {
        let multiplier = 3;
        let multiply = |value: i32| value * multiplier;
        let add_ten = |value: i32| value + 10;

        let composed = compose!(add_ten, multiply);
        // add_ten(multiply(5)) = add_ten(15) = 25
        assert_eq!(composed(5), 25);
    }

    #[test]
    fn test_compose_with_trailing_comma() {
        fn add_one(value: i32) -> i32 {
            value + 1
        }
        fn double(value: i32) -> i32 {
            value * 2
        }

        // Should accept trailing comma
        let composed = compose!(add_one, double,);
        assert_eq!(composed(5), 11);
    }

    #[test]
    fn test_compose_result_can_be_reused() {
        fn add_one(value: i32) -> i32 {
            value + 1
        }
        fn double(value: i32) -> i32 {
            value * 2
        }

        let composed = compose!(add_one, double);
        // Can call multiple times
        assert_eq!(composed(1), 3);
        assert_eq!(composed(2), 5);
        assert_eq!(composed(3), 7);
    }
}
