//! Integration tests for Either with for_! macro.

#![cfg(all(feature = "control", feature = "compose"))]

use lambars::control::Either;
use lambars::for_;
use rstest::rstest;

// =============================================================================
// Basic Usage Tests
// =============================================================================

#[rstest]
fn test_for_macro_with_right() {
    let result = for_! {
        x <= Either::<String, i32>::Right(42);
        yield x * 2
    };
    assert_eq!(result, vec![84]);
}

#[rstest]
fn test_for_macro_with_left() {
    let result = for_! {
        x <= Either::<String, i32>::Left("error".to_string());
        yield x * 2
    };
    assert_eq!(result, Vec::<i32>::new());
}

// =============================================================================
// Nested Iteration Tests
// =============================================================================

#[rstest]
fn test_for_macro_nested_either() {
    let numbers = vec![1, 2, 3];
    let multiplier: Either<String, i32> = Either::Right(10);

    let result = for_! {
        n <= numbers;
        m <= multiplier.clone();
        yield n * m
    };
    assert_eq!(result, vec![10, 20, 30]);
}

#[rstest]
fn test_for_macro_nested_either_left() {
    let numbers = vec![1, 2, 3];
    let multiplier: Either<String, i32> = Either::Left("error".to_string());

    let result = for_! {
        n <= numbers;
        m <= multiplier.clone();
        yield n * m
    };
    assert_eq!(result, Vec::<i32>::new());
}

// =============================================================================
// Vec<Either> Flattening Tests
// =============================================================================

#[rstest]
fn test_for_macro_vec_either_flatten() {
    let eithers = vec![
        Either::<String, i32>::Right(1),
        Either::Left("error".to_string()),
        Either::Right(3),
    ];

    let result = for_! {
        either <= eithers;
        value <= either;
        yield value * 2
    };
    assert_eq!(result, vec![2, 6]);
}

#[rstest]
fn test_for_macro_vec_either_all_left() {
    let eithers = vec![
        Either::<String, i32>::Left("error1".to_string()),
        Either::Left("error2".to_string()),
    ];

    let result = for_! {
        either <= eithers;
        value <= either;
        yield value * 2
    };
    assert_eq!(result, Vec::<i32>::new());
}

#[rstest]
fn test_for_macro_vec_either_all_right() {
    let eithers = vec![
        Either::<String, i32>::Right(1),
        Either::Right(2),
        Either::Right(3),
    ];

    let result = for_! {
        either <= eithers;
        value <= either;
        yield value
    };
    assert_eq!(result, vec![1, 2, 3]);
}

// =============================================================================
// Guard Expression Tests
// =============================================================================

#[rstest]
fn test_for_macro_either_with_guard() {
    let eithers = vec![
        Either::<String, i32>::Right(1),
        Either::Right(2),
        Either::Right(3),
        Either::Left("error".to_string()),
    ];

    let result = for_! {
        either <= eithers;
        value <= either;
        if value % 2 == 1;
        yield value
    };
    assert_eq!(result, vec![1, 3]);
}

#[rstest]
fn test_for_macro_either_with_multiple_guards() {
    let eithers = vec![
        Either::<String, i32>::Right(1),
        Either::Right(5),
        Either::Right(10),
        Either::Right(15),
    ];

    let result = for_! {
        either <= eithers;
        value <= either;
        if value > 3;
        if value < 12;
        yield value
    };
    assert_eq!(result, vec![5, 10]);
}

// =============================================================================
// Let Binding Tests
// =============================================================================

#[rstest]
fn test_for_macro_either_with_let() {
    let result = for_! {
        x <= Either::<String, i32>::Right(42);
        let doubled = x * 2;
        let tripled = doubled + x;
        yield tripled
    };
    assert_eq!(result, vec![126]); // 42 * 2 + 42 = 126
}

#[rstest]
fn test_for_macro_either_with_let_and_guard() {
    let eithers = vec![
        Either::<String, i32>::Right(10),
        Either::Right(20),
        Either::Right(30),
    ];

    let result = for_! {
        either <= eithers;
        value <= either;
        let doubled = value * 2;
        if doubled > 25;
        yield doubled
    };
    assert_eq!(result, vec![40, 60]);
}

// =============================================================================
// Complex Scenario Tests
// =============================================================================

#[rstest]
fn test_error_handling_pipeline() {
    fn parse_int(s: &str) -> Either<String, i32> {
        s.parse::<i32>()
            .map(Either::Right)
            .unwrap_or_else(|_| Either::Left(format!("Failed to parse: {}", s)))
    }

    let inputs = ["1", "2", "not_a_number", "4"];
    let eithers: Vec<Either<String, i32>> = inputs.iter().map(|s| parse_int(s)).collect();

    let result = for_! {
        either <= eithers;
        value <= either;
        yield value * 10
    };
    assert_eq!(result, vec![10, 20, 40]);
}

#[rstest]
fn test_validation_chain() {
    fn validate_positive(n: i32) -> Either<String, i32> {
        if n > 0 {
            Either::Right(n)
        } else {
            Either::Left("Must be positive".to_string())
        }
    }

    fn validate_even(n: i32) -> Either<String, i32> {
        if n % 2 == 0 {
            Either::Right(n)
        } else {
            Either::Left("Must be even".to_string())
        }
    }

    let numbers = vec![-2, 1, 2, 3, 4];

    let result = for_! {
        n <= numbers;
        positive <= validate_positive(n);
        even <= validate_even(positive);
        yield even
    };
    assert_eq!(result, vec![2, 4]);
}

#[rstest]
fn test_mixed_option_either() {
    let option_value: Option<i32> = Some(10);
    let either_value: Either<String, i32> = Either::Right(5);

    let result = for_! {
        x <= option_value;
        y <= either_value.clone();
        yield x + y
    };
    assert_eq!(result, vec![15]);
}

#[rstest]
fn test_mixed_option_either_none() {
    let option_value: Option<i32> = None;
    let either_value: Either<String, i32> = Either::Right(5);

    let result = for_! {
        x <= option_value;
        y <= either_value.clone();
        yield x + y
    };
    assert_eq!(result, Vec::<i32>::new());
}

#[rstest]
fn test_mixed_option_either_left() {
    let option_value: Option<i32> = Some(10);
    let either_value: Either<String, i32> = Either::Left("error".to_string());

    let result = for_! {
        x <= option_value;
        y <= either_value.clone();
        yield x + y
    };
    assert_eq!(result, Vec::<i32>::new());
}
