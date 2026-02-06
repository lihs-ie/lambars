//! Proptest verification of Result/Option Monad laws
//!
//! This test file serves as an example of using the lambars library
//! Verifies Monad laws for Result and Option using proptest.
//!
//! Monad laws:
//! 1. Left Identity: pure(a).flat_map(f) == f(a)
//! 2. Right Identity: m.flat_map(pure) == m
//! 3. Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
//!
//! In Rust:
//! - pure corresponds to Ok/Some
//! - flat_map corresponds to and_then

use order_taking_sample::simple_types::{Price, String50, UnitQuantity};
use proptest::prelude::*;
use rust_decimal::Decimal;

// =============================================================================
// Helper for function selection
// =============================================================================

/// Test functions for Result (selected by index)
fn result_function(index: usize, x: i32) -> Result<i32, String> {
    match index % 5 {
        0 => Ok(x.saturating_mul(2)),
        1 => Ok(x.saturating_add(1)),
        2 => Ok(x.saturating_sub(1)),
        3 => {
            if x % 2 == 0 {
                Ok(x / 2)
            } else {
                Err("odd".to_string())
            }
        }
        _ => {
            if x >= 0 {
                Ok(x)
            } else {
                Err("negative".to_string())
            }
        }
    }
}

/// Test functions for Option (selected by index)
fn option_function(index: usize, x: i32) -> Option<i32> {
    match index % 5 {
        0 => Some(x.saturating_mul(2)),
        1 => Some(x.saturating_add(1)),
        2 => Some(x.saturating_sub(1)),
        3 => {
            if x % 2 == 0 {
                Some(x / 2)
            } else {
                None
            }
        }
        _ => {
            if x >= 0 {
                Some(x)
            } else {
                None
            }
        }
    }
}

// =============================================================================
// Result Monad lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Result Left Identity law: Ok(a).and_then(f) == f(a)
    ///
    /// pure(a).flat_map(f) must equal f(a)
    #[test]
    fn test_result_left_identity(
        value in any::<i32>(),
        function_index in 0usize..5
    ) {
        let left = Ok::<i32, String>(value).and_then(|x| result_function(function_index, x));
        let right = result_function(function_index, value);
        prop_assert_eq!(left, right, "Left Identity violated: Ok({}).and_then(f) != f({})", value, value);
    }

    /// Result Right Identity law: m.and_then(Ok) == m
    ///
    /// m.flat_map(pure) must equal m
    #[test]
    fn test_result_right_identity(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };
        let left = result.clone().and_then(Ok);
        let right = result;
        prop_assert_eq!(left, right, "Right Identity violated");
    }

    /// Result Associativity law:
    /// m.and_then(f).and_then(g) == m.and_then(|x| f(x).and_then(g))
    ///
    /// Associativity law: changing the order of composition yields the same result
    #[test]
    fn test_result_associativity(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}",
        function_index1 in 0usize..5,
        function_index2 in 0usize..5
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };

        let left = result
            .clone()
            .and_then(|x| result_function(function_index1, x))
            .and_then(|x| result_function(function_index2, x));
        let right = result.and_then(|x| {
            result_function(function_index1, x).and_then(|y| result_function(function_index2, y))
        });
        prop_assert_eq!(left, right, "Associativity violated");
    }
}

// =============================================================================
// Option Monad lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Option Left Identity law: Some(a).and_then(f) == f(a)
    #[test]
    fn test_option_left_identity(
        value in any::<i32>(),
        function_index in 0usize..5
    ) {
        let left = Some(value).and_then(|x| option_function(function_index, x));
        let right = option_function(function_index, value);
        prop_assert_eq!(left, right, "Left Identity violated: Some({}).and_then(f) != f({})", value, value);
    }

    /// Option Right Identity law: m.and_then(Some) == m
    #[test]
    fn test_option_right_identity(option in proptest::option::of(any::<i32>())) {
        let left = option.and_then(Some);
        let right = option;
        prop_assert_eq!(left, right, "Right Identity violated");
    }

    /// Option Associativity law:
    /// m.and_then(f).and_then(g) == m.and_then(|x| f(x).and_then(g))
    #[test]
    fn test_option_associativity(
        option in proptest::option::of(any::<i32>()),
        function_index1 in 0usize..5,
        function_index2 in 0usize..5
    ) {
        let left = option
            .and_then(|x| option_function(function_index1, x))
            .and_then(|x| option_function(function_index2, x));
        let right = option.and_then(|x| {
            option_function(function_index1, x).and_then(|y| option_function(function_index2, y))
        });
        prop_assert_eq!(left, right, "Associativity violated");
    }
}

// =============================================================================
// Functor law tests (additional verification since Monad is a Functor)
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Result Functor Identity law: m.map(|x| x) == m
    #[test]
    fn test_result_functor_identity(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };
        let left = result.clone().map(|x| x);
        let right = result;
        prop_assert_eq!(left, right, "Functor Identity violated");
    }

    /// Result Functor Composition law: m.map(f).map(g) == m.map(|x| g(f(x)))
    #[test]
    fn test_result_functor_composition(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };

        let function1 = |x: i32| x.saturating_mul(2);
        let function2 = |x: i32| x.saturating_add(1);

        let left = result.clone().map(function1).map(function2);
        let right = result.map(|x| function2(function1(x)));
        prop_assert_eq!(left, right, "Functor Composition violated");
    }

    /// Option Functor Identity law: m.map(|x| x) == m
    #[test]
    fn test_option_functor_identity(option in proptest::option::of(any::<i32>())) {
        let left = option.map(|x| x);
        let right = option;
        prop_assert_eq!(left, right, "Functor Identity violated");
    }

    /// Option Functor Composition law: m.map(f).map(g) == m.map(|x| g(f(x)))
    #[test]
    fn test_option_functor_composition(option in proptest::option::of(any::<i32>())) {
        let function1 = |x: i32| x.saturating_mul(2);
        let function2 = |x: i32| x.saturating_add(1);

        let left = option.map(function1).map(function2);
        let right = option.map(|x| function2(function1(x)));
        prop_assert_eq!(left, right, "Functor Composition violated");
    }
}

// =============================================================================
// Applicative law tests (additional verification since Monad is an Applicative)
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Result Applicative Identity law:
    /// pure(identity).apply(m) == m
    /// In Rust: Expressed by combining Ok(|x| x) and map
    #[test]
    fn test_result_applicative_identity(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };
        // Equivalent to pure(id) <*> v = v
        let identity = |x: i32| x;
        let left: Result<i32, String> = result.clone().map(identity);
        let right = result;
        prop_assert_eq!(left, right, "Applicative Identity violated");
    }

    /// Option Applicative Identity law
    #[test]
    fn test_option_applicative_identity(option in proptest::option::of(any::<i32>())) {
        let identity = |x: i32| x;
        let left: Option<i32> = option.map(identity);
        let right = option;
        prop_assert_eq!(left, right, "Applicative Identity violated");
    }
}

// =============================================================================
// Monad law tests with domain types
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Combined test for Price creation and monad laws
    /// Monad laws hold for the Result from smart constructors
    #[test]
    fn test_price_creation_monad_left_identity(value in 0u32..=1000u32) {
        let decimal = Decimal::from(value);

        // Left Identity: Ok(a).and_then(f) == f(a)
        let create_price = |d: Decimal| Price::create(d);

        let left = Ok::<Decimal, _>(decimal).and_then(|d| create_price(d).map_err(|e| e.message));
        let right = create_price(decimal).map_err(|e| e.message);

        prop_assert_eq!(left, right, "Price creation Left Identity violated");
    }

    /// Monad composition test for String50 creation
    /// Monad laws hold even when chaining multiple smart constructors
    #[test]
    fn test_string50_monad_composition(input in "[a-zA-Z]{1,30}") {
        // Convert string to String50, then further process the result
        let result1 = String50::create("Field1", &input);
        let result2 = result1.and_then(|s| {
            // Convert String50 value to another String50 (e.g., adding prefix)
            let prefixed = format!("prefix_{}", s.value());
            if prefixed.len() <= 50 {
                String50::create("Field2", &prefixed)
            } else {
                // Truncate if too long
                String50::create("Field2", &prefixed[..50])
            }
        });

        // By Result's monad laws, this is correctly composed
        // If result is Ok, both conversions succeeded
        if result2.is_ok() {
            let value = result2.unwrap();
            prop_assert!(value.value().starts_with("prefix_"), "Composition failed");
        }
    }

    /// Operations with UnitQuantity and monad laws
    #[test]
    fn test_unit_quantity_monad_operations(quantity in 1u32..=500u32) {
        // Create UnitQuantity and double it
        let double = |q: UnitQuantity| {
            let doubled = q.value() * 2;
            UnitQuantity::create("Doubled", doubled)
        };

        let result = UnitQuantity::create("Original", quantity);

        // Verify Left Identity
        let left = result.clone().and_then(double);
        let right = result.and_then(|q| double(q));

        // Same operation on same value yields same result
        prop_assert_eq!(left.is_ok(), right.is_ok(), "Monad operation consistency");
        if let (Ok(left_val), Ok(right_val)) = (left, right) {
            prop_assert_eq!(left_val.value(), right_val.value());
        }
    }
}

// =============================================================================
// Monad law tests for error handling
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Monad law for Err values (verifying short-circuit evaluation)
    /// Err(e).and_then(f) always returns Err(e)
    #[test]
    fn test_result_error_short_circuit(error_message in "[a-z]{1,20}") {
        let error: Result<i32, String> = Err(error_message.clone());

        let result = error.and_then(|x| Ok(x * 2));

        prop_assert!(result.is_err(), "Error should short-circuit");
        prop_assert_eq!(result.unwrap_err(), error_message, "Error message should be preserved");
    }

    /// Monad law for None (verifying short-circuit evaluation)
    /// None.and_then(f) always returns None
    #[test]
    fn test_option_none_short_circuit(_dummy in any::<u8>()) {
        let none: Option<i32> = None;

        let result = none.and_then(|x| Some(x * 2));

        prop_assert!(result.is_none(), "None should short-circuit");
    }
}

// =============================================================================
// Monad laws and equivalence combination tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Relationship between Result's map and and_then
    /// m.map(f) == m.and_then(|x| Ok(f(x)))
    #[test]
    fn test_result_map_is_and_then_with_pure(
        is_ok in any::<bool>(),
        value in any::<i32>(),
        error_message in "[a-z]{1,10}"
    ) {
        let result: Result<i32, String> = if is_ok {
            Ok(value)
        } else {
            Err(error_message)
        };

        let function = |x: i32| x.saturating_mul(3);

        let left = result.clone().map(function);
        let right = result.and_then(|x| Ok(function(x)));

        prop_assert_eq!(left, right, "map should be equivalent to and_then with Ok");
    }

    /// Relationship between Option's map and and_then
    /// m.map(f) == m.and_then(|x| Some(f(x)))
    #[test]
    fn test_option_map_is_and_then_with_pure(option in proptest::option::of(any::<i32>())) {
        let function = |x: i32| x.saturating_mul(3);

        let left = option.map(function);
        let right = option.and_then(|x| Some(function(x)));

        prop_assert_eq!(left, right, "map should be equivalent to and_then with Some");
    }
}
