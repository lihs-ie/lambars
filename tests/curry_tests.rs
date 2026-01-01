//! Unit tests for the curry! macro family.
//!
//! Tests for converting multi-argument functions to curried form.

#![cfg(feature = "compose")]
#![allow(unused_imports)]

use lambars::{curry2, curry3, curry4, curry5, curry6};

// =============================================================================
// curry2! tests (2-argument functions)
// =============================================================================

mod curry2_tests {
    use lambars::curry2;

    fn add(first: i32, second: i32) -> i32 {
        first + second
    }

    fn divide(numerator: f64, denominator: f64) -> f64 {
        numerator / denominator
    }

    fn concat(first: &str, second: &str) -> String {
        format!("{}{}", first, second)
    }

    #[test]
    fn test_curry2_basic() {
        let curried_add = curry2!(add);
        assert_eq!(curried_add(5)(3), 8);
    }

    #[test]
    fn test_curry2_partial_application() {
        let curried_add = curry2!(add);
        let add_five = curried_add(5);

        assert_eq!(add_five(3), 8);
        assert_eq!(add_five(10), 15);
        assert_eq!(add_five(-5), 0);
    }

    #[test]
    fn test_curry2_with_floats() {
        let curried_divide = curry2!(divide);
        let divide_ten_by = curried_divide(10.0);

        assert!((divide_ten_by(2.0) - 5.0).abs() < f64::EPSILON);
        assert!((divide_ten_by(5.0) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_curry2_with_references() {
        let curried_concat = curry2!(concat);
        let hello = curried_concat("Hello, ");

        assert_eq!(hello("World"), "Hello, World");
        assert_eq!(hello("Rust"), "Hello, Rust");
    }

    #[test]
    fn test_curry2_with_closure() {
        let multiply = |first: i32, second: i32| first * second;
        let curried_multiply = curry2!(multiply);
        let double = curried_multiply(2);

        assert_eq!(double(5), 10);
        assert_eq!(double(100), 200);
    }

    #[test]
    fn test_curry2_reusable() {
        let curried_add = curry2!(add);
        let add_five = curried_add(5);

        // The partial function should be reusable
        for i in 0..100 {
            assert_eq!(add_five(i), 5 + i);
        }
    }
}

// =============================================================================
// curry3! tests (3-argument functions)
// =============================================================================

mod curry3_tests {
    use lambars::curry3;

    fn add_three(first: i32, second: i32, third: i32) -> i32 {
        first + second + third
    }

    fn volume(width: f64, height: f64, depth: f64) -> f64 {
        width * height * depth
    }

    #[test]
    fn test_curry3_basic() {
        let curried = curry3!(add_three);
        assert_eq!(curried(1)(2)(3), 6);
    }

    #[test]
    fn test_curry3_step_by_step() {
        let curried = curry3!(add_three);
        let with_first = curried(10);
        let with_first_second = with_first(20);
        let result = with_first_second(30);

        assert_eq!(result, 60);
    }

    #[test]
    fn test_curry3_volume() {
        let curried_volume = curry3!(volume);
        let with_width = curried_volume(2.0);
        let with_width_height = with_width(3.0);
        let result = with_width_height(4.0);

        assert!((result - 24.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_curry3_partial_reusable() {
        let curried = curry3!(add_three);
        let with_first = curried(100);

        // with_first can be reused
        assert_eq!(with_first(1)(2), 103);
        assert_eq!(with_first(10)(20), 130);
    }
}

// =============================================================================
// curry4! tests (4-argument functions)
// =============================================================================

mod curry4_tests {
    use lambars::curry4;

    fn sum_four(a: i32, b: i32, c: i32, d: i32) -> i32 {
        a + b + c + d
    }

    #[test]
    fn test_curry4_basic() {
        let curried = curry4!(sum_four);
        assert_eq!(curried(1)(2)(3)(4), 10);
    }

    #[test]
    fn test_curry4_step_by_step() {
        let curried = curry4!(sum_four);
        let step1 = curried(100);
        let step2 = step1(200);
        let step3 = step2(300);
        let result = step3(400);

        assert_eq!(result, 1000);
    }

    #[test]
    fn test_curry4_partial_reusable() {
        let curried = curry4!(sum_four);
        let with_first = curried(1);
        let with_first_second = with_first(2);

        assert_eq!(with_first_second(3)(4), 10);
        assert_eq!(with_first_second(30)(40), 73);
    }
}

// =============================================================================
// curry5! tests (5-argument functions)
// =============================================================================

mod curry5_tests {
    use lambars::curry5;

    fn sum_five(a: i32, b: i32, c: i32, d: i32, e: i32) -> i32 {
        a + b + c + d + e
    }

    #[test]
    fn test_curry5_basic() {
        let curried = curry5!(sum_five);
        assert_eq!(curried(1)(2)(3)(4)(5), 15);
    }

    #[test]
    fn test_curry5_step_by_step() {
        let curried = curry5!(sum_five);
        let s1 = curried(10);
        let s2 = s1(20);
        let s3 = s2(30);
        let s4 = s3(40);
        let result = s4(50);

        assert_eq!(result, 150);
    }
}

// =============================================================================
// curry6! tests (6-argument functions)
// =============================================================================

mod curry6_tests {
    use lambars::curry6;

    fn sum_six(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32 {
        a + b + c + d + e + f
    }

    #[test]
    fn test_curry6_basic() {
        let curried = curry6!(sum_six);
        assert_eq!(curried(1)(2)(3)(4)(5)(6), 21);
    }

    #[test]
    fn test_curry6_step_by_step() {
        let curried = curry6!(sum_six);
        let s1 = curried(1);
        let s2 = s1(2);
        let s3 = s2(3);
        let s4 = s3(4);
        let s5 = s4(5);
        let result = s5(6);

        assert_eq!(result, 21);
    }
}

// =============================================================================
// Integration with compose! and pipe!
// =============================================================================

mod integration {
    use lambars::{compose, curry2, pipe};

    fn multiply(first: i32, second: i32) -> i32 {
        first * second
    }

    fn add(first: i32, second: i32) -> i32 {
        first + second
    }

    #[test]
    fn test_curry_with_compose() {
        let double = curry2!(multiply)(2);
        let add_ten = curry2!(add)(10);

        let double_then_add_ten = compose!(add_ten, double);
        // double(5) = 10, add_ten(10) = 20
        assert_eq!(double_then_add_ten(5), 20);
    }

    #[test]
    fn test_curry_with_pipe() {
        let double = curry2!(multiply)(2);
        let add_ten = curry2!(add)(10);

        let result = pipe!(5, double, add_ten);
        // double(5) = 10, add_ten(10) = 20
        assert_eq!(result, 20);
    }

    #[test]
    fn test_curry_compose_multiple() {
        let double = curry2!(multiply)(2);
        let triple = curry2!(multiply)(3);
        let add_one = curry2!(add)(1);

        // compose!(add_one, triple, double)(x) = add_one(triple(double(x)))
        let six_times_plus_one = compose!(add_one, triple, double);
        // double(5) = 10, triple(10) = 30, add_one(30) = 31
        assert_eq!(six_times_plus_one(5), 31);
    }
}

// =============================================================================
// Edge cases
// =============================================================================

mod edge_cases {
    use lambars::curry2;

    #[test]
    fn test_curry_with_clone_type() {
        fn repeat_string(text: String, count: usize) -> String {
            text.repeat(count)
        }

        let curried = curry2!(repeat_string);
        let repeat_hello = curried(String::from("hello"));

        assert_eq!(repeat_hello(3), "hellohellohello");
        // Can call multiple times because String is Clone
        assert_eq!(repeat_hello(2), "hellohello");
    }

    #[test]
    fn test_curry_with_unit_return() {
        fn print_sum(first: i32, second: i32) {
            let _ = first + second;
        }

        let curried = curry2!(print_sum);
        curried(5)(3);
        // Should compile and run without errors
    }
}
