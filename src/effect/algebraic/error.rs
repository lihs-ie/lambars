//! Error effect for error handling.
//!
//! This module provides the `ErrorEffect<E>` type that represents computations
//! that can fail with an error of type `E`.
//!
//! # Operations
//!
//! - [`ErrorEffect::throw`]: Raises an error
//! - [`catch`]: Catches and potentially recovers from an error
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::algebraic::{ErrorEffect, ErrorHandler, Handler, Eff};
//!
//! // Computation that may fail
//! let computation: Eff<ErrorEffect<String>, i32> =
//!     ErrorEffect::throw("error occurred".to_string());
//!
//! let result = ErrorHandler::new().run(computation);
//! assert_eq!(result, Err("error occurred".to_string()));
//! ```

use super::eff::{Eff, EffInner, OperationTag};
use super::effect::Effect;
use super::handler::Handler;
use std::marker::PhantomData;

mod error_operations {
    use super::OperationTag;
    pub const THROW: OperationTag = OperationTag::new(30);
}

/// Error effect: provides error handling capability.
///
/// `ErrorEffect<E>` represents the capability to fail with an error of type `E`.
/// Errors short-circuit the computation and propagate until handled.
///
/// # Type Parameters
///
/// - `E`: The type of the error
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{ErrorEffect, ErrorHandler, Handler, Eff};
///
/// let computation: Eff<ErrorEffect<String>, i32> =
///     ErrorEffect::throw("something went wrong".to_string());
///
/// let result = ErrorHandler::new().run(computation);
/// assert!(result.is_err());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ErrorEffect<E>(PhantomData<E>);

impl<E: 'static> Effect for ErrorEffect<E> {
    const NAME: &'static str = "Error";
}

impl<E: Clone + Send + Sync + 'static> ErrorEffect<E> {
    /// Raises an error, short-circuiting the computation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{ErrorEffect, ErrorHandler, Handler, Eff};
    ///
    /// let computation: Eff<ErrorEffect<&str>, i32> = ErrorEffect::throw("error");
    /// let result = ErrorHandler::new().run(computation);
    /// assert_eq!(result, Err("error"));
    /// ```
    pub fn throw<A: 'static>(error: E) -> Eff<Self, A> {
        Eff::<Self, A>::perform_raw::<A>(error_operations::THROW, error)
    }
}

/// Handler for the Error effect.
///
/// `ErrorHandler<E>` interprets Error operations and returns `Result<A, E>`.
/// If no error is thrown, returns `Ok(result)`. If an error is thrown,
/// returns `Err(error)`.
///
/// # Type Parameters
///
/// - `E`: The type of the error
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{ErrorEffect, ErrorHandler, Handler, Eff};
///
/// let handler = ErrorHandler::<String>::new();
///
/// // Successful computation
/// let ok_computation: Eff<ErrorEffect<String>, i32> = Eff::pure(42);
/// assert_eq!(handler.clone().run(ok_computation), Ok(42));
///
/// // Failed computation
/// let err_computation: Eff<ErrorEffect<String>, i32> =
///     ErrorEffect::throw("error".to_string());
/// assert_eq!(handler.run(err_computation), Err("error".to_string()));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ErrorHandler<E>(PhantomData<E>);

impl<E: Clone + 'static> ErrorHandler<E> {
    /// Creates a new `ErrorHandler`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::ErrorHandler;
    ///
    /// let handler = ErrorHandler::<String>::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }

    /// Runs the computation and returns a Result (internal).
    ///
    /// Uses an iterative approach for stack safety.
    fn run_internal<A: 'static>(computation: Eff<ErrorEffect<E>, A>) -> Result<A, E> {
        let normalized = computation.normalize();

        match normalized.inner {
            EffInner::Pure(value) => Ok(value),
            EffInner::Impure(operation) => match operation.operation_tag {
                error_operations::THROW => {
                    let error = *operation
                        .arguments
                        .downcast::<E>()
                        .expect("Type mismatch in Error::throw");
                    Err(error)
                }
                _ => panic!("Unknown Error operation: {:?}", operation.operation_tag),
            },
            EffInner::FlatMap(_) => {
                unreachable!("FlatMap should be normalized by normalize()")
            }
        }
    }
}

impl<E: Clone + 'static> Handler<ErrorEffect<E>> for ErrorHandler<E> {
    type Output<A> = Result<A, E>;

    fn run<A: 'static>(self, computation: Eff<ErrorEffect<E>, A>) -> Result<A, E> {
        Self::run_internal(computation)
    }
}

/// Catches and potentially recovers from an error.
///
/// This function provides the `catch` operation for Error effect.
/// It runs the computation and, if an error occurs, calls the recovery
/// function to potentially handle the error.
///
/// # Type Parameters
///
/// - `E`: The error type
/// - `A`: The result type
///
/// # Arguments
///
/// * `computation` - The computation that may throw an error
/// * `recovery` - A function that handles the error and returns a recovery computation
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{ErrorEffect, ErrorHandler, Handler, Eff, catch};
///
/// let handler = ErrorHandler::<String>::new();
///
/// // Recovery from error
/// let computation = catch(
///     ErrorEffect::throw("error".to_string()),
///     |_| Eff::pure(42)
/// );
/// assert_eq!(handler.run(computation), Ok(42));
/// ```
///
/// ```rust
/// use lambars::effect::algebraic::{ErrorEffect, ErrorHandler, Handler, Eff, catch};
///
/// let handler = ErrorHandler::<String>::new();
///
/// // Success passes through
/// let computation = catch(
///     Eff::<ErrorEffect<String>, i32>::pure(100),
///     |_| Eff::pure(0)
/// );
/// assert_eq!(handler.run(computation), Ok(100));
/// ```
pub fn catch<E, A, F>(computation: Eff<ErrorEffect<E>, A>, recovery: F) -> Eff<ErrorEffect<E>, A>
where
    E: Clone + Send + Sync + 'static,
    A: 'static,
    F: FnOnce(E) -> Eff<ErrorEffect<E>, A> + 'static,
{
    // Run the computation with a fresh handler
    match ErrorHandler::<E>::run_internal(computation) {
        Ok(value) => Eff::pure(value),
        Err(error) => recovery(error),
    }
}

/// Attempts to run a computation, returning the error as a value if it fails.
///
/// This is similar to `catch` but captures the error as an `Either`-like value
/// rather than requiring a recovery function.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{ErrorEffect, ErrorHandler, Handler, Eff, attempt};
///
/// let handler = ErrorHandler::<String>::new();
///
/// let computation = attempt(ErrorEffect::<String>::throw::<i32>("error".to_string()));
/// let result = handler.run(computation);
/// assert_eq!(result, Ok(Err("error".to_string())));
/// ```
pub fn attempt<E, A>(computation: Eff<ErrorEffect<E>, A>) -> Eff<ErrorEffect<E>, Result<A, E>>
where
    E: Clone + Send + Sync + 'static,
    A: 'static,
{
    match ErrorHandler::<E>::run_internal(computation) {
        Ok(value) => Eff::pure(Ok(value)),
        Err(error) => Eff::pure(Err(error)),
    }
}

#[cfg(test)]
#[allow(
    clippy::no_effect_underscore_binding,
    clippy::redundant_clone,
    clippy::items_after_statements
)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn error_effect_name_is_error() {
        assert_eq!(ErrorEffect::<String>::NAME, "Error");
    }

    #[rstest]
    fn error_effect_is_debug() {
        let effect: ErrorEffect<String> = ErrorEffect(PhantomData);
        let debug_string = format!("{effect:?}");
        assert!(debug_string.contains("ErrorEffect"));
    }

    #[rstest]
    fn error_effect_is_clone() {
        let effect: ErrorEffect<String> = ErrorEffect(PhantomData);
        let _cloned = effect;
    }

    #[rstest]
    fn error_effect_is_copy() {
        let effect: ErrorEffect<String> = ErrorEffect(PhantomData);
        let _copied = effect;
    }

    #[rstest]
    fn error_handler_new_creates_handler() {
        let _handler = ErrorHandler::<String>::new();
    }

    #[rstest]
    fn error_handler_is_debug() {
        let handler = ErrorHandler::<String>::new();
        let debug_string = format!("{handler:?}");
        assert!(debug_string.contains("ErrorHandler"));
    }

    #[rstest]
    fn error_handler_is_clone() {
        let handler = ErrorHandler::<String>::new();
        let _cloned = handler.clone();
    }

    #[rstest]
    #[allow(clippy::default_constructed_unit_structs)]
    fn error_handler_is_default() {
        let _handler = ErrorHandler::<String>::default();
    }

    // throw Operation Tests

    #[rstest]
    fn error_throw_returns_err() {
        let handler = ErrorHandler::<String>::new();
        let computation: Eff<ErrorEffect<String>, i32> = ErrorEffect::throw("error".to_string());
        let result = handler.run(computation);
        assert_eq!(result, Err("error".to_string()));
    }

    #[rstest]
    fn error_throw_with_different_types() {
        let handler = ErrorHandler::<i32>::new();
        let computation: Eff<ErrorEffect<i32>, String> = ErrorEffect::throw(42);
        let result = handler.run(computation);
        assert_eq!(result, Err(42));
    }

    #[rstest]
    fn error_throw_short_circuits() {
        let handler = ErrorHandler::<String>::new();
        let computation = ErrorEffect::<String>::throw::<i32>("early error".to_string())
            .flat_map(|x| Eff::pure(x + 1));
        let result = handler.run(computation);
        assert_eq!(result, Err("early error".to_string()));
    }

    #[rstest]
    fn error_pure_returns_ok() {
        let handler = ErrorHandler::<String>::new();
        let computation: Eff<ErrorEffect<String>, i32> = Eff::pure(42);
        let result = handler.run(computation);
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn error_pure_with_string() {
        let handler = ErrorHandler::<String>::new();
        let computation: Eff<ErrorEffect<String>, String> = Eff::pure("hello".to_string());
        let result = handler.run(computation);
        assert_eq!(result, Ok("hello".to_string()));
    }

    // catch Operation Tests

    #[rstest]
    fn catch_recovers_from_error() {
        let handler = ErrorHandler::<String>::new();
        let computation = catch(ErrorEffect::throw("error".to_string()), |_| Eff::pure(42));
        let result = handler.run(computation);
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn catch_passes_through_success() {
        let handler = ErrorHandler::<String>::new();
        let computation = catch(Eff::<ErrorEffect<String>, i32>::pure(100), |_| Eff::pure(0));
        let result = handler.run(computation);
        assert_eq!(result, Ok(100));
    }

    #[rstest]
    fn catch_receives_error_value() {
        let handler = ErrorHandler::<String>::new();
        let computation = catch(ErrorEffect::throw("original error".to_string()), |err| {
            Eff::pure(format!("recovered from: {err}"))
        });
        let result = handler.run(computation);
        assert_eq!(result, Ok("recovered from: original error".to_string()));
    }

    #[rstest]
    fn catch_can_rethrow() {
        let handler = ErrorHandler::<String>::new();
        let computation = catch(ErrorEffect::throw::<i32>("error".to_string()), |err| {
            ErrorEffect::throw(format!("rethrown: {err}"))
        });
        let result = handler.run(computation);
        assert_eq!(result, Err("rethrown: error".to_string()));
    }

    #[rstest]
    fn catch_nested() {
        let handler = ErrorHandler::<String>::new();
        let computation = catch(
            catch(
                ErrorEffect::throw::<String>("inner error".to_string()),
                |_| ErrorEffect::throw("outer error".to_string()),
            ),
            |err| Eff::pure(format!("caught: {err}")),
        );
        let result = handler.run(computation);
        assert_eq!(result, Ok("caught: outer error".to_string()));
    }

    // attempt Operation Tests

    #[rstest]
    fn attempt_captures_error() {
        let handler = ErrorHandler::<String>::new();
        let computation = attempt(ErrorEffect::<String>::throw::<i32>("error".to_string()));
        let result = handler.run(computation);
        assert_eq!(result, Ok(Err("error".to_string())));
    }

    #[rstest]
    fn attempt_captures_success() {
        let handler = ErrorHandler::<String>::new();
        let computation = attempt(Eff::<ErrorEffect<String>, i32>::pure(42));
        let result = handler.run(computation);
        assert_eq!(result, Ok(Ok(42)));
    }

    #[rstest]
    fn error_fmap_on_success() {
        let handler = ErrorHandler::<String>::new();
        let computation: Eff<ErrorEffect<String>, i32> = Eff::pure(21);
        let mapped = computation.fmap(|x| x * 2);
        let result = handler.run(mapped);
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn error_fmap_not_executed_on_error() {
        let handler = ErrorHandler::<String>::new();
        let computation: Eff<ErrorEffect<String>, i32> = ErrorEffect::throw("error".to_string());
        let mapped = computation.fmap(|x| x * 2);
        let result = handler.run(mapped);
        assert_eq!(result, Err("error".to_string()));
    }

    #[rstest]
    fn error_flat_map_on_success() {
        let handler = ErrorHandler::<String>::new();
        let computation: Eff<ErrorEffect<String>, i32> = Eff::pure(10);
        let chained = computation.flat_map(|x| Eff::pure(x + 5));
        let result = handler.run(chained);
        assert_eq!(result, Ok(15));
    }

    #[rstest]
    fn error_flat_map_chain_with_error() {
        let handler = ErrorHandler::<String>::new();
        let computation = Eff::<ErrorEffect<String>, i32>::pure(10)
            .flat_map(|_| ErrorEffect::throw("mid error".to_string()))
            .flat_map(|x: i32| Eff::pure(x + 1));
        let result = handler.run(computation);
        assert_eq!(result, Err("mid error".to_string()));
    }

    #[rstest]
    fn error_conditional_throw() {
        let handler = ErrorHandler::<String>::new();

        fn check_positive(x: i32) -> Eff<ErrorEffect<String>, i32> {
            if x >= 0 {
                Eff::pure(x)
            } else {
                ErrorEffect::throw("negative value".to_string())
            }
        }

        let positive = handler.clone().run(check_positive(10));
        let negative = handler.run(check_positive(-5));

        assert_eq!(positive, Ok(10));
        assert_eq!(negative, Err("negative value".to_string()));
    }

    #[rstest]
    fn error_deep_chain_is_stack_safe() {
        let handler = ErrorHandler::<String>::new();
        let mut computation: Eff<ErrorEffect<String>, i32> = Eff::pure(0);
        for _ in 0..1000 {
            computation = computation.flat_map(|x| Eff::pure(x + 1));
        }
        let result = handler.run(computation);
        assert_eq!(result, Ok(1000));
    }

    #[rstest]
    fn error_deep_fmap_is_stack_safe() {
        let handler = ErrorHandler::<String>::new();
        let mut computation: Eff<ErrorEffect<String>, i32> = Eff::pure(0);
        for _ in 0..1000 {
            computation = computation.fmap(|x| x + 1);
        }
        let result = handler.run(computation);
        assert_eq!(result, Ok(1000));
    }

    #[rstest]
    fn error_validation_pattern() {
        let handler = ErrorHandler::<Vec<String>>::new();

        fn validate_name(name: &str) -> Eff<ErrorEffect<Vec<String>>, String> {
            if name.is_empty() {
                ErrorEffect::throw(vec!["Name cannot be empty".to_string()])
            } else {
                Eff::pure(name.to_string())
            }
        }

        fn validate_age(age: i32) -> Eff<ErrorEffect<Vec<String>>, i32> {
            if age < 0 {
                ErrorEffect::throw(vec!["Age cannot be negative".to_string()])
            } else if age > 150 {
                ErrorEffect::throw(vec!["Age seems unrealistic".to_string()])
            } else {
                Eff::pure(age)
            }
        }

        // Valid case
        let valid_computation =
            validate_name("Alice").flat_map(|name| validate_age(30).fmap(|age| (name, age)));
        let valid_result = handler.clone().run(valid_computation);
        assert_eq!(valid_result, Ok(("Alice".to_string(), 30)));

        // Invalid name
        let invalid_name =
            validate_name("").flat_map(|name| validate_age(30).fmap(|age| (name, age)));
        let invalid_name_result = handler.clone().run(invalid_name);
        assert_eq!(
            invalid_name_result,
            Err(vec!["Name cannot be empty".to_string()])
        );

        // Invalid age
        let invalid_age =
            validate_name("Bob").flat_map(|name| validate_age(-5).fmap(|age| (name, age)));
        let invalid_age_result = handler.run(invalid_age);
        assert_eq!(
            invalid_age_result,
            Err(vec!["Age cannot be negative".to_string()])
        );
    }
}
