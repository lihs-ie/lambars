#![cfg(feature = "effect")]
//! Tests for ExceptT (Except Transformer).
//!
//! ExceptT adds error handling capability to any monad.

use lambars::effect::{ExceptT, IO};
use rstest::rstest;

// =============================================================================
// Basic Structure Tests
// =============================================================================

#[rstest]
fn except_transformer_new_and_run_with_option() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Ok(42)));
    let result = except_transformer.run();
    assert_eq!(result, Some(Ok(42)));
}

#[rstest]
fn except_transformer_new_and_run_with_option_error() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Err("error".to_string())));
    let result = except_transformer.run();
    assert_eq!(result, Some(Err("error".to_string())));
}

#[rstest]
fn except_transformer_run_returns_none_when_inner_is_none() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(None::<Result<i32, String>>);
    let result = except_transformer.run();
    assert_eq!(result, None);
}

// =============================================================================
// pure Tests
// =============================================================================

#[rstest]
fn except_transformer_pure_with_option() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> = ExceptT::pure_option(42);
    let result = except_transformer.run();
    assert_eq!(result, Some(Ok(42)));
}

#[rstest]
fn except_transformer_pure_with_result() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::pure_result(42);
    let result = except_transformer.run();
    assert_eq!(result, Ok(Ok(42)));
}

// =============================================================================
// throw Tests
// =============================================================================

#[rstest]
fn except_transformer_throw_option() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::<String, Option<Result<i32, String>>>::throw_option("error".to_string());
    let result = except_transformer.run();
    assert_eq!(result, Some(Err("error".to_string())));
}

#[rstest]
fn except_transformer_throw_result() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::<String, Result<Result<i32, String>, String>>::throw_result("error".to_string());
    let result = except_transformer.run();
    assert_eq!(result, Ok(Err("error".to_string())));
}

// =============================================================================
// lift Tests
// =============================================================================

#[rstest]
fn except_transformer_lift_option() {
    let inner: Option<i32> = Some(42);
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::lift_option(inner);
    let result = except_transformer.run();
    assert_eq!(result, Some(Ok(42)));
}

#[rstest]
fn except_transformer_lift_option_none() {
    let inner: Option<i32> = None;
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::lift_option(inner);
    let result = except_transformer.run();
    assert_eq!(result, None);
}

#[rstest]
fn except_transformer_lift_result() {
    let inner: Result<i32, String> = Ok(42);
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::lift_result(inner);
    let result = except_transformer.run();
    assert_eq!(result, Ok(Ok(42)));
}

#[rstest]
fn except_transformer_lift_result_error() {
    let inner: Result<i32, String> = Err("outer error".to_string());
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::lift_result(inner);
    let result = except_transformer.run();
    assert_eq!(result, Err("outer error".to_string()));
}

// =============================================================================
// fmap (Functor) Tests
// =============================================================================

#[rstest]
fn except_transformer_fmap_option_ok() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Ok(21)));
    let mapped = except_transformer.fmap_option(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, Some(Ok(42)));
}

#[rstest]
fn except_transformer_fmap_option_error() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Err("error".to_string())));
    let mapped = except_transformer.fmap_option(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, Some(Err("error".to_string())));
}

#[rstest]
fn except_transformer_fmap_option_none() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(None::<Result<i32, String>>);
    let mapped = except_transformer.fmap_option(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, None);
}

#[rstest]
fn except_transformer_fmap_result_ok() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::new(Ok(Ok(21)));
    let mapped = except_transformer.fmap_result(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, Ok(Ok(42)));
}

#[rstest]
fn except_transformer_fmap_result_inner_error() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::new(Ok(Err("inner error".to_string())));
    let mapped = except_transformer.fmap_result(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, Ok(Err("inner error".to_string())));
}

#[rstest]
fn except_transformer_fmap_result_outer_error() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> = ExceptT::new(
        Err::<Result<i32, String>, String>("outer error".to_string()),
    );
    let mapped = except_transformer.fmap_result(|value| value * 2);
    let result = mapped.run();
    assert_eq!(result, Err("outer error".to_string()));
}

// =============================================================================
// flat_map (Monad) Tests
// =============================================================================

#[rstest]
fn except_transformer_flat_map_option_ok_to_ok() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Ok(10)));

    let chained = except_transformer.flat_map_option(|value| ExceptT::new(Some(Ok(value * 2))));

    let result = chained.run();
    assert_eq!(result, Some(Ok(20)));
}

#[rstest]
fn except_transformer_flat_map_option_ok_to_error() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Ok(10)));

    let chained = except_transformer.flat_map_option(|_value| {
        ExceptT::new(Some(Err::<i32, String>("error in chain".to_string())))
    });

    let result = chained.run();
    assert_eq!(result, Some(Err("error in chain".to_string())));
}

#[rstest]
fn except_transformer_flat_map_option_error_short_circuits() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Err("initial error".to_string())));

    let chained = except_transformer.flat_map_option(|value| ExceptT::new(Some(Ok(value * 2))));

    let result = chained.run();
    assert_eq!(result, Some(Err("initial error".to_string())));
}

#[rstest]
fn except_transformer_flat_map_option_none_short_circuits() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(None::<Result<i32, String>>);

    let chained = except_transformer.flat_map_option(|value| ExceptT::new(Some(Ok(value * 2))));

    let result = chained.run();
    assert_eq!(result, None);
}

#[rstest]
fn except_transformer_flat_map_result_ok_to_ok() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::new(Ok(Ok(10)));

    let chained = except_transformer.flat_map_result(|value| ExceptT::new(Ok(Ok(value * 2))));

    let result = chained.run();
    assert_eq!(result, Ok(Ok(20)));
}

#[rstest]
fn except_transformer_flat_map_result_ok_to_inner_error() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::new(Ok(Ok(10)));

    let chained = except_transformer
        .flat_map_result(|_value| ExceptT::new(Ok(Err::<i32, String>("inner error".to_string()))));

    let result = chained.run();
    assert_eq!(result, Ok(Err("inner error".to_string())));
}

#[rstest]
fn except_transformer_flat_map_result_inner_error_short_circuits() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::new(Ok(Err("initial inner error".to_string())));

    let chained = except_transformer.flat_map_result(|value| ExceptT::new(Ok(Ok(value * 2))));

    let result = chained.run();
    assert_eq!(result, Ok(Err("initial inner error".to_string())));
}

#[rstest]
fn except_transformer_flat_map_result_outer_error_short_circuits() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> = ExceptT::new(
        Err::<Result<i32, String>, String>("outer error".to_string()),
    );

    let chained = except_transformer.flat_map_result(|value| ExceptT::new(Ok(Ok(value * 2))));

    let result = chained.run();
    assert_eq!(result, Err("outer error".to_string()));
}

// =============================================================================
// catch Tests
// =============================================================================

#[rstest]
fn except_transformer_catch_option_error_recovers() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Err("error".to_string())));

    let recovered = ExceptT::catch_option(except_transformer, |error| {
        ExceptT::new(Some(Ok(error.len() as i32)))
    });

    let result = recovered.run();
    assert_eq!(result, Some(Ok(5))); // "error".len() == 5
}

#[rstest]
fn except_transformer_catch_option_ok_passes_through() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Ok(42)));

    let recovered = ExceptT::catch_option(except_transformer, |error| {
        ExceptT::new(Some(Ok(error.len() as i32)))
    });

    let result = recovered.run();
    assert_eq!(result, Some(Ok(42)));
}

#[rstest]
fn except_transformer_catch_option_none_passes_through() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(None::<Result<i32, String>>);

    let recovered = ExceptT::catch_option(except_transformer, |error| {
        ExceptT::new(Some(Ok(error.len() as i32)))
    });

    let result = recovered.run();
    assert_eq!(result, None);
}

#[rstest]
fn except_transformer_catch_result_error_recovers() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::new(Ok(Err("error".to_string())));

    let recovered = ExceptT::catch_result(except_transformer, |error| {
        ExceptT::new(Ok(Ok(error.len() as i32)))
    });

    let result = recovered.run();
    assert_eq!(result, Ok(Ok(5)));
}

#[rstest]
fn except_transformer_catch_result_ok_passes_through() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> =
        ExceptT::new(Ok(Ok(42)));

    let recovered = ExceptT::catch_result(except_transformer, |error| {
        ExceptT::new(Ok(Ok(error.len() as i32)))
    });

    let result = recovered.run();
    assert_eq!(result, Ok(Ok(42)));
}

#[rstest]
fn except_transformer_catch_result_outer_error_passes_through() {
    let except_transformer: ExceptT<String, Result<Result<i32, String>, String>> = ExceptT::new(
        Err::<Result<i32, String>, String>("outer error".to_string()),
    );

    let recovered = ExceptT::catch_result(except_transformer, |error| {
        ExceptT::new(Ok(Ok(error.len() as i32)))
    });

    let result = recovered.run();
    // Outer error is not caught, only inner ExceptT errors are caught
    assert_eq!(result, Err("outer error".to_string()));
}

// =============================================================================
// ExceptT with IO Tests
// =============================================================================

#[rstest]
fn except_transformer_with_io_basic() {
    let except_transformer: ExceptT<String, IO<Result<i32, String>>> =
        ExceptT::new(IO::pure(Ok(42)));

    let io_result = except_transformer.run();
    let result = io_result.run_unsafe();
    assert_eq!(result, Ok(42));
}

#[rstest]
fn except_transformer_lift_io() {
    let inner = IO::pure(42);
    let except_transformer: ExceptT<String, IO<Result<i32, String>>> = ExceptT::lift_io(inner);

    let io_result = except_transformer.run();
    let result = io_result.run_unsafe();
    assert_eq!(result, Ok(42));
}

#[rstest]
fn except_transformer_throw_io() {
    let except_transformer: ExceptT<String, IO<Result<i32, String>>> =
        ExceptT::<String, IO<Result<i32, String>>>::throw_io("error".to_string());

    let io_result = except_transformer.run();
    let result = io_result.run_unsafe();
    assert_eq!(result, Err("error".to_string()));
}

#[rstest]
fn except_transformer_fmap_io() {
    let except_transformer: ExceptT<String, IO<Result<i32, String>>> =
        ExceptT::new(IO::pure(Ok(21)));

    let mapped = except_transformer.fmap_io(|value| value * 2);

    let io_result = mapped.run();
    let result = io_result.run_unsafe();
    assert_eq!(result, Ok(42));
}

#[rstest]
fn except_transformer_flat_map_io() {
    let except_transformer: ExceptT<String, IO<Result<i32, String>>> =
        ExceptT::new(IO::pure(Ok(10)));

    let chained = except_transformer.flat_map_io(|value| ExceptT::new(IO::pure(Ok(value * 2))));

    let io_result = chained.run();
    let result = io_result.run_unsafe();
    assert_eq!(result, Ok(20));
}

#[rstest]
fn except_transformer_catch_io() {
    let except_transformer: ExceptT<String, IO<Result<i32, String>>> =
        ExceptT::new(IO::pure(Err("error".to_string())));

    let recovered = ExceptT::catch_io(except_transformer, |error| {
        ExceptT::new(IO::pure(Ok(error.len() as i32)))
    });

    let io_result = recovered.run();
    let result = io_result.run_unsafe();
    assert_eq!(result, Ok(5));
}

// =============================================================================
// Clone Tests
// =============================================================================

#[rstest]
fn except_transformer_clone() {
    let except_transformer: ExceptT<String, Option<Result<i32, String>>> =
        ExceptT::new(Some(Ok(42)));
    let cloned = except_transformer.clone();

    assert_eq!(except_transformer.run(), Some(Ok(42)));
    assert_eq!(cloned.run(), Some(Ok(42)));
}

// =============================================================================
// Practical Examples
// =============================================================================

#[rstest]
fn except_transformer_validation_example() {
    fn validate_positive(value: i32) -> ExceptT<String, Option<Result<i32, String>>> {
        if value > 0 {
            ExceptT::pure_option(value)
        } else {
            ExceptT::<String, Option<Result<i32, String>>>::throw_option(
                "Value must be positive".to_string(),
            )
        }
    }

    fn validate_less_than_100(value: i32) -> ExceptT<String, Option<Result<i32, String>>> {
        if value < 100 {
            ExceptT::pure_option(value)
        } else {
            ExceptT::<String, Option<Result<i32, String>>>::throw_option(
                "Value must be less than 100".to_string(),
            )
        }
    }

    // Valid case
    let valid_computation = validate_positive(50)
        .flat_map_option(validate_less_than_100)
        .fmap_option(|value| value * 2);

    assert_eq!(valid_computation.run(), Some(Ok(100)));

    // Invalid case - negative
    let invalid_computation = validate_positive(-5)
        .flat_map_option(validate_less_than_100)
        .fmap_option(|value| value * 2);

    assert_eq!(
        invalid_computation.run(),
        Some(Err("Value must be positive".to_string()))
    );

    // Invalid case - too large
    let too_large_computation = validate_positive(150)
        .flat_map_option(validate_less_than_100)
        .fmap_option(|value| value * 2);

    assert_eq!(
        too_large_computation.run(),
        Some(Err("Value must be less than 100".to_string()))
    );
}

#[rstest]
fn except_transformer_recovery_example() {
    fn risky_division(
        numerator: i32,
        denominator: i32,
    ) -> ExceptT<String, Option<Result<i32, String>>> {
        if denominator == 0 {
            ExceptT::<String, Option<Result<i32, String>>>::throw_option(
                "Division by zero".to_string(),
            )
        } else {
            ExceptT::pure_option(numerator / denominator)
        }
    }

    // Without recovery - error
    let error_result = risky_division(10, 0).run();
    assert_eq!(error_result, Some(Err("Division by zero".to_string())));

    // With recovery - default to 0
    let recovered_result =
        ExceptT::catch_option(risky_division(10, 0), |_error| ExceptT::pure_option(0));
    assert_eq!(recovered_result.run(), Some(Ok(0)));

    // Success case
    let success_result = risky_division(10, 2).run();
    assert_eq!(success_result, Some(Ok(5)));
}
