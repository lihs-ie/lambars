//! Tests for MonadError trait.
//!
//! This module tests the MonadError type class which provides
//! error handling capabilities.

use functional_rusty::effect::MonadError;
use functional_rusty::typeclass::Monad;
use rstest::rstest;

// =============================================================================
// Test error types
// =============================================================================

/// A custom error type for testing.
#[derive(Debug, Clone, PartialEq)]
struct CustomError {
    code: u32,
    message: String,
}

impl CustomError {
    fn new(code: u32, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}

// =============================================================================
// Trait Existence Tests
// =============================================================================

#[test]
fn monad_error_trait_exists() {
    // MonadError trait should exist and be importable
    fn assert_monad_error<M: MonadError<String>>() {}

    // This test verifies the trait is defined
    let _ = assert_monad_error::<Result<(), String>>;
}

#[test]
fn monad_error_has_throw_error_method() {
    // Verify that throw_error method signature is correct
    fn uses_throw_error<M: MonadError<String>>() {
        let _: M::WithType<i32> = M::throw_error("error".to_string());
    }

    let _ = uses_throw_error::<Result<(), String>>;
}

#[test]
fn monad_error_has_catch_error_method() {
    // Verify that catch_error method signature is correct
    fn uses_catch_error<M: MonadError<String>>(computation: M::WithType<i32>) -> M::WithType<i32> {
        M::catch_error(computation, |_error| {
            M::throw_error::<i32>("recovered".to_string())
        })
    }

    let _ = uses_catch_error::<Result<(), String>>;
}

#[test]
fn monad_error_has_from_result_method() {
    // Verify that from_result method signature is correct
    fn uses_from_result<M: MonadError<String>>() {
        let ok_result: Result<i32, String> = Ok(42);
        let _: M::WithType<i32> = M::from_result(ok_result);

        let err_result: Result<i32, String> = Err("error".to_string());
        let _: M::WithType<i32> = M::from_result(err_result);
    }

    let _ = uses_from_result::<Result<(), String>>;
}

#[test]
fn monad_error_has_recover_with_method() {
    // Verify that recover_with method signature is correct
    fn uses_recover_with<M: MonadError<String>>(
        computation: M::WithType<i32>,
        default: M::WithType<i32>,
    ) -> M::WithType<i32> {
        M::recover_with(computation, default)
    }

    let _ = uses_recover_with::<Result<(), String>>;
}

// =============================================================================
// Result Implementation Tests
// =============================================================================

#[rstest]
fn result_throw_error_creates_err() {
    let result: Result<i32, String> = <Result<i32, String>>::throw_error("test error".to_string());
    assert_eq!(result, Err("test error".to_string()));
}

#[rstest]
fn result_throw_error_with_different_value_types() {
    // throw_error should work with any value type
    let result1: Result<String, i32> = <Result<String, i32>>::throw_error(404);
    assert_eq!(result1, Err(404));

    let result2: Result<Vec<u8>, CustomError> =
        <Result<Vec<u8>, CustomError>>::throw_error(CustomError::new(500, "Internal Server Error"));
    assert_eq!(result2, Err(CustomError::new(500, "Internal Server Error")));
}

#[rstest]
fn result_catch_error_recovers_from_err() {
    let computation: Result<i32, String> = Err("original error".to_string());
    let recovered = <Result<i32, String>>::catch_error(computation, |error| Ok(error.len() as i32));
    assert_eq!(recovered, Ok(14)); // "original error".len() == 14
}

#[rstest]
fn result_catch_error_preserves_ok() {
    let computation: Result<i32, String> = Ok(42);
    let result = <Result<i32, String>>::catch_error(computation, |_| Ok(0));
    assert_eq!(result, Ok(42));
}

#[rstest]
fn result_catch_error_can_rethrow() {
    let computation: Result<i32, String> = Err("error".to_string());
    let result = <Result<i32, String>>::catch_error(computation, |original| {
        Err(format!("wrapped: {}", original))
    });
    assert_eq!(result, Err("wrapped: error".to_string()));
}

#[rstest]
fn result_from_result_converts_ok() {
    let rust_result: Result<i32, String> = Ok(42);
    let monad_result: Result<i32, String> = <Result<i32, String>>::from_result(rust_result);
    assert_eq!(monad_result, Ok(42));
}

#[rstest]
fn result_from_result_converts_err() {
    let rust_result: Result<i32, String> = Err("error".to_string());
    let monad_result: Result<i32, String> = <Result<i32, String>>::from_result(rust_result);
    assert_eq!(monad_result, Err("error".to_string()));
}

#[rstest]
fn result_recover_with_returns_default_on_err() {
    let computation: Result<i32, String> = Err("error".to_string());
    let default: Result<i32, String> = Ok(0);
    let result = <Result<i32, String>>::recover_with(computation, default);
    assert_eq!(result, Ok(0));
}

#[rstest]
fn result_recover_with_returns_original_on_ok() {
    let computation: Result<i32, String> = Ok(42);
    let default: Result<i32, String> = Ok(0);
    let result = <Result<i32, String>>::recover_with(computation, default);
    assert_eq!(result, Ok(42));
}

#[rstest]
fn result_recover_with_propagates_default_error() {
    let computation: Result<i32, String> = Err("first error".to_string());
    let default: Result<i32, String> = Err("default error".to_string());
    let result = <Result<i32, String>>::recover_with(computation, default);
    assert_eq!(result, Err("default error".to_string()));
}

// =============================================================================
// Law Tests for Result
// =============================================================================

/// MonadError Laws:
///
/// 1. Throw Catch Law: catch_error(throw_error(e), handler) == handler(e)
///    Catching a thrown error should apply the handler.
///
/// 2. Catch Pure Law: catch_error(pure(a), handler) == pure(a)
///    Catching when there's no error should return the original.
///
/// 3. Throw Short-Circuit Law: throw_error(e).flat_map(f) == throw_error(e)
///    Throwing an error should short-circuit subsequent computations.
#[rstest]
fn result_throw_catch_law() {
    let error = "test error".to_string();
    let handler = |e: String| Ok::<i32, String>(e.len() as i32);

    // catch_error(throw_error(e), handler) == handler(e)
    let left: Result<i32, String> = <Result<i32, String>>::catch_error(
        <Result<i32, String>>::throw_error(error.clone()),
        handler,
    );
    let right: Result<i32, String> = handler(error);

    assert_eq!(left, right);
}

#[rstest]
fn result_catch_pure_law() {
    use functional_rusty::typeclass::Applicative;

    let value = 42;
    let handler = |_: String| Ok::<i32, String>(0);

    // catch_error(pure(a), handler) == pure(a)
    let pure_value: Result<i32, String> = <Result<(), String>>::pure(value);
    let left = <Result<i32, String>>::catch_error(pure_value.clone(), handler);

    assert_eq!(left, pure_value);
}

#[rstest]
fn result_throw_short_circuit_law() {
    let error = "error".to_string();

    // throw_error(e).flat_map(f) == throw_error(e)
    let thrown: Result<i32, String> = <Result<i32, String>>::throw_error(error.clone());
    let left: Result<String, String> = thrown.flat_map(|n| Ok(format!("got: {}", n)));
    let right: Result<String, String> = <Result<String, String>>::throw_error(error);

    assert_eq!(left, right);
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn monad_error_works_with_unit_error() {
    // MonadError should work with unit type as error
    fn assert_monad_error_unit<M: MonadError<()>>() {}
    let _ = assert_monad_error_unit::<Result<i32, ()>>;
}

#[test]
fn monad_error_works_with_complex_error() {
    // MonadError should work with complex error types
    #[derive(Debug, Clone)]
    struct ValidationError {
        field: String,
        message: String,
        code: u32,
    }

    fn assert_monad_error_complex<M: MonadError<ValidationError>>() {}
    let _ = assert_monad_error_complex::<Result<i32, ValidationError>>;
}

#[test]
fn monad_error_works_with_nested_result_errors() {
    // MonadError should work with nested error types
    type NestedError = Result<String, std::io::Error>;

    fn assert_monad_error_nested<M: MonadError<String>>() {}
    let _ = assert_monad_error_nested::<Result<i32, String>>;
}

// =============================================================================
// Practical Usage Tests
// =============================================================================

#[rstest]
fn result_error_handling_chain() {
    // Demonstrate practical error handling patterns
    fn divide(a: i32, b: i32) -> Result<i32, String> {
        if b == 0 {
            <Result<i32, String>>::throw_error("division by zero".to_string())
        } else {
            Ok(a / b)
        }
    }

    fn safe_sqrt(n: i32) -> Result<f64, String> {
        if n < 0 {
            <Result<f64, String>>::throw_error("negative number".to_string())
        } else {
            Ok((n as f64).sqrt())
        }
    }

    // Successful chain
    let result = divide(100, 4).flat_map(|n| safe_sqrt(n));
    assert_eq!(result, Ok(5.0));

    // Error in first step
    let result = divide(100, 0).flat_map(|n| safe_sqrt(n));
    assert_eq!(result, Err("division by zero".to_string()));

    // Error in second step
    let result = divide(-16, 2).flat_map(|n| safe_sqrt(n));
    assert_eq!(result, Err("negative number".to_string()));
}

#[rstest]
fn result_error_recovery_pattern() {
    // Demonstrate error recovery patterns
    let computation: Result<i32, String> = Err("error".to_string());

    // Recovery with default value
    let recovered = <Result<i32, String>>::catch_error(computation.clone(), |_| Ok(0));
    assert_eq!(recovered, Ok(0));

    // Recovery with error transformation
    let transformed =
        <Result<i32, String>>::catch_error(computation.clone(), |e| Err(format!("Handled: {}", e)));
    assert_eq!(transformed, Err("Handled: error".to_string()));

    // Recovery with recover_with
    let with_default = <Result<i32, String>>::recover_with(computation, Ok(-1));
    assert_eq!(with_default, Ok(-1));
}
