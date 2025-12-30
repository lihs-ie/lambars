//! Unit tests for the partial! macro.
//!
//! Tests for partial function application with placeholder support.
//!
//! Note: The `__` placeholder is a literal token in the macro pattern.
//! Do NOT import `functional_rusty::compose::__` as it will shadow the literal.

#![cfg(feature = "compose")]
#![allow(unused_imports)]

use functional_rusty::partial;

// =============================================================================
// 2-argument function tests
// =============================================================================

mod two_argument_functions {
    use functional_rusty::partial;

    fn add(first: i32, second: i32) -> i32 {
        first + second
    }

    fn divide(numerator: f64, denominator: f64) -> f64 {
        numerator / denominator
    }

    fn subtract(minuend: i32, subtrahend: i32) -> i32 {
        minuend - subtrahend
    }

    #[test]
    fn test_partial_first_argument_fixed() {
        let add_five = partial!(add, 5, __);
        assert_eq!(add_five(3), 8);
        assert_eq!(add_five(10), 15);
        assert_eq!(add_five(-5), 0);
    }

    #[test]
    fn test_partial_second_argument_fixed() {
        let add_ten = partial!(add, __, 10);
        assert_eq!(add_ten(5), 15);
        assert_eq!(add_ten(-10), 0);
    }

    #[test]
    fn test_partial_both_arguments_fixed() {
        let thunk = partial!(add, 3, 5);
        assert_eq!(thunk(), 8);
    }

    #[test]
    fn test_partial_no_arguments_fixed() {
        let same_as_add = partial!(add, __, __);
        assert_eq!(same_as_add(3, 5), 8);
        assert_eq!(same_as_add(10, 20), 30);
    }

    #[test]
    fn test_partial_divide_numerator_fixed() {
        let divide_ten_by = partial!(divide, 10.0, __);
        assert!((divide_ten_by(2.0) - 5.0).abs() < f64::EPSILON);
        assert!((divide_ten_by(5.0) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_partial_divide_denominator_fixed() {
        let half = partial!(divide, __, 2.0);
        assert!((half(10.0) - 5.0).abs() < f64::EPSILON);
        assert!((half(7.0) - 3.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_partial_subtract_minuend_fixed() {
        let subtract_from_ten = partial!(subtract, 10, __);
        assert_eq!(subtract_from_ten(3), 7);
        assert_eq!(subtract_from_ten(15), -5);
    }

    #[test]
    fn test_partial_subtract_subtrahend_fixed() {
        let subtract_five = partial!(subtract, __, 5);
        assert_eq!(subtract_five(10), 5);
        assert_eq!(subtract_five(3), -2);
    }

    #[test]
    fn test_partial_can_be_called_multiple_times() {
        let add_five = partial!(add, 5, __);
        // The partial function should be reusable
        for i in 0..100 {
            assert_eq!(add_five(i), 5 + i);
        }
    }
}

// =============================================================================
// 3-argument function tests
// =============================================================================

mod three_argument_functions {
    use functional_rusty::partial;

    fn add_three(first: i32, second: i32, third: i32) -> i32 {
        first + second + third
    }

    fn format_greeting(greeting: &str, name: &str, punctuation: &str) -> String {
        format!("{}, {}{}", greeting, name, punctuation)
    }

    #[test]
    fn test_partial_first_fixed() {
        let add_with_ten = partial!(add_three, 10, __, __);
        assert_eq!(add_with_ten(2, 3), 15);
        assert_eq!(add_with_ten(0, 0), 10);
    }

    #[test]
    fn test_partial_second_fixed() {
        let add_with_100 = partial!(add_three, __, 100, __);
        assert_eq!(add_with_100(1, 2), 103);
    }

    #[test]
    fn test_partial_third_fixed() {
        let add_with_1000 = partial!(add_three, __, __, 1000);
        assert_eq!(add_with_1000(1, 2), 1003);
    }

    #[test]
    fn test_partial_first_and_second_fixed() {
        let add_to_15 = partial!(add_three, 10, 5, __);
        assert_eq!(add_to_15(3), 18);
    }

    #[test]
    fn test_partial_first_and_third_fixed() {
        let add_with_10_and_1000 = partial!(add_three, 10, __, 1000);
        assert_eq!(add_with_10_and_1000(5), 1015);
    }

    #[test]
    fn test_partial_second_and_third_fixed() {
        let add_5_and_1000 = partial!(add_three, __, 5, 1000);
        assert_eq!(add_5_and_1000(10), 1015);
    }

    #[test]
    fn test_partial_all_fixed() {
        let thunk = partial!(add_three, 1, 2, 3);
        assert_eq!(thunk(), 6);
    }

    #[test]
    fn test_partial_none_fixed() {
        let same_as_add_three = partial!(add_three, __, __, __);
        assert_eq!(same_as_add_three(1, 2, 3), 6);
    }

    #[test]
    fn test_partial_greeting_first_and_third_fixed() {
        let hello_with_exclamation = partial!(format_greeting, "Hello", __, "!");
        assert_eq!(hello_with_exclamation("Alice"), "Hello, Alice!");
        assert_eq!(hello_with_exclamation("Bob"), "Hello, Bob!");
    }

    #[test]
    fn test_partial_greeting_second_fixed() {
        let greet_world = partial!(format_greeting, __, "World", __);
        assert_eq!(greet_world("Hello", "!"), "Hello, World!");
        assert_eq!(greet_world("Goodbye", "."), "Goodbye, World.");
    }
}

// =============================================================================
// 4-argument function tests
// =============================================================================

mod four_argument_functions {
    use functional_rusty::partial;

    fn sum_four(first: i32, second: i32, third: i32, fourth: i32) -> i32 {
        first + second + third + fourth
    }

    #[test]
    fn test_partial_first_fixed() {
        let with_first = partial!(sum_four, 1, __, __, __);
        assert_eq!(with_first(2, 3, 4), 10);
    }

    #[test]
    fn test_partial_second_fixed() {
        let with_second = partial!(sum_four, __, 2, __, __);
        assert_eq!(with_second(1, 3, 4), 10);
    }

    #[test]
    fn test_partial_third_fixed() {
        let with_third = partial!(sum_four, __, __, 3, __);
        assert_eq!(with_third(1, 2, 4), 10);
    }

    #[test]
    fn test_partial_fourth_fixed() {
        let with_fourth = partial!(sum_four, __, __, __, 4);
        assert_eq!(with_fourth(1, 2, 3), 10);
    }

    #[test]
    fn test_partial_first_and_third_fixed() {
        let with_first_and_third = partial!(sum_four, 1, __, 3, __);
        assert_eq!(with_first_and_third(2, 4), 10);
    }

    #[test]
    fn test_partial_second_and_fourth_fixed() {
        let with_second_and_fourth = partial!(sum_four, __, 2, __, 4);
        assert_eq!(with_second_and_fourth(1, 3), 10);
    }

    #[test]
    fn test_partial_first_three_fixed() {
        let with_first_three = partial!(sum_four, 1, 2, 3, __);
        assert_eq!(with_first_three(4), 10);
    }

    #[test]
    fn test_partial_last_three_fixed() {
        let with_last_three = partial!(sum_four, __, 2, 3, 4);
        assert_eq!(with_last_three(1), 10);
    }

    #[test]
    fn test_partial_all_fixed() {
        let thunk = partial!(sum_four, 1, 2, 3, 4);
        assert_eq!(thunk(), 10);
    }

    #[test]
    fn test_partial_none_fixed() {
        let same = partial!(sum_four, __, __, __, __);
        assert_eq!(same(1, 2, 3, 4), 10);
    }
}

// =============================================================================
// 5-argument function tests
// =============================================================================

mod five_argument_functions {
    use functional_rusty::partial;

    fn sum_five(a: i32, b: i32, c: i32, d: i32, e: i32) -> i32 {
        a + b + c + d + e
    }

    #[test]
    fn test_partial_first_fixed() {
        let with_first = partial!(sum_five, 1, __, __, __, __);
        assert_eq!(with_first(2, 3, 4, 5), 15);
    }

    #[test]
    fn test_partial_middle_fixed() {
        let with_middle = partial!(sum_five, __, __, 3, __, __);
        assert_eq!(with_middle(1, 2, 4, 5), 15);
    }

    #[test]
    fn test_partial_last_fixed() {
        let with_last = partial!(sum_five, __, __, __, __, 5);
        assert_eq!(with_last(1, 2, 3, 4), 15);
    }

    #[test]
    fn test_partial_all_fixed() {
        let thunk = partial!(sum_five, 1, 2, 3, 4, 5);
        assert_eq!(thunk(), 15);
    }

    #[test]
    fn test_partial_none_fixed() {
        let same = partial!(sum_five, __, __, __, __, __);
        assert_eq!(same(1, 2, 3, 4, 5), 15);
    }
}

// =============================================================================
// 6-argument function tests
// =============================================================================

mod six_argument_functions {
    use functional_rusty::partial;

    fn sum_six(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32 {
        a + b + c + d + e + f
    }

    #[test]
    fn test_partial_first_fixed() {
        let with_first = partial!(sum_six, 1, __, __, __, __, __);
        assert_eq!(with_first(2, 3, 4, 5, 6), 21);
    }

    #[test]
    fn test_partial_last_fixed() {
        let with_last = partial!(sum_six, __, __, __, __, __, 6);
        assert_eq!(with_last(1, 2, 3, 4, 5), 21);
    }

    #[test]
    fn test_partial_all_fixed() {
        let thunk = partial!(sum_six, 1, 2, 3, 4, 5, 6);
        assert_eq!(thunk(), 21);
    }

    #[test]
    fn test_partial_none_fixed() {
        let same = partial!(sum_six, __, __, __, __, __, __);
        assert_eq!(same(1, 2, 3, 4, 5, 6), 21);
    }
}

// =============================================================================
// Integration with compose! and pipe!
// =============================================================================

mod integration {
    use functional_rusty::{compose, partial, pipe};

    fn multiply(first: i32, second: i32) -> i32 {
        first * second
    }

    fn add(first: i32, second: i32) -> i32 {
        first + second
    }

    #[test]
    fn test_partial_with_compose() {
        let double = partial!(multiply, 2, __);
        let add_ten = partial!(add, 10, __);

        let double_then_add_ten = compose!(add_ten, double);
        // double(5) = 10, add_ten(10) = 20
        assert_eq!(double_then_add_ten(5), 20);
    }

    #[test]
    fn test_partial_with_pipe() {
        let double = partial!(multiply, 2, __);
        let add_ten = partial!(add, 10, __);

        let result = pipe!(5, double, add_ten);
        // double(5) = 10, add_ten(10) = 20
        assert_eq!(result, 20);
    }

    #[test]
    fn test_multiple_partial_in_compose() {
        let triple = partial!(multiply, 3, __);
        let add_five = partial!(add, 5, __);
        let subtract_two = partial!(add, -2, __);

        let composed = compose!(subtract_two, add_five, triple);
        // triple(4) = 12, add_five(12) = 17, subtract_two(17) = 15
        assert_eq!(composed(4), 15);
    }
}

// =============================================================================
// Edge cases
// =============================================================================

mod edge_cases {
    use functional_rusty::partial;

    #[test]
    fn test_partial_with_clone_type() {
        fn repeat_string(text: String, count: usize) -> String {
            text.repeat(count)
        }

        let repeat_hello = partial!(repeat_string, String::from("hello"), __);
        assert_eq!(repeat_hello(3), "hellohellohello");
        // Can call multiple times because String is Clone
        assert_eq!(repeat_hello(2), "hellohello");
    }

    #[test]
    fn test_partial_with_closure() {
        let add_closure = |first: i32, second: i32| first + second;
        let add_five = partial!(add_closure, 5, __);
        assert_eq!(add_five(10), 15);
    }
}
