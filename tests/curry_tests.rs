//! Integration tests for the curry! macro.
//!
//! Tests for converting multi-argument closures or functions to curried form.
//!
//! # Supported Input Forms
//!
//! 1. Closure form: `curry!(|a, b| body)`
//! 2. Function name + arity form: `curry!(function_name, arity)`
//!
//! Note: The `redundant_closure` lint is intentionally allowed because
//! the curry! macro also accepts closure expressions.

#![cfg(feature = "compose")]
#![allow(unused_imports)]
#![allow(clippy::redundant_closure)]

use rstest::rstest;

// =============================================================================
// Test helper functions
// =============================================================================

fn add(first: i32, second: i32) -> i32 {
    first + second
}

fn multiply(first: i32, second: i32) -> i32 {
    first * second
}

fn add_three(first: i32, second: i32, third: i32) -> i32 {
    first + second + third
}

fn sum_four(a: i32, b: i32, c: i32, d: i32) -> i32 {
    a + b + c + d
}

fn sum_five(a: i32, b: i32, c: i32, d: i32, e: i32) -> i32 {
    a + b + c + d + e
}

fn sum_six(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32 {
    a + b + c + d + e + f
}

fn concat(first: String, second: String) -> String {
    format!("{}{}", first, second)
}

// =============================================================================
// 2-argument closure tests
// =============================================================================

/// 2-argument closure basic currying test
#[rstest]
#[case(1, 2, 3)]
#[case(0, 0, 0)]
#[case(-5, 10, 5)]
#[case(100, -50, 50)]
fn test_curry_two_arguments_basic(#[case] first: i32, #[case] second: i32, #[case] expected: i32) {
    let curried = lambars::curry!(|a, b| add(a, b));
    assert_eq!(curried(first)(second), expected);
}

/// 2-argument closure partial application test
#[rstest]
fn test_curry_two_arguments_partial_application() {
    let curried = lambars::curry!(|first, second| multiply(first, second));
    let double = curried(2);

    // double can be reused multiple times
    assert_eq!(double(5), 10);
    assert_eq!(double(10), 20);
    assert_eq!(double(0), 0);

    // Create another partial application
    let triple = curried(3);
    assert_eq!(triple(5), 15);

    // double is not affected
    assert_eq!(double(5), 10);
}

/// Inline closure currying test
#[rstest]
fn test_curry_with_inline_closure() {
    let curried = lambars::curry!(|first: i32, second: i32| first + second);
    assert_eq!(curried(10)(20), 30);
}

// =============================================================================
// 3-argument closure tests
// =============================================================================

/// 3-argument closure basic currying test
#[rstest]
fn test_curry_three_arguments_basic() {
    let curried = lambars::curry!(|a, b, c| add_three(a, b, c));
    assert_eq!(curried(1)(2)(3), 6);
}

/// 3-argument closure step-by-step application test
#[rstest]
fn test_curry_three_arguments_step_by_step() {
    let curried = lambars::curry!(|a, b, c| add_three(a, b, c));
    let with_first = curried(10);
    let with_first_second = with_first(20);
    let result = with_first_second(30);

    assert_eq!(result, 60);
}

/// 3-argument closure inline test
#[rstest]
fn test_curry_three_arguments_inline() {
    let curried = lambars::curry!(|a: i32, b: i32, c: i32| a * b + c);
    assert_eq!(curried(2)(3)(4), 10);
}

// =============================================================================
// 4-argument closure tests
// =============================================================================

/// 4-argument closure basic currying test
#[rstest]
fn test_curry_four_arguments_basic() {
    let curried = lambars::curry!(|a, b, c, d| sum_four(a, b, c, d));
    assert_eq!(curried(1)(2)(3)(4), 10);
}

/// 4-argument closure step-by-step application test
#[rstest]
fn test_curry_four_arguments_step_by_step() {
    let curried = lambars::curry!(|a, b, c, d| sum_four(a, b, c, d));
    let step1 = curried(100);
    let step2 = step1(200);
    let step3 = step2(300);
    let result = step3(400);

    assert_eq!(result, 1000);
}

// =============================================================================
// 5-argument closure tests
// =============================================================================

/// 5-argument closure basic currying test
#[rstest]
fn test_curry_five_arguments_basic() {
    let curried = lambars::curry!(|a, b, c, d, e| sum_five(a, b, c, d, e));
    assert_eq!(curried(1)(2)(3)(4)(5), 15);
}

/// 5-argument closure step-by-step application test
#[rstest]
fn test_curry_five_arguments_step_by_step() {
    let curried = lambars::curry!(|a, b, c, d, e| sum_five(a, b, c, d, e));
    let step1 = curried(10);
    let step2 = step1(20);
    let step3 = step2(30);
    let step4 = step3(40);
    let result = step4(50);

    assert_eq!(result, 150);
}

// =============================================================================
// 6-argument closure tests
// =============================================================================

/// 6-argument closure basic currying test
#[rstest]
fn test_curry_six_arguments_basic() {
    let curried = lambars::curry!(|a, b, c, d, e, f| sum_six(a, b, c, d, e, f));
    assert_eq!(curried(1)(2)(3)(4)(5)(6), 21);
}

/// 6-argument closure step-by-step application test
#[rstest]
fn test_curry_six_arguments_step_by_step() {
    let curried = lambars::curry!(|a, b, c, d, e, f| sum_six(a, b, c, d, e, f));
    let step1 = curried(1);
    let step2 = step1(2);
    let step3 = step2(3);
    let step4 = step3(4);
    let step5 = step4(5);
    let result = step5(6);

    assert_eq!(result, 21);
}

// =============================================================================
// Referential transparency tests
// =============================================================================

/// Referential transparency test - same arguments should always return same result
#[rstest]
fn test_referential_transparency() {
    let curried = lambars::curry!(|first, second| add(first, second));

    // Same arguments should always return same result
    let result1 = curried(5)(3);
    let result2 = curried(5)(3);
    let result3 = curried(5)(3);

    assert_eq!(result1, result2);
    assert_eq!(result2, result3);
}

// =============================================================================
// Clone type tests
// =============================================================================

/// Test with Clone types (String)
#[rstest]
fn test_curry_with_clone_types() {
    let curried = lambars::curry!(|first, second| concat(first, second));
    let hello = curried(String::from("Hello, "));

    // hello can be called multiple times
    assert_eq!(hello(String::from("World")), "Hello, World");
    assert_eq!(hello(String::from("Rust")), "Hello, Rust");
}

/// Test with non-Clone type as last argument
#[rstest]
fn test_curry_with_non_clone_last_argument() {
    struct NonClone(i32);

    let curried = lambars::curry!(|a: i32, b: NonClone| a + b.0);
    assert_eq!(curried(5)(NonClone(3)), 8);
}

// =============================================================================
// Integration with compose! and pipe!
// =============================================================================

/// Test curry! with compose!
#[rstest]
fn test_curry_with_compose() {
    let double = lambars::curry!(|a, b| multiply(a, b))(2);
    let add_ten = lambars::curry!(|a, b| add(a, b))(10);

    let double_then_add_ten = lambars::compose!(add_ten, double);
    assert_eq!(double_then_add_ten(5), 20);
}

/// Test curry! with pipe!
#[rstest]
fn test_curry_with_pipe() {
    let double = lambars::curry!(|a, b| multiply(a, b))(2);
    let add_ten = lambars::curry!(|a, b| add(a, b))(10);

    let result = lambars::pipe!(5, double, add_ten);
    assert_eq!(result, 20);
}

/// Test curry! with compose! multiple stages
#[rstest]
fn test_curry_with_compose_multiple_stages() {
    let curried = lambars::curry!(|a: i32, b: i32, c: i32| a + b + c);
    let partial1 = curried(1);
    let partial2 = partial1(2);

    // partial2 is a single-argument function, can be used with compose
    let increment = |x: i32| x + 1;
    let composed = lambars::compose!(partial2, increment);

    // increment(10) = 11, then 1 + 2 + 11 = 14
    assert_eq!(composed(10), 14);
}

// =============================================================================
// Edge cases
// =============================================================================

/// Test with unit return type
#[rstest]
fn test_curry_with_unit_return() {
    let curried = lambars::curry!(|first: i32, second: i32| {
        let _ = first + second;
    });
    curried(5)(3);
    // Should compile and run without errors
}

/// Test with closures capturing environment
#[rstest]
fn test_curry_with_capturing_closure() {
    let offset = 100;
    let curried = lambars::curry!(|a: i32, b: i32| a + b + offset);

    assert_eq!(curried(5)(3), 108);
}

/// Test reusability with many calls
#[rstest]
fn test_curry_reusable_many_calls() {
    let curried = lambars::curry!(|first, second| add(first, second));
    let add_five = curried(5);

    // The partial function should be reusable for many calls
    (0..100).for_each(|value| {
        assert_eq!(add_five(value), 5 + value);
    });
}

// =============================================================================
// Function name + arity form tests (v1.3.0)
// =============================================================================

// -----------------------------------------------------------------------------
// 2-argument function tests
// -----------------------------------------------------------------------------

/// 2-argument function basic currying test (function name + arity form)
#[rstest]
#[case(1, 2, 3)]
#[case(0, 0, 0)]
#[case(-5, 10, 5)]
#[case(100, -50, 50)]
fn test_curry_function_two_arguments_basic(
    #[case] first: i32,
    #[case] second: i32,
    #[case] expected: i32,
) {
    let curried = lambars::curry!(add, 2);
    assert_eq!(curried(first)(second), expected);
}

/// 2-argument function partial application test
#[rstest]
fn test_curry_function_two_arguments_partial_application() {
    let curried = lambars::curry!(multiply, 2);
    let double = curried(2);

    assert_eq!(double(5), 10);
    assert_eq!(double(10), 20);
    assert_eq!(double(0), 0);

    let triple = curried(3);
    assert_eq!(triple(5), 15);

    assert_eq!(double(5), 10);
}

// -----------------------------------------------------------------------------
// 3-argument function tests
// -----------------------------------------------------------------------------

/// 3-argument function basic currying test
#[rstest]
fn test_curry_function_three_arguments_basic() {
    let curried = lambars::curry!(add_three, 3);
    assert_eq!(curried(1)(2)(3), 6);
}

/// 3-argument function step-by-step application test
#[rstest]
fn test_curry_function_three_arguments_step_by_step() {
    let curried = lambars::curry!(add_three, 3);
    let with_first = curried(10);
    let with_first_second = with_first(20);
    let result = with_first_second(30);

    assert_eq!(result, 60);
}

// -----------------------------------------------------------------------------
// 4-argument function tests
// -----------------------------------------------------------------------------

/// 4-argument function basic currying test
#[rstest]
fn test_curry_function_four_arguments_basic() {
    let curried = lambars::curry!(sum_four, 4);
    assert_eq!(curried(1)(2)(3)(4), 10);
}

/// 4-argument function step-by-step application test
#[rstest]
fn test_curry_function_four_arguments_step_by_step() {
    let curried = lambars::curry!(sum_four, 4);
    let step1 = curried(100);
    let step2 = step1(200);
    let step3 = step2(300);
    let result = step3(400);

    assert_eq!(result, 1000);
}

// -----------------------------------------------------------------------------
// 5-argument function tests
// -----------------------------------------------------------------------------

/// 5-argument function basic currying test
#[rstest]
fn test_curry_function_five_arguments_basic() {
    let curried = lambars::curry!(sum_five, 5);
    assert_eq!(curried(1)(2)(3)(4)(5), 15);
}

/// 5-argument function step-by-step application test
#[rstest]
fn test_curry_function_five_arguments_step_by_step() {
    let curried = lambars::curry!(sum_five, 5);
    let step1 = curried(10);
    let step2 = step1(20);
    let step3 = step2(30);
    let step4 = step3(40);
    let result = step4(50);

    assert_eq!(result, 150);
}

// -----------------------------------------------------------------------------
// 6-argument function tests
// -----------------------------------------------------------------------------

/// 6-argument function basic currying test
#[rstest]
fn test_curry_function_six_arguments_basic() {
    let curried = lambars::curry!(sum_six, 6);
    assert_eq!(curried(1)(2)(3)(4)(5)(6), 21);
}

/// 6-argument function step-by-step application test
#[rstest]
fn test_curry_function_six_arguments_step_by_step() {
    let curried = lambars::curry!(sum_six, 6);
    let step1 = curried(1);
    let step2 = step1(2);
    let step3 = step2(3);
    let step4 = step3(4);
    let step5 = step4(5);
    let result = step5(6);

    assert_eq!(result, 21);
}

// -----------------------------------------------------------------------------
// Module path tests
// -----------------------------------------------------------------------------

mod math {
    pub fn multiply(first: i32, second: i32) -> i32 {
        first * second
    }

    pub fn add_three(first: i32, second: i32, third: i32) -> i32 {
        first + second + third
    }
}

/// Module path function with 2 arguments
#[rstest]
fn test_curry_function_with_module_path_two_arguments() {
    let curried = lambars::curry!(math::multiply, 2);
    assert_eq!(curried(3)(4), 12);
}

/// Module path function with 3 arguments
#[rstest]
fn test_curry_function_with_module_path_three_arguments() {
    let curried = lambars::curry!(math::add_three, 3);
    assert_eq!(curried(1)(2)(3), 6);
}

// -----------------------------------------------------------------------------
// Type::method tests
// -----------------------------------------------------------------------------

struct Calculator;

impl Calculator {
    fn add(first: i32, second: i32) -> i32 {
        first + second
    }

    fn multiply_three(first: i32, second: i32, third: i32) -> i32 {
        first * second * third
    }
}

/// Type::method with 2 arguments
#[rstest]
fn test_curry_function_with_type_method_two_arguments() {
    let curried = lambars::curry!(Calculator::add, 2);
    assert_eq!(curried(5)(3), 8);
}

/// Type::method with 3 arguments
#[rstest]
fn test_curry_function_with_type_method_three_arguments() {
    let curried = lambars::curry!(Calculator::multiply_three, 3);
    assert_eq!(curried(2)(3)(4), 24);
}

// =============================================================================
// Form equivalence tests
// =============================================================================

/// Both forms should produce equivalent results (2 arguments)
#[rstest]
#[case(1, 2)]
#[case(0, 0)]
#[case(-5, 10)]
fn test_form_equivalence_two_arguments(#[case] first: i32, #[case] second: i32) {
    let curried_closure = lambars::curry!(|a, b| add(a, b));
    let curried_function = lambars::curry!(add, 2);

    assert_eq!(
        curried_closure(first)(second),
        curried_function(first)(second)
    );
}

/// Both forms should produce equivalent results (3 arguments)
#[rstest]
fn test_form_equivalence_three_arguments() {
    let curried_closure = lambars::curry!(|a, b, c| add_three(a, b, c));
    let curried_function = lambars::curry!(add_three, 3);

    assert_eq!(curried_closure(1)(2)(3), curried_function(1)(2)(3));
}

/// Both forms should produce equivalent results (6 arguments)
#[rstest]
fn test_form_equivalence_six_arguments() {
    let curried_closure = lambars::curry!(|a, b, c, d, e, f| sum_six(a, b, c, d, e, f));
    let curried_function = lambars::curry!(sum_six, 6);

    assert_eq!(
        curried_closure(1)(2)(3)(4)(5)(6),
        curried_function(1)(2)(3)(4)(5)(6)
    );
}

// =============================================================================
// Partial application reusability tests (function form)
// =============================================================================

/// Partial application reusability test (function form)
#[rstest]
fn test_partial_application_reusability_function() {
    let curried = lambars::curry!(multiply, 2);
    let double = curried(2);

    assert_eq!(double(5), 10);
    assert_eq!(double(10), 20);
    assert_eq!(double(0), 0);

    let triple = curried(3);
    assert_eq!(triple(5), 15);

    assert_eq!(double(5), 10);
}

// =============================================================================
// Referential transparency tests (function form)
// =============================================================================

/// Referential transparency test (function form)
#[rstest]
fn test_referential_transparency_function() {
    let curried = lambars::curry!(add, 2);

    let result1 = curried(5)(3);
    let result2 = curried(5)(3);
    let result3 = curried(5)(3);

    assert_eq!(result1, result2);
    assert_eq!(result2, result3);
}

// =============================================================================
// Clone type tests (function form)
// =============================================================================

/// Test with Clone types (function form)
#[rstest]
fn test_curry_function_with_clone_types() {
    let curried = lambars::curry!(concat, 2);
    let hello = curried(String::from("Hello, "));

    assert_eq!(hello(String::from("World")), "Hello, World");
    assert_eq!(hello(String::from("Rust")), "Hello, Rust");
}

/// Test with non-Clone type as last argument (function form)
#[rstest]
fn test_curry_function_with_non_clone_last_argument() {
    struct NonClone(i32);

    fn add_non_clone(first: i32, second: NonClone) -> i32 {
        first + second.0
    }

    let curried = lambars::curry!(add_non_clone, 2);
    assert_eq!(curried(5)(NonClone(3)), 8);
}

// =============================================================================
// Integration with compose! and pipe! (function form)
// =============================================================================

/// Test curry! (function form) with compose!
#[rstest]
fn test_curry_function_with_compose() {
    let double = lambars::curry!(multiply, 2)(2);
    let add_ten = lambars::curry!(add, 2)(10);

    let double_then_add_ten = lambars::compose!(add_ten, double);
    assert_eq!(double_then_add_ten(5), 20);
}

/// Test curry! (function form) with pipe!
#[rstest]
fn test_curry_function_with_pipe() {
    let double = lambars::curry!(multiply, 2)(2);
    let add_ten = lambars::curry!(add, 2)(10);

    let result = lambars::pipe!(5, double, add_ten);
    assert_eq!(result, 20);
}

/// Test mixed forms with compose!
#[rstest]
fn test_curry_mixed_forms_with_compose() {
    let double = lambars::curry!(multiply, 2)(2);
    let add_ten = lambars::curry!(|a, b| add(a, b))(10);

    let double_then_add_ten = lambars::compose!(add_ten, double);
    assert_eq!(double_then_add_ten(5), 20);
}
